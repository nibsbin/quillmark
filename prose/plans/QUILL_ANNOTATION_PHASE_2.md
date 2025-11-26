# Quill Annotation Revamp - Phase 2 Plan

**Status**: Design Phase
**Goal**: Establish JSON Schema as the single source of truth for Quill field metadata that supports dynamic UI generation
**Related**: [`prose/QUILL_ANNOTATION.md`](../QUILL_ANNOTATION.md), [`prose/designs/SCHEMAS.md`](../designs/SCHEMAS.md), [`prose/designs/QUILL.md`](../designs/QUILL.md)

---

## Overview

Phase 2 extends Quill metadata to enable dynamic wizard UIs in frontend applications by:

1. **Adding field-level UI metadata** - Section grouping and tooltip help text
2. **Maintaining JSON Schema as authoritative** - Single source of truth for validation and metadata
3. **Enabling rich metadata exposure** - Custom properties flow naturally through existing WASM API

This plan follows the architectural principle: **transform at construction, then rely on the schema**. TOML fields are parsed and immediately converted to JSON Schema with custom properties. The JSON Schema then serves as the authoritative source for validation, defaults extraction, and API exposure.

---

## Current State Analysis

### Metadata Flow Architecture

**TOML → FieldSchema → JSON Schema (Authoritative)**

1. **Input**: `Quill.toml` with `[fields]` section
   ```toml
   [fields.author]
   description = "Author of the document"
   type = "str"
   default = "Anonymous"
   ```

2. **Parse**: `QuillConfig::from_toml()` creates `HashMap<String, FieldSchema>`
   ```rust
   pub struct FieldSchema {
       pub name: String,
       pub r#type: Option<String>,
       pub description: String,
       pub default: Option<QuillValue>,
       pub example: Option<QuillValue>,
       pub examples: Option<QuillValue>,
   }
   ```

3. **Transform**: `build_schema_from_fields()` generates JSON Schema (single source of truth)
   ```rust
   // Location: crates/core/src/schema.rs:11-90
   pub fn build_schema_from_fields(
       field_schemas: &HashMap<String, FieldSchema>,
   ) -> Result<QuillValue, RenderError>
   ```

4. **Store**: JSON Schema stored in `Quill.schema` as authoritative source
   ```rust
   pub struct Quill {
       pub schema: QuillValue,  // Authoritative JSON Schema
       pub defaults: HashMap<String, QuillValue>,  // Cached from schema
       pub examples: HashMap<String, Vec<QuillValue>>,  // Cached from schema
   }
   ```

5. **Use**: All downstream operations use the JSON Schema:
   - **Validation**: `validate_document()` uses `jsonschema::Validator` on `schema`
   - **Defaults**: `extract_defaults_from_schema()` reads from `schema.properties[field].default`
   - **Examples**: `extract_examples_from_schema()` reads from `schema.properties[field].examples`
   - **WASM API**: `QuillInfo.schema` exposes the complete JSON Schema

**Key Insight**: `FieldSchema` is an input format (TOML representation). The JSON Schema is the actual data model used throughout the system. FieldSchema instances are discarded after schema generation.

**Location**:
- Schema generation: `crates/core/src/schema.rs:11-90`
- Quill construction: `crates/core/src/quill.rs:671-782`
- Extraction utilities: `crates/core/src/schema.rs:92-167`

### UI Generation Gap

**Problem**: Frontend wizard UIs need:
- **Section grouping**: Organize fields into collapsible panels ("Author Info", "Document Settings")
- **Tooltips**: Provide contextual help without cluttering the main UI

**Current limitations**:
- `FieldSchema.description` is verbose (used for full documentation)
- No way to group related fields into UI sections
- Frontend must hardcode UI organization logic

**Example use case**: A resume template with 15 fields needs:
- Section "Personal Info": name, email, phone
- Section "Education": degree, university, graduation_date
- Section "Experience": job_title, company, start_date, end_date
- Section "Skills": skills_list, certifications

---

## Desired State

### 1. Extended FieldSchema (TOML Input)

**Add two new optional fields** to `FieldSchema`:

```rust
pub struct FieldSchema {
    pub name: String,
    pub r#type: Option<String>,
    pub description: String,
    pub default: Option<QuillValue>,
    pub example: Option<QuillValue>,
    pub examples: Option<QuillValue>,
    pub section: Option<String>,    // NEW: UI section grouping
    pub tooltip: Option<String>,    // NEW: Short help text
}
```

**TOML representation**:
```toml
[fields.author]
description = "The full name of the document author. This will appear in the document header and metadata."
type = "str"
section = "Author Info"
tooltip = "Your full name"

[fields.email]
description = "Contact email address for the author."
type = "str"
section = "Author Info"
tooltip = "Your email address"

[fields.title]
description = "The main title of the document. This will be prominently displayed at the top."
type = "str"
section = "Document Settings"
tooltip = "Document title"
```

