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
- Body is stored under special `BODY_FIELD = "BODY"` constant
- Quill tag is stored as a non-optional String, defaulting to `__default__`
- Private fields enforce access through validated methods

**Public API:**
- `new(fields)` - Constructor (sets quill_tag to `__default__`)
- `with_quill_tag(fields, quill_tag)` - Constructor with explicit quill tag
- `body()` - Returns `Option<&str>` for document body
- `get_field(name)` - Returns `Option<&QuillValue>` for any field
- `fields()` - Returns reference to entire field map
- `quill_tag()` - Returns `&str` for the quill tag (never None)

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
CARD: sub_documents
title: First Sub-Document
---
Body of first sub-document.

---
CARD: sub_documents
title: Second Sub-Document
---
Body of second sub-document.
```

**Resulting structure:**
```json
{
  "title": "Global Metadata",
  "BODY": "Main document body.",
  "CARDS": [
    {"CARD": "sub_document", "title": "First Sub-Document", "BODY": "Body of first sub-document."},
    {"CARD": "sub_document", "title": "Second Sub-Document", "BODY": "Body of second sub-document."}
  ]
}
```

### Rules

- **CARD key**: Creates collections - blocks with same card name are aggregated into the `CARDS` array
- **QUILL key**: Specifies which quill template to use (defaults to `__default__` if not specified)
- **Card names**: Must match `[a-z_][a-z0-9_]*` pattern
- **Single global**: Only one block (the global frontmatter at top of document) without CARD allowed
- **Independent names**: Global field names and card names are independent namespaces (can share names)
- **Horizontal rule disambiguation**: `---` with blank lines above AND below is treated as markdown horizontal rule
- **Default quill tag**: When no QUILL directive is present, ParsedDocument.quill_tag is set to `__default__` at parse time

### Parsing Flow

1. Scan document for all `---` delimiters
2. Parse global frontmatter (if present)
3. Parse card metadata blocks
4. Assemble final structure with merged global fields and `CARDS` array

### Validation

The parser validates:
- Multiple global frontmatter blocks → error
- Reserved field names in cards → error
- Invalid card name syntax → error
- Both CARD and QUILL in same block → error
