# Composable Cards Architecture

> **Status**: Implemented
> **Related**: [SCHEMAS.md](SCHEMAS.md), [QUILL.md](QUILL.md)

## Overview

Cards are structured metadata blocks inline within document content. All cards are stored in a single `CARDS` array, discriminated by the `CARD` field.

## Data Model

```rust
pub struct CardSchema {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub fields: HashMap<String, FieldSchema>,
    pub ui: Option<UiContainerSchema>,
}
```

`QuillConfig` stores `cards: Vec<CardSchema>` where index 0 is always the **main** card (document-level fields). Named card definitions start at index 1 and are accessed via `card_definitions()` / `card_definition(name)`.

## Quill.yaml Configuration

```yaml
main:
  fields:
    # ... document-level fields ...

cards:
  indorsement:
    title: Routing Indorsement
    description: Chain of routing endorsements for multi-level correspondence.
    fields:
      from:
        title: From office/symbol
        type: string
        description: Office symbol of the endorsing official.
      for:
        title: To office/symbol
        type: string
        description: Office symbol receiving the endorsed memo.
      signature_block:
        title: Signature block lines
        type: array
        required: true
        ui:
          group: Addressing
        description: Name, grade, and duty title.
```

## Public Schema YAML Output

```yaml
cards:
  indorsement:
    title: Routing Indorsement
    description: Chain of routing endorsements for multi-level correspondence.
    fields:
      from:
        type: string
      for:
        type: string
      signature_block:
        type: array
        required: true
```

Public schema is emitted from `QuillConfig::public_schema_yaml()` and keeps the same `cards.<name>.fields` shape as `Quill.yaml`. The `cards` key is omitted entirely when no named cards are defined.

## Markdown Syntax

```markdown
---
CARD: indorsement
from: ORG1/SYMBOL
for: ORG2/SYMBOL
signature_block:
  - "JOHN DOE, Lt Col, USAF"
  - "Commander"
---

Indorsement body content.
```

## Backend Consumption

- **All backends**: cards are delivered as `data.CARDS`, an array of objects each containing a `CARD` discriminator field, the card's metadata fields, and a `BODY` field with the card's body Markdown.
- **`Quill::compile_data()`** returns the fully coerced and validated JSON, including `CARDS`.
