# Markdown Parsing and Decomposition

This document details the markdown parsing and Extended YAML Metadata Standard in Quillmark.

> **Implementation**: `quillmark-core/src/parse.rs`

## Overview

Quillmark uses a **frontmatter-aware markdown parser** that separates YAML metadata from document content.

**Key capabilities:**
- Parse YAML frontmatter delimited by `---` markers
- Support inline metadata sections with SCOPE/QUILL keys (Extended YAML Metadata Standard)
- Aggregate scoped blocks into collections (arrays of objects)
- Extract frontmatter fields into `HashMap<String, QuillValue>`
- Preserve markdown body content separately
- Cross-platform line ending support (`\n` and `\r\n`)
- Horizontal rule disambiguation

## Design Principles

### 1. Separation of Concerns

The parser decomposes markdown documents into:
- **Frontmatter fields**: YAML key-value pairs accessible via `HashMap<String, QuillValue>`
- **Body content**: Raw markdown text stored under the reserved `BODY_FIELD` constant

### 2. Error Handling Strategy

**Strict fail-fast** for malformed YAML:
- **Invalid YAML**: Returns error with descriptive message
- **Unclosed frontmatter**: Returns error if `---` opening exists but closing marker is missing
- **No frontmatter**: Gracefully treats entire content as body (not an error)

### 3. YAML-Only Policy

Only YAML frontmatter is supported. Backends can convert to their native formats via filters.

## Core Data Structures

### ParsedDocument

Stores both frontmatter fields and document body in a single `HashMap<String, QuillValue>`.
- Body is stored under special `BODY_FIELD = "body"` constant
- Private fields enforce access through validated methods

**Public API:**
- `new(fields)` - Constructor
- `body()` - Returns `Option<&str>` for document body
- `get_field(name)` - Returns `Option<&QuillValue>` for any field
- `fields()` - Returns reference to entire field map

## Parsing Algorithm

### High-Level Flow

1. **Metadata block discovery** - Scan for all `---` delimiters
2. **Block classification** - Distinguish metadata blocks from horizontal rules
3. **Scope/Quill key extraction** - Parse YAML to check for special keys
4. **YAML parsing** - Convert YAML content to `QuillValue`
5. **Body extraction** - Extract body content between blocks
6. **Collection aggregation** - Group blocks with same scope name
7. **Validation** - Check for collisions, reserved names, invalid syntax
8. **Result assembly** - Merge global fields, body, and tagged collections

## Edge Cases

The parser handles various edge cases:

1. **Empty Frontmatter** - Returns empty frontmatter map with body starting at first blank line
2. **No Frontmatter** - Entire content becomes body
3. **Unclosed Frontmatter** - Returns error to prevent ambiguous interpretation
4. **Nested YAML Structures** - Full YAML support including nested maps, arrays, and all scalar types
5. **Line Endings** - Supports both Unix (`\n`) and Windows (`\r\n`) line endings
6. **Horizontal Rules** - `---` with blank lines both above and below is treated as markdown horizontal rule, not metadata delimiter

## Usage

See `quillmark-core/src/parse.rs` for complete API documentation and examples.

Basic usage:
- `ParsedDocument::from_markdown(markdown)` - Parse markdown with frontmatter
- `doc.body()` - Access body content
- `doc.get_field(name)` - Access frontmatter fields
- `doc.fields()` - Access all fields

## Extended YAML Metadata Standard

### Overview

The extended standard allows metadata blocks to appear anywhere in the document using **SCOPE** and **QUILL** special keys.

**Motivation**: Support structured sub-documents, repeated elements, and hierarchical content.

### Syntax

```markdown
---
title: Global Metadata
---
Main document body.

---
SCOPE: sub_documents
title: First Sub-Document
---
Body of first sub-document.

---
SCOPE: sub_documents
title: Second Sub-Document
---
Body of second sub-document.
```

**Resulting structure:**
```json
{
  "title": "Global Metadata",
  "body": "Main document body.",
  "sub_documents": [
    {"title": "First Sub-Document", "body": "Body of first sub-document."},
    {"title": "Second Sub-Document", "body": "Body of second sub-document."}
  ]
}
```

### Rules

- **SCOPE key**: Creates collections - blocks with same scope name are aggregated into arrays
- **QUILL key**: Specifies which quill template to use
- **Scope names**: Must match `[a-z_][a-z0-9_]*` pattern
- **Reserved names**: Cannot use `body` as scope name
- **Single global**: Only one block without SCOPE/QUILL allowed
- **No collisions**: Global field names cannot conflict with scope names
- **Horizontal rule disambiguation**: `---` with blank lines above AND below is treated as markdown horizontal rule

### Parsing Flow

1. Scan document for all `---` delimiters
2. Parse global frontmatter (if present)
3. Parse scoped metadata blocks
4. Assemble final structure with merged global fields and scoped arrays

### Validation

The parser validates:
- Multiple global frontmatter blocks → error
- Name collisions between global fields and scoped attributes → error
- Reserved field names in scopes → error
- Invalid scope name syntax → error
- Both SCOPE and QUILL in same block → error

---

## Cross-References

**Related Design Documents:**
- [ARCHITECTURE.md](ARCHITECTURE.md) - Parsing in the overall workflow
- [ERROR.md](ERROR.md) - Parse error handling and diagnostics
- [QUILL_VALUE.md](QUILL_VALUE.md) - Value types used in parsed documents

**Implementation:**
- `quillmark-core/src/parse.rs` - Parser implementation and tests
- `quillmark-core/src/lib.rs` - `ParsedDocument` public API


