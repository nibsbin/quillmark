# Extended YAML Metadata Standard

This document defines the extended markdown syntax for embedding structured metadata in Quillmark documents. It is intended to be authoritative and implementable in any system.

## Definitions

- **Metadata block**: A YAML section delimited by `---` markers
- **Global block**: The frontmatter at the document start (no CARD key)
- **Card block**: A metadata block containing a `CARD` key
- **Body**: Markdown content following a metadata block, up to the next block or end of document

## Metadata Block Syntax

A metadata block begins and ends with a line containing **exactly** `---` (three hyphens, no leading/trailing whitespace, no other content on the line).

```
---
key: value
---
```

### Delimiter Rules

- **`---` is reserved for metadata blocks only** — never treated as a thematic break
- **Exact match required** — `---` with any other characters on the same line is not a delimiter
- **Fenced code blocks** — `---` inside fenced code blocks (`` ``` `` or `~~~`) is not processed as a delimiter
- **No `...` closer** — only `---` closes a metadata block (unlike Pandoc)

## CommonMark Compatibility

### Thematic Breaks (Horizontal Rules)

Use `***` or `___` for horizontal rules. The `---` syntax is not available for thematic breaks.

- ✅ `***` (supported)
- ✅ `___` (supported)
- ❌ `---` (reserved for metadata blocks)

### Setext Headers

**Setext-style headers are not supported.** In standard CommonMark, a line of `---` under text creates an h2 header. This conflicts with metadata block syntax.

- ✅ `# Heading` (ATX-style, supported)
- ❌ `Heading\n---` (setext-style, not supported)

## YAML Subset

Metadata blocks follow **YAML 1.2** with one exception:

- **Tags (`!`) are not supported** — custom and standard tags are ignored

All other YAML 1.2 features are supported, including anchors, aliases, flow/block styles, and multi-line strings.

## Special Keys

| Key | Purpose | Constraints |
|-----|---------|-------------|
| `CARD` | Declares a card block with a named type | Value must match `[a-z_][a-z0-9_]*` |
| `QUILL` | Specifies which quill template to use | Only valid in global block; defaults to `__default__` |
| `BODY` | Reserved for body content | Cannot be used in YAML |

## Document Structure

### No Frontmatter

If a document has no metadata blocks, the entire document content becomes the body with no fields and no cards.

### Global Block (Optional)

The first metadata block in a document, if it lacks a `CARD` key, is the global block. Only one global block is permitted. An empty global block (no YAML content) is valid.

**If the first block contains a `CARD` key**, no global block exists. The document has no global fields, the global body is empty, and that first block becomes the first card.

### Card Blocks

Metadata blocks containing a `CARD` key are aggregated into a `CARDS` array in parse order. A non-global metadata block without a `CARD` key is invalid.

### Body Extraction Rules

Body content is extracted verbatim from the line after a block's closing `---` to the line before the next metadata block (or end of document).

- **Whitespace preservation**: Leading and trailing blank lines are preserved exactly as written
- **Empty body**: If no content exists between blocks, BODY is an empty string (`""`)
- **No trimming**: Implementations must not trim or normalize whitespace

### Flat Structure

Cards are collected into a flat `CARDS` array. Nested or hierarchical card structures are not supported.

## Example

```markdown
---
title: My Document
QUILL: blog_post
---
Main document body.

***

More content after horizontal rule.

---
CARD: section
heading: Introduction
---
Introduction content.

---
CARD: section
heading: Conclusion
---
Conclusion content.
```

**Parsed structure:**
```json
{
  "title": "My Document",
  "QUILL": "blog_post",
  "BODY": "Main document body.\n\n***\n\nMore content after horizontal rule.",
  "CARDS": [
    {"CARD": "section", "heading": "Introduction", "BODY": "Introduction content."},
    {"CARD": "section", "heading": "Conclusion", "BODY": "Conclusion content."}
  ]
}
```

## Validation Rules

1. **Single global block**: Multiple blocks without `CARD` key → error
2. **Reserved field names**: Using `BODY` or `CARDS` in YAML → error
3. **Invalid card name**: Card name not matching `[a-z_][a-z0-9_]*` → error
4. **Conflicting keys**: Both `CARD` and `QUILL` in same block → error
5. **Unclosed block**: Opening `---` without closing `---` → error

## Supported Syntax

The following Markdown features are **supported**:
- **Headings**: ATX-style (`# Heading`)
- **Paragraphs**: Standard
- **Emphasis**: `*italic*`, `**bold**`, `__underline__`, `~~strike~~`
- **Lists**: Ordered and unordered (nested)
- **Links**: `[text](url)`

The following features are **NOT supported** (and will be rendered as plain text or ignored):
- **Images**: Inline `![alt](src)`
- **Block Quotes**: `> quote`
- **Fenced Code Blocks**: ` ``` ` blocks (rendered as plain text)
- **HTML Blocks**: `<div>...</div>`
- **Tables**: GFM tables
- **Math**: `$latex$`
- **Footnotes**: `[^1]`
- **Thematic Breaks**: `***`, `___`, `---` (all ignored or reserved)
