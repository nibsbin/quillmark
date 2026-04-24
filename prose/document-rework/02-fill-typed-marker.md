# 02 — `!fill` as a Typed Marker

**Status:** Draft
**Depends on:** 01 (frontmatter item model)
**Blocks:** nothing

## Background

`!fill` is a YAML tag authors use to mark placeholder values in example
documents (see `crates/fixtures/resources/quills/cmu_letter/0.1.0/example.md`).
Today the parser accepts the tag and drops it
(`crates/core/src/value.rs::test_yaml_custom_tags_ignored`), so a round
trip turns `recipient: !fill` into `recipient: ""` with no trace of the
author's intent. Downstream wizards re-scan the source to recover fill
markers.

## Change

Promote `!fill` to a first-class typed marker on fields; round-trip it on
emit.

```rust
// Extends the FrontmatterItem::Field variant from tasking 01.
pub enum FrontmatterItem {
    Field {
        key: String,
        value: QuillValue,
        fill: bool,              // was !fill-tagged in source
        trailing_comment: Option<String>,
    },
    Comment(String),
}
```

`fill: bool` is sufficient — a field is either fill-tagged or not. The
placeholder content lives in `value` (string, empty, or otherwise). No
separate `Fill(Option<String>)` enum; YAGNI.

### Parser

- `key: !fill value` → `Field { key, value: QuillValue::from("value"), fill: true, … }`.
- `key: !fill` (no value) → `Field { key, value: QuillValue::String(""), fill: true, … }`.
- Any **other** custom tag (`!include`, `!env`, `!anything`) → reject with
  a parse warning (`unsupported_yaml_tag`) and drop the tag, keeping the
  raw scalar value. This preserves the current "don't fail on unknown
  tags" behaviour but stops silently hiding them; consumers can see the
  warning and decide.

### Emitter

- `Field { fill: true, value, … }` → `key: !fill <canonical-value>` when
  `value` is non-empty, or `key: !fill` when `value` is an empty string.
- `Field { fill: false, … }` → unchanged canonical emission.

### Data-model surface

- `doc.frontmatter` (the map-keyed getter from tasking 01) continues to
  return values only. A fill-tagged empty field appears as `""` there —
  consistent with today.
- `doc.frontmatterItems` (from tasking 01) exposes `fill: true` per item
  so consumers drive wizard UI off the data model.
- New mutator `Document::set_fill(key, bool)` toggles the marker without
  touching the value. `set_field(key, value)` leaves `fill` untouched
  (preserves the marker through value edits).

### WASM surface

- `FrontmatterItem` TS type gains `fill: boolean`.
- No changes to the `frontmatter` record getter.

## Validation

Required-field-is-filled validation is **out of scope** for this tasking.
A `!fill` on a required field will not error at parse or at `projectForm`
time here. Follow-on tasking may gate render on it; left open by design.

## Non-goals

- Generic custom-tag preservation (`!include`, etc.). Explicitly rejected
  with a warning.
- Placeholder text as a distinct concept from value. `!fill with text`
  and `!fill` differ only by what `value` holds.
- Render-time enforcement of fill state.

## Done when

- `!fill` round-trips through `fromMarkdown → toMarkdown`.
- `lossiness_tests.rs::custom_tags_lose_tag_but_keep_value` is rewritten
  to assert preservation for `!fill` and rejection-with-warning for
  other tags.
- `cmu_letter` example markdown round-trips byte-identically (modulo
  canonical quoting normalization from unrelated fields).
- `frontmatterItems` exposes `fill: boolean` and a WASM test exercises it.
- `MARKDOWN.md` gains a short section documenting `!fill` as the one
  supported custom tag.
