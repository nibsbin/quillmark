# Quill Annotation Revamp - Phase 2 Plan

**Status**: Design Phase
**Goal**: Establish a single source of truth for Quill configuration that supports dynamic UI generation
**Related**: [`prose/QUILL_ANNOTATION.md`](../QUILL_ANNOTATION.md), [`prose/designs/SCHEMAS.md`](../designs/SCHEMAS.md), [`prose/designs/QUILL.md`](../designs/QUILL.md)

---

## Overview

Phase 2 extends Quill metadata to enable dynamic wizard UIs in frontend applications by:

1. **Adding field-level UI metadata** - Section grouping and tooltip help text
2. **Maintaining jsonschema as internal** - Keep implementation details private
3. **Enabling rich metadata exposure** - Serializable fields for WASM/Python consumers

This plan follows the architectural principle: **extend the schema at the source, not at the API boundary**. By adding metadata fields to `FieldSchema` (the TOML representation), they automatically flow through the entire system—internal validation, JSON schema generation, and WASM/Python bindings—without requiring API-specific transformations.

---

## Current State Analysis

### Metadata Flow Architecture

**TOML → Rust → JSON Schema → WASM**

1. **Source**: `Quill.toml` with `[fields]` section
   ```toml
   [fields.author]
   description = "Author of the document"
   type = "str"
   default = "Anonymous"
   ```

2. **Parse**: `QuillConfig` struct with `fields: HashMap<String, FieldSchema>`
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

3. **Transform**: Generate JSON schema from `FieldSchema` (in `Quill::from_config()`)
   ```rust
   pub struct Quill {
       pub schema: QuillValue,  // Generated JSON schema
       pub defaults: HashMap<String, QuillValue>,  // Extracted from schema
       pub examples: HashMap<String, Vec<QuillValue>>,  // Extracted from schema
   }
   ```

4. **Expose**: WASM `QuillInfo` serializes `schema`, `defaults`, `examples`
   ```rust
   pub struct QuillInfo {
       pub schema: serde_json::Value,
       pub defaults: serde_json::Value,
       pub examples: serde_json::Value,
   }
   ```

**Location**: `crates/core/src/quill.rs:12-24` (FieldSchema), `crates/core/src/quill.rs:384-403` (Quill), `crates/bindings/wasm/src/types.rs:148-166` (QuillInfo)

### UI Generation Gap

**Problem**: Frontend wizard UIs need:
- **Section grouping**: Organize fields into collapsible panels ("Author Info", "Document Settings")
- **Tooltips**: Provide contextual help without cluttering the main UI

**Current limitations**:
- `FieldSchema.description` is human-readable but too verbose for tooltips
- No way to group related fields into UI sections
- Frontend must hardcode UI organization logic

**Example use case**: A resume template with 15 fields needs:
- Section "Personal Info": name, email, phone
- Section "Education": degree, university, graduation_date
- Section "Experience": job_title, company, start_date, end_date
- Section "Skills": skills_list, certifications

---

## Desired State

### 1. Extended FieldSchema

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

**Benefits**:
- **Single source of truth**: TOML fields define all UI metadata
- **DRY principle**: Section/tooltip propagate automatically to JSON schema
- **Backward compatible**: Optional fields don't break existing Quills

### 2. JSON Schema Extension

**Extend JSON schema generation** to include section/tooltip as custom properties:

