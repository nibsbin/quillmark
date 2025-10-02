# Markdown to Typst Conversion

This document details the design and implementation of the markdown-to-Typst conversion system in `quillmark-typst/src/convert.rs`.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Public API](#public-api)
4. [Escape Functions](#escape-functions)
5. [Event-Based Conversion Flow](#event-based-conversion-flow)
6. [Markdown Element Handling](#markdown-element-handling)
7. [Implementation Notes and Gotchas](#implementation-notes-and-gotchas)
8. [Examples](#examples)

---

## Overview

The conversion module provides functionality to transform CommonMark markdown into Typst markup language. This is a critical component of the Typst backend, enabling markdown content to be embedded in Typst templates through the `Content` filter.

**Key Design Principles:**

* **Event-based parsing** using `pulldown_cmark` for robust markdown parsing
* **Character escaping** to handle Typst's reserved characters
* **Stateful conversion** to manage context like list nesting and formatting
* **Minimal output** that leverages Typst's natural text flow where possible

---

## Architecture

The conversion system consists of three main components:

```
┌─────────────────┐
│  mark_to_typst  │  Public entry point
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   push_typst    │  Core conversion logic (private)
└────────┬────────┘
         │
         ├──► escape_markup()  (for text content)
         └──► escape_string()  (for string literals)
```

### Data Flow

1. **Input**: Raw markdown string
2. **Parse**: Create `pulldown_cmark::Parser` with strikethrough support
3. **Convert**: Process event stream, building Typst markup string
4. **Output**: Complete Typst markup ready for compilation

---

## Public API

### `mark_to_typst(markdown: &str) -> String`

Primary conversion function that transforms markdown into Typst markup.

**Parameters:**
- `markdown`: Input markdown string (CommonMark-compliant)

**Returns:**
- Typst markup string suitable for compilation or embedding

**Features Enabled:**
- Strikethrough support via `Options::ENABLE_STRIKETHROUGH`

**Example:**
```rust
use quillmark_typst::convert::mark_to_typst;

let markdown = "This is **bold** and _italic_ text.";
let typst = mark_to_typst(markdown);
// Output: "This is *bold* and _italic_ text.\n\n"
```

---

## Escape Functions

### `escape_markup(s: &str) -> String`

Escapes text content for safe use in Typst markup context. This function must be applied to all user-provided text to prevent interpretation of special characters.

**Characters Escaped:**
- `\` → `\\` (backslash - **must be first**)
- `*` → `\*` (bold/strong markers)
- `_` → `\_` (emphasis markers)
- `` ` `` → ``\` `` (inline code markers)
- `#` → `\#` (headings and Typst functions)
- `[` → `\[` (link/reference delimiters)
- `]` → `\]` (link/reference delimiters)
- `$` → `\$` (math mode)
- `<` → `\<` (angle brackets)
- `>` → `\>` (angle brackets)
- `@` → `\@` (references)

**Critical Note:** Backslash must be escaped first to prevent double-escaping of subsequent escape sequences.

### `escape_string(s: &str) -> String`

Escapes text for embedding in Typst string literals (within quotes). Used primarily for filter outputs and JSON injection.

**Characters Escaped:**
- `\` → `\\`
- `"` → `\"`
- `\n` → `\n`
- `\r` → `\r`
- `\t` → `\t`
- Control characters → `\u{...}` (Unicode escape sequences)

**Use Case:** When wrapping Typst markup in `eval()` calls or embedding in JSON structures.

---

## Event-Based Conversion Flow

The `push_typst` function processes a stream of markdown events from `pulldown_cmark::Parser`. It maintains conversion state and builds the output string incrementally.

### State Management

The converter maintains three pieces of state:

1. **`end_newline: bool`** - Tracks whether we're currently at the end of a line (used to avoid duplicate newlines)
2. **`list_stack: Vec<ListType>`** - Stack tracking nested list contexts (bullet vs. ordered)
3. **`in_list_item: bool`** - Tracks whether we're inside a list item (affects paragraph spacing)

### List Type

```rust
enum ListType {
    Bullet,   // Unordered list (markdown `-` → Typst `+`)
    Ordered,  // Ordered list (markdown `1.` → Typst `1.`)
}
```

**Important:** Markdown uses `-` for bullet lists, but Typst uses `+`. This conversion is handled automatically.

---

## Markdown Element Handling

The converter handles markdown events in a match expression, processing both start and end tags for structural elements.

### Text Formatting

| Markdown | Typst | Implementation |
|----------|-------|----------------|
| `**bold**` | `*bold*` | `Tag::Strong` → `*`, `TagEnd::Strong` → `*` |
| `*italic*` or `_italic_` | `_italic_` | `Tag::Emphasis` → `_`, `TagEnd::Emphasis` → `_` |
| `~~strike~~` | `#strike[strike]` | `Tag::Strikethrough` → `#strike[`, `TagEnd::Strikethrough` → `]` |
| `` `code` `` | `` `code` `` | `Event::Code` → wrap in backticks |

### Paragraphs

**Outside Lists:**
- `Tag::Paragraph` → Add newline if not at start of line
- `TagEnd::Paragraph` → Two newlines for paragraph separation

**Inside Lists:**
- `Tag::Paragraph` → No extra spacing (natural flow within list items)
- `TagEnd::Paragraph` → No extra spacing

This dual behavior prevents excessive whitespace in list items while maintaining proper paragraph separation elsewhere.

### Lists

**Unordered Lists:**
```markdown
- Item 1
- Item 2
```
↓
```typst
+ Item 1
+ Item 2
```

**Ordered Lists:**
```markdown
1. First
2. Second
```
↓
```typst
1. First
1. Second
```

**Note:** Typst auto-numbers ordered lists, so we always use `1.`

**Nested Lists:**
- Each nesting level adds 2-space indentation
- List stack tracks current nesting depth
- Example:
  ```typst
  + Level 1
    + Level 2
      + Level 3
  ```

### Links

```markdown
[Link text](https://example.com)
```
↓
```typst
#link("https://example.com")[Link text]
```

**Implementation:**
- `Tag::Link` → `#link("url")[`
- Link text (as markdown events)
- `TagEnd::Link` → `]`
- URL is escaped with `escape_markup()` to handle special characters

### Line Breaks

| Markdown | Typst | Event |
|----------|-------|-------|
| Two spaces + newline | `\n` (hard break) | `Event::HardBreak` |
| Single newline | ` ` (space) | `Event::SoftBreak` |

This preserves markdown's distinction between soft line wrapping and explicit line breaks.

### Text Content

- `Event::Text` → Escaped with `escape_markup()` and appended
- Updates `end_newline` based on whether text ends with newline

### Unsupported Elements

The following markdown features are intentionally not implemented (per requirements):

- HTML tags
- Math expressions (raw)
- Footnotes
- Tables
- Images (to be handled separately by asset system)
- Headings (template controls structure)
- Block quotes
- Code blocks

These are either handled by the template system or not required for the current use case.

---

## Implementation Notes and Gotchas

### Character Escaping Order

**Critical:** Backslash must be escaped first in `escape_markup()`:

```rust
s.replace('\\', "\\\\")  // MUST BE FIRST
 .replace('*', "\\*")    // Then other chars
 .replace('_', "\\_")
 // ...
```

If other replacements come first, you'll double-escape their backslashes.

### List Item Spacing

The `in_list_item` flag prevents paragraphs within list items from adding extra newlines. Without this:

```markdown
- Item with

  multiple paragraphs
```

Would produce excessive spacing in Typst. The flag ensures natural text flow within list items.

### List Marker Conversion

Markdown's `-` for bullet lists becomes Typst's `+`:

```rust
ListType::Bullet => output.push_str(&format!("{}+ ", indent))
```

This is because `-` in Typst is used for different purposes (like ranges and negative numbers).

### Ordered List Numbering

Typst automatically numbers ordered lists, so we always emit `1.`:

```rust
ListType::Ordered => output.push_str(&format!("{}1. ", indent))
```

Typst will render: 1., 2., 3., etc. automatically.

### Newline Management

The `end_newline` flag prevents duplicate newlines:

```rust
if !end_newline {
    output.push('\n');
    end_newline = true;
}
```

This ensures clean output without excessive blank lines.

### List Stack Depth Calculation

Indentation for nested lists uses:

```rust
let indent = "  ".repeat(list_stack.len().saturating_sub(1));
```

The `saturating_sub(1)` prevents underflow and starts indentation at the second level (first level has no indent).

---

## Examples

### Basic Text Formatting

**Input:**
```markdown
This is **bold**, _italic_, and ~~strikethrough~~ text.
```

**Output:**
```typst
This is *bold*, _italic_, and #strike[strikethrough] text.

```

### Lists

**Input:**
```markdown
- Item 1
- Item 2
  - Nested item
- Item 3
```

**Output:**
```typst
+ Item 1
+ Item 2
  + Nested item
+ Item 3

```

### Mixed Content

**Input:**
```markdown
A paragraph with **bold** and a [link](https://example.com).

Another paragraph with `inline code`.

- A list item
- Another item
```

**Output:**
```typst
A paragraph with *bold* and a #link("https://example.com")[link].

Another paragraph with `inline code`.

+ A list item
+ Another item

```

### Escaping Special Characters

**Input:**
```markdown
Typst uses * for bold and # for functions.
```

**Output:**
```typst
Typst uses \* for bold and \# for functions.

```

---

## Integration with Quillmark

The `mark_to_typst` function is used by the `Content` filter in `filters.rs`:

```rust
pub fn content_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    // ... value extraction ...
    let markup = mark_to_typst(&content);
    Ok(Value::from_safe_string(format!(
        "eval(\"{}\", mode: \"markup\")",
        escape_string(&markup)
    )))
}
```

This allows markdown body content to be embedded in Typst templates:

```typst
= {{ title | String }}

{{ body | Content }}
```

The two-stage escaping (markup → string) ensures safe evaluation in the Typst context.

---

## Testing

When testing the conversion, consider:

1. **Character escaping** - Ensure all Typst special characters are properly escaped
2. **List nesting** - Test multiple levels of nested lists (both bullet and ordered)
3. **Mixed content** - Combine various markdown features in one document
4. **Edge cases** - Empty strings, consecutive formatting markers, etc.
5. **Newline handling** - Verify no excessive blank lines in output

Example test structure:

```rust
#[test]
fn test_markdown_to_typst_conversion() {
    let markdown = "**bold** and _italic_";
    let typst = mark_to_typst(markdown);
    assert_eq!(typst, "*bold* and _italic_\n\n");
}
```

---

## Future Enhancements

Potential areas for extension:

1. **Headings** - Convert markdown headings to Typst heading syntax
2. **Code blocks** - Support fenced code blocks with language hints
3. **Block quotes** - Convert `>` quotes to Typst quote blocks
4. **Images** - Integration with asset system for image embedding
5. **Tables** - Support markdown table syntax
6. **Custom extensions** - Plugin system for domain-specific markdown features

These would require careful consideration of how they interact with the template system and Quillmark's overall architecture.
