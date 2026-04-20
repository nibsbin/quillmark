# Quillmark Markdown vs CommonMark

**Status:** Analysis — input for a future Quillmark Markdown specification
**Goal:** Quillmark Markdown should be a **superset** of
[CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/). Every valid
CommonMark document should parse to an equivalent AST; additional syntax may
be layered on top.

This document inventories the current parsing behaviour and evaluates each
deviation under the superset goal. Each item is classified as:

- **Extension** — adds syntax or semantics beyond CommonMark without
  contradicting it. Preserve.
- **Conflict** — reassigns or suppresses syntax CommonMark gives meaning to.
  Must be reconciled before the superset property can be claimed.
- **Gap** — CommonMark feature that is unimplemented. If *YAGNI* — deferrable
  until a real use case appears. If *needed* — fill before shipping the
  standard.

## Pipeline

| Stage | Crate / file | Notes |
|---|---|---|
| 1. Frontmatter split | `quillmark-core` — `crates/core/src/parse.rs` | Splits on bare `---` lines into YAML blocks + body strings. |
| 1a. Normalize | `crates/core/src/normalize.rs` | Strips Unicode bidi controls; repairs HTML-comment fences. |
| 2. Body render | `quillmark-typst` — `crates/backends/typst/src/convert.rs` | `pulldown-cmark` 0.13 with `ENABLE_STRIKETHROUGH` + `ENABLE_TABLES`, events mapped to Typst markup. |

Only stage 2 uses a CommonMark parser. Stage 1 interprets `---` before
`pulldown-cmark` ever sees the text.

---

## Conflicts (must resolve to be a superset)

### C1. `---` reserved as frontmatter delimiter

**Current behaviour** — any line containing exactly `---` opens or closes a
YAML metadata block (`parse.rs:267-500`), even mid-document.

**Necessary?** The `QUILL` / `CARD` data model is core to Quillmark and must
survive. The *blanket* reservation is what causes the conflict, not the
feature itself.

**Collateral damage**
- Setext H2 (`Heading\n---`) — CommonMark §4.3 — unreachable.
- Thematic break `---` — CommonMark §4.1 — unreachable.

**Resolution (adopted)** — keep `---` as the single metadata fence, but make
fence detection contextual so unrelated `---` lines fall through to
CommonMark. Two rules govern whether a `---` line opens a fence:

1. **Sentinel rule.** A `---` / `---` pair is a metadata fence only if the
   content between, stripped of leading whitespace, begins with `QUILL:`
   (first block of the document) or `CARD:` (any subsequent block). Any
   `---` pair whose content does not start with the appropriate sentinel
   falls through — both delimiters become ordinary CommonMark tokens. This
   is the existing semantic constraint (first block *must* have `QUILL`,
   others *must* have `CARD`) promoted to a lexical precondition.
2. **Leading-blank rule.** A `---` line opens a fence only if the line
   immediately above it is blank, or the `---` sits at line 1. A `---`
   directly below a non-blank paragraph line stays a setext H2 underline
   per CommonMark §4.3.

With both rules in force:

- `Heading\n---\n` → setext H2 (rule 2 fails).
- Blank line, `---`, blank line → CommonMark thematic break (rule 1 fails
  — nothing after).
- Blank line, `---`, `QUILL: resume`, `---` → frontmatter block.
- Blank line, `---`, `CARD: profile`, `---` → card block.

**Terminology note.** With C1 resolved, "the first block" is recognised for
what it already is: *frontmatter*. It carries `QUILL`, the document's global
fields, and the top-level `BODY` (prose between frontmatter close and the
first card fence). It is the mandatory entrypoint that the Typst backend
binds to its document function — not a "main card." Card blocks remain
supplementary records in `CARDS[]`.

**Parser cost.** On seeing a candidate `---` line, the tokenizer looks back
one line (blank check) and scans forward for a matching `---` plus the first
non-blank content line (sentinel check). Bounded lookahead.

**New failure mode.** A user mistyping `Card:` (wrong case) or
`CARDS:` turns an intended card into two thematic breaks with literal YAML
text between them. Surface via a linter warning: "a `---` pair surrounding a
`<word>:` line that isn't `QUILL`/`CARD` is usually a typo."

### C2. `__text__` renders as underline

