# Scope Schema Implementation Plan

**Goal**: Implement `[scopes.*]` configuration in Quill.toml to enable field validation, defaults, and JSON Schema generation for SCOPE blocks.

**Design Reference**: [SCOPES.md](../designs/SCOPES.md)

---

## Current State

- SCOPE blocks are parsed as collections (arrays) per [PARSE.md](../designs/PARSE.md)
- No validation or defaults applied to scope fields
- No JSON Schema generated for scopes
- Quill.toml only supports `[fields.*]` for document-level schemas

---

## Desired State

- `[scopes.*]` sections in Quill.toml define scope field schemas
- Scope items are validated against their schemas during parsing
- Default values are applied to scope fields
- JSON Schema includes scope definitions
- UI metadata is available for scope wizards

---

## Phase 1: Configuration Loading

### Changes Required

1. **Extend Quill.toml Parser**
   - Parse `[scopes.*]` sections alongside `[fields.*]`
   - Build `ScopeConfig` with description, fields, and UI metadata
   - Reuse existing `FieldSchema` parsing for scope fields

2. **Extend QuillConfig**
   - Add `scopes: HashMap<String, ScopeConfig>` field
   - Serialize scope configurations to Quill struct

3. **Extend Quill Struct**
   - Add `scope_schemas` for JSON Schema per scope
   - Add `scope_defaults` for cached default values

### Affected Files

- `crates/quillmark-core/src/quill.rs` - QuillConfig and Quill struct extensions
- `crates/quillmark-core/src/schema.rs` - ScopeConfig type definition (if separate)

---

## Phase 2: Validation Integration

### Changes Required

1. **Scope Field Validation**
   - During document parsing, look up scope schema from Quill
   - Validate each scope item against its field schema
   - Apply default values to missing fields

2. **Error Handling**
   - Generate validation errors with scope context (scope name, item index)
   - Use existing error infrastructure from [ERROR.md](../designs/ERROR.md)

### Affected Files

- `crates/quillmark-core/src/parse.rs` - Add validation hooks after scope aggregation
- `crates/quillmark-core/src/workflow.rs` - Wire validation into render workflow

---

## Phase 3: JSON Schema Generation

### Changes Required

1. **Scope Schema Generation**
   - Generate array-of-objects schema for each defined scope
   - Include `x-ui` metadata for scope-level UI configuration
   - Merge scope schemas into main JSON Schema output

2. **Schema API**
   - Extend `Quill::schema()` to include scope definitions
   - Provide `Quill::scope_schema(name)` for individual scope schemas

### Affected Files

- `crates/quillmark-core/src/quill.rs` - Schema generation methods

---

## Phase 4: Documentation and Migration

### Changes Required

1. **Update Design Index**
   - Add SCOPES.md to [INDEX.md](../designs/INDEX.md)

2. **Update Example Quills**
   - Add `[scopes.*]` configuration to USAF memo quill
   - Add examples demonstrating scope field validation

3. **Binding Updates**
   - Ensure WASM bindings expose scope schema methods
   - Update Python bindings if applicable

---

## Verification

### Unit Tests

- Parse Quill.toml with `[scopes.*]` sections
- Validate scope items against defined schemas
- Apply defaults to scope fields
- Generate JSON Schema including scopes
- Handle unknown scope names (lenient mode)
- Handle malformed scope configurations (error cases)

### Integration Tests

- End-to-end render with scope validation
- USAF memo with endorsement scopes

### Manual Verification

- Render USAF memo with endorsements and verify output
- Inspect generated JSON Schema for scope definitions
