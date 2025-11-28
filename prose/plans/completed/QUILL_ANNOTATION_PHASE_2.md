# Quill Annotation Revamp - Phase 2 Plan

**Status**: Design Phase
**Goal**: Establish JSON Schema as the single source of truth for Quill field metadata, including UI layout via `x-ui` properties.
**Related**: [`prose/QUILL_ANNOTATION.md`](../QUILL_ANNOTATION.md), [`prose/designs/SCHEMAS.md`](../designs/SCHEMAS.md)

---

## Overview

Phase 2 extends Quill metadata to enable dynamic wizard UIs by embedding UI layout configuration directly into the JSON Schema using a custom `x-ui` property.

1.  **Data Schema (JSON Schema)**: Authoritative source for validation, types, and data constraints.
2.  **UI Layout (x-ui)**: A nested object within each field definition that specifies grouping, components, and ordering.

This approach keeps all metadata co-located while providing a structured way to define UI behavior.

---

## Current State Analysis

### Metadata Flow Architecture

**TOML → FieldSchema → JSON Schema**

Currently, `Quill.toml` fields are parsed and converted into a single JSON Schema. This schema is used for everything: validation, defaults, and API exposure.

**Gap**: The current architecture lacks a way to express UI-specific concerns like widgets and explicit ordering without polluting the validation schema with flat `x-*` properties.

---

## Desired State

### 1. TOML Configuration (Input)

We introduce a nested `[ui]` table within each field definition in `Quill.toml`.

**`Quill.toml` Example**:
```toml
[fields.full_name]
type = "str"
[fields.full_name.ui]
group = "Personal Info"
component = "text-input"
order = 1

[fields.flavor]
type = "str"
[fields.flavor.ui]
group = "Preferences"
component = "select"
order = 2
```

### 2. JSON Schema (Validation & Layout)

The `build_schema_from_fields()` function generates a JSON Schema where each property includes an `x-ui` object.

```json
{
  "type": "object",
  "properties": {
    "full_name": {
      "type": "string",
      "x-ui": {
        "group": "Personal Info",
        "component": "text-input",
        "order": 1
      }
    },
    "flavor": {
      "type": "string",
      "x-ui": {
        "group": "Preferences",
        "component": "select",
        "order": 2
      }
    }
  }
}
```

### 3. WASM API

The `QuillInfo` struct exposed to WASM remains unchanged, as the `schema` field already carries the complete JSON Schema including the `x-ui` extensions.

```typescript
interface QuillInfo {
  id: string;
  // ...
  schema: JsonSchema; // Includes x-ui properties
}
```

---

## Implementation Plan

### Step 1: Update Config Structures

Modify `FieldSchema` to accept the optional `ui` table.

```rust
// crates/core/src/quill.rs

#[derive(Deserialize)]
pub struct FieldSchema {
    // ... existing fields
    pub ui: Option<UiSchema>,
}

#[derive(Deserialize, Serialize)]
pub struct UiSchema {
    pub group: Option<String>,
    pub component: Option<String>,
    pub order: Option<i32>,
    #[serde(flatten)]
    pub extra: HashMap<String, QuillValue>, // Allow arbitrary other UI props
}
```

### Step 2: Update JSON Schema Generation

Modify `build_schema_from_fields()` to inject the `x-ui` object into the generated schema.

```rust
// crates/core/src/schema.rs

// Inside the loop for each field:
if let Some(ref ui) = field_schema.ui {
    property.insert(
        "x-ui".to_string(),
        serde_json::to_value(ui).unwrap(),
    );
}
```

### Step 3: Frontend Integration

The frontend consumes the `schema` and parses the `x-ui` property to render the form.

```javascript
// Example logic
const fields = Object.entries(schema.properties).map(([key, prop]) => ({
  key,
  ...prop,
  ui: prop['x-ui'] || {}
}));

// Sort by order
fields.sort((a, b) => (a.ui.order || 999) - (b.ui.order || 999));

// Group by 'group'
const groups = groupBy(fields, f => f.ui.group || 'General');
```

---

## Architecture Decision: Embedded x-ui Object

**Decision**: Embed UI metadata in `x-ui` property within JSON Schema.

**Rationale**:
1.  **Co-location**: Keeps validation and UI logic for a field together.
2.  **Simplicity**: Single artifact (JSON Schema) to pass around.
3.  **Flexibility**: The `x-ui` object can contain any UI-specific metadata without cluttering the root of the field definition.

