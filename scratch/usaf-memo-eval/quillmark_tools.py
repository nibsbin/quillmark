"""Quillmark MCP tool implementations: get_blueprint and create_document."""

from __future__ import annotations

import json
import os
from functools import lru_cache
from pathlib import Path
from typing import Any

from quillmark import Document, Quill, QuillmarkError

DEFAULT_QUILL_PATH = Path(__file__).resolve().parent / "quill"


def quill_path() -> Path:
    override = os.environ.get("QUILLMARK_QUILL_PATH")
    return Path(override) if override else DEFAULT_QUILL_PATH


@lru_cache(maxsize=1)
def load_quill() -> Quill:
    path = quill_path()
    return Quill.from_path(path)


def _severity_name(value: Any) -> str:
    if isinstance(value, str):
        return value.lower()
    name = getattr(value, "name", None)
    if name:
        return str(name).lower()
    return str(value).lower().rsplit(".", 1)[-1]


def diagnostic_to_dict(diag: Any) -> dict[str, Any]:
    if isinstance(diag, dict):
        out = dict(diag)
        out["severity"] = _severity_name(out.get("severity", "error"))
        if "pretty" not in out:
            out["pretty"] = _pretty_from_dict(out)
        return out

    loc = getattr(diag, "location", None)
    location = None
    if loc is not None:
        location = {
            "file": loc.file,
            "line": loc.line,
            "column": loc.column,
        }
    out = {
        "severity": _severity_name(diag.severity),
        "code": diag.code,
        "message": diag.message,
        "path": diag.path,
        "hint": diag.hint,
        "location": location,
        "args": dict(diag.args) if getattr(diag, "args", None) else {},
        "source_chain": list(getattr(diag, "source_chain", []) or []),
        "pretty": str(diag),
    }
    return out


def _pretty_from_dict(d: dict[str, Any]) -> str:
    sev = str(d.get("severity", "error")).upper()
    msg = d.get("message") or ""
    code = d.get("code")
    line = f"[{sev}] {msg}"
    if code:
        line += f" ({code})"
    loc = d.get("location") or {}
    if loc.get("file"):
        line += f"\n  --> {loc.get('file')}:{loc.get('line')}:{loc.get('column')}"
    if d.get("path"):
        line += f"\n  at {d['path']}"
    if d.get("hint"):
        line += f"\n  hint: {d['hint']}"
    return line


def _is_blocking(diag: dict[str, Any]) -> bool:
    if diag.get("severity") == "error":
        return True
    code = diag.get("code") or ""
    return code == "validation::must_fill"


def get_blueprint() -> dict[str, Any]:
    """Return the quill's instruction header, format rules, and annotated blueprint."""
    quill = load_quill()
    name = quill.name if hasattr(quill, "name") else "usaf_memo"
    # Quill.quill_ref is the canonical name@version.
    ref = quill.quill_ref
    quill_name = ref.split("@", 1)[0]
    return {
        "quill_ref": ref,
        "instruction": Document.blueprint_instruction(quill_name),
        "format_rules": Document.format_rules(),
        "quill_ref_hint": Document.quill_ref_hint(),
        "blueprint": quill.blueprint,
    }


def create_document(content: str) -> dict[str, Any]:
    """Parse and validate markdown. Returns errors so the caller can revise and retry."""
    quill = load_quill()
    if not isinstance(content, str) or not content.strip():
        return {
            "ok": False,
            "stage": "input",
            "quill_ref": quill.quill_ref,
            "diagnostics": [
                {
                    "severity": "error",
                    "code": "mcp::empty_content",
                    "message": "create_document requires a non-empty `content` markdown string.",
                    "path": None,
                    "hint": "Pass the filled blueprint markdown as `content`.",
                    "location": None,
                    "args": {},
                    "pretty": "[ERROR] create_document requires a non-empty `content` markdown string.",
                }
            ],
            "pretty": "[ERROR] create_document requires a non-empty `content` markdown string.",
            "retry": True,
        }

    try:
        doc = quill.parse(content)
    except QuillmarkError as exc:
        diags = [diagnostic_to_dict(d) for d in exc.diagnostics]
        pretty = "\n\n".join(d["pretty"] for d in diags)
        return {
            "ok": False,
            "stage": "parse",
            "quill_ref": quill.quill_ref,
            "diagnostics": diags,
            "pretty": pretty,
            "retry": True,
        }

    parse_warnings = [diagnostic_to_dict(d) for d in doc.warnings]
    validation = [diagnostic_to_dict(d) for d in quill.validate(doc)]
    diagnostics = parse_warnings + validation
    blocking = [d for d in diagnostics if _is_blocking(d)]
    warnings = [d for d in diagnostics if d not in blocking]

    if blocking:
        pretty = "\n\n".join(d["pretty"] for d in blocking)
        return {
            "ok": False,
            "stage": "validate",
            "quill_ref": quill.quill_ref,
            "diagnostics": blocking,
            "warnings": warnings,
            "pretty": pretty,
            "emitted_markdown": doc.to_markdown(),
            "card_count": doc.card_count,
            "retry": True,
        }

    return {
        "ok": True,
        "stage": "ok",
        "quill_ref": quill.quill_ref,
        "warnings": warnings,
        "card_count": doc.card_count,
        "markdown": doc.to_markdown(),
        "retry": False,
    }


OPENAI_TOOLS = [
    {
        "type": "function",
        "name": "get_blueprint",
        "description": (
            "Return the usaf_memo authoring packet: instruction header, document "
            "format rules, $quill reference grammar, and the annotated Markdown "
            "blueprint. Call this before writing a document."
        ),
        "parameters": {"type": "object", "properties": {}, "additionalProperties": False},
    },
    {
        "type": "function",
        "name": "create_document",
        "description": (
            "Parse and validate a filled usaf_memo Markdown document. On failure, "
            "returns structured diagnostics (code, path, hint, pretty text) so you "
            "can revise `content` and call this tool again. On success, ok is true."
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The full filled-in Markdown document, including ~~~ card blocks and body prose.",
                }
            },
            "required": ["content"],
            "additionalProperties": False,
        },
    },
]


def dispatch_tool(name: str, arguments: dict[str, Any] | str | None) -> dict[str, Any]:
    if isinstance(arguments, str):
        try:
            arguments = json.loads(arguments) if arguments.strip() else {}
        except json.JSONDecodeError as exc:
            return {
                "ok": False,
                "stage": "tool",
                "diagnostics": [
                    {
                        "severity": "error",
                        "code": "mcp::invalid_arguments",
                        "message": f"Tool arguments were not valid JSON: {exc}",
                        "hint": "Pass a JSON object. create_document needs {\"content\": \"...markdown...\"}.",
                        "pretty": f"[ERROR] Tool arguments were not valid JSON: {exc}",
                    }
                ],
                "pretty": f"[ERROR] Tool arguments were not valid JSON: {exc}",
                "retry": True,
            }
    arguments = arguments or {}
    if name == "get_blueprint":
        return get_blueprint()
    if name == "create_document":
        return create_document(arguments.get("content", ""))
    return {
        "ok": False,
        "stage": "tool",
        "diagnostics": [
            {
                "severity": "error",
                "code": "mcp::unknown_tool",
                "message": f"Unknown tool: {name}",
                "pretty": f"[ERROR] Unknown tool: {name}",
            }
        ],
        "pretty": f"[ERROR] Unknown tool: {name}",
        "retry": True,
    }
