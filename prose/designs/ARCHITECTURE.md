# Quillmark Architecture

## TL;DR

Quillmark converts Markdown with YAML frontmatter into output artifacts (PDF, SVG, PNG, TXT). A `Workflow` orchestrates the pipeline; backends do the heavy compilation.

## Data Flow

1. **Parse** — YAML frontmatter extraction, bidi stripping, HTML fence normalization
2. **Normalize** — Type coercion, schema defaults, field validation
3. **Compile** — Backend's `open()` receives plate + JSON data and returns a `RenderSession`; `RenderSession::render()` produces artifacts

## Crate Structure

### `quillmark-core`

Foundation types and traits. No backend dependencies; backends depend on this crate.

Key exports: `Backend`, `Artifact`, `OutputFormat`, `RenderOptions`, `RenderSession`, `ParsedDocument`, `Quill`, `FileTreeNode`, `QuillIgnore`, `RenderError`, `Diagnostic`, `Severity`, `Location`, `RenderResult`, `QuillValue`, `QuillReference`, `Version`, `VersionSelector`, `BODY_FIELD`.

### `quillmark` (orchestration)

High-level API: `Quillmark` (engine), `Workflow` (pipeline), `QuillRef`. Handles parse → normalize → compile, schema coercion, and backend auto-registration.

### `backends/quillmark-typst`

Implements `Backend` for PDF, SVG, and PNG. Converts Markdown fields to Typst markup inside `open()`. Resolves fonts and assets. See [GLUE_METADATA.md](GLUE_METADATA.md).

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

- **`Quillmark`** — Engine managing registered backends; auto-registers `TypstBackend` when the `typst` feature is enabled
- **`Workflow`** — Rendering pipeline (parse → normalize → compile); supports dynamic asset/font injection and `dry_run` validation
- **`Backend`** — Trait for output formats (`Send + Sync`): `id()`, `supported_formats()`, `open()`
- **`RenderSession`** — Opaque handle returned by `Backend::open()`; call `render(opts)` to produce artifacts
- **`Quill`** — Format bundle (plate + assets/packages/metadata)
- **`ParsedDocument`** — Frontmatter fields + body from Markdown
- **`Diagnostic`** — Structured error with severity, code, message, location, hint, source chain
- **`RenderResult`** — Output artifacts + accumulated warnings

## Data Injection

`Backend::open()` receives:
- `plate_content` — raw plate string from `Quill.plate` (empty string for plate-less backends)
- `json_data` — JSON object after coercion, defaults, normalization
- `quill` — bundle with static assets/packages plus any dynamic assets/fonts injected via `Workflow::add_asset` / `add_font`

See [GLUE_METADATA.md](GLUE_METADATA.md) for the Typst helper package.

## Backend Implementation

Implement three methods of the `Backend` trait: `id()`, `supported_formats()`, `open()`. Return a `RenderSession` wrapping a `SessionHandle` that handles format-specific rendering.

See `backends/quillmark-typst` for the reference implementation.
