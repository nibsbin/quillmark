# Composable Cards Architecture

> **Status**: Implemented
> **Related**: [SCHEMAS.md](SCHEMAS.md), [PARSE.md](PARSE.md), [QUILL.md](QUILL.md)
> ~~[SCOPES.md](SCOPES.md)~~ — Superseded by this document

## Overview

Cards are structured metadata blocks inline within document content. All cards are stored in a single `CARDS` array, discriminated by the `CARD` field.

## Data Model

```rust
pub struct CardSchema {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub fields: HashMap<String, FieldSchema>,
}
```

`QuillConfig` has a `cards: HashMap<String, CardSchema>` field alongside `fields`.

## Quill.toml Configuration

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

## JSON Schema Output

```json
{
  "$schema": "https://json-schema.org/draft/2019-09/schema",
  "type": "object",
  "$defs": {
    "indorsements_card": {
      "type": "object",
      "title": "Routing Indorsements",
      "properties": {
        "CARD": { "const": "indorsements" },
        "from": { "type": "string", ... },
        "for": { "type": "string", ... }
      },
      "required": ["CARD", "from", "for"]
    }
  },
  "properties": {
    "CARDS": {
      "type": "array",
      "items": {
        "oneOf": [{ "$ref": "#/$defs/indorsements_card" }],
        "x-discriminator": {
          "propertyName": "CARD",
          "mapping": { "indorsements": "#/$defs/indorsements_card" }
        }
      }
    }
  }
}
```

`x-discriminator` follows OpenAPI 3.0 semantics. Each card schema includes `"CARD": { "const": "..." }` and adds `"CARD"` to `required`.

## Markdown Syntax

```markdown
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

## Backend Consumption

- **Typst**: cards at `data.CARDS`; markdown fields pre-converted to Typst markup
- **AcroForm**: `CARDS` available in MiniJinja context
- **Bindings**: `Workflow::compile_data()` exposes the exact JSON
