# Composable Cards Architecture

> **Status**: Implemented
>
> This document defines the composable cards system for structured, repeatable content blocks.

> **Related Documents**:
> - [SCHEMAS.md](SCHEMAS.md) - Field validation and JSON Schema generation
> - [PARSE.md](PARSE.md) - CARD block parsing (Extended YAML Metadata Standard)
> - [QUILL.md](QUILL.md) - Quill.toml structure and data types
> - ~~[SCOPES.md](SCOPES.md)~~ - **Superseded by this document**

---

## Overview

Cards are structured metadata blocks that appear inline within document content. Unlike frontmatter fields (which define document-level metadata), cards represent repeatable, typed content units that can be composed in any order.

**Key capabilities:**

1. **Unified array** - All cards stored in single `CARDS` array
2. **Discriminated union** - Cards typed via `CARD` discriminator field
3. **Composable ordering** - Multiple card types interleaved freely
4. **Schema referencing** - Card schemas in `$defs` with OpenAPI 3.0-style discriminator

---

## Design Principles

### 1. Cards Are First-Class Types

Cards have their own `CardSchema` struct, separate from `FieldSchema`. This reflects their distinct purpose:
- **Fields**: Document-level metadata (frontmatter)
- **Cards**: Structured in-body content blocks

### 2. Unified CARDS Array

All parsed cards flow into a single `CARDS` array in `ParsedDocument`. Cards are discriminated by the `CARD` field which matches the card type name.

### 3. OpenAPI 3.0 Discriminator Pattern

JSON Schema output uses `$defs` with `oneOf` and an explicit discriminator hint, maximizing LLM consumability.

### 4. TOML Configuration Symmetry

Card field definitions use `[cards.X.fields.Y]` syntax, mirroring top-level `[fields.Y]` for consistency.

---

## Data Model

### CardSchema Struct

```rust
pub struct CardSchema {
    /// Card type name (e.g., "indorsements")
    pub name: String,
    /// Short label for the card type
    pub title: Option<String>,
    /// Detailed description of this card type
    pub description: Option<String>,
    /// Field definitions for this card type
    pub fields: HashMap<String, FieldSchema>,
}
```

### FieldSchema (Unchanged)

Regular fields remain unchanged. The `items` field is **removed** since cards are no longer nested properties:

```rust
pub struct FieldSchema {
    pub name: String,
    pub title: Option<String>,
    pub r#type: Option<String>,
    pub description: String,
    pub default: Option<QuillValue>,
    pub examples: Option<QuillValue>,
    pub ui: Option<UiSchema>,
    pub required: bool,
    // NO items field - cards have their own schema
}
```

### QuillConfig

```rust
pub struct QuillConfig {
    // ... existing fields ...
    pub fields: HashMap<String, FieldSchema>,   // [fields.X]
    pub cards: HashMap<String, CardSchema>,     // [cards.X]
}
```

---

## Quill.toml Configuration

### Card Definition

```toml
[cards.indorsements]
title = "Routing Indorsements"
description = "Chain of routing endorsements for multi-level correspondence."

[cards.indorsements.fields.from]
title = "From office/symbol"
type = "string"
required = true
description = "Office symbol of the endorsing official."

[cards.indorsements.fields.for]
title = "To office/symbol"
type = "string"
required = true
description = "Office symbol receiving the endorsed memo."

[cards.indorsements.fields.signature_block]
title = "Signature block lines"
type = "array"
required = true
ui.group = "Signature"
description = "Name, grade, and duty title."
```

### Key Changes from SCOPES.md

| Old (`[cards.X.items.Y]`) | New (`[cards.X.fields.Y]`) |
|---------------------------|----------------------------|
| `items` keyword | `fields` keyword |
| Nested in FieldSchema | Separate CardSchema |
| Schema as nested property | Schema in `$defs` |

---

## JSON Schema Output

### Structure

```json
{
  "$schema": "https://json-schema.org/draft/2019-09/schema",
  "type": "object",
  "$defs": {
    "indorsements_card": {
      "type": "object",
      "title": "Routing Indorsements",
      "description": "Chain of routing endorsements...",
      "properties": {
        "CARD": { "const": "indorsements" },
        "from": { "type": "string", "title": "From office/symbol", ... },
        "for": { "type": "string", "title": "To office/symbol", ... }
      },
      "required": ["CARD", "from", "for"]
    }
  },
  "properties": {
    "subject": { "type": "string", ... },
    "CARDS": {
      "type": "array",
      "items": {
        "oneOf": [
          { "$ref": "#/$defs/indorsements_card" }
        ],
        "x-discriminator": {
          "propertyName": "CARD",
          "mapping": {
            "indorsements": "#/$defs/indorsements_card"
          }
        }
      }
    }
  }
}
```

### OpenAPI 3.0 Discriminator

The `x-discriminator` extension follows OpenAPI 3.0 semantics:

| Property | Purpose |
|----------|---------|
| `propertyName` | Field to inspect for type discrimination |
| `mapping` | Maps discriminator values to schema refs |

Each card schema includes:
- `"CARD": { "const": "..." }` — Enforces discriminator value
- `"required": ["CARD", ...]` — Ensures discriminator is present

---

## Consumption in Backends

- **Typst**: Cards appear under `data.CARDS` via the helper package. Markdown fields marked with `contentMediaType = "text/markdown"` are pre-converted to Typst markup.
- **AcroForm**: Cards are available in the JSON context for MiniJinja templates as `CARDS`.
- **Bindings**: `Workflow::compile_data()` exposes the exact JSON used for rendering.

---

## Markdown Syntax

Cards use the `CARD:` key in inline metadata blocks:

```markdown
---
title: Main Document
---

Main content here.

---
CARD: indorsements
from: ORG1/SYMBOL
for: ORG2/SYMBOL
signature_block:
  - "JOHN DOE, Lt Col, USAF"
  - "Commander"
---

Indorsement body content.
```

---

## Cross-References

- **Parsing**: CARD block parsing in [PARSE.md](PARSE.md) §Extended YAML Metadata Standard
- **Field Types**: Type mappings in [SCHEMAS.md](SCHEMAS.md) §Quill Field
- **Quill Structure**: QuillConfig in [QUILL.md](QUILL.md) §Metadata Handling
