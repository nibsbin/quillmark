#!/usr/bin/env python3
"""OpenAI function-calling eval against the Quillmark get_blueprint / create_document tools."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import threading
import time
import traceback
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from openai import OpenAI
from openai import APIStatusError, RateLimitError

from quillmark_tools import OPENAI_TOOLS, dispatch_tool
from tasks import TASKS, Task

ROOT = Path(__file__).resolve().parent
DATASET = ROOT / "dataset"
TRACES = DATASET / "traces.jsonl"
SUMMARY = DATASET / "summary.json"
PROGRESS = DATASET / "progress.json"

SYSTEM = """You author official U.S. Air Force and Space Force memoranda with the usaf_memo@0.3.0 quill.

Workflow:
1. Call get_blueprint to retrieve the instruction header, format rules, $quill grammar, and the annotated Markdown blueprint.
2. Fill that blueprint for the user's task. Replace every `!must_fill` placeholder with a real value and drop the tag. Keep `$quill: usaf_memo@0.3.0`. Edit the body prose. Delete blueprint cards you do not need (for example the sample indorsement) rather than shipping leftover placeholders.
3. Submit the filled markdown as `content` to create_document.
4. If create_document returns ok=false, read every diagnostic (code, path, hint, pretty text) and revise the markdown, then call create_document again.
5. Stop when create_document returns ok=true.

