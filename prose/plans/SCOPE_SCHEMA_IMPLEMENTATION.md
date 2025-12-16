# Scope Schema Implementation Plan

**Goal**: Implement `type = "scope"` for fields in Quill.toml to enable validation, defaults, and JSON Schema generation for SCOPE blocks.

**Design Reference**: [SCOPES.md](../designs/SCOPES.md)

---

## Current State

- SCOPE blocks are parsed as collections (arrays) per [PARSE.md](../designs/PARSE.md)
- No validation or defaults applied to scope fields
- No JSON Schema generated for scopes
- Quill.toml only supports flat `[fields.*]` without nested item schemas
- **Partial implementation exists**: Separate `[scopes.*]` approach (to be removed)

---

## Desired State

- Fields with `type = "scope"` define scope item schemas via `[fields.X.items.*]`
- Scope items are validated against their item schemas during parsing
- Default values are applied to scope item fields
- JSON Schema generates array properties for scope-typed fields
- No separate `scopes` namespace or structs

---

## Phase 1: Implement Unified Configuration

### Changes Required

1. **Extend FieldSchema**
   - Add `items: Option<HashMap<String, FieldSchema>>` for scope item fields
   - Update `from_quill_value` to recognize `items` key and recursively parse nested field schemas
   - Parse `[fields.X.items.*]` sections when `type = "scope"`

2. **Update JSON Schema Generation**
   - When `type = "scope"`, generate `{ "type": "array", "items": { ... } }`
   - Recursively call existing field schema building for items

### Affected Files

- `crates/core/src/quill.rs` - Extend FieldSchema with items field
- `crates/core/src/schema.rs` - Update build_schema_from_fields for scope type

---

## Phase 2: Validation Integration

### Changes Required

1. **Scope Field Validation**
   - During document parsing, detect scope-typed fields
   - Validate each scope item against `items.*` schema
   - Apply default values to missing item fields

2. **Error Handling**
   - Generate validation errors with scope context (field name, item index)
   - Use existing error infrastructure from [ERROR.md](../designs/ERROR.md)

### Affected Files

- `crates/core/src/parse.rs` - Add validation hooks after scope aggregation
- `crates/core/src/schema.rs` - Add scope item validation

---

## Phase 3: Documentation and Migration

### Changes Required

1. **Update Example Quills**
   - Add `type = "scope"` fields to USAF memo quill
   - Add `[fields.X.items.*]` field definitions

2. **Binding Updates**
   - No API changes needed (scopes are just fields)

---

## Verification

### Unit Tests

- Parse Quill.toml with `type = "scope"` fields
- Parse nested `[fields.X.items.*]` sections
- Generate JSON Schema with array-typed properties
- Validate scope items against item schemas
- Apply defaults to scope item fields
- Handle unknown scope names (lenient mode)

### Integration Tests

- End-to-end render with scope validation
- USAF memo with endorsement scopes

### Manual Verification

- Render USAF memo with endorsements and verify output
- Inspect generated JSON Schema for scope array properties
