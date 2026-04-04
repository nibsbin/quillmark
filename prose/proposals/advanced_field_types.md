# Proposal: Advanced Field Types — Object Removal, Recursive Coercion, Markdown Fields

## Problem

Three issues with the current Quill.yaml field type system:

1. **`type: object`/`dict` is underspecified and unused standalone.** Every real usage is `type: array` with `items: { type: object, properties: {...} }` — a typed table. Standalone `type: object` suggests free-form key-value maps, but there's no `additionalProperties` support, no recursive coercion, and no clear UI rendering story. It's a trap.

2. **Coercion doesn't recurse into array items.** `coerce_value` for `FieldType::Object` is `value.clone()` — a no-op. A number property written as `"5"` in YAML silently stays a string. This breaks backends that expect typed data.

3. **No multi-line or markdown-in-metadata story for UI consumers.** Fields are either single-line strings or the BODY editor. There's no way to express "this metadata field should be a text box" or "this metadata field contains markdown."

## Decisions

### 1. Remove standalone `type: object`/`dict`

Remove `FieldType::Object` as a top-level field type. The `object` keyword is only valid inside array `items` as an anonymous row shape.

**Allowed:**
```yaml
cells:
  type: array
  items:
    type: object
    properties:
      category: { type: string }
      skills: { type: string }
```

**Rejected:**
```yaml
address:
  type: object
  properties:
    street: { type: string }
    city: { type: string }
```

For the rejected case, use separate fields with `ui: { group: Address }` instead. This gives better UI (each field gets its own label, validation, visibility rules) without requiring a nested form widget.

Single structured records are deferred until proven need.

### 2. Fix recursive coercion for array items

When an array field has `items: { type: object, properties: {...} }`, coercion must recurse into each array element and coerce property values according to their declared types.

```yaml
# Quill.yaml
scores:
  type: array
  items:
    type: object
    properties:
      name: { type: string }
      value: { type: number }
```

```yaml
# User document — YAML parses "95" as string
scores:
  - name: Math
    value: "95"
```

After coercion: `value` becomes numeric `95`.

This applies to `QuillConfig::coerce_fields` (`config.rs`) and the schema-based `coerce_value` (`schema.rs`). Both must recurse into array items when `items.properties` is defined.

### 3. Markdown fields as the multi-line story

Two text field types, no ambiguity:

| Quill.yaml | JSON Schema | UI rendering | Backend processing |
|---|---|---|---|
| `type: string` | `"type": "string"` | Single-line input | None |
| `type: markdown` | `"type": "string"` + `contentMediaType: text/markdown` | Auto-expanding text input | Backend converts (e.g., markdown → Typst) |

The `type: markdown` widget starts as a single line and expands as content grows. For fields expected to have substantial content, a sizing hint controls initial presentation:

```yaml
summary:
  type: markdown
  ui:
    multiline: true   # start as a larger text box
```

`multiline` is a UI hint only — no effect on validation or backend processing. It is only meaningful on `markdown` fields; ignored on other types.

Markdown processing remains **backend-only**. The core parsing layer treats markdown fields as opaque strings. The backend's `transform_fields()` method identifies fields with `contentMediaType: text/markdown` and converts them. This code path is already unified — BODY and metadata markdown fields flow through the same `transform_markdown_fields()` function.

### 4. Add `multiline` to `UiFieldSchema`

```rust
pub struct UiFieldSchema {
    pub group: Option<String>,
    pub order: Option<i32>,
    pub visible_when: Option<HashMap<String, Vec<String>>>,
    pub compact: Option<bool>,
    pub multiline: Option<bool>,  // new
}
```

Serialized as `"x-ui": { "multiline": true }` in JSON Schema. UI consumers use this to control initial text box size for markdown fields.

## Scope

### In scope
- Remove `FieldType::Object` variant and `"object"` | `"dict"` parsing
- Move `properties` from `FieldSchema` into array-items-only context (validation that `properties` is only set when inside `items`)
- Recursive coercion in `QuillConfig::coerce_value` and `schema.rs` coercion
- Add `multiline: Option<bool>` to `UiFieldSchema`
- Update `ui_key` constants
- Update classic_resume fixture (already uses object inside items — just needs validation)
- Update tests in `quill/tests.rs` (remove standalone object test, add recursive coercion tests)
- Update docs: `creating-quills.md`, `quill-yaml-reference.md`

### Out of scope
- Free-form dictionaries / `additionalProperties` — deferred indefinitely
- Single structured record type — deferred until proven need
- Core-layer markdown parsing or validation — no foreseeable use case
- Rich markdown editor UI implementation — consumer responsibility

## Files affected

| File | Change |
|------|--------|
| `crates/core/src/quill/types.rs` | Remove `Object` variant, add `multiline` to `UiFieldSchema` |
| `crates/core/src/quill/config.rs` | Recursive coercion in `coerce_value`, reject standalone object |
| `crates/core/src/schema.rs` | Recursive coercion, update `build_field_property` |
| `crates/core/src/quill/tests.rs` | Remove standalone object tests, add coercion + multiline tests |
| `docs/guides/creating-quills.md` | Remove dict section, update type table |
| `docs/guides/quill-yaml-reference.md` | Remove standalone object, document multiline |
| `prose/designs/SCHEMAS.md` | Update type mapping table |
