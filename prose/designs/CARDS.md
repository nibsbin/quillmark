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

## Quill.yaml Configuration

```yaml
cards:
  indorsements:
    title: Routing Indorsements
    description: Chain of routing endorsements for multi-level correspondence.
    fields:
      from:
        title: From office/symbol
        type: string
        required: true
        description: Office symbol of the endorsing official.
      for:
        title: To office/symbol
        type: string
        required: true
        description: Office symbol receiving the endorsed memo.
      signature_block:
        title: Signature block lines
        type: array
        required: true
        ui:
          group: Signature
        description: Name, grade, and duty title.
```

## Public Schema YAML Output

```yaml
cards:
  indorsements:
    title: Routing Indorsements
    description: Chain of routing endorsements for multi-level correspondence.
    fields:
      from:
        type: string
      for:
        type: string
      signature_block:
        type: array
        items:
          type: string
```

Public schema is emitted from `QuillConfig::public_schema_yaml()` and keeps the same `cards.<name>.fields` shape as `Quill.yaml`.

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
- **Bindings**: `Workflow::compile_data()` exposes the exact JSON
