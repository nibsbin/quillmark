# Throwaway usaf_memo eval harness

Scratch MCP server plus OpenAI eval for `usaf_memo@0.3.0`. Not product code.

## Tools

`get_blueprint` returns Quillmark's instruction header, format rules, `$quill` grammar, and the annotated Markdown blueprint.

`create_document` parses with `Quill.parse` and validates with `Quill.validate`. Parse failures, validation errors, and leftover `validation::must_fill` markers return `ok: false` with structured diagnostics so the caller can revise and retry.

## MCP server

```bash
export QUILLMARK_QUILL_PATH=/absolute/path/to/usaf_memo/0.3.0   # optional; defaults to ./quill
python scratch/usaf-memo-eval/mcp_server.py
```

The vendored `quill/` directory contains only `Quill.yaml` (enough for blueprint / parse / validate).

## Eval

```bash
export OPENAI_API_KEY=...
python scratch/usaf-memo-eval/run_eval.py --budget 18 --workers 6
```

Writes `dataset/traces.jsonl` (one trial per line) and `dataset/summary.json`.

Models: latest low/medium OpenAI (`gpt-5.6-luna`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5-mini`, `gpt-5-nano`, `gpt-4.1-mini`).