**Current behaviour** — `convert.rs:264-280` inspects the source and maps
`__`-bounded strong spans to `#underline[...]` instead of `#strong[...]`.

**Resolution (adopted)** — keep as an intentional deviation. Discord uses
the same mapping (`__` = underline, `**` = bold), so the pattern has
real-world precedent and author muscle memory. The spec must declare this
explicitly: Quillmark Markdown is a superset of CommonMark *except* that
`__` is rebound from strong to underline. Authors who want strong inside
an underscore word reach for `**`.

### C3. Intraword `__` and `~~` permitted

**Current behaviour** — `convert.rs:1048-1230` preprocesses the source,
replacing intraword `__…__` and `~~…~~` runs with placeholder characters
before handing off to `pulldown-cmark`, which would otherwise (correctly)
treat them as literal per CommonMark §6.2 / GFM delimiter rules.

**Resolution (adopted)** — **remove the preprocessor entirely.**
Delimiter-run semantics for `*`, `_`, `**`, `__`, and `~~` revert to
CommonMark and GFM. `__` is still rebound to underline per C2, but
word-bounded per the standard rule. Intraword and arbitrary-range
underline is served by the `<u>…</u>` HTML exception in C6.

Net effect: zero delimiter-run bends. The `__`-as-underline deviation is
scoped purely to target semantics, not to where the delimiter matches.

### C4. Thematic break dropped for `***` and `___`

**Current behaviour** — `pulldown-cmark` emits `Event::Rule`; `convert.rs`
has no handler, so the event is silently discarded.

**Necessary?** No — this is just unfinished. `---` is tangled up in C1, but
`***` and `___` have no excuse. Map to Typst `#line(length: 100%)` (see
`CONVERT.md:906-947`). Effectively a gap, classified here because CommonMark
requires it.

### C5. Block quotes dropped

**Current behaviour** — no handler for `Tag::BlockQuote`. Outer wrapping
disappears; inline text may still leak out as paragraphs.

**Necessary?** No. Map to `#quote(block: true)[...]` (Typst has native
support). Nested quotes can flatten or preserve depth per
`CONVERT.md:616-693`.

### C6. Raw HTML stripped

**Current behaviour** — all HTML events dropped except `<br>` family, which
is rewritten to `HardBreak` (`convert.rs:969-975`).

**Resolution (adopted)** — conscious deviation, with **`<u>…</u>` as the
single allowlisted tag**. Typst has no HTML renderer and arbitrary
passthrough creates an injection surface for downstream HTML-producing
tooling, so all HTML is dropped — *except* `<u>`, which renders as
`#underline[...]`. `<u>` is admitted because no CommonMark-native syntax
covers arbitrary-range (including intraword) underline; `__` as underline
(C2) follows standard word-bounded delimiter rules.

`<br>` recognition is removed. Authors use CommonMark-native hard breaks
(trailing two spaces plus newline, or trailing `\\` plus newline).

---

## Extensions (keep — these are the point)

### E1. YAML metadata blocks (`QUILL` / `CARD`)

Core Quillmark feature. Reclassified from “conflict” once C1 resolves the
`---` ambiguity.

### E2. Strikethrough `~~text~~`

GFM extension, enabled via `ENABLE_STRIKETHROUGH`. Standard superset
material; word-bounded per GFM.

### E3. GFM pipe tables

Enabled via `ENABLE_TABLES`. Mapped to `#table(...)` with column alignment
preserved (`convert.rs:302-340`). Confirmed use-case need.

### E4. `<u>…</u>` for underline

The single HTML tag admitted through the raw-HTML-drop rule (see C6).
Provides intraword and arbitrary-range underline that word-bounded `__`
(C2) cannot express.

### E5. Bidi control stripping (pre-parse normalization)

`normalize.rs:71-133` removes U+061C / U+200E-F / U+202A-E / U+2066-9 before
parsing. Defensive; CommonMark is silent on invisible characters. This is
preprocessing, not a syntax extension — authors never encounter it.

### E6. HTML comment fence repair (pre-parse normalization)

`normalize.rs:135-247` inserts a newline after `-->` when text trails on
the same line, so trailing text reaches the paragraph parser instead of
being swallowed by the HTML block rule. Preprocessing, not a syntax
extension.

---

## Gaps (CommonMark features not yet implemented)

