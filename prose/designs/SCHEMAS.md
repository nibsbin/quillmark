# Schema and Validation

## Backend Trait

The `Backend` trait interface:

- `id() -> &str` — backend identifier (e.g., `"typst"`)
- `supported_formats() -> &[OutputFormat]` — output formats supported
- `plate_extension_types() -> &[&str]` — accepted plate extensions (e.g., `[".typ"]`); empty slice means no plate required
- `compile(plate, quill, opts, json_data)` — compile plate content + JSON document data into artifacts
- `transform_fields(fields, schema)` — optional backend-specific field shaping before JSON serialization
- `default_quill() -> Option<Quill>` — optional embedded default quill for zero-config use

## Quill Fields (`main.fields`)

Field properties:

- `name` — key under `main.fields` in YAML (e.g., `main.fields.title` → name `"title"`)
- `title` — short label (`title` in JSON Schema)
- `description` — required; used as JSON Schema `description`
- `type` — `"string"`, `"number"`, `"boolean"`, `"array"`, `"object"` (for typed table rows under `items`), `"date"`, `"datetime"`, or `"markdown"`
- `default` — default value
- `required` — bool, default `false`
- `examples` — array of example values
- `ui` — UI metadata table (see below)

**Type mapping (YAML → JSON Schema):**

| YAML | JSON Schema |
|------|-------------|
| `"string"` | `"string"` |
| `"number"` | `"number"` |
| `"boolean"` | `"boolean"` |
| `"array"` | `"array"` |
| `"object"` / `"dict"` | `"object"` + `properties` |
| `"date"` | `"string"` + `format: "date"` |
| `"datetime"` | `"string"` + `format: "date-time"` |
| `"markdown"` | `"string"` + `contentMediaType: "text/markdown"` |

> `type: object` with `properties` is supported inside `items` for typed array rows (e.g. `items: {type: object, properties: {...}}`). Top-level object fields and nested object properties are not supported.

`contentMediaType = "text/markdown"` marks fields the Typst backend converts to Typst markup via `transform_fields`.

## UI Configuration (`[ui]`)

- `group` — UI group/section name ✅
- `order` — display order index (auto-generated from YAML field position) ✅
- `compact` — compact rendering hint for dense lists ✅
- `multiline` — start as a larger text box (`string` and `markdown` fields) ✅

Serialized into `x-ui` in generated JSON Schema. Validation ignores `x-ui`.

### `multiline`

For `string` and `markdown` fields, `multiline: true` signals the UI to present a larger initial text box. On `string` fields this means plain text with newlines preserved (e.g. mailing addresses, multi-line signatures). On `markdown` fields the content is additionally processed as Markdown by the backend. Serialized as `"x-ui": { "multiline": true }`. No effect on backend processing beyond what the field type already implies.

```json
{
  "x-ui": {
    "multiline": true
  }
}
```

## Quill Registration Constraints

- `name` must not already be registered
- `backend` must already be registered
- `description` cannot be empty
- `plate_file` extension must be in the backend's `plate_extension_types` if provided
