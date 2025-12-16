---
title: Extended Metadata Demo
author: Quillmark Team
version: 1.0
---

This document demonstrates the new **extended YAML metadata standard** for Quillmark.

The extended standard allows you to define inline metadata sections throughout your document using reserved keys.

## Features Demonstrated

---
CARD: features
name: Tag Directives
status: implemented
---

Use `CARD: tag_name` syntax to create collections of related items. Each tagged block creates an entry in an array.

---
CARD: features
name: Structured Content
status: implemented
---

Break your document into logical sections with their own metadata. Perfect for catalogs, lists, and structured documents.

---
CARD: features
name: Backward Compatible
status: stable
---

Documents without tag directives continue to work exactly as before. No breaking changes!

## Use Cases

---
CARD: use_cases
category: Documentation
example: Technical specifications with multiple sections
---

Perfect for API documentation, user manuals, and technical guides where you need structured metadata for each section.

---
CARD: use_cases
category: Content Management
example: Product catalogs, blog posts, portfolios
---

Ideal for content-heavy sites where each item needs its own metadata (price, category, tags, etc.).

## Technical Details

- **Tag pattern**: `[a-z_][a-z0-9_]*`
- **Blank lines**: Allowed within metadata blocks
- **Horizontal rules**: `---` with blank lines both above and below
- **Reserved names**: Cannot use `body` as tag directive
- **Collections**: Same tag name creates array of objects
