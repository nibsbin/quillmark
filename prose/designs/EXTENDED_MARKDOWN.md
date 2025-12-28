# Quillmark Extended Markdown

**Status:** Draft Standard
**Editor:** Quillmark Team

This document describes the **Quillmark Extended Markdown** format. It is a strict superset of CommonMark designed to embed structured metadata blocks alongside standard content. This enables a "document as database" model where files serve as reliable data sources while remaining human-readable.

## Document Structure

A Extended Markdown file is composed of **Segments**. A segment is either a **Metadata Block** followed by **Body Content**, or (in the case of the first segment only) just Body Content.

### 1. Metadata Blocks

A metadata block is delimited by lines containing exactly three hyphens (`---`).

```markdown
---
key: value
---
```

**Key Rules:**
*   **Delimiters:** The `---` marker must form its own line with no leading/trailing whitespace.
*   **Line Endings:** Both Unix (`\n`) and Windows (`\r\n`) line endings are supported.
*   **Exclusivity:** `---` is reserved for metadata and is never treated as a setext header underline.
*   **Horizontal Rule Exception:** A `---` line preceded by a blank line AND followed by a blank line is treated as a horizontal rule in the body content (not a metadata delimiter).
*   **Context:** `---` markers inside fenced code blocks are ignored.

**Content:**
The content inside the block is standard YAML 1.2.
*   **Whitespace Normalization:** Content that contains only whitespace (spaces, tabs, newlines) is treated as empty.
*   **Custom Tags:** Custom YAML tags (like `!fill`) are silently stripped during parsing. For example, `key: !fill value` becomes `key: "value"`. This ensures the data model remains simple JSON.
*   **Recursion Limit:** Nesting is limited to 100 levels to prevent stack overflows.
*   **Reserved Keys:** `BODY` and `CARDS` are reserved system keys and cannot be used in the YAML.

### 2. Body Content

The text following a metadata block is the "Body". It captures everything up to the next metadata block or the end of the file.
*   Whitespace is preserved exactly as written.
*   If two blocks are adjacent, the body between them is an empty string.

## Data Model

The parsed document results in a flat JSON structure:

```typescript
interface Document {
  // Global fields from the first block (if it's not a card)
  [key: string]: any;

  // Reserved fields filled by the parser
  BODY: string;       // The content of the main/global body
  CARDS: Card[];      // List of all named card blocks
}

interface Card {
  CARD: string;       // The type of card (e.g. "section", "profile")
  BODY: string;       // The content associated with this card
  [key: string]: any; // Other fields defined in the block
}
```

**Block Logic:**
*   **Global Block:** The first block in the file is the "Global" block, unless it contains a `CARD` key.
*   **Card Blocks:** Any block containing a `CARD` key is added to the `CARDS` array.
*   **Validity:** Any block after the first one *must* have a `CARD` key.
*   **CARDS Array:** The `CARDS` field is always present in the parsed document, even if empty (as an empty array `[]`).
*   **Name Collisions:** Field names in the global block and `CARD` type names can overlap without error. For example, a global field `items` and a card type `CARD: items` can coexist.

## Markdown Support

Quillmark supports a specific subset of CommonMark to ensure security and consistency.

### Supported Features
*   **Headings:** ATX-style only (`# Heading`).
*   **Text:** Paragraphs, Bold (`**`), Italic (`*`), Strike (`~~`), Underline (`__`).
*   **Lists:** Ordered and unordered.
*   **Links:** Standard `[text](url)`.
*   **Code:** Inline code and Fenced Code Blocks.
    *   **Strict Fence Rules:** Only exactly three backticks (```) are recognized as code fence delimiters.
    *   Tildes (`~~~`) are NOT treated as code fences.
    *   Four or more backticks (````) are NOT treated as code fences.

### Unsupported Features
These features are intentionally ignored or rendered as plain text:
*   **Thematic Breaks:** `***`, `___`, `---` (ignored).
*   **Images:** `![alt](src)` (ignored).
*   **HTML:** Raw HTML tags are ignored, except for comments.
*   **Complex formatting:** Tables, Math, Footnotes, Blockquotes.

### HTML Comments
Standard `<!-- comments -->` are supported as non-rendering content. Nested comments are handled safely.

## System Limits

To ensure performance and stability, the system enforces the following hard limits:

*   **Max Input Size:** 10 MB
*   **Max YAML Size:** 1 MB per block
*   **Max YAML Depth:** 100 levels
*   **Max Item Count:** 1000 fields or cards
