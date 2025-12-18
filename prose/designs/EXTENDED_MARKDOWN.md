# Extended YAML Metadata Standard

This document defines the extended markdown syntax for embedding structured metadata in Quillmark documents. It is intended to be authoritative and implementable in any system.

## Definitions

- **Metadata block**: A YAML section delimited by `---` markers
- **Global block**: The frontmatter at the document start (no CARD key)
- **Card block**: A metadata block containing a `CARD` key
- **Body**: Markdown content following a metadata block, up to the next block or end of document

## Metadata Block Syntax

A metadata block begins and ends with a line containing exactly `---` (three hyphens, no other content).

```
---
key: value
---
```

## Horizontal Rule Disambiguation

A `---` line is treated as a **horizontal rule** (not a metadata block delimiter) if:
- It is NOT at the start of the document, AND
- The preceding line is blank

Otherwise, `---` is treated as a metadata block delimiter.

## Special Keys

| Key | Purpose | Constraints |
|-----|---------|-------------|
| `CARD` | Declares a card block with a named type | Value must match `[a-z_][a-z0-9_]*` |
| `QUILL` | Specifies which quill template to use | Only valid in global block |
| `BODY` | Reserved for body content | Cannot be used in YAML |

## Document Structure

### Global Block (Optional)

The first metadata block in a document, if it lacks a `CARD` key, is the global block. Only one global block is permitted.

### Card Blocks

Metadata blocks containing a `CARD` key are aggregated into a `CARDS` array in parse order.

### Body Content

Content between a metadata block's closing `---` and the next metadata block (or end of document) becomes that block's `BODY` field.

## Example

```markdown
---
title: My Document
QUILL: blog_post
---
Main document body.

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
  "BODY": "Main document body.",
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
