# Quillmark Architecture

## System Overview

Quillmark converts Markdown with YAML frontmatter into output artifacts (PDF, SVG, TXT). Data flow:

1. **Parse** — YAML/frontmatter extraction, CARD aggregation, bidi stripping, HTML fence normalization
2. **Normalize** — Type coercion, defaults from Quill schema, backend `transform_fields`
3. **Compile** — Backend processes plate content with injected JSON data

## Crate Structure

### `quillmark-core`

Types: `Backend`, `Artifact`, `OutputFormat`, `ParsedDocument`, `Quill`, `RenderError`, `Diagnostic`, `Severity`, `Location`, `QuillValue`.

No external backend dependencies; backends depend on core.

### `quillmark` (orchestration)

High-level API: `Quillmark` (engine), `Workflow` (pipeline). Handles parse → compose → compile, schema coercion, backend auto-registration, and default Quill registration.

### `backends/quillmark-typst`

Implements `Backend` for PDF/SVG. Markdown→Typst via `transform_fields`. Injects JSON as `@local/quillmark-helper` package. Resolves fonts and assets. See [GLUE_METADATA.md](GLUE_METADATA.md).

### `bindings/quillmark-python`

PyO3 bindings published as `quillmark` on PyPI. See [PYTHON.md](PYTHON.md).

### `bindings/quillmark-wasm`

wasm-bindgen bindings published as `@quillmark-test/wasm`. Supports bundler, Node.js, and web targets. See [WASM.md](WASM.md).

### `bindings/quillmark-cli`

Standalone binary. See [CLI.md](CLI.md).

### `quillmark-fixtures`

Test resources under `resources/`. Helper functions for test setup.

### `quillmark-fuzz`

Fuzz tests for parsing, templating, and rendering.

## Core Interfaces

- **`Quillmark`** — Engine managing backends and quills
- **`Workflow`** — Rendering pipeline (parse → template → compile)
- **`Backend`** — Trait for output formats (`Send + Sync`)
- **`Quill`** — Template bundle (plate + assets/packages)
- **`ParsedDocument`** — Frontmatter + body from markdown
- **`Diagnostic`** — Structured error with severity, code, message, location, hint, source chain
- **`RenderResult`** — Output artifacts + warnings

## Data Injection

Backends receive:
- `plate_content` — raw plate from `Quill.plate` (empty for plate-less backends)
- `json_data` — JSON after coercion, defaults, normalization, and `transform_fields`
- `quill` — bundle with assets, packages, and any dynamic assets/fonts injected

See [GLUE_METADATA.md](GLUE_METADATA.md) for the Typst helper package.

## Backend Implementation

Implement the `Backend` trait: `id()`, `supported_formats()`, `plate_extension_types()`, `transform_fields()`, `compile()`. Optionally provide `default_quill()`.

See `backends/quillmark-typst` for reference implementation.
