# Quill Versioning System

Status: **Implemented** (2026-03-22)  
Sources: `crates/core/src/version.rs`, `crates/quillmark/src/orchestration/engine.rs`

## Essentials
- Quills must declare `Quill.version` in `Quill.yaml` (`MAJOR.MINOR[.PATCH]`; two segments default patch to `0`).
- Version selectors in `QUILL` or workflow names:
  - `name@1.2.3` (exact)
  - `name@1.2` (latest patch in minor)
  - `name@1` (latest minor/patch in major)
  - `name@latest` or `name` (highest available)
- Multiple versions can be registered simultaneously; selection is resolved when creating a workflow.

## Validation & Errors
- Invalid or missing `version` → quill load error.
- Missing requested version produces a diagnostic listing available versions and suggesting selectors.
- If no QUILL is present, parser assigns `__default__@latest`; workflows still fail if that quill is not registered.

Related: [QUILL.md](QUILL.md) (bundle format), [PARSE.md](PARSE.md) (QUILL parsing).
