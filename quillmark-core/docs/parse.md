# Parsing Module

Parsing functionality for markdown documents with YAML frontmatter.

## Overview

The `parse` module provides the `decompose` function for parsing markdown documents
and the `ParsedDocument` type for accessing parsed content.

## Key Types

- **`ParsedDocument`**: Container for parsed frontmatter fields and body content
- **`BODY_FIELD`**: Constant for the field name storing document body

## Examples

### Basic Parsing

```rust
use quillmark_core::decompose;

let markdown = r#"---
title: My Document
author: John Doe
---

# Introduction

Document content here.
"#;

let doc = decompose(markdown).unwrap();
let title = doc.get_field("title")
    .and_then(|v| v.as_str())
    .unwrap_or("Untitled");
```

### Extended Metadata with Tags

```rust
use quillmark_core::decompose;

let markdown = r#"---
catalog_title: Product Catalog
---

# Products

---
!products
name: Widget
price: 19.99
---

A versatile widget for all occasions.
"#;

let doc = decompose(markdown).unwrap();

// Access tagged collections
if let Some(products) = doc.get_field("products")
    .and_then(|v| v.as_sequence())
{
    for product in products {
        let name = product.get("name").and_then(|v| v.as_str()).unwrap();
        let price = product.get("price").and_then(|v| v.as_f64()).unwrap();
        println!("{}: ${}", name, price);
    }
}
```

## Error Handling

The `decompose` function returns errors for:
- Malformed YAML syntax
- Unclosed frontmatter blocks
- Multiple global frontmatter blocks
- Invalid tag directive syntax
- Reserved field name usage
- Name collisions

See [PARSE.md](../PARSE.md) for comprehensive documentation of the Extended YAML Metadata Standard.
