# Scope Schema Configuration

> **Status**: Design Phase
>
> This document defines the schema configuration for SCOPE blocks in Quill.toml.

> **Related Documents**:
> - [SCHEMAS.md](SCHEMAS.md) - Field validation and JSON Schema generation
> - [PARSE.md](PARSE.md) - SCOPE block parsing (Extended YAML Metadata Standard)
> - [QUILL.md](QUILL.md) - Quill.toml structure

---

## Overview

SCOPE blocks (documented in [PARSE.md](PARSE.md)) are always parsed as **collections** (arrays of objects). This design adds schema annotations for scope fields via `[scopes.*]` sections in Quill.toml, enabling:

1. Field validation for scope items
2. Default values for scope fields
3. UI metadata for scope wizards
4. JSON Schema generation for scopes

---

## Design Principles

### 1. Collections Only

All scopes are collections. Multiple markdown SCOPE blocks with the same name become items in an array. Zero blocks produce an empty array.

### 2. Lenient Unknown Scopes

Unknown scope names (not defined in `[scopes.*]`) are allowed and default to collection behavior with no validation.

### 3. Consistent Field Schema

Scope fields use the same schema format as document fields (see [SCHEMAS.md](SCHEMAS.md)):
- Same type mappings (`str`, `array`, `dict`, `date`, `datetime`, `number`)
- Same UI metadata (`group`, `order`)
- Same default/required logic

---

## Quill.toml Structure

### Scope Definition

```toml
[scopes.NAME]
description = "Human-readable description"
ui.group = "UI grouping"            # optional
ui.icon = "icon-name"               # optional
ui.add_button_text = "Add Item"     # optional
ui.item_label = "{{ordinal}} Item"  # optional
```

### Scope Field Definition

```toml
[scopes.NAME.fields.FIELD_NAME]
type = "str"                        # required: str, number, array, dict, date, datetime
title = "Field Label"               # optional
description = "Field description"   # optional
default = "default value"           # optional: makes field optional
examples = ["example1"]             # optional
ui.group = "Field Group"            # optional
```

---

## Behavior Rules

### Parse-Time Behavior

| Scenario | Behavior |
|----------|----------|
| Zero SCOPE blocks | Empty array: `scope_name = []` |
| One SCOPE block | Single-item array: `scope_name = [{...}]` |
| Multiple SCOPE blocks | Multi-item array: `scope_name = [{...}, {...}]` |
| Unknown scope name | Collection behavior, no validation |

### Validation

- Each item in a scope collection is validated against `[scopes.X.fields.*]`
- Default values are applied to each item
- Unknown fields within scope items are allowed (lenient)

### Not Supported in v1

- `min_items` / `max_items` constraints
- Singular (non-array) scopes

---

## JSON Schema Generation

Scope schemas generate as arrays with object items:

```json
{
  "scope_name": {
    "type": "array",
    "description": "Scope description",
    "items": {
      "type": "object",
      "properties": {
        "field_name": { "type": "string", ... }
      }
    },
    "x-ui": { "group": "...", "icon": "..." }
  }
}
```

---

## Data Model Changes

### QuillConfig Extension

Add to existing `QuillConfig` struct (see [QUILL.md](QUILL.md)):

- `scopes`: Map of scope name → `ScopeConfig`

### ScopeConfig

New configuration type for scope metadata:

- `description`: Human-readable description
- `fields`: Map of field name → `FieldSchema` (same as document fields)
- `ui`: Optional UI metadata table

### Quill Extension

Add to existing `Quill` struct:

- `scope_schemas`: Map of scope name → JSON Schema (for each scope)
- `scope_defaults`: Cached defaults for scope fields

---

## UI Metadata

Scope-level UI metadata enables rich wizard experiences:

| Property | Purpose |
|----------|---------|
| `group` | Groups scopes in UI sidebar |
| `icon` | Icon for scope type |
| `add_button_text` | Text for "add item" button |
| `item_label` | Template for item labels (supports `{{ordinal}}`) |

Field-level UI metadata uses the same structure as document fields (see [SCHEMAS.md](SCHEMAS.md)).

---

## Cross-References

- **Parsing**: SCOPE block parsing is defined in [PARSE.md](PARSE.md) §Extended YAML Metadata Standard
- **Field Types**: Type mappings and validation rules are defined in [SCHEMAS.md](SCHEMAS.md) §Quill Field
- **Quill Structure**: Base Quill.toml structure is defined in [QUILL.md](QUILL.md) §Metadata Handling
