# Extended YAML Metadata Standard

This document defines the extended markdown syntax for embedding structured metadata in Quillmark documents.

> **Implementation**: `quillmark-core/src/parse.rs`

## Overview

The extended standard allows metadata blocks to appear anywhere in the document using **CARD** and **QUILL** special keys.

**Motivation**: Support structured sub-documents, repeated elements, and hierarchical content.

## Syntax

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

## Rules

- **CARD key**: Creates collections - blocks with same card name are aggregated into the `CARDS` array
- **QUILL key**: Specifies which quill template to use (defaults to `__default__` if not specified)
- **Card names**: Must match `[a-z_][a-z0-9_]*` pattern
- **Single global**: Only one block (the global frontmatter at top of document) without CARD allowed
- **Independent names**: Global field names and card names are independent namespaces (can share names)
- **Horizontal rule disambiguation**: `---` with blank lines above AND below is treated as markdown horizontal rule
- **Default quill tag**: When no QUILL directive is present, ParsedDocument.quill_tag is set to `__default__` at parse time

## Parsing Flow

1. Scan document for all `---` delimiters
2. Parse global frontmatter (if present)
3. Parse card metadata blocks
4. Assemble final structure with merged global fields and `CARDS` array

## Validation

The parser validates:
- Multiple global frontmatter blocks → error
- Reserved field names in cards → error
- Invalid card name syntax → error
- Both CARD and QUILL in same block → error
