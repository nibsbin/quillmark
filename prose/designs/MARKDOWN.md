# Quillmark Markdown

**Status:** Draft specification
**Editor:** Quillmark Team
**Base:** [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/)

Quillmark Markdown is a **strict superset of CommonMark** with two declared
deviations. It layers a structured-data system (YAML frontmatter + named
card blocks) on top of ordinary markdown, and selects a small, stable set of
GFM extensions. This document is the authoritative syntax standard.

## 1. Superset Statement

Every valid CommonMark 0.31.2 document parses to the same block / inline
structure under this spec, *except* for the two deviations declared in
§6.2. Additionally, this spec defines:

- **Structured data** — YAML frontmatter and card blocks (§3).
- **Extensions** — pipe tables, strikethrough, intraword underline via
  `__`, and `<br>` recognition (§6.1).

Documents containing neither frontmatter nor card blocks are ordinary
CommonMark, parsed as such.

## 2. Document Grammar

A document is a sequence of three kinds of regions, in order:

```
Document = Frontmatter Body (CardFence CardBody)*
```

- **Frontmatter** — required. One metadata fence at the top of the
  document, carrying `QUILL` plus any document-level fields.
- **Body** — markdown content between the frontmatter close and the first
  card fence (or EOF).
- **Card fence + card body** — zero or more. Each card fence declares a
  typed structured record with its own fields; its body is the markdown
  that follows, up to the next card fence or EOF.

Frontmatter and card fences share the same delimiter (`---`) and detection
rules (§4). They differ only in role: frontmatter is the document's
entrypoint and must carry `QUILL`; cards must carry `CARD`.

## 3. Metadata Fences

A metadata fence is a pair of lines each containing exactly `---` (with
optional trailing whitespace). The content between is parsed as YAML.

- **Line endings.** `\n` and `\r\n` are equally accepted.
- **Whitespace-only content.** A fence whose content is only whitespace
  yields an empty field set.
- **Fences inside fenced code blocks.** `---` lines inside an open
  CommonMark fenced code block (triple-backtick or triple-tilde) are
  ignored for fence-detection purposes.
- **Reserved keys.** `QUILL`, `CARD`, `BODY`, and `CARDS` are reserved and
  may not appear as user-defined field names. `QUILL` is permitted only in
  the frontmatter; `CARD` is required in every non-frontmatter fence.

## 4. Fence Detection Rules

A `---` line opens a metadata fence **iff both** of the following hold:

**F1 — Sentinel.** The first non-blank line of content between the opening
`---` and the next `---` line matches `KEY: value`, where `KEY` is:

- `QUILL` if this is the first metadata fence in the document, or
- `CARD` if any metadata fence has already been recognised.

**F2 — Leading blank.** The opening `---` is on line 1 of the document, or
the line immediately above it is blank.

A `---` line that fails either rule is delegated to CommonMark unchanged:

- If the line above is non-blank paragraph text, `---` is a setext H2
  underline.
- Otherwise, `---` is a thematic break.

### 4.1 Worked Examples

```markdown
---
QUILL: resume
title: CV
---

Main Body Heading
-----------------      # Setext H2 — F2 fails (paragraph above)

Some prose.

---                    # Thematic break — F1 fails (no QUILL:/CARD: after)

More prose.

---
CARD: profile
name: Alice
---

Profile body.
```

### 4.2 Failure Mode and Linter Guidance

A `---`/`---` pair whose content's first key is almost-but-not-quite
`CARD` (e.g. `Card:`, `CARDS:`, `card:`) fails F1 and is interpreted as
two thematic breaks with literal YAML between. Implementations **should**
emit a lint warning when they encounter a `---`/`---` pair whose content's
first non-blank line matches `[A-Za-z][A-Za-z0-9_]*:` but whose key is not
the expected sentinel.

## 5. Data Model

Parsing yields:

```typescript
interface Document {
  QUILL: string;          // template name, from frontmatter
  BODY: string;           // body prose between frontmatter and first card
  CARDS: Card[];          // zero or more cards, in document order
  [field: string]: any;   // other frontmatter fields
}

interface Card {
  CARD: string;           // card type, matches /^[a-z_][a-z0-9_]*$/
  BODY: string;           // card body prose
  [field: string]: any;   // other card fields
}
```

