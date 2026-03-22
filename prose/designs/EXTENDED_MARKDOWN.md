# Quillmark Extended Markdown

Status: **Implemented** (2026-03-22)  
Scope: Surface syntax for markdown + metadata consumed by the parser and Typst converter.

## Document Structure
- File is a sequence of segments: a metadata block (delimited by `---`) followed by body text.
- `---` is a delimiter **unless** it sits inside a strict ``` fence or has blank lines both above and below (horizontal rule/body text).
- The first block may be global or a card; every later block must be a card.
- Reserved keys: `BODY`, `CARDS`. `QUILL` and `CARD` cannot appear together.
- Card names: `[a-z_][a-z0-9_]*`.

## YAML Rules
- Parsed with `serde_saphyr`; whitespace-only blocks are ignored.
- Custom YAML tags are accepted and stripped.
- Limits: 1 MB YAML per block, depth 100; max 10 MB input; max 1000 cards/fields.

## Parsed Shape
```
BODY: string
CARDS: [{ CARD: "<type>", BODY: string, ... }, ...]   // always present, may be empty
<other fields from first block>
```

## Markdown Subset (Typst backend)
- ATX headings only; setext headings are suppressed.
- Text: paragraphs, **bold**, *italic*, ~~strike~~, __underline__.
- Lists: ordered + unordered.
- Links: `[text](url)`.
- Code: inline code, fenced code with **exactly** ```; `~~~~` or 4+ backticks are ignored as fences.
- Tables: GFM pipe tables (alignment supported).
- `<br>` becomes a hard line break inside table cells.
- HTML comments are preserved; other raw HTML is ignored.
- Unsupported/treated as text: images, blockquotes, math, footnotes, thematic breaks (`***`, `___`, `---`).

## Quill Tag
- `QUILL: <name[@selector]>` is only valid in the first block. Missing → `__default__@latest`.

See [PARSE.md](./PARSE.md) for parsing logic and [CARDS.md](./CARDS.md) for card semantics.
