# Quillmark Extended Markdown

**Status:** Draft Standard
**Editor:** Quillmark Team

This document describes the **Quillmark Extended Markdown** format. It is a strict superset of CommonMark designed to embed structured metadata blocks alongside standard content. This enables a "document as database" model where files serve as reliable data sources while remaining human-readable.

## Document Structure

A Quillmark document is composed of one required **Global Block** (with the `QUILL` key), optional additional **Card Blocks**, and **Body Content** sections between them.

### 1. Metadata Blocks

A metadata block is delimited by lines containing exactly three hyphens (`---`).

```markdown
---
QUILL: my_quill
key: value
---
```

**Key Rules:**
*   **Delimiters:** The `---` marker must appear on its own line with no leading or trailing whitespace.
*   **Line Endings:** Both Unix (`\n`) and Windows (`\r\n`) line endings are supported.
*   **Always a delimiter:** `---` is always treated as a metadata block delimiter — never as a setext heading underline or thematic break, regardless of surrounding blank lines.
*   **Context:** `---` markers inside fenced code blocks are ignored.

**Content:**
The content inside the block is parsed as YAML.
*   **Whitespace Normalization:** Content that contains only whitespace (spaces, tabs, newlines) is treated as empty.
*   **Recursion Limit:** YAML nesting is limited to 100 levels to prevent stack overflows.
*   **Reserved Keys:** `BODY`, `CARDS`, `QUILL`, and `CARD` are reserved system keys and cannot be used as user-defined YAML fields.

### 2. Body Content

The text following a metadata block is the "Body". It captures everything up to the next metadata block or the end of the file.
*   Whitespace is preserved exactly as written.
*   If two blocks are adjacent, the body between them is an empty string.

## Data Model

The parsed document results in a flat field map:

```typescript
interface Document {
  // User-defined global fields from the first block
  [key: string]: any;

  // Reserved fields filled by the parser
  BODY: string;       // The content of the main/global body
  CARDS: Card[];      // List of all card blocks (always present, may be empty)
}

interface Card {
  CARD: string;       // The card type value (e.g. "section", "profile")
  BODY: string;       // The body content associated with this card
  [key: string]: any; // Other fields defined in the card block
}
```

**Block Logic:**
*   **Global Block:** The first block must contain the `QUILL` key (specifying the template). Any other fields in that block become global document fields.
*   **Card Blocks:** Any subsequent block must contain a `CARD` key whose value names the card type. These are collected into the `CARDS` array.
*   **Validity:** Any block after the first one *must* have a `CARD` key. Using `QUILL` in a non-first block is an error.
*   **CARDS Array:** Always present in the parsed document, even if empty (`[]`).
*   **Card name pattern:** `CARD` values must match `[a-z_][a-z0-9_]*`.
*   **Name Collisions:** Global field names and `CARD` type values can overlap without error.

## Markdown Support

Quillmark supports a specific subset of CommonMark to ensure security and consistency.

### Supported Features
*   **Headings:** ATX-style only (`# Heading`).
*   **Text:** Paragraphs, Bold (`**`), Italic (`*`), Strike (`~~`), Underline (`__`).
*   **Lists:** Ordered and unordered.
*   **Links:** Standard `[text](url)`.
*   **Line breaks:** `<br>`, `<br/>`, and `<br />` are converted to hard line breaks.
*   **Code:** Inline code and Fenced Code Blocks.
    *   **Fenced blocks:** CommonMark rules apply: lines of three or more matching backticks or tildes open and close a fence; the closing run must be at least as long as the opening run. Metadata `---` delimiters inside a fenced block are ignored (see **Context** under Metadata Blocks).
*   **Tables:** GFM-style pipe tables with optional column alignment.

### Unsupported Features
These features are intentionally ignored (silently dropped) during rendering:
*   **Setext Headings:** Underline-style headings using `===` or `---` are NOT supported. Only ATX-style headings (`# Heading`) are recognized.
*   **Thematic Breaks:** `***`, `___`, `---` are dropped.
*   **Images:** `![alt](src)` is dropped.
*   **Blockquotes:** `>` quoted blocks are dropped.
*   **Raw HTML:** HTML tags other than `<br>` are stripped. HTML comments (`<!-- -->`) pass through as non-rendering content.
*   **Complex formatting:** Math, Footnotes.

### Input Normalization
Before parsing, the body content is normalized:
*   **Bidi stripping:** Unicode bidirectional control characters (U+061C, U+200E–U+200F, U+202A–U+202E, U+2066–U+2069) are removed to prevent delimiter-recognition attacks.
*   **HTML comment fences:** Text immediately following `-->` on the same line is moved to the next line to prevent content loss.

## System Limits

To ensure performance and stability, the system enforces the following hard limits:

*   **Max Input Size:** 10 MB
*   **Max YAML Size:** 1 MB per block
*   **Max YAML Depth:** 100 levels
*   **Max Nesting Depth:** 100 levels (markdown rendering)
*   **Max Item Count:** 1000 fields or cards
