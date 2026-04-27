# Quillmark Architecture

## TL;DR

Quillmark converts Markdown with YAML frontmatter into output artifacts (PDF, SVG, PNG, TXT). A `Quill` (the renderable shape) orchestrates the pipeline; backends do the heavy compilation.

## Data Flow

1. **Parse** — YAML frontmatter extraction, bidi stripping, HTML fence normalization
2. **Normalize** — Type coercion, schema defaults, field validation
3. **Compile** — Backend's `open()` receives plate + JSON data and returns a `RenderSession`; `RenderSession::render()` produces artifacts

## Crate Structure

### `quillmark-core`

Foundation types and traits. No backend dependencies; backends depend on this crate.

Key exports: `Backend`, `Artifact`, `OutputFormat`, `RenderOptions`, `RenderSession`, `Document`, `QuillSource`, `FileTreeNode`, `QuillIgnore`, `RenderError`, `Diagnostic`, `Severity`, `Location`, `RenderResult`, `QuillValue`, `QuillReference`, `Version`, `VersionSelector`.

### `quillmark` (orchestration)

High-level API: `Quillmark` (engine), `Quill` (renderable source + backend). Handles parse → normalize → compile, schema coercion, and backend auto-registration. Filesystem walking for `engine.quill_from_path` lives here; core is filesystem-agnostic.

### `backends/quillmark-typst`

Implements `Backend` for PDF, SVG, and PNG. Converts Markdown fields to Typst markup inside `open()`. Resolves fonts and assets. See [GLUE_METADATA.md](GLUE_METADATA.md).

### `bindings/quillmark-python`

PyO3 bindings published as `quillmark` on PyPI.

### `bindings/quillmark-wasm`

wasm-bindgen bindings published as `@quillmark/wasm`. Supports bundler and Node.js targets. Builds with `--weak-refs` so wasm-bindgen handles are reclaimed by `FinalizationRegistry`; `.free()` remains as the eager teardown hook. Requires Node 14.6+ / current evergreen browsers.

### `bindings/quillmark-cli`

Standalone binary. See [CLI.md](CLI.md).

### `quillmark-fixtures`

Test resources under `resources/`. Helper functions for test setup.

### `quillmark-fuzz`

Fuzz tests for parsing, templating, and rendering.

## Core Interfaces

- **`Quillmark`** — Engine managing registered backends; auto-registers `TypstBackend` when the `typst` feature is enabled
- **`Quill`** — Renderable shape in `quillmark`: pairs a `QuillSource` with a resolved `Backend`. Exposes `render`, `open`, `dry_run`, `compile_data`
- **`QuillSource`** — Pure data in `quillmark-core`: file bundle + config + metadata; no render ability
- **`Backend`** — Trait for output formats (`Send + Sync`): `id()`, `supported_formats()`, `open(plate, &QuillSource, json)`
- **`RenderSession`** — Opaque handle returned by `Backend::open()`; call `render(opts)` to produce artifacts
- **`Document`** — Typed in-memory representation of a Quillmark Markdown file (frontmatter, body, cards)
- **`Diagnostic`** — Structured error with severity, code, message, location, hint, source chain
- **`RenderResult`** — Output artifacts + accumulated warnings

## Data Injection

`Backend::open()` receives:
- `plate_content` — raw plate string from `QuillSource.plate` (empty string for plate-less backends)
- `source` — `&QuillSource` with static assets/packages, config, metadata
- `json_data` — JSON object after coercion, defaults, normalization

See [GLUE_METADATA.md](GLUE_METADATA.md) for the Typst helper package.

## Backend Implementation

Implement three methods of the `Backend` trait: `id()`, `supported_formats()`, `open()`. Return a `RenderSession` wrapping a `SessionHandle` that handles format-specific rendering.

See `backends/quillmark-typst` for the reference implementation.
