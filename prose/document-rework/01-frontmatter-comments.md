# 01 — Frontmatter Comments as First-Class Items

**Status:** Draft
**Depends on:** nothing
**Blocks:** 02, 03

## Background

YAML comments are stripped at parse today (`crates/core/src/document/tests/lossiness_tests.rs::yaml_comments_disappear_on_round_trip`). The `Document` frontmatter is an `IndexMap<String, QuillValue>` — key-value pairs in insertion order, nothing else. A round-trip of

```markdown
---
QUILL: q
# recipient's full name
recipient: Jane
---
```

loses the comment. Downstream editors that want to preserve author intent
ship a second YAML parser (`parseBlocks`) to keep a comment-aware AST
alongside our parsed values.

## Change

Replace the map-shaped frontmatter with an ordered list of typed items.

```rust
pub enum FrontmatterItem {
    Field { key: String, value: QuillValue },
    Comment(String), // text excludes the leading `#` and one optional space.
}

pub struct Frontmatter {
    items: Vec<FrontmatterItem>,
}
```

`Frontmatter` provides both ordered iteration and map-keyed access
(`get`, `contains_key`, `insert`, `remove`) so existing callers that treat
the frontmatter as a map keep working. Internally the map-keyed accessors
walk the item vec; field count is small enough that linear scan is fine.

### Unified: cards use the same type

`Card::fields()` becomes `Card::frontmatter()` returning `&Frontmatter`.
Document frontmatter and card frontmatter share one representation,
parser, emitter, and mutator surface. `CARD:` stays hoisted into
`Card::tag` as today; `QUILL:` stays hoisted into
`Document::quill_reference` as today — neither appears as an item.

### Parser

- Standalone comment lines (first non-whitespace char is `#`) between the
  opening `---` and the closing `---` become `Comment(text)` items in
  source order.
- Trailing comments on value lines (`key: value  # note`) are normalized
  to standalone `Comment` items on the next line on round-trip. This is
  a deliberate canonical-formatting choice (opinionated layout beats two
  code paths). The parser produces a `Field` followed by a `Comment`
  item.
- Comments *inside* nested values (arrays, maps) are dropped silently.
  Emit one `comments_in_nested_yaml_dropped` warning per document the
  first time this is encountered.
- Banner comments above the F1 sentinel line (already tolerated by
  MARKDOWN.md §4 F1) land in the item list in source order.

### Emitter

Walk `items` in order, one per line. For `Field`: emit `key: value`
(canonical quoting). For `Comment`: emit `# <text>`. No blank-line
inference; blank lines are not modeled.

### Mutator behaviour

- `set_field(key, value)` — updates the existing `Field` entry in place;
  or appends a new `Field` at the end if the key is absent. Adjacent
  comments are untouched.
- `remove_field(key)` — drops the `Field` entry. Adjacent comments stay.
  Orphaned comments are the caller's problem; we don't infer attachment.

### WASM surface

- `Document.frontmatter` (getter) keeps its current shape:
  `Record<string, unknown>` of values only. Comments are invisible here.
- Add `Document.frontmatterItems` (getter) returning the ordered
  `FrontmatterItem[]` for consumers that care.
- `Card.fields` → `Card.frontmatter` (same shape as Document's
  `frontmatterItems`).

## Non-goals

- Nested-value comments.
- Blank-line preservation.
- Trailing-comment round-trip as trailing (they become own-line).
- Comment-editing mutators.
- Attachment inference ("this comment belongs to this key").

## Done when

- Round-tripping a document with top-level comments produces output where
  all such comments appear as own-line comments, in source order, with
  their original text.
- `lossiness_tests.rs::yaml_comments_disappear_on_round_trip` is
  rewritten to assert the opposite and passes.
- `set_field` / `remove_field` on a commented frontmatter leave comments
  in place (new tests).
- Card frontmatter exhibits the same round-trip behaviour (new tests).
- `Document.frontmatterItems` and `Card.frontmatter` are exposed in WASM
  and Rust; basic test in `basic.test.js` covers the round-trip.
- `MARKDOWN.md` §3 gains a one-paragraph note that top-level comments
  round-trip (as own-line comments), trailing comments are normalized
  to own-line, and nested comments are dropped with a warning.
