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

### Links and Images

```markdown
[Link text](https://example.com)
![Alt text](image.png)
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
---
```

Note: To use `---` as a horizontal rule (not a metadata delimiter), ensure there are blank lines both above and below it.

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

### Scoped Metadata Blocks

Use the special `SCOPE` key to create reusable metadata sections:

```markdown
---
title: Main Document
---

# Introduction

Some content here.

---
SCOPE: author
name: Alice
affiliation: University
---

More content about Alice's research.

---
SCOPE: author
name: Bob
affiliation: Institute
---

Content about Bob's contributions.
```

The scoped blocks are collected into an array accessible as `author`:

```python
# Access in template or code
authors = parsed.get_field("author")
# Returns: [{"name": "Alice", "affiliation": "University"}, 
#           {"name": "Bob", "affiliation": "Institute"}]
```

### QUILL Key (Reserved)

The `QUILL` key is reserved for future use and specifies which Quill template to use for a section.

## Body Content

The document body (everything after frontmatter) is stored under the special field `body` and can be accessed in templates:

```jinja
#{{ body | Content }}
```

## Validation

Frontmatter can be validated against JSON schemas defined in your Quill's `Quill.toml`:

```toml
[fields]
title = { description = "Document title", type = "str" }
author = { description = "Author name", type = "str", default = "Anonymous" }
date = { description = "Publication date", type = "str" }
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