Do not invent a different document format. The blueprint is the document shape. Quote YAML scalars that contain `: ` or that start with `*` or `&`. Numbers such as font_size must be unquoted. Dates are YYYY-MM-DD."""

# Latest low-to-medium OpenAI models as of 2026-09. Prices are USD per 1M tokens.
MODELS: list[dict[str, Any]] = [
    {
        "id": "gpt-5.6-luna",
        "input": 0.20,
        "cached": 0.02,
        "output": 1.20,
        "share": 0.30,
        "reasoning": "low",
        "tier": "low",
    },
    {
        "id": "gpt-5.6-luna",
        "input": 0.20,
        "cached": 0.02,
        "output": 1.20,
        "share": 0.08,
        "reasoning": "medium",
        "tier": "low",
        "label": "gpt-5.6-luna:medium",
    },
    {
        "id": "gpt-5.4-mini",
        "input": 0.75,
        "cached": 0.075,
        "output": 4.50,
        "share": 0.16,
        "reasoning": "low",
        "tier": "medium",
    },
    {
        "id": "gpt-5.4-nano",
        "input": 0.20,
        "cached": 0.02,
        "output": 1.25,
        "share": 0.12,
        "reasoning": "low",
        "tier": "low",
    },
    {
        "id": "gpt-5-mini",
        "input": 0.25,
        "cached": 0.025,
        "output": 2.00,
        "share": 0.14,
        "reasoning": "low",
        "tier": "medium",
    },
    {
        "id": "gpt-5-nano",
        "input": 0.05,
        "cached": 0.005,
        "output": 0.40,
        "share": 0.10,
        "reasoning": "low",
        "tier": "low",
    },
    {
        "id": "gpt-4.1-mini",
        "input": 0.40,
        "cached": 0.10,
        "output": 1.60,
        "share": 0.10,
        "reasoning": None,
        "tier": "medium",
    },
]

STATE = {
    "spent": 0.0,
    "traces": 0,
    "ok": 0,
    "fail": 0,
    "stop": False,
    "skipped_models": set(),
    "per_model_spent": {},
}
LOCK = threading.Lock()
FILE_LOCK = threading.Lock()
COMMIT_LOCK = threading.Lock()
LAST_COMMIT = 0.0


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def model_label(spec: dict[str, Any]) -> str:
    return spec.get("label") or spec["id"]


def cost_of(spec: dict[str, Any], usage: dict[str, Any]) -> float:
    inp = int(usage.get("input_tokens") or 0)
    out = int(usage.get("output_tokens") or 0)
    cached = int(usage.get("cached_tokens") or 0)
    uncached = max(inp - cached, 0)
    return (
        uncached * spec["input"]
        + cached * spec["cached"]
        + out * spec["output"]
    ) / 1_000_000.0


def usage_from_response(resp: Any) -> dict[str, Any]:
    u = getattr(resp, "usage", None)
    if u is None:
        return {}
    cached = 0
    reasoning = 0
    itd = getattr(u, "input_tokens_details", None)
    otd = getattr(u, "output_tokens_details", None)
    if itd is not None:
        cached = int(getattr(itd, "cached_tokens", 0) or 0)
    if otd is not None:
        reasoning = int(getattr(otd, "reasoning_tokens", 0) or 0)
    return {
        "input_tokens": int(getattr(u, "input_tokens", 0) or 0),
        "output_tokens": int(getattr(u, "output_tokens", 0) or 0),
        "total_tokens": int(getattr(u, "total_tokens", 0) or 0),
        "cached_tokens": cached,
        "reasoning_tokens": reasoning,
    }


def dump_item(item: Any) -> dict[str, Any]:
    if hasattr(item, "model_dump"):
        data = item.model_dump()
        # Keep traces from exploding on huge binary blobs.
        return json.loads(json.dumps(data, default=str))
    return {"type": getattr(item, "type", None), "repr": repr(item)}


def append_jsonl(path: Path, row: dict[str, Any]) -> None:
    line = json.dumps(row, ensure_ascii=False, default=str)
    with FILE_LOCK:
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a", encoding="utf-8") as f:
            f.write(line + "\n")
            f.flush()


def write_json(path: Path, obj: Any) -> None:
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(obj, indent=2, ensure_ascii=False, default=str) + "\n", encoding="utf-8")
    tmp.replace(path)


def load_done_keys() -> set[tuple[str, str, int]]:
    done: set[tuple[str, str, int]] = set()
    if not TRACES.exists():
        return done
    with TRACES.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            key = (row.get("model_label"), row.get("task_id"), int(row.get("repeat") or 0))
            if key[0] and key[1]:
                done.add(key)
    return done


def summarize() -> dict[str, Any]:
    per_model: dict[str, dict[str, Any]] = {}
    codes: dict[str, int] = {}
    categories: dict[str, dict[str, int]] = {}
    total_spent = 0.0
    n = 0
    ok = 0
    if TRACES.exists():
        with TRACES.open(encoding="utf-8") as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                n += 1
                total_spent += float(row.get("cost_usd") or 0)
                label = row.get("model_label") or row.get("model") or "unknown"
                cat = row.get("category") or "unknown"
                success = bool(row.get("success"))
                if success:
                    ok += 1
                bucket = per_model.setdefault(
                    label,
                    {
                        "n": 0,
                        "ok": 0,
                        "cost_usd": 0.0,
                        "input_tokens": 0,
                        "output_tokens": 0,
                        "reasoning_tokens": 0,
                        "create_document_attempts": 0,
                        "got_blueprint": 0,
                    },
                )
                bucket["n"] += 1
                bucket["ok"] += int(success)
                bucket["cost_usd"] += float(row.get("cost_usd") or 0)
                usage = row.get("usage_total") or {}
                bucket["input_tokens"] += int(usage.get("input_tokens") or 0)
                bucket["output_tokens"] += int(usage.get("output_tokens") or 0)
                bucket["reasoning_tokens"] += int(usage.get("reasoning_tokens") or 0)
                bucket["create_document_attempts"] += int(row.get("create_document_attempts") or 0)
                bucket["got_blueprint"] += int(bool(row.get("called_get_blueprint")))
                cb = categories.setdefault(cat, {"n": 0, "ok": 0})
                cb["n"] += 1
                cb["ok"] += int(success)
                for code in row.get("diagnostic_codes") or []:
                    codes[code] = codes.get(code, 0) + 1
    out = {
        "updated_at": utc_now(),
        "traces": n,
        "successes": ok,
        "success_rate": (ok / n) if n else 0.0,
        "cost_usd": total_spent,
        "per_model": per_model,
        "per_category": categories,
        "diagnostic_codes": dict(sorted(codes.items(), key=lambda kv: (-kv[1], kv[0]))),
    }
    write_json(SUMMARY, out)
    return out


def git_commit_push(message: str) -> None:
    global LAST_COMMIT
    with COMMIT_LOCK:
        try:
            subprocess.run(["git", "add", "scratch/usaf-memo-eval"], cwd="/agent/repos/quillmark", check=True)
            st = subprocess.run(
                ["git", "status", "--porcelain", "--", "scratch/usaf-memo-eval"],
                cwd="/agent/repos/quillmark",
                check=True,
                capture_output=True,
                text=True,
            )
            if not st.stdout.strip():
                return
            subprocess.run(["git", "commit", "-m", message], cwd="/agent/repos/quillmark", check=True)
            subprocess.run(
                ["git", "push", "-u", "origin", "HEAD"],
                cwd="/agent/repos/quillmark",
                check=True,
            )
            LAST_COMMIT = time.time()
        except subprocess.CalledProcessError as exc:
            print(f"git commit/push failed: {exc}", file=sys.stderr)


def maybe_commit(force: bool = False) -> None:
    if not force and time.time() - LAST_COMMIT < 90:
        return
    with LOCK:
        n = STATE["traces"]
        spent = STATE["spent"]
    git_commit_push(f"scratch: usaf_memo eval traces n={n} spent=${spent:.2f}")


def call_model(client: OpenAI, spec: dict[str, Any], input_list: list[Any], instructions: str) -> Any:
    kwargs: dict[str, Any] = {
        "model": spec["id"],
        "tools": OPENAI_TOOLS,
        "input": input_list,
        "instructions": instructions,
        "store": False,
    }
    if spec.get("reasoning"):
        kwargs["reasoning"] = {"effort": spec["reasoning"]}
    last_err: Exception | None = None
    for attempt in range(6):
        try:
            return client.responses.create(**kwargs)
        except RateLimitError as exc:
            last_err = exc
            time.sleep(min(2 ** attempt, 32))
        except APIStatusError as exc:
            last_err = exc
            body = ""
            try:
                body = str(exc.body) if getattr(exc, "body", None) else str(exc)
            except Exception:
                body = str(exc)
            lowered = body.lower()
            if spec.get("reasoning") and "reasoning" in lowered and exc.status_code in (400, 422):
                kwargs.pop("reasoning", None)
                spec = dict(spec)
                spec["reasoning"] = None
                continue
            if exc.status_code in (429, 500, 502, 503, 529):
                time.sleep(min(2 ** attempt, 32))
                continue
            raise
    assert last_err is not None
    raise last_err


def run_trial(
    client: OpenAI,
    spec: dict[str, Any],
    task: Task,
    repeat: int,
    max_rounds: int,
    budget: float,
) -> dict[str, Any]:
    trial_id = str(uuid.uuid4())
    started = time.time()
    rounds: list[dict[str, Any]] = []
    usage_total = {
        "input_tokens": 0,
        "output_tokens": 0,
        "total_tokens": 0,
        "cached_tokens": 0,
        "reasoning_tokens": 0,
        "cost_usd": 0.0,
    }
    called_get_blueprint = False
    create_attempts = 0
    success = False
    stop_reason = "max_rounds"
    last_create: dict[str, Any] | None = None
    diagnostic_codes: list[str] = []
    final_markdown: str | None = None
    error: str | None = None

    input_list: list[Any] = [{"role": "user", "content": task["prompt"]}]

    try:
        for rnd in range(max_rounds):
            with LOCK:
                if STATE["stop"] or STATE["spent"] >= budget:
                    stop_reason = "budget"
                    break
            resp = call_model(client, spec, input_list, SYSTEM)
            usage = usage_from_response(resp)
            round_cost = cost_of(spec, usage)
            with LOCK:
                STATE["spent"] += round_cost
                label = model_label(spec)
                STATE["per_model_spent"][label] = STATE["per_model_spent"].get(label, 0.0) + round_cost
                if STATE["spent"] >= budget:
                    STATE["stop"] = True
            for k in ("input_tokens", "output_tokens", "total_tokens", "cached_tokens", "reasoning_tokens"):
                usage_total[k] += int(usage.get(k) or 0)
            usage_total["cost_usd"] += round_cost

            output = list(getattr(resp, "output", []) or [])
            round_row: dict[str, Any] = {
                "round": rnd,
                "usage": usage,
                "cost_usd": round_cost,
                "output": [dump_item(item) for item in output],
                "tool_calls": [],
            }

            function_calls = [item for item in output if getattr(item, "type", None) == "function_call"]
            if not function_calls:
                stop_reason = "no_tool_call"
                text = getattr(resp, "output_text", None)
                round_row["output_text"] = text
                rounds.append(round_row)
                break

            input_list = input_list + output
            for item in function_calls:
                name = item.name
                raw_args = item.arguments
                result = dispatch_tool(name, raw_args)
                if name == "get_blueprint":
                    called_get_blueprint = True
                if name == "create_document":
                    create_attempts += 1
                    last_create = result
                    for d in result.get("diagnostics") or []:
                        code = d.get("code")
                        if code:
                            diagnostic_codes.append(code)
                    if result.get("ok"):
                        success = True
                        stop_reason = "success"
                        final_markdown = result.get("markdown")
                    else:
                        final_markdown = result.get("emitted_markdown")
                round_row["tool_calls"].append(
                    {
                        "name": name,
                        "arguments": raw_args if name != "create_document" else "<omitted in index; see result>",
                        "arguments_len": len(raw_args or "") if isinstance(raw_args, str) else None,
                        "result_ok": result.get("ok"),
                        "result_stage": result.get("stage"),
                        "result": result if name != "get_blueprint" else {
                            "quill_ref": result.get("quill_ref"),
                            "instruction": result.get("instruction"),
                            "blueprint_chars": len(result.get("blueprint") or ""),
                        },
                    }
                )
                # Keep full create_document content in the trace via submitted_markdown.
                if name == "create_document":
                    content = None
                    if isinstance(raw_args, str):
                        try:
                            content = json.loads(raw_args).get("content")
                        except json.JSONDecodeError:
                            content = raw_args
                    elif isinstance(raw_args, dict):
                        content = raw_args.get("content")
                    round_row["tool_calls"][-1]["submitted_markdown"] = content
                    round_row["tool_calls"][-1]["result"] = result

                input_list.append(
                    {
                        "type": "function_call_output",
                        "call_id": item.call_id,
                        "output": json.dumps(result, ensure_ascii=False, default=str),
                    }
                )
            rounds.append(round_row)
            if success:
                break
        else:
            if not success and stop_reason == "max_rounds":
                stop_reason = "max_rounds"
    except APIStatusError as exc:
        error = f"APIStatusError {exc.status_code}: {exc}"
        stop_reason = "api_error"
        if exc.status_code in (404, 400) and "model" in str(exc).lower():
            with LOCK:
                STATE["skipped_models"].add(spec["id"])
    except Exception as exc:
        error = f"{type(exc).__name__}: {exc}"
        stop_reason = "exception"
        rounds.append({"exception": traceback.format_exc()})

    row = {
        "trial_id": trial_id,
        "ts": utc_now(),
        "elapsed_s": round(time.time() - started, 3),
        "model": spec["id"],
        "model_label": model_label(spec),
        "reasoning": spec.get("reasoning"),
        "tier": spec.get("tier"),
        "task_id": task["id"],
        "category": task["category"],
        "difficulty": task.get("difficulty"),
        "traps": task.get("traps") or [],
        "expect_indorsement": task.get("expect_indorsement"),
        "repeat": repeat,
        "success": success,
        "stop_reason": stop_reason,
        "called_get_blueprint": called_get_blueprint,
        "create_document_attempts": create_attempts,
        "diagnostic_codes": diagnostic_codes,
        "last_create_ok": None if last_create is None else bool(last_create.get("ok")),
        "last_create_stage": None if last_create is None else last_create.get("stage"),
        "final_markdown": final_markdown,
        "usage_total": usage_total,
        "cost_usd": usage_total["cost_usd"],
        "rounds": rounds,
        "error": error,
        "prompt": task["prompt"],
    }
    append_jsonl(TRACES, row)
    with LOCK:
        STATE["traces"] += 1
        if success:
            STATE["ok"] += 1
        else:
            STATE["fail"] += 1
        STATE["spent"] = max(STATE["spent"], 0.0)
        progress = {
            "updated_at": utc_now(),
            "spent": STATE["spent"],
            "traces": STATE["traces"],
            "ok": STATE["ok"],
            "fail": STATE["fail"],
            "skipped_models": sorted(STATE["skipped_models"]),
        }
    write_json(PROGRESS, progress)
    return row


def plan_jobs(repeats: int, done: set[tuple[str, str, int]]) -> list[tuple[dict[str, Any], Task, int]]:
    streams: list[list[tuple[dict[str, Any], Task, int]]] = []
    for spec in MODELS:
        stream: list[tuple[dict[str, Any], Task, int]] = []
        label = model_label(spec)
        for r in range(max(repeats, 1)):
            for task in TASKS:
                if (label, task["id"], r) not in done:
                    stream.append((spec, task, r))
        streams.append(stream)
    jobs: list[tuple[dict[str, Any], Task, int]] = []
    maxlen = max((len(s) for s in streams), default=0)
    for i in range(maxlen):
        for stream in streams:
            if i < len(stream):
                jobs.append(stream[i])
    return jobs


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--budget", type=float, default=18.0)
    parser.add_argument("--max-rounds", type=int, default=8)
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--no-commit", action="store_true")
    parser.add_argument("--smoke", action="store_true", help="One cheap trial then exit")
    args = parser.parse_args()

    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        sys.exit("OPENAI_API_KEY is not set")

    DATASET.mkdir(parents=True, exist_ok=True)
    client = OpenAI(api_key=api_key, timeout=180.0)

    if args.smoke:
        spec = next(m for m in MODELS if m["id"] == "gpt-5-nano")
        row = run_trial(client, spec, TASKS[0], 0, max_rounds=4, budget=args.budget)
        summarize()
        print(json.dumps({k: row[k] for k in ("success", "stop_reason", "cost_usd", "model", "task_id")}, indent=2))
        return

    done = load_done_keys()
    if TRACES.exists():
        # Reconstruct spent from existing traces so reruns don't overshoot.
        with TRACES.open(encoding="utf-8") as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                with LOCK:
                    STATE["spent"] += float(row.get("cost_usd") or 0)
                    STATE["traces"] += 1
                    if row.get("success"):
                        STATE["ok"] += 1
                    else:
                        STATE["fail"] += 1
                    lab = row.get("model_label") or row.get("model")
                    if lab:
                        STATE["per_model_spent"][lab] = STATE["per_model_spent"].get(lab, 0.0) + float(
                            row.get("cost_usd") or 0
                        )

    jobs = plan_jobs(args.repeats, done)
    print(
        f"planned {len(jobs)} jobs, already done {len(done)}, spent ${STATE['spent']:.3f}, budget ${args.budget:.2f}",
        flush=True,
    )

    def worker(job: tuple[dict[str, Any], Task, int]) -> str:
        spec, task, repeat = job
        with LOCK:
            if STATE["stop"] or STATE["spent"] >= args.budget:
                return "skipped-budget"
            if spec["id"] in STATE["skipped_models"]:
                return "skipped-model"
        row = run_trial(client, spec, task, repeat, args.max_rounds, args.budget)
        if not args.no_commit:
            maybe_commit(force=False)
        return f"{row['model_label']} {row['task_id']} r{repeat} success={row['success']} ${row['cost_usd']:.4f} {row['stop_reason']}"

    try:
        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            futs = [pool.submit(worker, job) for job in jobs]
            for fut in as_completed(futs):
                try:
                    msg = fut.result()
                except Exception as exc:
                    msg = f"worker crashed: {exc}"
                print(msg, flush=True)
                with LOCK:
                    if STATE["stop"] or STATE["spent"] >= args.budget:
                        # Let in-flight finish; remaining queued jobs self-skip.
                        pass
    finally:
        summarize()
        if not args.no_commit:
            maybe_commit(force=True)
        print(json.dumps(summarize(), indent=2), flush=True)


if __name__ == "__main__":
    main()