**Implications**:
- `FieldSchema` gets a `ui` field.
- `build_schema_from_fields` serializes `ui` to `x-ui`.
- No changes to WASM API surface.

**Benefits**:
- Single source of truth (JSON Schema)
- Clear transformation boundary (TOML → Schema)
- Validation and metadata use same source
- Forward compatible (validators ignore `x-*` properties)

---

## Migration Strategy

### Step 1: Extend FieldSchema Struct

**Add** optional `section` and `tooltip` fields to `FieldSchema`:

**Location**: `crates/core/src/quill.rs:12-24`

**Changes**:
```rust
pub struct FieldSchema {
    pub name: String,
    pub r#type: Option<String>,
    pub description: String,
    pub default: Option<QuillValue>,
    pub example: Option<QuillValue>,
    pub examples: Option<QuillValue>,
    pub section: Option<String>,    // Add this
    pub tooltip: Option<String>,    // Add this
}
```

**Update**:
- Add Serde `#[serde(skip_serializing_if = "Option::is_none")]` for new fields
- Update `FieldSchema::new()` constructor (if exists)
- Update `FieldSchema::from_quill_value()` to parse new fields from TOML

**Impact**:
- Existing Quills continue to work (fields are optional)
- TOML deserialization automatically handles new fields

### Step 2: Update JSON Schema Generation

**Modify** `build_schema_from_fields()` to include custom properties:

**Location**: `crates/core/src/schema.rs:11-90`

**Add after line 48** (after inserting description):
```rust
// Add section if specified
if let Some(ref section) = field_schema.section {
    property.insert(
        "x-section".to_string(),
        Value::String(section.clone()),
    );
}

// Add tooltip if specified
if let Some(ref tooltip) = field_schema.tooltip {
    property.insert(
        "x-tooltip".to_string(),
        Value::String(tooltip.clone()),
    );
}
```

**Impact**:
- Generated JSON schemas now include UI metadata
- WASM/Python consumers automatically receive this data via `schema` field
- Validation logic unaffected (validators ignore unknown `x-*` properties)
- Extraction utilities (`extract_defaults_from_schema()`, etc.) unaffected

### Step 3: Update SCHEMAS.md Design Document

**Document** the new fields in the design specification:

**Location**: `prose/designs/SCHEMAS.md`

**Already updated** with:
- New FieldSchema field properties (`section`, `tooltip`)
- JSON Schema custom properties documentation (`x-section`, `x-tooltip`)
- Example JSON Schema showing custom properties

### Step 4: Create Example Quill with Sections

**Create** or update an example Quill to demonstrate the feature:

**Location**: `examples/` directory (choose appropriate example)

**Example TOML**:
```toml
[Quill]
name = "resume-template"
description = "Professional resume template"
backend = "typst"

[fields.name]
description = "Your full legal name as you want it to appear on the resume"
type = "str"
section = "Personal Information"
tooltip = "Full name"

[fields.email]
description = "Professional email address for contact purposes"
type = "str"
section = "Personal Information"
tooltip = "Email address"

[fields.phone]
description = "Phone number with country code"
type = "str"
section = "Personal Information"
tooltip = "Phone number"

[fields.degree]
description = "Your highest degree or current educational pursuit"
type = "str"
section = "Education"
tooltip = "Degree (e.g., Bachelor of Science)"
```

**Impact**:
- Demonstrates best practices for section/tooltip usage
- Provides reference for Quill developers
- Tests the entire metadata flow (TOML → Schema → WASM)

---

## Cross-Cutting Concerns

### Backward Compatibility

**Breaking Changes**: None

- New fields are optional in `FieldSchema`
- Existing Quills without section/tooltip continue to work
- JSON Schema `x-*` properties are ignored by standard validators
- WASM API surface unchanged (metadata embedded in existing `schema` field)

**Migration path**:
- Old Quills: Work unchanged, no sections/tooltips in generated schema
- New Quills: Can opt-in to sections/tooltips for better UX
- Gradual adoption: Quill developers update at their own pace

**Compatibility verification**:
- Load existing example Quills without modifications
- Verify they parse, register, and validate correctly
- Confirm WASM `getQuillInfo()` returns schemas without `x-*` properties

### Testing Strategy

