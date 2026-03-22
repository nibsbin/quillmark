# Quillmark Architecture

Status: **Implemented** (2026-03-22)

## System Flow
1. **Parse** (`ParsedDocument::from_markdown`)  
   - Extended markdown → fields + body, cards into `CARDS`, assign `quill_ref` (default `__default__@latest`).  
2. **Coerce & Validate** (`Workflow::compile_data`)  
   - Coerce to schema types, validate JSON Schema, then normalize (bidi strip + HTML comment fence fix).  
3. **Transform & Defaults**  
   - Backend `transform_fields` (e.g., markdown→Typst markup) → apply cached defaults/examples.  
4. **Render**  
   - Prepare quill (inject dynamic assets/fonts) → backend compile to artifacts.

## Crates
- **crates/core**: parsing, quill loading (`Quill.yaml`), schema generation/validation, diagnostics.
- **crates/quillmark**: engine + workflow orchestration, normalization, default quill registration.
- **crates/backends/typst**: Typst backend, markdown→Typst converter, helper package `@local/quillmark-helper`.
- **crates/backends/acroform**: PDF form filler via MiniJinja.
- **crates/bindings/cli | python | wasm**: CLIs and language bindings sharing core types.
- **crates/fixtures**: test quills/markdown; helper utilities.

## Key Traits & Types
- `Backend`: `id`, `supported_formats`, `plate_extension_types`, `compile`, `transform_fields`, `default_quill`.
- `Workflow`: sealed orchestration; exposes `render`, `compile_data`, `dry_run`, dynamic assets/fonts.
- `Quill`: template bundle (file tree, schema, defaults/examples, optional plate/example).
- `ParsedDocument`: map of fields + `BODY` + `CARDS` + `quill_reference`.
- `RenderResult`/`Artifact`: output bytes + format + warnings.

## Assets & Packages
- Quill files loaded into an in-memory tree; `.quillignore` honored.
- Dynamic runtime assets/fonts prefixed `DYNAMIC_ASSET__*` / `DYNAMIC_FONT__*` injected before render.
- Typst backend resolves packages from quill tree (including `packages/`) and helper package embeds JSON.

## Error Model
- Core `RenderError` + `SerializableDiagnostic` carry severity, code, location, hint, and source chain.
- Shared across CLI, Python, and WASM; exit code `1` on errors in CLI.

Related: [PARSE.md](PARSE.md), [EXTENDED_MARKDOWN.md](EXTENDED_MARKDOWN.md), [GLUE_METADATA.md](GLUE_METADATA.md).
