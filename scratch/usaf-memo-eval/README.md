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
python scratch/usaf-memo-eval/run_eval.py --budget 18 --workers 12
```

Writes `dataset/traces.jsonl` (one trial per line) and `dataset/summary.json`.

Models: latest low/medium OpenAI (`gpt-5.6-luna`, `gpt-5.6-terra`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5-mini`, `gpt-5-nano`, `gpt-4.1-mini`, `gpt-4.1-nano`).

## Dataset snapshot (this branch)

5189 trials, **$17.78** API spend, 85.4% eventually valid.

| model | n | success | cost |
|---|---:|---:|---:|
| gpt-5.6-terra | 359 | 99.4% | $5.28 |
| gpt-5.4-mini | 606 | 99.8% | $3.75 |
| gpt-5-mini | 605 | 99.8% | $1.87 |
| gpt-4.1-mini | 604 | 52.5% | $1.79 |
| gpt-5.4-nano | 606 | 97.0% | $1.40 |
| gpt-4.1-nano | 592 | 34.5% | $1.37 |
| gpt-5.6-luna (medium) | 606 | 99.8% | $0.92 |
| gpt-5.6-luna (low) | 606 | 99.8% | $0.78 |
| gpt-5-nano | 605 | 90.7% | $0.62 |

Hardest task families: CUI (76%), SEE DISTRIBUTION (79%), long body / body format / MFR (~80%). Dominant retry diagnostics: `parse::missing_quill`, `parse::yaml_error_with_location`, `validation::must_fill`. Almost every trial called `get_blueprint` first; 4.1-nano accounts for most failed retries and token burn on the cheap tier.
