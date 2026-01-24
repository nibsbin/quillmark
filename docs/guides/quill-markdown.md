# Quill Markdown

Quillmark extends standard Markdown with YAML frontmatter for structured metadata. This guide covers the Markdown syntax supported by Quillmark.

## Basic Markdown

Quillmark supports standard CommonMark syntax for the document body:

### Headings

```markdown
# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6
```

### Text Formatting

```markdown
**Bold text**
*Italic text*
***Bold and italic***
~~Strikethrough~~
`Inline code`
```

### Lists

Unordered lists:

```markdown
- Item 1
- Item 2
  - Nested item
  - Another nested item
- Item 3
```

Ordered lists:

```markdown
1. First item
2. Second item
3. Third item
```

### Links

```markdown
[Link text](https://example.com)
```

### Code Blocks

````markdown
```python
def hello():
    print("Hello, world!")
```
````

### Blockquotes

```markdown
> This is a blockquote
> It can span multiple lines
```

### Horizontal Rules

```markdown
***
```

or

```markdown
___
```

Note: The `---` syntax is **not available** for horizontal rules as it is reserved for metadata block delimiters. Use `***` or `___` instead.

## YAML Frontmatter

Quillmark documents begin with YAML frontmatter delimited by `---` markers:

```markdown
---
title: My Document
author: John Doe
date: 2025-01-15
tags: ["important", "draft"]
---

# Document content starts here
```

### Frontmatter Data Types

YAML supports various data types:

**Strings:**
```yaml
title: Simple String
quoted: "String with special chars: $%^"
multiline: |
  This is a
  multiline string
```

**Numbers:**
```yaml
count: 42
price: 19.99
```

**Booleans:**
```yaml
published: true
draft: false
```

**Arrays:**
```yaml
tags: ["tech", "tutorial"]
# or
authors:
  - Alice
  - Bob
```

**Objects:**
```yaml
author:
  name: John Doe
  email: john@example.com
```

**Nested Structures:**
```yaml
document:
  metadata:
    title: Complex Doc
    version: 1.0
  settings:
    page_size: A4
    margins: [1, 1, 1, 1]
```

## Extended YAML Metadata

Quillmark supports an Extended YAML Metadata Standard that allows metadata blocks throughout the document, not just at the beginning.

### Card Blocks

Use the special `CARD` key to create reusable metadata sections:

```markdown
---
title: Main Document
---

# Introduction

Some content here.

---
CARD: products
name: Widget
price: 19.99
---

Widget description.

---
CARD: products
name: Gadget
price: 29.99
---

Gadget description.
```

The card blocks are collected into a CARDS array.

### QUILL Key

The `QUILL` key specifies which Quill template to use for rendering:

```markdown
---
QUILL: my-custom-template
title: Document Title
author: Jane Doe
---

# Content
```

If no `QUILL` key is specified, Quillmark uses the `__default__` template provided by the backend (if available).

### Rules for Extended Metadata

- **CARD key**: Creates card blocks - all blocks with CARD keys are collected into a CARDS array
- **QUILL key**: Specifies which quill template to use (defaults to `__default__` if not specified)
- **Card names**: Must match `[a-z_][a-z0-9_]*` pattern
- **Reserved names**: Cannot use `BODY` or `CARDS` in YAML frontmatter
- **Single global**: Only one block without CARD/QUILL allowed
- **QUILL placement**: QUILL key can only appear in the first (global) block, not in inline blocks
- **Horizontal rule**: `---` is reserved for metadata delimiters only. Use `***` or `___` for horizontal rules
- **Each card block includes a `BODY` field**: Content between metadata blocks is stored in the `BODY` field

## Body Content

The document body (everything after frontmatter) is stored under the special field `BODY` and is injected into JSON for backends. For Typst, `transform_fields` converts markdown fields (including `BODY`) to Typst markup strings that you render with `eval-markup(data.BODY)`.

## Validation

Frontmatter can be validated against JSON schemas defined in your Quill's `Quill.yaml`:

```yaml
fields:
  title:
    description: Document title
    type: string
  author:
    description: Author name
    type: string
    default: Anonymous
  date:
    description: Publication date
    type: string
```

When validation is enabled, the parser will check that:
- Required fields are present
- Field types match the schema
- Values meet any constraints

## Best Practices

1. **Use meaningful field names** - Choose descriptive names for frontmatter fields
2. **Provide defaults** - Define sensible defaults in your Quill schema
3. **Keep frontmatter simple** - Complex nested structures can be hard to maintain
4. **Use comments** - YAML supports `#` comments for documentation
5. **Validate early** - Test your frontmatter against your Quill schema

Example with comments:

```yaml
---
# Document metadata
title: Research Paper
author: Dr. Smith

# Publication info
date: 2025-01-15
journal: Science Today

# Categories for indexing
tags:
  - research
  - biology
  - genetics
---
```

## Error Handling

Quillmark provides clear error messages for common issues:

- **Malformed YAML** - Syntax errors in frontmatter
- **Unclosed frontmatter** - Opening `---` without closing `---`
- **Invalid field types** - Type mismatches with schema
- **Missing required fields** - Required fields not provided

Example error:

```
YAML parsing error at line 3: expected ':', found newline
```

## Cross-Platform Support

Quillmark handles both Unix (`\n`) and Windows (`\r\n`) line endings automatically.

## Next Steps

- [Create your own Quill](creating-quills.md)
- [Learn about the Typst backend](typst-backend.md)
- [Explore template filters](creating-quills.md#available-filters)
