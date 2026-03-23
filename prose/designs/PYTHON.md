# Python Bindings

> **Status**: Implemented
> **Package**: `quillmark` (PyPI), Python 3.10+
> **Implementation**: `crates/bindings/python/src/`

## API

### `Quillmark`

```python
engine = Quillmark()
engine.register_quill(quill)
engine.workflow(name)             # by quill name
engine.workflow(parsed)           # from ParsedDocument with QUILL field
engine.workflow_from_quill(quill) # from Quill object
engine.registered_backends()     # list[str]
engine.registered_quills()       # list[str]
```

### `Workflow`

```python
workflow.render(parsed, format=None)  # → RenderResult
workflow.dry_run(parsed)              # raises on validation failure
workflow.backend_id                   # property
workflow.supported_formats            # property
workflow.quill_ref                    # property
# Dynamic assets/fonts:
workflow.add_asset(filename, contents)
workflow.add_assets(assets)
workflow.clear_assets()
workflow.dynamic_asset_names()
workflow.add_font(filename, contents)
workflow.add_fonts(fonts)
workflow.clear_fonts()
workflow.dynamic_font_names()
```

### `Quill`

```python
quill = Quill.from_path("path/to/quill")
quill.name, quill.backend, quill.plate, quill.metadata, quill.schema, quill.example
```

### `ParsedDocument`

```python
parsed = ParsedDocument.from_markdown(markdown)
parsed.body()
parsed.get_field("key")
parsed.fields()
parsed.quill_ref()
```

### `RenderResult`, `Artifact`

```python
result.artifacts          # list[Artifact]
result.warnings           # list[Diagnostic]
result.output_format
artifact.bytes
artifact.output_format
artifact.mime_type
artifact.save(path)
```

### Enums

- `OutputFormat.PDF`, `.SVG`, `.TXT`
- `Severity.ERROR`, `.WARNING`, `.NOTE`

### Exceptions

- `QuillmarkError` (base) → `ParseError`, `TemplateError`, `CompilationError`
- `CompilationError.diagnostics` — list of `SerializableDiagnostic`

## Module Structure

```
crates/bindings/python/src/
├── lib.rs       # PyO3 module entry point
├── engine.rs
├── workflow.rs
├── quill.rs
├── types.rs     # RenderResult, Artifact, Diagnostic
├── enums.rs     # OutputFormat, Severity
└── errors.rs    # Exception definitions and error mapping
```
