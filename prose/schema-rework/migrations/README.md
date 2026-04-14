# Schema Rework: Consumer Migration Guides

Reference docs for downstream consumers adapting to the schema rework
described in `../00-overview.md`.

Each doc is self-contained: read only the one that matches your
integration point.

| Consumer | File | What you're migrating from |
|---|---|---|
| Form builders & UI renderers | `form-builders.md` | JSON Schema with `x-ui` hints and `CARDS` array |
| WASM / JS / TS apps | `wasm-consumers.md` | `QuillInfo.schema` as object + `getStrippedSchema()` |
| Rust crates depending on `quillmark-core` | `rust-consumers.md` | `quillmark_core::schema::*` module |

Not covered here (but affected): Python bindings (`quill.schema` became
`str` — same trajectory as the WASM guide), CLI pipeline users (`quillmark
schema` output format flipped JSON Schema → YAML), and LLM prompt
templates (swap the embedded schema for the YAML emitted by
`public_schema_yaml()`).

Everyone is migrating toward the same target: the YAML subset documented
in `../00-overview.md` §"The new public contract".
