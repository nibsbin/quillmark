# Document Validation

Validate documents before full rendering for faster feedback loops.

## Overview

Validation runs against native `QuillConfig` schema rules (no JSON Schema runtime).

- Parse markdown to `ParsedDocument`
- Render-ready quill via `engine.quill_from_path(...)` / `engine.quillFromTree(...)`
- Optional `workflow.dry_run()` for fast validation without compilation (Python)

## Python

```python
from quillmark import Quillmark, ParsedDocument, QuillmarkError

engine = Quillmark()
quill = engine.quill_from_path("./my-quill")
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
import { ParsedDocument, Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
const quill = engine.quillFromTree(tree);
const parsed = ParsedDocument.fromMarkdown(markdown);

// render() performs validation + compilation
const result = quill.render(parsed, { format: "pdf" });
```

## Passing schema to LLMs

Python exposes schema as YAML text:

```python
schema_yaml = quill.schema
prompt = f"""Use this schema YAML to generate valid frontmatter:\n\n{schema_yaml}"""
```

## Error shape

Validation errors are path-aware and include field-level context, for example:

- `missing required field 'memo_for'`
- `field 'format' value 'weird' not in allowed set ["standard", "informal", "separate_page"]`

These errors are intended to be fed back into retry loops directly.
