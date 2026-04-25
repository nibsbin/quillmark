# Schema Model (`QuillConfig`)

## TL;DR

`QuillConfig` is the only schema model in quillmark. Validation, coercion, defaults/examples extraction, and public schema emission all read directly from it.

## Quill.yaml DSL

Schema authoring lives in `Quill.yaml` under:

- `main.fields`
- `card_types.<card_name>.fields`
- optional `ui` hints on fields/card_types/main

Supported field types:

| Quill.yaml Type | Meaning |
|---|---|
| `string` | UTF-8 text |
| `number` | Numeric value (integers and decimals) |
| `integer` | Integer-only numeric value |
| `boolean` | `true` / `false` |
| `array` | Ordered list; use `items:` |
| `object` | Structured map; use `properties:` |
| `date` | `YYYY-MM-DD` |
| `datetime` | ISO 8601 |
| `markdown` | Rich text; backends handle conversion |

## Type coercion

`QuillConfig::coerce_frontmatter` and `QuillConfig::coerce_card` run before validation.

- `coerce_frontmatter(&IndexMap<String, QuillValue>)` — coerces main-card frontmatter fields; returns `Result<IndexMap<String, QuillValue>, CoercionError>`
- `coerce_card(card_tag, &IndexMap<String, QuillValue>)` — coerces a single card's fields against the matching card-type schema; returns the input unchanged when the tag is unknown
- Both fail fast (`Err`) on the first value that cannot be coerced
- Coercion rules per type: array wrapping, boolean from string/int/float, number/integer from string, string/markdown pass-through, date/datetime format validation, object property recursion

## Native validation

Validation is implemented by a native walker over `QuillConfig` in `quill/validation.rs`.

- Entry point: `QuillConfig::validate_document(&Document)` (dispatches to `validate_typed_document`)
- Returns `Result<(), Vec<ValidationError>>`
- Collects all errors (does not short-circuit)
- Emits path-aware errors for top-level fields and card fields
- Validates `CARDS` array: each element must have a `CARD` discriminator matching a known card type

## Public schema emission

External schema contract is emitted by `QuillConfig::public_schema_yaml()`.

- Output is YAML text
- Shape is a subset projection of `Quill.yaml`
- Includes `name`, `description`, optional `example`, `fields`, and `card_types`
- Preserves `ui` hints as `ui:` (no renaming)

See `PUBLIC_SCHEMA.md` for the output contract.
