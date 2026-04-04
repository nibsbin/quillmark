# Schema and Validation

## Backend Trait

The `Backend` trait interface:

- `id() -> &str` — backend identifier (e.g., `"typst"`)
- `supported_formats() -> &[OutputFormat]` — output formats supported
- `plate_extension_types() -> &[&str]` — accepted plate extensions (e.g., `[".typ"]`); empty slice means no plate required
- `compile(plate, quill, opts, json_data)` — compile plate content + JSON document data into artifacts
- `transform_fields(fields, schema)` — optional backend-specific field shaping before JSON serialization
- `default_quill() -> Option<Quill>` — optional embedded default quill for zero-config use

## Quill Fields (`[fields]`)

Field properties:

- `name` — key under `fields` in YAML (e.g., `fields: { title: ... }` → name `"title"`)
- `title` — short label (`title` in JSON Schema)
- `description` — required; used as JSON Schema `description`
- `type` — `"string"`, `"number"`, `"boolean"`, `"array"`, `"date"`, `"datetime"`, or `"markdown"`
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
| `"date"` | `"string"` + `format: "date"` |
| `"datetime"` | `"string"` + `format: "date-time"` |
| `"markdown"` | `"string"` + `contentMediaType: "text/markdown"` |

> `type: object` is only valid inside `items` for typed array rows (e.g. `items: {type: object, properties: {...}}`). Standalone `type: object` fields are rejected at parse time with a warning.

`contentMediaType = "text/markdown"` marks fields the Typst backend converts to Typst markup via `transform_fields`.

## UI Configuration (`[ui]`)

- `group` — UI group/section name ✅
- `order` — display order index (auto-generated from YAML field position) ✅
- `compact` — compact rendering hint for dense lists ✅
- `multiline` — start as a larger text box (only meaningful on `markdown` fields) ✅

Serialized into `x-ui` in generated JSON Schema. Validation ignores `x-ui`.

### `visible_when`

`visible_when` maps sibling field names to arrays of accepted values. AND across keys; OR within values. Absent means always visible. This is a UI hint only — not a validation constraint.

```json
{
  "x-ui": {
    "group": "Addressing",
    "order": 0,
    "visible_when": {
      "format": ["standard", "separate_page"]
    }
  }
}
```

### `multiline`

For `markdown` fields, `multiline: true` signals the UI to present a larger initial text box. Serialized as `"x-ui": { "multiline": true }`. No effect on backend processing.

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