```json
{
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
- JSON Schema standard supports custom `x-*` properties
- Keeps section/tooltip separate from validation logic
- Frontend can extract and use these properties for UI generation
- Validation logic ignores unknown `x-*` properties (forward compatible)

**Location**: Schema generation in `crates/core/src/quill.rs` (around line 500-600)

### 3. WASM Metadata Access

**No changes required** to `QuillInfo` structure:

The `schema` field already exposes the complete JSON schema, which now includes `x-section` and `x-tooltip` properties. Frontend consumers can parse these properties from the schema.

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

**Impact**:
- Add Serde deserialization for new fields
- Update `FieldSchema::new()` constructor if exists
- Existing Quills continue to work (fields are optional)

### Step 2: Update TOML Deserialization

**Ensure** `QuillConfig` deserialization handles new fields:

**Location**: `crates/core/src/quill.rs:407-430`

**Impact**:
- Serde automatically handles new optional fields
- Test with Quill.toml files that include/exclude these fields
- Verify backward compatibility with existing Quills

### Step 3: Extend JSON Schema Generation

**Modify** schema generation logic to include `x-section` and `x-tooltip`:

**Location**: Schema generation in `crates/core/src/quill.rs` (look for `to_json_schema()` or similar)

**Logic**:
```rust
// When building JSON schema for a field:
if let Some(section) = &field_schema.section {
    json_field["x-section"] = json!(section);
}
if let Some(tooltip) = &field_schema.tooltip {
    json_field["x-tooltip"] = json!(tooltip);
}
```

**Impact**:
- Generated JSON schemas now include UI metadata
- WASM/Python consumers automatically receive this data
- Validation logic unaffected (validators ignore unknown `x-*` properties)

### Step 4: Update SCHEMAS.md Design Document

**Document** the new fields in the design specification:

**Location**: `prose/designs/SCHEMAS.md`

**Add** to Quill Field properties section:
```markdown
- section -> Option[str]: UI section grouping (e.g., "Author Info", "Document Settings")
- tooltip -> Option[str]: Short help text for field (displayed in UI hints)
```

**Add** JSON Schema extension documentation:
```markdown
### JSON Schema Custom Properties

Field schemas support custom `x-*` properties for UI metadata:

- `x-section`: Groups fields into collapsible UI sections
- `x-tooltip`: Provides short help text for tooltips/hints

