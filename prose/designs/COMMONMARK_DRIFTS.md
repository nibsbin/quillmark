# CommonMark Drifts in Quillmark Parsing

**Status:** Analysis
**Scope:** `quillmark_core::parse`, `quillmark_core::normalize`, `quillmark_typst::convert`

This document catalogs where Quillmark's two-layer parsing pipeline deviates
from the [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/) standard. It
is descriptive, not normative — the normative specification lives in
[`EXTENDED_MARKDOWN.md`](./EXTENDED_MARKDOWN.md).

## Pipeline Summary

Quillmark does not have a single markdown parser. Parsing happens in two
independent stages:

| Stage | Crate | File | Responsibility |
|---|---|---|---|
| 1. Frontmatter split | `quillmark-core` | `crates/core/src/parse.rs` | Splits input into YAML metadata blocks and body strings |
| 1a. Body normalization | `quillmark-core` | `crates/core/src/normalize.rs` | Strips bidi chars and repairs HTML-comment fences in each body |
| 2. Body rendering | `quillmark-typst` | `crates/backends/typst/src/convert.rs` | Runs `pulldown-cmark` v0.13 over each normalized body and emits Typst markup |

Only stage 2 uses a CommonMark parser (`pulldown-cmark`, with
`ENABLE_STRIKETHROUGH` and `ENABLE_TABLES` enabled —
`convert.rs:1236-1237`). Stage 1 operates on raw bytes and interprets `---`
fences on its own terms.

## Drifts by Category

### A. Structural — Frontmatter Hijacks `---`

The top-level splitter (`parse.rs:267-500`) consumes every line that is
exactly `---` as a metadata-block delimiter before `pulldown-cmark` ever sees
the content.

| CommonMark feature | Quillmark behaviour | Reference |
|---|---|---|
| Setext H2 underline (`---` under a line of text) | Not recognized; the `---` line splits the document into metadata blocks. Text above stays in the preceding body. | `parse.rs:267-350`, `EXTENDED_MARKDOWN.md:26` |
| Thematic break (`---`) | Not emitted; the line becomes a block delimiter. Only `***` and `___` thematic breaks reach `pulldown-cmark`, and those are also dropped by stage 2 (see below). | `parse.rs:267-350`, `convert.rs` (no `Event::Rule` handler) |
| `---` inside fenced code | Correctly ignored — the splitter tracks fence state. | `parse.rs:159-249` |

This is an intentional, documented reservation of the `---` token.

### B. Block Elements Silently Dropped

For these tags `convert.rs` has no matching arm, so pulldown-cmark events are
consumed without emitting Typst output. Inline text inside them may leak into
the surrounding paragraph; block-level wrappers disappear.

| Feature | CommonMark status | Quillmark behaviour |
|---|---|---|
| Thematic break (`***`, `___`) | Required | Dropped |
| Block quote (`> …`) | Required | Dropped (inner text may still render as paragraphs) |
| Image (`![alt](src)`) | Required | Dropped |
| Raw HTML block / inline | Required passthrough | Stripped except `<br>`, `<br/>`, `<br />`, which are rewritten to `HardBreak` (`convert.rs:970`) |
| HTML comment (`<!-- -->`) | Treated as HTML block | Stripped from output; however stage 1a repairs any content after `-->` on the same line so it is not lost (`normalize.rs:135-247`) |

Setext H1/H2 is handled separately: pulldown-cmark emits the heading events,
but `convert.rs:534-549` detects the setext source span and suppresses the
heading tags while keeping the text as a paragraph (`convert.rs:977-991`).

### C. Inline Emphasis

Stage 2 adds one custom emphasis style and diverges from the CommonMark
delimiter-run rules to support intraword formatting.

| Markdown | CommonMark | Quillmark → Typst |
|---|---|---|
| `*x*`, `_x_` | italic | `#emph[x]` |
| `**x**` | strong | `#strong[x]` |
| `__x__` | strong (same as `**`) | `#underline[x]` — diverges (`convert.rs:264-280`) |
| `foo__bar__baz` | literal (intraword `_` forbidden) | `foo#underline[bar]baz` — diverges; custom preprocessor at `convert.rs:1048-1230` recognizes intraword `__` via placeholder characters |
| `foo~~bar~~baz` | n/a (strikethrough is GFM) | `foo#strike[bar]baz` — intraword strike is allowed, diverges from GFM's own delimiter-run rule |
| `~~x~~` | n/a | `#strike[x]` (GFM extension) |

The placeholder pass uses U+FFF9/U+FFFA/U+FFFB/U+2060 as internal markers
(`convert.rs:1028-1230`); these code points in source text could in theory
collide but there is no current mitigation.

### D. Line Breaks

