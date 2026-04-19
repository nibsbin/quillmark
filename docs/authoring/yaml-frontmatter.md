# YAML Frontmatter

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

## Frontmatter Data Types

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

> [!WARNING]
> YAML objects and nested structures are syntactically valid in frontmatter. However, when using schema-driven fields from `Quill.yaml`, Quill currently only supports `type: object` inside `array.items` (typed array rows). Top-level fields declared as `type: object` are not supported. See [Quill.yaml Reference: Field Types](../format-designer/quill-yaml-reference.md#field-types).

## QUILL Key

The `QUILL` key specifies which Quill format to use for rendering:

```markdown
---
QUILL: my-custom-format
title: Document Title
author: Jane Doe
---

# Content
```

If no `QUILL` key is specified, Quillmark uses the `__default__` format provided by the backend (if available).

### Version Selectors

You can pin a specific version of a Quill format using `@version` syntax:

```markdown
---
QUILL: my-format@2.1
title: Document Title
---
```

Supported version selectors:

| Syntax | Meaning |
|--------|---------|
| `format` | Latest version (default) |
| `format@latest` | Latest version (explicit) |
| `format@2` | Latest 2.x.x |
| `format@2.1` | Latest 2.1.x |
| `format@2.1.0` | Exact version 2.1.0 |

## Frontmatter Rules

- `QUILL` can only appear in the first (global) metadata block.
- `BODY` and `CARDS` are reserved field names and cannot be used in frontmatter.
- A document can only have one global metadata block.
- The body content is stored in the special `BODY` field.

## Validation

Frontmatter can be validated against JSON schemas defined in your Quill's `Quill.yaml`:

```yaml
main:
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

When validation is enabled, the parser checks:
- Required fields are present
- Field types match the schema
- Values meet constraints
