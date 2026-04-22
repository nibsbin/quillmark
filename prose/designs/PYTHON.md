# Python Bindings

> **Status**: Implemented
> **Package**: `quillmark` (PyPI), Python 3.10+
> **Implementation**: `crates/bindings/python/src/`

## API

### `Quillmark`

```python
engine = Quillmark()
engine.quill_from_path(path)      # → Quill (load quill, attach backend)
engine.workflow(quill)            # → Workflow
engine.registered_backends()     # → list[str]
```

### `Quill`

Obtained via `engine.quill_from_path(path)`.

```python
quill.name, quill.backend, quill.plate, quill.metadata, quill.schema
quill.defaults          # dict of field defaults
quill.examples          # dict of field example lists
quill.example           # optional raw example string
quill.print_tree        # file tree string
quill.supported_formats()         # → list[OutputFormat]
quill.render(parsed, format=None) # → RenderResult
quill.open(parsed)                # → RenderSession
```

### `Workflow`

```python
workflow.render(parsed, format=None)  # → RenderResult
workflow.open(parsed)                 # → RenderSession
workflow.dry_run(parsed)              # raises on validation failure
workflow.backend_id                   # property
workflow.supported_formats            # property
workflow.quill_ref                    # property
```

### `RenderSession`

Obtained via `quill.open(parsed)` or `workflow.open(parsed)`. Allows inspecting before rendering.

```python
session.page_count                          # property
session.render(format=None, pages=None)     # → RenderResult
```

### `ParsedDocument`

```python
parsed = ParsedDocument.from_markdown(markdown)
parsed.body()
parsed.get_field("key")
parsed.fields           # property → dict
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

### `Diagnostic`, `Location`

```python
diag.severity        # Severity enum
diag.message         # str
diag.code            # optional str
diag.primary         # optional Location
diag.hint            # optional str
diag.source_chain    # list[str]

loc.file, loc.line, loc.col
```

### Enums

- `OutputFormat.PDF`, `.SVG`, `.TXT`, `.PNG`
- `Severity.ERROR`, `.WARNING`, `.NOTE`
- Both enums expose `.name` property and `.all()` static method.

### Exceptions

- `QuillmarkError` (base) → `ParseError`, `TemplateError`, `CompilationError`
- `CompilationError.diagnostics` — list of `Diagnostic`
- `ParseError.diagnostic` — single `Diagnostic`

## Module Structure

```
crates/bindings/python/src/
├── lib.rs       # PyO3 module entry point; registers all classes/enums/exceptions
├── types.rs     # All pyclass wrappers: Quillmark, Workflow, Quill, ParsedDocument,
│                #   RenderResult, RenderSession, Artifact, Diagnostic, Location
├── enums.rs     # OutputFormat, Severity
└── errors.rs    # Exception definitions and error mapping
```
