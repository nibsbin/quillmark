# Document Validation

Validate documents before full rendering for faster feedback loops.

## Overview

Validation runs against native `QuillConfig` schema rules (no JSON Schema runtime).

- Parse markdown to `ParsedDocument`
- Resolve workflow from required `QUILL`
- Run `dry_run()` for parse + validation + coercion checks

## Python

```python
from quillmark import Quillmark, Quill, ParsedDocument, QuillmarkError

engine = Quillmark()
quill = Quill.from_path("./my-quill")
workflow = engine.workflow(quill)

parsed = ParsedDocument.from_markdown(markdown)

try:
    workflow.dry_run(parsed)
    print("✓ Document valid")
except QuillmarkError as e:
    print(f"✗ Validation error: {e}")
```

## JavaScript/WASM

```javascript
import { Quill, Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
const enc = new TextEncoder();
const quill = Quill.fromTree(new Map([
  ["Quill.yaml", enc.encode(quillYamlString)],
  ["plate.typ", enc.encode(plateTypString)],
]));
engine.registerQuill(quill);
const parsed = Quillmark.parseMarkdown(markdown);

// dryRun uses QUILL from parsed frontmatter
engine.dryRun(markdown);
```

## Passing schema to LLMs

Schema is exposed as YAML text:

- Python: `quill.schema` (`str`)
- WASM: `quillInfo.schema` (`string`) or `getQuillSchema(name)`

```python
schema_yaml = quill.schema
prompt = f"""Use this schema YAML to generate valid frontmatter:\n\n{schema_yaml}"""
```

## LLM retry loop

```python
def generate_with_retries(prompt: str, quill_path: str, attempts: int = 3):
    engine = Quillmark()
    quill = Quill.from_path(quill_path)
    workflow = engine.workflow(quill)
    schema_yaml = quill.schema

    llm_prompt = f"{prompt}\n\nSchema YAML:\n{schema_yaml}"

    for _ in range(attempts):
        markdown = call_llm(llm_prompt)
        parsed = ParsedDocument.from_markdown(markdown)
        try:
            workflow.dry_run(parsed)
            return workflow.render(parsed)
        except Exception as e:
            llm_prompt = f"{llm_prompt}\n\nPrevious validation error:\n{e}"

    raise RuntimeError("Failed to generate valid document")
```

## Error shape

Validation errors are path-aware and include field-level context, for example:

- `missing required field 'memo_for'`
- `field 'format' value 'weird' not in allowed set ["standard", "informal", "separate_page"]`

These errors are intended to be fed back into retry loops directly.
