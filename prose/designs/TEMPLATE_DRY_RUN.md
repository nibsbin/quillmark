# Template Dry Run Validation

**Status:** Implemented

Dry run provides a lightweight validation path that stops before backend compilation. It is exposed as `Workflow::dry_run(&ParsedDocument)` and mirrored in the WASM bindings (`dryRun(markdown)`).

## What Runs

1. Type coercion against the Quill schema (`ParsedDocument::with_coercion`)
2. JSON Schema validation (`schema::validate_document`)
3. Normalization (bidi stripping, HTML fence fixes)

No plate composition or backend compilation occurs; errors are limited to parsing/validation/normalization.

## Error Surfacing

- Failures return `RenderError::ValidationFailed` with a single `Diagnostic`
- Input size/depth limits and YAML parse errors propagate as `RenderError::InvalidFrontmatter`

## Usage

```rust
let workflow = engine.workflow("my-quill")?;
let parsed = ParsedDocument::from_markdown(markdown)?;
workflow.dry_run(&parsed)?; // Ok(()) on success
```

Bindings:
- **Python**: `workflow.dry_run(parsed)` raises `QuillmarkError` on failure
- **WASM**: `engine.dryRun(markdown)` throws a `WasmError` with diagnostic payload