**Purpose**: `FieldSchema` is the TOML input format. New fields enable authors to specify UI metadata alongside field definitions.

### 2. JSON Schema with Custom Properties (Authoritative)

**Extend `build_schema_from_fields()`** to include `x-section` and `x-tooltip`:

```rust
// In build_schema_from_fields() at crates/core/src/schema.rs

// After adding description (around line 48)
property.insert(
    "description".to_string(),
    Value::String(field_schema.description.clone()),
);

// ADD: Include section if specified
if let Some(ref section) = field_schema.section {
    property.insert(
        "x-section".to_string(),
        Value::String(section.clone()),
    );
}

// ADD: Include tooltip if specified
if let Some(ref tooltip) = field_schema.tooltip {
    property.insert(
        "x-tooltip".to_string(),
        Value::String(tooltip.clone()),
    );
}
```

**Generated JSON Schema**:
```json
{
  "$schema": "https://json-schema.org/draft/2019-09/schema",
  "type": "object",
  "properties": {
    "author": {
      "type": "string",
      "description": "The full name of the document author...",
      "default": "Anonymous",
      "x-section": "Author Info",
      "x-tooltip": "Your full name"
    },
    "email": {
      "type": "string",
      "description": "Contact email address for the author.",
      "x-section": "Author Info",
      "x-tooltip": "Your email address"
    }
  }
}
```

**Rationale**:
- JSON Schema standard supports custom `x-*` properties for extensions
- Validators ignore unknown `x-*` properties (forward compatible)
- Custom properties are part of the schema (not separate metadata)
- Extraction utilities (`extract_defaults_from_schema()`, etc.) unaffected

**Location**: `crates/core/src/schema.rs:11-90` (modify `build_schema_from_fields()`)

### 3. WASM Metadata Access (No Changes)

**No changes required** to `QuillInfo` structure:

The `schema` field already exposes the complete JSON Schema including custom properties. Frontend consumers parse `x-section` and `x-tooltip` from the schema.

**Frontend usage example**:
```typescript
const quillInfo = engine.getQuillInfo('resume');
const schema = quillInfo.schema;

// Extract fields with sections
const fieldsBySection = {};
for (const [fieldName, fieldSchema] of Object.entries(schema.properties)) {
  const section = fieldSchema['x-section'] || 'General';
  const tooltip = fieldSchema['x-tooltip'] || fieldSchema.description;

  if (!fieldsBySection[section]) {
    fieldsBySection[section] = [];
  }

  fieldsBySection[section].push({
    name: fieldName,
    type: fieldSchema.type,
    description: fieldSchema.description,
    tooltip: tooltip,
    default: fieldSchema.default,
  });
}

// Render wizard UI with sections
for (const [section, fields] of Object.entries(fieldsBySection)) {
  renderSection(section, fields);  // Collapsible panel with tooltips
}
```

**Benefits**:
- Zero API surface changes (backward compatible)
- Frontend has full control over UI rendering
- Metadata flows naturally through existing schema mechanism
- JSON Schema remains single source of truth

---

## Architecture Decision: JSON Schema as Single Source of Truth

**Decision**: Keep JSON Schema as the authoritative source for all field metadata

**Flow**:
```
TOML [fields]
  ↓ parse
FieldSchema (temporary)
  ↓ transform
JSON Schema (authoritative)
  ↓ extract
Defaults, Examples (cached)
  ↓ expose
WASM QuillInfo.schema
```

**Why JSON Schema is Authoritative**:

1. **Validation uses JSON Schema**
   - `validate_document()` compiles `schema` with `jsonschema::Validator`
   - All validation logic operates on the JSON Schema, not FieldSchema

2. **Caching extracts from JSON Schema**
   - `extract_defaults_from_schema()` reads `schema.properties[field].default`
   - `extract_examples_from_schema()` reads `schema.properties[field].examples`
   - Cached values (`Quill.defaults`, `Quill.examples`) are derived from schema

3. **API exposes JSON Schema**
   - WASM `QuillInfo.schema` serializes the JSON Schema
   - Python bindings expose schema (not FieldSchema)
   - Frontend consumers never see FieldSchema

4. **FieldSchema is ephemeral**
   - Only exists during `Quill::from_config()` construction
   - Discarded after `build_schema_from_fields()` completes
   - Not stored in `Quill` struct

**Implications for Phase 2**:

- **Source of input**: TOML `[fields]` (parsed to `FieldSchema`)
- **Transformation point**: `build_schema_from_fields()` (add `x-*` properties)
- **Authoritative storage**: `Quill.schema` (JSON Schema with custom properties)
- **API exposure**: `QuillInfo.schema` (no changes needed)

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