These properties are ignored by validation logic but consumed by frontend UIs.
```

### Step 5: Create Example Quill with Sections

**Create** or update an example Quill to demonstrate the feature:

**Location**: `examples/` directory (choose appropriate example)

**Example**:
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
- Tests the entire metadata flow

---

## Architecture Decision: JSON Schema as Internal

**Decision**: Keep jsonschema as internal implementation detail

**Rationale** (from Phase 1):
- **Source of truth**: TOML `[fields]` in `Quill.toml`
- **Validation**: Use generated JSON schema internally
- **API**: Expose schema via `QuillInfo.schema` but don't require consumers to understand jsonschema internals

**Implications for Phase 2**:

1. **Schema generation is internal logic**
   - Consumers receive JSON schema but aren't expected to generate it
   - Custom `x-*` properties are implementation details exposed for convenience

2. **TOML fields remain authoritative**
   - `section` and `tooltip` are defined in `FieldSchema`
   - JSON schema `x-section` and `x-tooltip` are derived, not primary

3. **Future flexibility**
   - Can change schema generation logic without breaking API
   - Can optimize caching/extraction independently
   - Can add more `x-*` properties without API changes

**Benefits**:
- Frontend doesn't need jsonschema libraries
- Rust code owns validation logic
- Clear separation: TOML (input) → JSON Schema (internal) → WASM (output)

---

## Cross-Cutting Concerns

### Backward Compatibility

**Breaking Changes**: None

- New fields are optional in `FieldSchema`
- Existing Quills without section/tooltip continue to work
- JSON Schema `x-*` properties are ignored by validators
- WASM API surface unchanged (metadata embedded in existing `schema` field)

**Migration path**:
- Old Quills: Work unchanged, no sections/tooltips in UI
- New Quills: Can opt-in to sections/tooltips for better UX
- Gradual adoption: Quill developers update at their own pace

### Testing Strategy

**Unit tests**:
- Parse TOML with section/tooltip fields
- Parse TOML without section/tooltip fields (backward compat)
- Generate JSON schema with `x-section` and `x-tooltip`
- Verify schema validates documents correctly (ignores custom properties)

**Integration tests**:
- Load example Quill with sections
- Retrieve via WASM `getQuillInfo()`
- Parse schema and extract section/tooltip metadata
- Verify defaults and examples unaffected

**Regression tests**:
- Load all existing example Quills
- Verify they parse and register correctly
- Confirm schemas still validate documents

### Validation Behavior

**Important**: JSON Schema validators MUST ignore unknown `x-*` properties

**Verification**:
- Test that documents validate correctly regardless of `x-section`/`x-tooltip` presence
- Ensure validation errors don't reference custom properties
- Confirm schema evolution (adding new `x-*` fields) doesn't break validation

**Fallback behavior**:
- Missing `section`: Frontend groups in default "General" section
- Missing `tooltip`: Frontend uses `description` as tooltip
- Malformed `x-*` properties: Frontend ignores and falls back to defaults

---

## Implementation Checklist

Phase 2 implementation follows this sequence:

- [ ] **Step 1**: Extend `FieldSchema` struct
  - Add `section: Option<String>` field
  - Add `tooltip: Option<String>` field
  - Update struct documentation

- [ ] **Step 2**: Verify TOML deserialization
  - Test with Quill.toml containing new fields
  - Test with Quill.toml missing new fields
  - Ensure backward compatibility

- [ ] **Step 3**: Update JSON Schema generation
  - Find schema generation logic in quill.rs
  - Add `x-section` property when `field.section` is `Some`
  - Add `x-tooltip` property when `field.tooltip` is `Some`
  - Test generated schema structure

- [ ] **Step 4**: Update SCHEMAS.md design document
  - Document new FieldSchema fields
  - Document JSON Schema custom properties
  - Add usage examples

- [ ] **Step 5**: Create/update example Quill
  - Choose appropriate example (or create new one)
  - Add section and tooltip to multiple fields
  - Demonstrate different section groupings

- [ ] **Testing**: Run comprehensive test suite
  - Unit tests for FieldSchema parsing
  - Integration tests for schema generation
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
- Schema design document should be updated with new fields
- Maintains single source of truth for field definitions

---

## Success Criteria

Phase 2 is complete when:

1. ✅ `FieldSchema` includes optional `section` and `tooltip` fields
2. ✅ JSON Schema generation includes `x-section` and `x-tooltip` properties
3. ✅ WASM `QuillInfo.schema` exposes UI metadata (no API changes needed)
4. ✅ Example Quill demonstrates section grouping and tooltips
5. ✅ All existing tests pass (backward compatibility maintained)
6. ✅ Documentation updated (`SCHEMAS.md` reflects new fields)

**Verification**:
- Run `cargo test --all-features`
- Load example Quill and inspect generated schema
- Retrieve via WASM and verify `x-section`/`x-tooltip` present
- Confirm existing Quills still work without modifications

---

## Notes

### Why Field-Level Section (Not Quill-Level)?

The initial specification said "Add Section field to Quill definition" which could be interpreted as:
- **Option A**: Quill-level metadata (e.g., categorize entire quill as "Letter" vs "Report")
- **Option B**: Field-level metadata (e.g., group fields into sections within a quill)

**Chosen**: Option B (field-level)

**Rationale**:
- UI generation goal requires grouping **fields within a quill**, not quills themselves
- Example use case (resume template) needs sections like "Personal Info", "Education", "Experience"
- Quill-level categorization can be added later as `QuillConfig.category` if needed
- Matches mental model: sections organize form fields, not entire templates

**Alternative considered**: Quill-level `category` field
- **Use case**: Template marketplace UI ("Browse Letters", "Browse Reports")
- **Decision**: Out of scope for Phase 2 (focused on wizard UI generation)
- **Future work**: Can add `QuillConfig.category` in separate enhancement

### Why x-* Properties in JSON Schema?

JSON Schema specification reserves `x-*` properties for custom extensions:

**Standard compliance**:
- Validators ignore unknown `x-*` properties (forward compatible)
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
