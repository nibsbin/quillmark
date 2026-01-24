# Scope Schema Configuration

> **Status**: Superseded by [CARDS.md](CARDS.md)
>
> **Note**: This document describes the legacy SCOPE system. The implementation has been replaced with the CARDS architecture which uses a unified `CARDS` array with typed card blocks. See [CARDS.md](CARDS.md) for the current design.
>
> This document defines how SCOPE blocks were configured as fields with `type = "scope"` in Quill.yaml.

> **Related Documents**:
> - [SCHEMAS.md](SCHEMAS.md) - Field validation and JSON Schema generation
> - [PARSE.md](PARSE.md) - SCOPE block parsing (Extended YAML Metadata Standard)
> - [QUILL.md](QUILL.md) - Quill.yaml structure

---

## Overview

SCOPE blocks (documented in [PARSE.md](PARSE.md)) are always parsed as **collections** (arrays of objects). This design unifies scope configuration into the existing `[fields.*]` structure using `type = "scope"`, enabling:

1. Unified mental model - scopes are just fields with array-of-objects type
2. Field validation for scope items
3. Default values for scope fields  
4. JSON Schema generation as array properties

---

## Design Principles

### 1. Unified Field Model

Scopes are fields with `type = "scope"`. No separate `[scopes.*]` namespace exists.

### 2. Collections Only

All scopes are collections. Multiple markdown SCOPE blocks with the same name become items in an array. Zero blocks produce an empty array.

### 3. Lenient Unknown Scope Items

> [!IMPORTANT]
> **Design Decision**: Unknown scope names (not defined as scope-typed fields) are allowed and default to collection behavior with no validation. This supports forward compatibility and mixed-version workflows.

### 4. Consistent Schema Reuse

Scope item fields use the same `FieldSchema` structure as document fields (see [SCHEMAS.md](SCHEMAS.md)).

### 5. Naming: `items` over `fields` or `scope_fields`

Nested scope field definitions use `fields.X.items.*` syntax because:

1. **JSON Schema alignment** - Arrays use `items` to define element schema
2. **Avoids confusion** - Parent is already `[fields.*]`, reusing `fields` would be ambiguous
3. **Concise** - `scope_fields` is verbose without adding clarity

### 6. No Nested Scopes (v1)

Scope items cannot themselves be scopes. If `type = "scope"` appears within `fields.X.items.*`, parsing fails with an error. This simplification is intentional for v1; nested structures can be reconsidered post-launch.

---

## Quill.yaml Structure

### Scope Field Definition

```yaml
fields:
  endorsements:
    type: scope
    title: Endorsements
    description: Chain of endorsements for routing
    ui:
      group: Routing
```

### Scope Item Field Definition

```yaml
fields:
  endorsements:
    items:
      name:
        type: string
        title: Endorser Name
        description: Name of the endorsing official
        required: true  # Explicitly required
      org:
        type: string
        title: Organization
        default: Unknown  # Optional (has default)
```

---

## Behavior Rules

### Parse-Time Behavior

| Scenario | Behavior |
|----------|----------|
| Zero SCOPE blocks | Empty array: `endorsements = []` |
| One SCOPE block | Single-item array: `endorsements = [{...}]` |
| Multiple SCOPE blocks | Multi-item array: `endorsements = [{...}, {...}]` |
| Unknown scope name | Collection behavior, no validation |

### Validation

- Fields with `type = "scope"` validate each item against `fields.X.items.*`
- Default values are applied to each item

> [!IMPORTANT]
> **Design Decision**: Unknown fields within scope items are allowed (lenient). This mirrors document-level field leniency and prevents breakage when scope schemas evolve.

### Required Field Propagation

Fields are **optional by default** (aligns with JSON Schema standard). Use `required = true` to mark mandatory fields:

| Item Field Configuration | JSON Schema Result |
|--------------------------|-------------------|
| `required = true` | In `items.required` array |
| No `required` (or `required = false`) | Not in `required` array (optional) |

This applies to both document-level fields and scope item fields.

### Not Supported in v1

- `min_items` / `max_items` constraints
- Singular (non-array) scopes

---

## JSON Schema Generation

Scope-typed fields generate as array properties:

```json
{
  "endorsements": {
    "type": "array",
    "title": "Endorsements",
    "description": "Chain of endorsements for routing",
    "items": {
      "type": "object",
      "properties": {
        "name": { "type": "string", ... },
        "org": { "type": "string", "default": "Unknown", ... }
      }
    },
    "x-ui": { "group": "Routing" }
  }
}
```

---

## Data Model Changes

### FieldSchema Extension

Extend existing `FieldSchema` struct (see [QUILL.md](QUILL.md)):

- Add `items: Option<HashMap<String, FieldSchema>>` for scope item fields

### Type Recognition

- `type = "scope"` indicates array-of-objects behavior
- Presence of `items` sub-fields enables item validation

### No Separate Structs

- No `ScopeConfig` struct needed
- No `scopes` dictionary in `QuillConfig`
- No `scope_schemas` / `scope_defaults` in `Quill`

---

## UI Metadata

Scope-typed fields use the same UI metadata as other fields:

| Property | Purpose |
|----------|---------|
| `ui.group` | Groups scope in UI sidebar |
| `ui.order` | Display order (from position in Quill.yaml) |

Item fields within scopes also use the same UI structure.

---

## Cross-References

- **Parsing**: SCOPE block parsing is defined in [PARSE.md](PARSE.md) §Extended YAML Metadata Standard
- **Field Types**: Type mappings and validation rules are defined in [SCHEMAS.md](SCHEMAS.md) §Quill Field
- **Quill Structure**: Base Quill.yaml structure is defined in [QUILL.md](QUILL.md) §Metadata Handling
