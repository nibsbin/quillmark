# 04 — `stripHtmlComments` Utility

**Status:** Draft
**Depends on:** nothing
**Blocks:** nothing

## Background

Downstream editors ship a `stripMarkdownHtmlComments` helper that removes
`<!-- … -->` sequences from markdown text before handing it to a
ProseMirror-style renderer. The stripping itself is generic; only the
*policy* of when to strip is product-specific. Generic ≈ 10 lines — a
fine fit for a core utility, a poor fit for per-consumer reimplementation.

## Change

Expose a pure function.

```rust
pub fn strip_html_comments(input: &str) -> String;
```

### Semantics

- Remove every `<!-- … -->` sequence from `input`. Both single-line and
  multi-line comments are stripped; nesting is not a concern (HTML
  comments don't nest).
- Input is raw markdown text. No awareness of code fences — if the
  author put an HTML comment inside a code block, it gets stripped too.
  (Callers that need fence-awareness are doing something consumer-
  specific; that's policy, not this function.)
- Leaves surrounding whitespace as-is; does not collapse blank lines
  left behind.

### WASM surface

```ts
export function stripHtmlComments(input: string): string;
```

Free function, not a method on `Document` — it's orthogonal to any
document instance.

## Non-goals

- Fence-aware stripping (skip comments inside ``` blocks). Policy.
- Whitespace normalization after stripping. Policy.
- Stripping other HTML-ish constructs.

## Done when

- `strip_html_comments` is covered by unit tests including single-line,
  multi-line, and multiple-comments-per-input cases.
- `stripHtmlComments` is exposed in WASM with a basic.test.js case.
- No consumer API beyond the single function.