| Source | CommonMark | Quillmark |
|---|---|---|
| Two trailing spaces + `\n` | hard break | `#linebreak()` |
| `\\` + `\n` | hard break | `#linebreak()` (via pulldown-cmark) |
| Single `\n` | soft break | rendered as a space (`convert.rs:479-481`) |
| Literal `<br>`, `<br/>`, `<br />` | raw HTML | rewritten to `HardBreak` and emitted as `#linebreak()` (`convert.rs:970`) |

### E. Links

`[text](url "title")` — the title argument is parsed by pulldown-cmark but
discarded; only `#link("url")[text]` is emitted (`convert.rs:285-291`).
Reference-style links and autolinks work as pulldown-cmark supplies them.

### F. Tables

GFM pipe tables are enabled (`ENABLE_TABLES`) and mapped to `#table(...)` with
column alignment preserved (`convert.rs:302-340`). This is a GFM extension, not
core CommonMark.

### G. Features Not Implemented

Math (`$…$`), footnotes, definition lists, and GFM task lists are not handled.
`$` is escaped rather than interpreted (`convert.rs:1277, 1489`).

## Pre-parse Input Normalization

Applied in `normalize_markdown` (`normalize.rs:249-277`) before stage 2:

1. **Bidi control stripping** — removes U+061C, U+200E–U+200F, U+202A–U+202E,
   U+2066–U+2069 so invisible characters cannot desynchronize delimiter runs
   (`normalize.rs:71-133`). This is defensive and does not correspond to any
   CommonMark rule.
2. **HTML comment fence repair** — if text follows `-->` on the same line, a
   newline is inserted so the trailing text escapes the HTML block rule and
   reaches the paragraph parser (`normalize.rs:135-247`). This inverts
   CommonMark HTML-block type 2 behaviour.

## Typst-Specific Escaping

`escape_markup` (`convert.rs:55-71`) escapes `\`, `//`, `~`, `*`, `_`, `` ` ``,
`#`, `[`, `]`, `{`, `}`, `$`, `<`, `>`, and `@` in emitted text. This is a
backend concern, not a CommonMark deviation, but it means round-tripping
markdown through Quillmark is not lossless for these glyphs when they appear
as literal text.

## Resource Limits

| Limit | Value | Source |
|---|---|---|
| Max input size | 10 MB | `parse.rs:498` |
| Max YAML per block | 1 MB | `parse.rs:301` |
| Max YAML depth | 100 | `parse.rs:257-263` |
| Max markdown nesting depth | 100 | `convert.rs:164-177` via `MAX_NESTING_DEPTH` |
| Max fields / cards | 1000 | `error.rs` constants |

Deeply nested CommonMark constructs that exceed `MAX_NESTING_DEPTH` return a
parse error rather than the CommonMark-specified unlimited nesting.

## Drift Summary Table

| Feature | CommonMark | Quillmark | Drift |
|---|---|---|---|
| ATX headings | ✅ | ✅ | — |
| Setext headings | ✅ | ❌ suppressed | intentional |
| Thematic break | ✅ | ❌ dropped | intentional; `---` is reserved |
| Paragraphs | ✅ | ✅ | — |
| Block quote | ✅ | ❌ dropped | intentional |
| Ordered / unordered list | ✅ | ✅ | — |
| Indented code block | ✅ | ✅ (via pulldown-cmark) | — |
| Fenced code block | ✅ | ✅ | — |
| HTML block | ✅ | ❌ stripped (except `<br>`) | intentional |
| Link ref / autolink / inline link | ✅ | ✅ (titles ignored) | minor |
| Image | ✅ | ❌ dropped | intentional |
| Emphasis `*` / `_` | ✅ | ✅ | — |
| Strong `**` | ✅ | ✅ → `#strong` | — |
| Strong `__` | ✅ | ❌ reinterpreted as underline | intentional extension |
| Intraword `_` | forbidden | allowed for `__` | extension |
| Hard / soft break | ✅ | ✅ | — |
| Backslash escapes | ✅ | ✅ | — |
| Entity / numeric refs | ✅ | pass-through via pulldown-cmark | — |
| Strikethrough (GFM) | n/a | ✅ | GFM |
| Tables (GFM) | n/a | ✅ | GFM |
| Task lists (GFM) | n/a | ❌ | — |
| Footnotes | n/a | ❌ | — |
| Math | n/a | ❌ (literal `$` escaped) | — |
| Frontmatter `---` blocks | n/a | ✅ required | extension |

## References

- `crates/core/src/parse.rs`
- `crates/core/src/normalize.rs`
- `crates/backends/typst/src/convert.rs`
- `prose/designs/EXTENDED_MARKDOWN.md`
- `prose/designs/PARSE.md`
- `crates/backends/typst/docs/designs/CONVERT.md`
- `docs/authoring/markdown-syntax.md`
