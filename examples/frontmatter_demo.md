---
title: QuillMark Frontmatter Demo  
author: QuillMark Team
date: 2024-01-01
version: 1.0
tags:
  - demo
  - frontmatter
  - yaml
  - markdown
description: >
  This document demonstrates QuillMark's ability to parse 
  YAML frontmatter and separate it from markdown content.
---

# Welcome to QuillMark Frontmatter Demo

This document demonstrates the new frontmatter parsing capabilities of QuillMark.

## How It Works

QuillMark now parses **YAML frontmatter** at the beginning of markdown documents. The frontmatter fields are extracted into a dictionary, while the markdown body is converted to the target format.

### Key Features

- **YAML Parsing**: Supports standard YAML syntax in frontmatter
- **Field Extraction**: All frontmatter fields are available as dictionary entries  
- **Body Separation**: Only the markdown body (not frontmatter) gets converted
- **Backward Compatible**: Documents without frontmatter work as before

## Example Usage

When you process this document with QuillMark:

1. The **frontmatter** is parsed into fields like `title`, `author`, `tags`, etc.
2. The **body** (this markdown content) is converted to the target format
3. Backends can access both the frontmatter dictionary and the converted body

### Supported YAML Types

- **Strings**: `title: "My Document"`
- **Numbers**: `version: 1.0` 
- **Arrays**: `tags: [demo, yaml]`
- **Objects**: `author: {name: "John", email: "john@example.com"}`
- **Multi-line**: Using `>` or `|` syntax

## Implementation

The parsing logic is implemented in `quillmark-core`:

```rust
use quillmark_core::{parse_markdown, BODY_FIELD};

let parsed = parse_markdown(markdown_content)?;
let frontmatter_title = parsed.get_field("title");
let body_content = parsed.body();
```

This enables clean separation of concerns between document metadata and content.

---

*This document was generated to demonstrate QuillMark's frontmatter capabilities.*