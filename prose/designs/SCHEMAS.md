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
| `number` | Numeric value (integers and decimals) |
| `integer` | Integer-only numeric value |
| `boolean` | `true` / `false` |
| `array` | Ordered list; use `items:` |
| `object` | Structured map; use `properties:` |
| `date` | `YYYY-MM-DD` |
| `datetime` | ISO 8601 |
| `markdown` | Rich text; backends handle conversion |

## Type coercion

`QuillConfig::coerce(&HashMap<String, QuillValue>)` runs before validation.

- Returns `Result<HashMap<String, QuillValue>, CoercionError>`
- Coerces top-level fields and card fields in `CARDS` to their declared types
- Fails fast (`Err`) on the first value that cannot be coerced
- Coercion rules per type: array wrapping, boolean from string/int/float, number/integer from string, string/markdown pass-through, date/datetime format validation, object property recursion

## Native validation

Validation is implemented by a native walker over `QuillConfig` in `quill/validation.rs`.

- Entry point: `QuillConfig::validate(&HashMap<String, QuillValue>)` (dispatches to `validate_document`)
- Returns `Result<(), Vec<ValidationError>>`
- Collects all errors (does not short-circuit)
- Emits path-aware errors for top-level fields and card fields
- Validates `CARDS` array: each element must have a `CARD` discriminator matching a known card type

## Public schema emission

External schema contract is the value returned by `QuillConfig::public_schema()`.
`QuillConfig::public_schema_yaml()` is a convenience wrapper that YAML-encodes
the same value; the wasm `quill.metadata.schema` getter returns the same value
as JSON.

The wire format is pinned by serde attributes on `FieldSchema`, `CardSchema`,
`UiFieldSchema`, and `UiContainerSchema` directly — there is no parallel
"public" mirror struct. Top-level keys: `name`, `main`, optional `card_types`
(map keyed by card name), optional `example`. `main` and each entry in
`card_types` share the same `CardSchema` shape: `fields` (map keyed by field
name), optional `title`, `description`, `ui`. Each `FieldSchema` includes
`type`, optional `title`/`description`/`default`/`examples`/`ui`/`enum`/
`properties`/`items`, and optional `required` (omitted when false).
