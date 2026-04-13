# Schema Model (`QuillConfig`)

## TL;DR

`QuillConfig` is the only schema model in quillmark. Validation, coercion, defaults/examples extraction, and public schema emission all read directly from it.

## Quill.yaml DSL

Schema authoring lives in `Quill.yaml` under:

- `main.fields`
- `cards.<card_name>.fields`
- optional `ui` hints on fields/cards/main

Supported field types:

| Quill.yaml Type | Meaning |
|---|---|
| `string` | UTF-8 text |
| `number` | Numeric value |
| `boolean` | `true` / `false` |
| `array` | Ordered list; use `items:` |
| `object` | Structured map; use `properties:` |
| `date` | `YYYY-MM-DD` |
| `datetime` | ISO 8601 |
| `markdown` | Rich text; backends handle conversion |

## Native validation

Validation is implemented by a native walker over `QuillConfig` in `quill/validation.rs`.

- Entry point: `QuillConfig::validate(&HashMap<String, QuillValue>)`
- Returns `Result<(), Vec<ValidationError>>`
- Collects all errors (does not short-circuit)
- Emits path-aware errors for top-level fields and card fields

## Public schema emission

External schema contract is emitted by `QuillConfig::public_schema_yaml()`.

- Output is YAML text
- Shape is a subset projection of `Quill.yaml`
- Includes `name`, `description`, optional `example`, `fields`, and `cards`
- Preserves `ui` hints as `ui:` (no renaming)

See `PUBLIC_SCHEMA.md` for the output contract.
