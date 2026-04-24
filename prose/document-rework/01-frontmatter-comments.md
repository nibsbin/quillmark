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
    Field {
        key: String,
        value: QuillValue,
        // Trailing `# …` on the same line, if any.  `None` otherwise.
        trailing_comment: Option<String>,
    },
    Comment(String), // standalone `# …` line; text excludes the leading `#` and one optional space.
}

pub struct Frontmatter {
    items: Vec<FrontmatterItem>,
}
```

`Frontmatter` provides both ordered iteration and map-keyed access
(`get`, `contains_key`, `insert`, `remove`) so existing callers that treat
the frontmatter as a map keep working. Internally the map-keyed accessors
walk the item vec; field count is small enough that linear scan is fine.

### Parser

- Standalone comment lines (first non-whitespace char is `#`) between the
  opening `---` and the closing `---` become `Comment(text)` items in
  source order.
- Trailing comments on key lines (`key: value  # note`) attach as
  `Field.trailing_comment`.
- Comments *inside* nested values (arrays, maps) are dropped silently.
  Emit one `comments_in_nested_yaml_dropped` warning per document the
  first time this is encountered.
- Banner comments above the F1 sentinel line (already tolerated by
  MARKDOWN.md §4 F1) land in the item list in source order.

### Emitter

Walk `items` in order. For `Field`: emit `key: value` (canonical quoting)
with the trailing comment appended after two spaces, if present. For
`Comment`: emit `# <text>` on its own line. No blank-line inference —
blank lines are not modeled; emit items one per line.

### Mutator behaviour

- `set_field(key, value)` — updates the existing `Field` entry in place;
  or appends a new `Field` at the end if the key is absent. Adjacent
  comments are untouched.
- `remove_field(key)` — drops the `Field` entry. Adjacent comments stay.
  Orphaned comments are the caller's problem; we don't infer attachment.
- `set_trailing_comment(key, Option<String>)` — new; mostly for tests.
  Consumers that want to edit comments re-emit full YAML through a
  different path (out of scope here).

### WASM surface

- `Document.frontmatter` (getter) keeps its current shape:
  `Record<string, unknown>` of values only. Comments are invisible here.
- Add `Document.frontmatterItems` (getter) returning the ordered
  `FrontmatterItem[]` for consumers that care.

## Non-goals

- Nested-value comments.
- Blank-line preservation.
- Comment-editing mutators beyond the trivial `set_trailing_comment`.
- Attachment inference ("this comment belongs to this key").

## Done when

- Round-tripping a document with top-level comments produces output where
  all such comments appear in their original positions and text.
- `lossiness_tests.rs::yaml_comments_disappear_on_round_trip` is
  rewritten to assert the opposite and passes.
- `set_field` / `remove_field` on a commented frontmatter leave comments
  in place (new tests).
- `Document.frontmatterItems` is exposed in WASM and Rust; basic test in
  `basic.test.js` covers the round-trip.
- `MARKDOWN.md` §3 gains a one-paragraph note that top-level comments
  round-trip and nested comments don't.
