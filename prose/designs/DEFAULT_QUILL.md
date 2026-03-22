# Default Quill System

Status: **Implemented** (2026-03-22)

## Contract
- Reserved name: `__default__`.
- Parser always assigns `__default__@latest` when no `QUILL` tag is present.
- `Quillmark::register_backend` auto-registers a backend’s embedded default quill if `__default__` is not already registered.
- If no default exists and no `QUILL` is provided, workflows error with a clear diagnostic.

## Typst Backend Default
- Location: `crates/backends/typst/default_quill/` (Quill.yaml, plate.typ, example.md).
- Loaded via `Backend::default_quill()` implementation.

## Expectations
- Default quill must validate like any other quill (semver version required).
- Consumers can still override by providing `QUILL:` explicitly or `--quill` in the CLI.

Related: [PARSE.md](PARSE.md) (default assignment), [QUILL.md](QUILL.md) (bundle format).