**Unit tests** (add to `crates/core/src/schema.rs`):
- Test `build_schema_from_fields()` with section/tooltip fields
- Test `build_schema_from_fields()` without section/tooltip (backward compat)
- Verify generated schema includes `x-section` and `x-tooltip` properties
- Verify schema validates documents correctly (ignores custom properties)

**Integration tests** (add to `crates/core/src/quill.rs`):
- Load Quill TOML with section/tooltip fields
- Verify `Quill.schema` includes custom properties
- Extract defaults and examples (ensure extraction logic unaffected)
- Test via WASM bindings (parse schema and extract metadata)

**Regression tests**:
- Load all existing example Quills
- Verify they parse and register correctly
- Confirm schemas validate documents as before
- Ensure WASM tests pass unchanged

**Test cases**:
```rust
#[test]
fn test_build_schema_with_section_and_tooltip() {
    let mut fields = HashMap::new();
    let mut schema = FieldSchema::new(
        "author".to_string(),
        "Document author name".to_string(),
    );
    schema.r#type = Some("str".to_string());
    schema.section = Some("Author Info".to_string());
    schema.tooltip = Some("Your full name".to_string());
    fields.insert("author".to_string(), schema);

    let json_schema = build_schema_from_fields(&fields)
        .unwrap()
        .as_json()
        .clone();

    assert_eq!(
        json_schema["properties"]["author"]["x-section"],
        "Author Info"
    );
    assert_eq!(
        json_schema["properties"]["author"]["x-tooltip"],
        "Your full name"
    );
}

#[test]
fn test_build_schema_without_section_tooltip() {
    // Backward compatibility: fields without section/tooltip
    let mut fields = HashMap::new();
    let mut schema = FieldSchema::new(
        "title".to_string(),
        "Document title".to_string(),
    );
    schema.r#type = Some("str".to_string());
    fields.insert("title".to_string(), schema);

    let json_schema = build_schema_from_fields(&fields)
        .unwrap()
        .as_json()
        .clone();

    // Should not have x-section or x-tooltip
    assert!(!json_schema["properties"]["title"]
        .as_object()
        .unwrap()
        .contains_key("x-section"));
    assert!(!json_schema["properties"]["title"]
        .as_object()
        .unwrap()
        .contains_key("x-tooltip"));
}
```

### Validation Behavior

**Important**: JSON Schema validators MUST ignore unknown `x-*` properties

**Verification**:
- Test that documents validate correctly regardless of `x-section`/`x-tooltip` presence
- Ensure validation errors don't reference custom properties
- Confirm schema evolution (adding new `x-*` fields) doesn't break validation

**Standard compliance**:
- JSON Schema Draft 2019-09 and later support custom `x-*` extensions
- The `jsonschema` crate (used in `validate_document()`) complies with the standard
- Validators treat `x-*` properties as annotations, not constraints

**Fallback behavior** (frontend):
- Missing `x-section`: Frontend groups in default "General" section
- Missing `x-tooltip`: Frontend uses `description` field as tooltip
- Malformed `x-*` properties: Frontend ignores and falls back to defaults

---

## Implementation Checklist

Phase 2 implementation follows this sequence:

- [ ] **Step 1**: Extend `FieldSchema` struct
  - Add `section: Option<String>` field
  - Add `tooltip: Option<String>` field
  - Update Serde attributes for optional fields
  - Update `FieldSchema::from_quill_value()` to parse new fields

- [ ] **Step 2**: Update JSON Schema generation
  - Modify `build_schema_from_fields()` in `crates/core/src/schema.rs`
  - Add `x-section` property when `field.section` is `Some`
  - Add `x-tooltip` property when `field.tooltip` is `Some`
  - Test generated schema structure

- [ ] **Step 3**: Documentation already updated
  - ✅ `prose/designs/SCHEMAS.md` updated with new fields
  - ✅ JSON Schema custom properties documented

- [ ] **Step 4**: Create/update example Quill
  - Choose appropriate example (or create new one)
  - Add section and tooltip to multiple fields
  - Demonstrate different section groupings
  - Test end-to-end (TOML → Schema → WASM)

- [ ] **Testing**: Run comprehensive test suite
  - Unit tests for `build_schema_from_fields()` with new fields
  - Unit tests for backward compatibility (without new fields)
  - Integration tests for Quill loading and schema generation
  - Regression tests for existing Quills
  - WASM bindings tests for metadata access

---

## Dependencies

