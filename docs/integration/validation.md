# Document Validation

Validate documents before full rendering for faster feedback loops.

## Overview

Validation runs against native `QuillConfig` schema rules (no JSON Schema runtime). Two levels are available:

- **Dry run** (Rust/Python): coercion + schema validation only, no compilation cost
- **render()** (all bindings): validation runs as part of the full render pipeline

## Python

```python
from quillmark import Quillmark, ParsedDocument, QuillmarkError

engine = Quillmark()
quill = engine.quill_from_path("./my-quill")
workflow = engine.workflow(quill)

parsed = ParsedDocument.from_markdown(markdown)

try:
    workflow.dry_run(parsed)
    print("Document valid")
except QuillmarkError as e:
    print(f"Validation error: {e}")
```

## JavaScript/WASM

WASM has no separate dry-run method. Call `render()` — validation runs before compilation and errors are thrown before any output is produced.

```javascript
import { ParsedDocument, Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
const quill = engine.quill(tree);
const parsed = ParsedDocument.fromMarkdown(markdown);

try {
    const result = quill.render(parsed, { format: "pdf" });
} catch (e) {
    // e is a WasmError object with diagnostic payload
    console.error(e.message, e.code);
}
```

## CLI

Validate a quill's configuration (not a document):

```sh
quillmark validate ./my-quill
quillmark validate --verbose ./my-quill
```

## Passing schema to LLMs

Python exposes schema as YAML text:

```python
schema_yaml = quill.schema
prompt = f"""Use this schema YAML to generate valid frontmatter:\n\n{schema_yaml}"""
```

## Error shape

Validation errors include field-level context, for example:

- `missing required field 'memo_for'`
- `field 'format' value 'weird' not in allowed set ["standard", "informal", "separate_page"]`

These errors are intended to be fed back into retry loops directly.

> For the full diagnostic type hierarchy and FFI shape, see [prose/designs/ERROR.md](../../prose/designs/ERROR.md).
