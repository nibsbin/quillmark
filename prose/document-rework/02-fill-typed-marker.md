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
        fill: bool,
    },
    Comment(String),
}
```

`fill: bool` is sufficient — a field is either fill-tagged or not. The
value lives in `value` with its natural YAML type. No separate enum.

### Parser

`!fill` tags **any** scalar. The tagged scalar keeps its parsed YAML
type:

- `key: !fill "2d lt example"` → `Field { value: String("2d lt example"), fill: true }`.
- `key: !fill 42`              → `Field { value: Integer(42),            fill: true }`.
- `key: !fill 3.14`            → `Field { value: Float(3.14),            fill: true }`.
- `key: !fill true`            → `Field { value: Bool(true),             fill: true }`.
- `key: !fill`  (no value)     → `Field { value: Null,                   fill: true }`.

Non-scalar `!fill` (tagged map or sequence) is rejected at parse with
`unsupported_fill_target` — `!fill` on structured values is YAGNI until
a use case exists.

Any **other** custom tag (`!include`, `!env`, `!anything`) → reject with
a parse warning (`unsupported_yaml_tag`) and drop the tag, keeping the
raw scalar value. This preserves the current "don't fail on unknown
tags" behaviour but stops silently hiding them.

### Emitter

- `Field { fill: true, value: Null, … }` → `key: !fill`.
- `Field { fill: true, value: scalar, … }` → `key: !fill <canonical-scalar>`.
- `Field { fill: false, … }` → unchanged canonical emission.

### Data-model surface

- `doc.frontmatter` (the map-keyed getter from tasking 01) continues to
  return values only. A fill-tagged null field appears as `null` there.
- `doc.frontmatterItems` exposes `fill: boolean` per item so consumers
  drive wizard UI off the data model.

### Mutators — two explicit methods

```rust
/// Set a field's value. Always clears the fill marker.
/// This is the "user filled this in" path.
fn set_field<V: Into<QuillValue>>(&mut self, key: &str, value: V);

/// Set a field's value AND mark it as fill.
/// This is the "reset to placeholder" path. Empty value = `key: !fill`.
fn set_fill<V: Into<QuillValue>>(&mut self, key: &str, value: V);
```

Two methods, two intents. The common wizard flow ("user typed something,
clear the placeholder") is the default `set_field`; the rarer reset is
an explicit `set_fill`. No options struct, no boolean parameter to
forget in JS.

### WASM surface

- `FrontmatterItem` TS type gains `fill: boolean`.
- `Document.setField(key, value)` unchanged signature; clears fill.
- `Document.setFill(key, value)` new; sets fill=true with the given value.
- `frontmatter` record getter unchanged.

## Validation

Required-field-is-filled validation is **out of scope** for this tasking.
A `!fill` on a required field will not error at parse or at `projectForm`
time here. Follow-on tasking may gate render on it.

## Non-goals

- Generic custom-tag preservation (`!include`, etc.). Rejected with a
  warning.
- `!fill` on maps / sequences. Rejected with a warning.
- Render-time enforcement of fill state.

## Done when

- `!fill` round-trips through `fromMarkdown → toMarkdown` for all scalar
  types (string, int, float, bool, null).
- `lossiness_tests.rs::custom_tags_lose_tag_but_keep_value` is rewritten
  to assert preservation for `!fill` and rejection-with-warning for
  other tags.
- `cmu_letter` example markdown round-trips byte-identically (modulo
  canonical quoting normalization from unrelated fields).
- `frontmatterItems` exposes `fill: boolean` and a WASM test exercises
  `setField` clearing fill and `setFill` setting it.
- `MARKDOWN.md` gains a short section documenting `!fill` as the one
  supported custom tag.