**Depends on**: Phase 1 (Core Refactoring & Cleanup)
- Centralized registry makes metadata retrieval simpler
- Clean workflow creation enables easier testing

**Blocks**: Phase 3 (WASM API & UI Integration)
- Phase 3 builds wizard UIs using section/tooltip metadata
- Cannot implement dynamic UI without this foundation

**Related**: [`prose/designs/SCHEMAS.md`](../designs/SCHEMAS.md)
- Schema design document updated with new fields
- Maintains single source of truth for field definitions

---

## Success Criteria

Phase 2 is complete when:

1. ✅ `FieldSchema` includes optional `section` and `tooltip` fields
2. ✅ `build_schema_from_fields()` generates JSON Schema with `x-section` and `x-tooltip`
3. ✅ WASM `QuillInfo.schema` exposes UI metadata (no API changes needed)
4. ✅ Example Quill demonstrates section grouping and tooltips
5. ✅ All existing tests pass (backward compatibility maintained)
6. ✅ Documentation updated (already complete)

**Verification**:
- Run `cargo test --all-features`
- Load example Quill and inspect generated schema
- Retrieve via WASM and verify `x-section`/`x-tooltip` present in schema
- Confirm existing Quills still work without modifications
- Build WASM bindings: `wasm-pack build crates/bindings/wasm`

---

## Notes

### Why Field-Level Section (Not Quill-Level)?

The specification mentions "Add Section field to Quill definition" which could mean:
- **Option A**: Quill-level metadata (categorize entire quill as "Letter" vs "Report")
- **Option B**: Field-level metadata (group fields into sections within a quill)

**Chosen**: Option B (field-level)

**Rationale**:
- UI generation goal requires grouping **fields within a quill**, not quills themselves
- Example use case (resume template) needs sections like "Personal Info", "Education", "Experience"
- Quill-level categorization can be added later as `QuillConfig.category` if needed
- Matches mental model: sections organize form fields, not entire templates

**Alternative**: Quill-level `category` field for marketplace UI
- **Use case**: Template browsing ("Browse Letters", "Browse Reports")
- **Decision**: Out of scope for Phase 2
- **Future work**: Can add `QuillConfig.category` in separate enhancement

### Why x-* Properties in JSON Schema?

JSON Schema specification reserves `x-*` properties for custom extensions:

**Standard compliance**:
- Validators ignore unknown `x-*` properties per spec (forward compatible)
- Clear separation: standard properties (type, description) vs custom (x-section, x-tooltip)
- No risk of conflicting with future JSON Schema standards

**Alternative considered**: Top-level `ui_metadata` object
- **Downside**: Separates UI metadata from field definitions
- **Downside**: Frontend must merge metadata with schema properties
- **Chosen approach**: Embed metadata directly in field schemas for convenience

### Tooltip vs Description

**Question**: Why both `tooltip` and `description`?

**Answer**: Different UI contexts and verbosity levels

| Field | Purpose | Example |
|-------|---------|---------|
| `description` | Full explanation for documentation/help pages | "The full name of the document author. This will appear in the document header and PDF metadata." |
| `tooltip` | Short hint for inline UI help | "Your full name" |

**Usage**:
- Wizard UI: Show `tooltip` on hover, link to full `description`
- Documentation: Show `description` with examples
- API docs: Generate from `description` field

**Fallback**: If `tooltip` missing, frontend can use truncated `description` or first sentence

### JSON Schema as Single Source: Implementation Pattern

**Pattern**: Transform once, use everywhere

```rust
// 1. Parse TOML (ephemeral representation)
let config = QuillConfig::from_toml(&toml_content)?;

// 2. Transform to JSON Schema (authoritative)
let schema = build_schema_from_fields(&config.fields)?;

// 3. Extract cached values (derived from schema)
let defaults = extract_defaults_from_schema(&schema);
let examples = extract_examples_from_schema(&schema);

// 4. Store schema + caches (FieldSchema discarded)
let quill = Quill {
    schema,        // Authoritative
    defaults,      // Cache
    examples,      // Cache
    // ...
};

// 5. All operations use schema
validate_document(&quill.schema, &fields)?;       // Validation
let info = QuillInfo { schema: quill.schema };    // WASM API
```

**Key benefits**:
- FieldSchema is implementation detail (TOML parser)
- JSON Schema is the data model (validation, API, caching)
- Single transformation point (easy to maintain)
- Clear ownership (schema owns the data)