- `CARDS` is always present, possibly empty.
- Frontmatter fields and card-field names may collide freely; each card is
  its own scope.
- Body text is preserved verbatim — whitespace, line endings, and inline
  CommonMark are untouched by the splitter.

## 6. Markdown Content

Body regions (the document body and every card body) are rendered as
CommonMark 0.31.2 with the extensions and deviations below.

### 6.1 Extensions

| Feature | Syntax | Notes |
|---|---|---|
| Strikethrough | `~~text~~` | GFM rules: word-bounded delimiter runs only. |
| Pipe tables | GFM pipe-table syntax with alignment rows | Supports `:---`, `:---:`, `---:` alignment. |
| Intraword underline | `foo__bar__baz` → `foo<u>bar</u>baz` | Deviation from CommonMark §6.2 delimiter-run rules, scoped to `__` only (see 6.2). |
| Line break via `<br>` | Literal `<br>`, `<br/>`, `<br />` | Rewritten to a hard line break. The only HTML tag with rendered output. |

### 6.2 Declared Deviations from CommonMark

1. **`__text__` renders as underline, not strong.** Use `**text**` for
   strong emphasis. Precedent: Discord. Consequence: `__init__` in prose
   renders as underlined "init"; wrap code-like tokens in backticks
   (`` `__init__` ``) — standard practice.
2. **Raw HTML other than `<br>` is accepted syntactically but produces no
   output.** The parser recognises HTML per CommonMark §4.6 / §6.11 and
   discards the events. Rationale: Typst has no HTML renderer, and
   arbitrary passthrough would create an injection vector for downstream
   HTML-producing tooling.

No other syntax deviates from CommonMark.

### 6.3 Out of Scope

The following are parsed where CommonMark or pulldown-cmark already
handles them, but produce no Quillmark-specific output and may be
implemented in a future revision:

- Images (`![alt](src)`) — reserved for the asset-resolver integration;
  required for v1 of this spec.
- Link titles (`[text](url "title")`) — title is discarded.
- Math (`$…$`, `$$…$$`), footnotes, task lists, definition lists — not
  supported; `$` is literal.
- HTML comments — accepted syntactically, not rendered (see §6.2).

## 7. Input Normalization

Before CommonMark parsing, each body region is normalized:

1. **Bidi control stripping.** Remove U+061C, U+200E, U+200F,
   U+202A–U+202E, U+2066–U+2069. These invisible characters can
   desynchronize delimiter runs when copy-pasted from bidi-aware sources.
2. **HTML comment fence repair.** If `-->` is followed by non-whitespace
   text on the same line, insert a newline after `-->` so the trailing
   text reaches the paragraph parser instead of being consumed by the
   CommonMark HTML-block rule (type 2).

Normalization is applied identically to the document body and every card
body. It is not applied to YAML field values.

## 8. Limits

Conforming parsers MUST enforce these limits and MUST surface a parse
error when any is exceeded:

| Limit | Value |
|---|---|
| Document size | 10 MB |
| YAML size per fence | 1 MB |
| YAML nesting depth | 100 |
| Markdown block nesting depth | 100 |
| Field count per fence | 1000 |
| Card count per document | 1000 |

## 9. Errors

Parse errors include:

- Missing frontmatter (no opening `---` on line 1, or no closing `---`
  before EOF).
- Frontmatter missing the `QUILL` key.
- Card fence missing the `CARD` key.
- `QUILL` appearing outside the frontmatter.
- Card `CARD` value failing the `/^[a-z_][a-z0-9_]*$/` pattern.
- Invalid YAML inside any fence.
- Use of a reserved key (`BODY`, `CARDS`) as a user-defined field.
- Any §8 limit exceeded.

## 10. References

- [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/)
- [GitHub Flavored Markdown](https://github.github.com/gfm/) (pipe tables,
  strikethrough)
- [`COMMONMARK_DRIFTS.md`](./COMMONMARK_DRIFTS.md) — analysis that produced
  this spec
- [`CARDS.md`](./CARDS.md) — downstream card-handling semantics
- [`PARSE.md`](./PARSE.md) — parser implementation notes
