# Validation Implementation Plan

This document outlines the medium-to-high level plan for implementing comprehensive schema validation in Quillmark, ensuring full compatibility with `designs/SCHEMAS.md`.

## Overview

The validation system will provide two levels of validation:
1. **Strict validation** during Quill loading - ensures Quills are properly configured
2. **Loose validation** during rendering - validates ParsedDocument data against the schema defined in Quill.toml

## Architecture

### Component Responsibilities

#### Backend Trait Extensions
The Backend trait will be extended to support schema validation requirements:
- Add `glue_extension_types()` method returning `&'static [&'static str]` - accepted glue file extensions
- Add `allow_auto_glue()` method returning `bool` - whether auto glue generation is allowed
- Do NOT maintain backwards compatability. Remove legacy/redundant logic (e.g., `Backend::glue_type`) from codebase.

#### Quill Validation (Strict)
Validation occurs in `quillmark-core/src/quill.rs::Quill::from_tree()`:

**Configuration Validation:**
- Validate `name` is present and non-empty
- Validate `backend` is present and matches a registered backend (deferred to registration time)
- Ensure `description` and other optional fields in `SCHEMAS.md` are optional
- If `glue_file` is specified:
  - Ensure file exists in the Quill file tree
  - Ensure extension matches one of `backend.glue_extension_types()`
- If `glue_file` is NOT specified:
  - Ensure `backend.allow_auto_glue()` is true
- If both `json_schema` and `[fields]` are defined:
  - Emit a warning that fields are overridden by json_schema

**Schema File Validation:**
- If `json_schema` path is specified, ensure the file exists in the Quill file tree
- Parse the JSON schema to validate it's valid JSON (syntax validation only). Consider also validating the JSON is a proper schema with `jsonschema`
- Store in the Quill struct in the `json_schema` property

#### ParsedDocument Validation (Loose)

Validation occurs in `quillmark/src/orchestration.rs` in two places:

**Internal Validation (in process_glue):**
- Before template composition, validate ParsedDocument against the schema
- Build JSON Schema from Quill configuration:
  - If `json_schema` is defined in Quill.toml, load and parse it
  - Otherwise, convert TOML `[fields]` section to JSON Schema
- Use `jsonschema` crate for validation
- Only validate fields that are specified in the schema
- Unspecified fields are allowed without constraint
- Collect validation errors and convert to RenderError with Diagnostic

**Public API:**
- Add `Workflow::validate(&self, parsed: &ParsedDocument) -> Result<(), RenderError>`
- Re-use internal validation logic for consistency
- Return validation errors as structured Diagnostics

## Implementation Details

### 1. Backend Trait Extension

**Location:** `quillmark-core/src/backend.rs`

Add new methods to the Backend trait:
```rust
fn glue_extension_types(&self) -> &'static [&'static str];
fn allow_auto_glue(&self) -> bool;
```

**Note:** The existing `glue_type()` method returns a single extension string (e.g., ".typ"). The new `glue_extension_types()` method returns an array to support backends that accept multiple glue file types. Delete the redundant `glue_type()` in favor of the new method.

Update all backend implementations:
- `quillmark-typst/src/lib.rs::TypstBackend` - return `&[".typ"]` for glue_extension_types, `true` for allow_auto_glue
- `quillmark-acroform/src/lib.rs::AcroformBackend` - return `&[".json"]` for glue_extension_types, `true` for allow_auto_glue

### 2. Schema Conversion Module

**Location:** `quillmark-core/src/validation.rs` (new file)

Create a new validation module containing:
- `FieldSchemaConverter` - converts TOML field definitions to JSON Schema
- `SchemaValidator` - wraps jsonschema validation logic
- Helper functions for building JSON Schema objects from field definitions

**Field Type Mapping:**
- `str` → `{"type": "string"}`
- `number` → `{"type": "number"}`
- `array` → `{"type": "array"}`
- `dict` → `{"type": "object"}`
- `date` → `{"type": "string", "format": "date"}`
- `datetime` → `{"type": "string", "format": "date-time"}`

**Required Fields:**
- If `default` is defined in field schema, field is optional (per SCHEMAS.md)
- Otherwise, field is required and added to `"required": []` array
- **Note:** The current `FieldSchema` struct has a `required` field. Delete this redundant field and modify existing Quills as necessary. During validation, the logic should be:
  - If `default` is present → field is optional
  - If `default` is absent and `required` is true → field is required
  - If `default` is absent and `required` is false → field is optional
  - This provides flexibility while honoring the SCHEMAS.md specification

**JSON Schema Structure:**
```json
{
  "$schema": "https://json-schema.org/draft/2019-09/schema",
  "type": "object",
  "properties": {
    "field_name": {
      "type": "...",
      "description": "..."
    }
  },
  "required": ["field1", "field2"],
  "additionalProperties": true
}
```

Note: `additionalProperties: true` allows unspecified fields.

### 3. Quill Loading Validation

**Location:** `quillmark-core/src/quill.rs`

**In `Quill::from_tree()`:**

After parsing Quill.toml, add validation:
```rust
// Validate glue_file vs auto_glue
if let Some(glue_file) = &quill.glue_file {
    // Validate file exists (already done)
    // Validate extension - requires backend reference (deferred to registration)
} else {
    // Validation of allow_auto_glue requires backend reference (deferred to registration)
}

// Validate json_schema file if specified
if let Some(json_schema_path) = metadata.get("json_schema") {
    let schema_file = root.get_file(json_schema_path)?;
    // Validate JSON syntax
    serde_json::from_slice::<serde_json::Value>(schema_file)?;
    
    // Warn if fields are also defined
    if !field_schemas.is_empty() {
        eprintln!("Warning: [fields] section is overridden by json_schema");
    }
}
```

