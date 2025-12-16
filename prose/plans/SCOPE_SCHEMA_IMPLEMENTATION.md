# Scope Schema Implementation Plan

**Goal**: Implement `type = "scope"` for fields in Quill.toml to enable validation, defaults, and JSON Schema generation for SCOPE blocks.

**Design Reference**: [SCOPES.md](../designs/SCOPES.md)

---

## Current State

- SCOPE blocks are parsed as collections (arrays) per [PARSE.md](../designs/PARSE.md)
- No validation or defaults applied to scope fields
- No JSON Schema generated for scopes
- Quill.toml only supports flat `[fields.*]` without nested item schemas
- `FieldSchema.from_quill_value()` validates known keys: `name`, `title`, `type`, `description`, `examples`, `default`, `ui`

---

## Desired State

- Fields with `type = "scope"` define scope item schemas via `[fields.X.items.*]`
- Scope items are validated against their item schemas during parsing
- Default values are applied to scope item fields
- JSON Schema generates array properties for scope-typed fields
- No separate `scopes` namespace or structs

---

## Phase 1: Implement Unified Configuration ✅

> **Status**: Completed (2025-12-15)

### Changes Required

1. **Extend FieldSchema struct** (`quill.rs` line 21-35) ✅
   - Add `items: Option<HashMap<String, FieldSchema>>` for scope item fields
   - Add `"items"` to known keys list (line 60)

2. **Update `from_quill_value`** (`quill.rs` line 51-133) ✅
   - Recognize `items` key and recursively parse nested field schemas
   - Parse `[fields.X.items.*]` sections when `type = "scope"`
   - Validate that `items` is only present when `type = "scope"`
   - Validate that nested scopes are rejected (no `type = "scope"` in items)

3. **Update JSON Schema Generation** (`schema.rs`) ✅
   - When `type = "scope"`, generate `{ "type": "array", "items": { ... } }`
   - Recursively call existing field schema building for items
   - Propagate required fields to items.required array

### Affected Files

- `crates/core/src/quill.rs` - Extend FieldSchema with items field
- `crates/core/src/schema.rs` - Update build_schema_from_fields for scope type

### Tests Added

- `quill.rs`: `test_parse_scope_field_type`, `test_parse_scope_items`, `test_scope_items_error_without_scope_type`, `test_scope_nested_scope_error`
- `schema.rs`: `test_schema_scope_generates_array`, `test_schema_scope_items_properties`


---

## Phase 2: Validation Integration ✅

> **Status**: Completed (2025-12-15)

### Changes Implemented

1. **Scope Item Defaults Extraction** (`schema.rs`)
   - Added `extract_scope_item_defaults()` function
   - Extracts default values from `items.properties.*.default` in JSON Schema
   - Returns `HashMap<scope_name, HashMap<item_field, default_value>>`

2. **Scope Item Defaults Application** (`schema.rs`)
   - Added `apply_scope_item_defaults()` function
   - Applies defaults to each item in scope arrays
   - Preserves existing values, only fills missing fields

3. **Integration in Workflow** (`workflow.rs`)
   - Updated `process_plate()` to apply scope item defaults
   - Applied after document-level defaults, before coercion

4. **Validation**
   - JSON Schema validation (via `jsonschema` crate) already validates scope items
   - Required item fields checked via `items.required` array in schema

### Affected Files

- `crates/core/src/schema.rs` - Added scope item default functions
- `crates/quillmark/src/orchestration/workflow.rs` - Integrated scope defaults

### Tests Added

- `test_extract_scope_item_defaults` - Extracts defaults from scope items
- `test_extract_scope_item_defaults_empty` - No scope fields
- `test_extract_scope_item_defaults_no_item_defaults` - Scope without item defaults
- `test_apply_scope_item_defaults` - Apply defaults to scope items
- `test_apply_scope_item_defaults_empty_scope` - Empty scope array
- `test_apply_scope_item_defaults_no_matching_scope` - Unrelated scope field
- `test_scope_validation_with_required_fields` - JSON Schema required field validation

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

Add to `crates/core/src/quill.rs` tests:

- `test_parse_scope_field_type` - Parse Quill.toml with `type = "scope"` field
- `test_parse_scope_items` - Parse nested `[fields.X.items.*]` sections
- `test_scope_items_inherit_ui_order` - Verify item fields get sequential order
- `test_scope_items_error_without_scope_type` - Error when `items` present on non-scope field
- `test_scope_nested_scope_error` - Error when `type = "scope"` appears in items (v1)

Add to `crates/core/src/schema.rs` tests:

- `test_schema_scope_generates_array` - Generate JSON Schema with array-typed properties
- `test_schema_scope_items_properties` - Item fields appear in schema items.properties
- `test_schema_scope_required_propagation` - Required item fields appear in items.required

### Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| `items` on non-scope field | Error: "items only valid for scope type" |
| Empty `items` table | Valid: produces `{ "items": { "properties": {} } }` |
| Nested scope (`type = "scope"` in items) | Error: "Nested scopes not supported in v1" |
| Missing `type` with `items` present | Error: requires explicit `type = "scope"` |

### Error Message Format

```
Field 'endorsements.items.name': description is required
Scope 'endorsements' item 0: missing required field 'name'
Field 'author': 'items' is only valid when type = "scope"
```

### Integration Tests

- End-to-end render with scope validation
- USAF memo with endorsement scopes

### Manual Verification

- Render USAF memo with endorsements and verify output
- Inspect generated JSON Schema for scope array properties
