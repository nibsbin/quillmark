# Template Dry Run Validation

**Status:** Implemented

Dry run provides a lightweight validation path that stops before backend compilation. It is exposed as `Workflow::dry_run(&ParsedDocument)` on the Rust/Python side. WASM does not expose a separate dry-run entry point; WASM callers use `quill.render()` which includes full validation.

## What Runs

1. Type coercion via `QuillConfig::coerce`
2. Native validation via `QuillConfig::validate`

No normalization, plate composition, or backend compilation occurs. Errors are limited to field coercion and schema validation.

## Error Surfacing

- Failures return `RenderError::ValidationFailed` with a single `Diagnostic`
- Input size/depth limits and YAML parse errors propagate as `RenderError::InvalidFrontmatter`

## Usage

```rust
let quill = engine.quill_from_path("./my-quill")?;
let workflow = engine.workflow(&quill)?;
let parsed = ParsedDocument::from_markdown(markdown)?;
workflow.dry_run(&parsed)?; // Ok(()) on success
```

Bindings:
- **Python**: `workflow.dry_run(parsed)` raises `QuillmarkError` on failure
- **WASM**: no `dryRun` method; use `quill.render()` which validates before compiling
- **CLI**: `quillmark validate <quill-path>` validates a quill's configuration (not a document)