### 4. Quill Registration Validation

**Location:** `quillmark/src/orchestration.rs`

**In `Quillmark::register_quill()`:**

Add validation that requires backend reference:
```rust
pub fn register_quill(&mut self, quill: Quill) -> Result<(), RenderError> {
    let name = quill.name.clone();
    
    // Check name uniqueness
    if self.quills.contains_key(&name) {
        return Err(RenderError::with_diagnostic(...));
    }
    
    // Get backend
    let backend_id = quill.backend.as_str();
    let backend = self.backends.get(backend_id).ok_or(...)?;
    
    // Validate glue_file extension or auto_glue
    if let Some(glue_file) = &quill.glue_file {
        let extension = Path::new(glue_file).extension()...;
        if !backend.glue_extension_types().contains(&extension) {
            return Err(...);
        }
    } else {
        if !backend.allow_auto_glue() {
            return Err(...);
        }
    }
    
    self.quills.insert(name, quill);
    Ok(())
}
```

Update callers to handle the Result return type.

### 5. Document Validation in Workflow

**Location:** `quillmark/src/orchestration.rs`

**Add private validation method:**
```rust
fn validate_document(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
    // Build or load JSON Schema
    let json_schema = if let Some(schema_path) = self.quill.metadata.get("json_schema") {
        // Load from quill files
        let schema_bytes = self.quill.get_file(schema_path)?;
        serde_json::from_slice(schema_bytes)?
    } else if !self.quill.field_schemas.is_empty() {
        // Convert from TOML fields
        validation::build_schema_from_fields(&self.quill.field_schemas)
    } else {
        // No schema defined, skip validation
        return Ok(());
    };
    
    // Compile JSON Schema
    let compiled = jsonschema::compile(&json_schema)?;
    
    // Convert ParsedDocument to JSON for validation
    let doc_json = parsed.to_json();
    
    // Validate
    if let Err(errors) = compiled.validate(&doc_json) {
        // Convert validation errors to Diagnostics
        return Err(RenderError::ValidationFailed { diag: ... });
    }
    
    Ok(())
}
```

**Update `process_glue()`:**
```rust
pub fn process_glue(&self, parsed: &ParsedDocument) -> Result<String, RenderError> {
    // Validate document against schema
    self.validate_document(parsed)?;
    
    // ... existing template composition logic
}
```

**Add public validation API:**
```rust
/// Validate a ParsedDocument against the Quill's schema
pub fn validate(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
    self.validate_document(parsed)
}
```

### 6. Dependencies

**Add to `Cargo.toml` workspace dependencies:**
```toml
jsonschema = "0.18"
```

**Add to `quillmark-core/Cargo.toml`:**
```toml
[dependencies]
jsonschema = { workspace = true }
```

### 7. Error Handling

**Add new error variant to `quillmark-core/src/error.rs`:**
```rust
#[error("Validation failed")]
ValidationFailed { diag: Diagnostic },

#[error("Invalid schema definition")]
InvalidSchema { diag: Diagnostic },
```

**Diagnostic codes:**
- `validation::field_required` - Required field missing
- `validation::field_type_mismatch` - Field value doesn't match expected type
- `validation::schema_invalid` - JSON schema file is invalid
- `quill::glue_extension_mismatch` - Glue file extension not supported by backend
- `quill::auto_glue_not_allowed` - Backend doesn't support auto glue
- `quill::name_collision` - Quill name already registered
- `quill::backend_not_found` - Backend specified in Quill.toml not registered

## Testing Strategy

### Unit Tests

**Backend trait tests** (`quillmark-core/src/backend.rs`):
- Test that backends correctly report glue extensions
- Test that backends correctly report auto_glue support

**Schema conversion tests** (`quillmark-core/src/validation.rs`):
- Convert simple field definitions to JSON Schema
- Convert complex field definitions with all types
- Handle required vs optional fields
- Handle default values

**Quill validation tests** (`quillmark-core/src/quill.rs`):
- Load Quill with valid glue_file
- Reject Quill with invalid glue extension (requires mock backend)
- Load Quill without glue_file when auto_glue is supported
- Reject Quill without glue_file when auto_glue not supported
- Load Quill with json_schema file
- Reject Quill with invalid json_schema file
- Warn when both json_schema and fields are defined

**Document validation tests** (`quillmark/src/orchestration.rs`):
- Validate document with all required fields present
- Reject document with missing required fields
- Reject document with wrong field types
- Accept document with extra fields not in schema
- Accept document when no schema is defined
- Validate using json_schema file
- Validate using TOML fields

### Integration Tests

**End-to-end validation** (`quillmark/tests/validation.rs`):
- Create Quillmark engine, register quill, validate documents
- Test full workflow with validation enabled
- Test validation errors propagate correctly to user

## Migration Notes

Since this is pre-1.0, breaking changes are acceptable:

1. **Backend trait changes** - All backend implementations must be updated
2. **Quillmark::register_quill()** - Now returns Result, callers must handle errors
3. **New validation step** - May reject previously accepted documents

## Future Enhancements

Not in scope for this implementation, but potential future work:

1. Custom validation error messages in field schemas
2. Pattern validation for strings
3. Enum validation for constrained string values
4. Cross-field validation rules
5. Validation warning mode (log warnings instead of errors)
6. Caching compiled JSON schemas for performance

## Success Criteria

Implementation is complete when:

1. All Backend implementations expose glue_extension_types and allow_auto_glue
2. Quill loading performs strict TOML validation per SCHEMAS.md
3. Workflow validates ParsedDocument against schema before rendering
4. Public Workflow::validate() API is available
5. All validation errors produce structured Diagnostics
6. Comprehensive test coverage for all validation paths
7. Documentation updated to reflect validation behavior