Each row is either fillable cheaply or deferrable under YAGNI.

| # | Feature | Impl effort | YAGNI? | Recommendation |
|---|---|---|---|---|
| G1 | Images `![alt](src)` | High — requires asset system / resolver | **No, but defer** — images are core CommonMark, but without the asset system the Typst output has nothing to point at | Declare required for v1 of the standard; implement when the asset resolver lands |
| G2 | Thematic break `***` / `___` | Trivial — `#line(length: 100%)` | No | Fill (C4) |
| G3 | Block quote `>` | Low — `#quote(block: true)` | No | Fill (C5) |
| G4 | Link titles `[x](url "title")` | Trivial — threaded through to Typst tooltip or ignored | **Yes** | Defer; document as accepted but not rendered |
| G5 | Indented code block | Already handled via pulldown-cmark | — | No drift; just note in spec |
| G6 | Autolink `<http://…>` | Already handled | — | No drift |
| G7 | Entity / numeric char refs | Already handled by pulldown-cmark | — | Verify in tests |
| G8 | Backslash escapes of ASCII punctuation | Already handled | — | Verify in tests |
| G9 | Setext headings | Unblocked by C1's leading-blank rule | — | Implement alongside C1 |
| G10 | Hard break via trailing `\\` | Already handled | — | Verify in tests |

### GFM / ecosystem features (beyond CommonMark)

| # | Feature | Recommendation |
|---|---|---|
| GE1 | Task lists `- [ ]` | YAGNI — defer |
| GE2 | Footnotes | YAGNI — defer |
| GE3 | Definition lists | YAGNI — defer |
| GE4 | Math `$…$`, `$$…$$` | Potentially useful (Typst has native math). Defer as a future extension; document that `$` is currently literal, which is a *compatible* stance. |
| GE5 | Autolinked URLs (bare) | YAGNI — defer |

None of these are CommonMark features, so not implementing them does not
break the superset property.

---

## Decision summary

Quillmark Markdown is specified as a superset of CommonMark with exactly
two explicit, documented deviations:

1. `__text__` is rebound from strong to underline (word-bounded, standard
   delimiter rules).
2. Raw HTML is non-rendering, except `<u>…</u>` which renders as underline.

Everything else either matches CommonMark exactly or is an additive
extension (GFM strikethrough, GFM pipe tables). No delimiter-run rules are
bent. `<br>` recognition is removed.

| ID | Status | Action |
|---|---|---|
| C1 | **Resolved** — contextual `---` fence detection (sentinel + leading-blank rules). Restores setext H2 and `---` thematic breaks. Folds "main card" into "frontmatter" naming. | Implement |
| C2 | **Resolved** — `__` = underline, word-bounded per CommonMark delimiter rules. One declared deviation. | Update convert: map `__` to `#underline[...]`; keep standard delimiter rules |
| C3 | **Resolved** — remove the intraword preprocessor entirely. No delimiter-run bends. | Delete the placeholder preprocessor |
| C4 | **Resolved** — implement `***` / `___` thematic breaks as `#line(length: 100%)`. | Implement |
| C5 | **Resolved** — implement block quotes as `#quote(block: true)[...]`. | Implement |
| C6 | **Resolved** — raw HTML is non-rendering, except `<u>…</u>` which maps to `#underline[...]`. `<br>` recognition removed. | Update convert: admit `<u>` only, drop `<br>` special case |
| G1 | **Deferred** — images land with the asset resolver. Required for v1. | Track with asset work |
| G9 | **Unblocked** by C1. Setext headings work once the `---` rules are contextual. | Implement alongside C1 |
| G2 / G3 | Folded into C4 / C5. | — |
| G4, GFM extras, math, footnotes, task/def lists | **Deferred (YAGNI).** Spec classifies each as "parsed but ignored" or "out of scope." | Document in spec |

No open conflicts. Remaining work is implementation + spec authoring.

## References

- `crates/core/src/parse.rs`
- `crates/core/src/normalize.rs`
- `crates/backends/typst/src/convert.rs`
- `prose/designs/MARKDOWN.md` (authoritative spec derived from this analysis)
- `prose/designs/PARSE.md`
- `crates/backends/typst/docs/designs/CONVERT.md`
- `docs/authoring/markdown-syntax.md`
