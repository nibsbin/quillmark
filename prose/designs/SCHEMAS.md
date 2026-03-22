# Schema and Validation

Status: **Implemented** (2026-03-22)  
Sources: `crates/core/src/schema.rs`, `crates/core/src/quill.rs`, `crates/quillmark/src/orchestration/workflow.rs`

## TL;DR
- JSON Schema is generated from `Quill.yaml` (`fields` + `cards`).
- Card schemas live in `$defs` with a `CARD` discriminator and `oneOf`.
- `contentMediaType = "text/markdown"` marks fields that Typst transforms to Typst markup.
- Defaults/examples are cached from the generated schema; applied after backend field transforms.
- Validation uses the same schema across CLI, Python, WASM, and Rust API.

## Field Model
- Types: `string`, `number`, `boolean`, `array`, `object`, `date`, `datetime`, `markdown` (→ `contentMediaType: "text/markdown"`).
- Common keys: `title`, `description`, `default`, `examples`, `enum`, `required`.
- Nested: `items` (for arrays), `properties` + nested required (for objects).
- UI hints → `x-ui`:
  - `group`, `order` (declaration order is default), `visible_when` (AND across keys, OR across values), `compact`.

## Cards
- Defined under `cards.<name>.fields.*` in `Quill.yaml`.
- JSON Schema places each card in `$defs` with:
  - `CARD` const property, required fields, optional `title/description`.
  - `x-ui` on cards supports `hide_body`.
- The document schema includes `CARDS` as an array with `oneOf` + discriminator mapping.

## Generation & Validation
1. Build field/card schemas → JSON Schema map.
2. Cache defaults/examples for quick access.
3. `Workflow::compile_data` pipeline:
   - `with_coercion(schema)` (string→number/boolean, boolean↔number, singular→array, etc.)
   - `validate_document(schema, fields)`
   - normalize, backend `transform_fields`, then apply cached defaults.
4. `Workflow::dry_run` performs coercion + validation only (no normalization/transform/backend).

## Coercion Highlights
- Strings `"true"/"false"` → bool.
- Numbers ↔ bool (0/1).
- Numeric strings → numbers.
- Singular values → arrays when schema expects array.

## References
- Parser surface: [EXTENDED_MARKDOWN.md](EXTENDED_MARKDOWN.md)
- Card design: [CARDS.md](CARDS.md)
- Data injection: [GLUE_METADATA.md](GLUE_METADATA.md)
