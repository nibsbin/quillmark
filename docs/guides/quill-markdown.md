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
SCOPE: products
name: Widget
price: 19.99
---

Widget description.

---
SCOPE: products
name: Gadget
price: 29.99
---

Gadget description.
```

The scoped blocks are collected into an array accessible as `products`:

```python
# Access in template or code
products = parsed.get_field("products")
# Returns: [{"name": "Widget", "price": 19.99, "body": "Widget description."}, 
#           {"name": "Gadget", "price": 29.99, "body": "Gadget description."}]
```

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

- **SCOPE key**: Creates collections - blocks with same scope name are aggregated into arrays
- **QUILL key**: Specifies which quill template to use (defaults to `__default__` if not specified)
- **Scope names**: Must match `[a-z_][a-z0-9_]*` pattern
- **Reserved names**: Cannot use `body` as scope name
- **Single global**: Only one block without SCOPE/QUILL allowed
- **No collisions**: Global field names cannot conflict with scope names
- **Horizontal rule disambiguation**: `---` with blank lines above AND below is treated as markdown horizontal rule
- **Each scoped block includes a `body` field**: Content between metadata blocks is stored in the `body` field

## Body Content

The document body (everything after frontmatter) is stored under the special field `body` and can be accessed in templates:

```jinja
#{{ body | Content }}
```

## Metadata Object

Quillmark provides a special `__metadata__` field in templates that contains all frontmatter fields except `body`. This is useful for iterating over metadata or separating content from metadata:

```jinja
{% for key, value in __metadata__ %}
  {{ key }}: {{ value }}
{% endfor %}
```

The `__metadata__` field is automatically created and includes all fields from the frontmatter (including SCOPE-based collections), but excludes the `body` field. You can still access fields individually at the top level (e.g., `{{ title }}`), but `__metadata__` provides convenient metadata-only access.

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
