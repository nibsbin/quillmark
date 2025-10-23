# Plan: Apply Default Field Values from Quill Schema to ParsedDocument

## Overview

This plan outlines how to apply default field values from a Quill's schema to a `ParsedDocument` within the end-to-end rendering flow. The solution will be clean, robust, and KISS-compliant.

## Problem Statement

Currently, Quill schemas can specify default values for fields (via the `default` key in `[fields]` or in a JSON schema), but these defaults are not automatically applied to ParsedDocuments. When a user omits a field that has a default value, the field simply doesn't exist in the ParsedDocument, which can cause validation failures or require template authors to handle missing values.

## Current Architecture

### Relevant Components

1. **`quillmark-core/src/parse.rs`**
   - `ParsedDocument` struct - immutable, contains `fields: HashMap<String, QuillValue>`
   - `from_markdown()` - parses markdown and creates ParsedDocument

2. **`quillmark-core/src/quill.rs`**
   - `FieldSchema` struct - contains `default: Option<QuillValue>`
   - `Quill` struct - contains `schema: QuillValue` (JSON Schema)

3. **`quillmark-core/src/validation.rs`**
   - `build_schema_from_fields()` - converts FieldSchema HashMap to JSON Schema
   - `validate_document()` - validates ParsedDocument against schema
   - Note: Current logic (line 57) has a bug - checks `default.is_none()` twice instead of checking `required`

4. **`quillmark/src/orchestration.rs`**
   - `Workflow::validate_document()` - calls validation (line 565)
   - `Workflow::process_glue()` - validates then renders (line 533-549)
   - **This is the key location** - validation happens right before template composition

### Current Flow

```
ParsedDocument::from_markdown(markdown)
  ↓
Workflow::render(parsed_doc)
  ↓
Workflow::process_glue(parsed_doc)
  ↓
validate_document(parsed_doc)  ← Current validation point
  ↓
template composition
  ↓
backend compilation
```

## Proposed Solution

### Design Principles

1. **Apply defaults near validation** - After parsing but before validation, in the workflow
2. **Minimal mutability** - Add a method to apply defaults rather than making ParsedDocument fully mutable
3. **KISS compliance** - Simple, straightforward implementation without complex state management
4. **Separation of concerns** - Default application is a transformation step, separate from parsing

### Implementation Plan

#### 1. Add Method to ParsedDocument (quillmark-core/src/parse.rs)

Add a method to create a new ParsedDocument with defaults applied:

```rust
impl ParsedDocument {
    /// Create a new ParsedDocument with default values applied from a schema
    pub fn with_defaults(
        &self, 
        field_schemas: &HashMap<String, FieldSchema>
    ) -> Self {
        let mut fields = self.fields.clone();
        
        for (field_name, schema) in field_schemas {
            // Only apply default if field is missing and default exists
            if !fields.contains_key(field_name) {
                if let Some(ref default_value) = schema.default {
                    fields.insert(field_name.clone(), default_value.clone());
                }
            }
        }
        
        Self {
            fields,
            quill_tag: self.quill_tag.clone(),
        }
    }
}
```

**Rationale:**
- Immutable approach - returns new ParsedDocument instead of mutating
- Simple HashMap operations - O(n) where n = number of schema fields
- Preserves existing fields - defaults only fill in missing values
- Clone-based - safe and straightforward, not a performance concern for typical document sizes

#### 2. Extract FieldSchemas from Quill Schema (quillmark-core/src/quill.rs)

Since schemas can come from either `[fields]` or `json_schema_file`, we need a way to extract default values from the compiled JSON Schema back to a usable format.

**Option A: Store field_schemas separately in Quill**
- Modify `Quill` struct to keep `field_schemas: HashMap<String, FieldSchema>`
- Modify `QuillConfig::from_toml` to preserve field schemas
- Modify `Quill::from_config` to store them

**Option B: Parse defaults from JSON Schema**
- Write a function to extract default values from compiled schema
- More complex but handles both `[fields]` and `json_schema_file` uniformly

**Recommended: Option A** - Simpler, more explicit, and we already have the field schemas during Quill construction.

Modifications:
```rust
pub struct Quill {
    // ... existing fields ...
    pub field_schemas: HashMap<String, FieldSchema>,  // Add this
}
```

#### 3. Apply Defaults in Workflow (quillmark/src/orchestration.rs)

Modify `process_glue` to apply defaults before validation:

```rust
pub fn process_glue(&self, parsed: &ParsedDocument) -> Result<String, RenderError> {
    // Apply defaults from field schemas
    let parsed_with_defaults = parsed.with_defaults(&self.quill.field_schemas);
    
    // Validate document against schema
    self.validate_document(&parsed_with_defaults)?;
    
    // Create glue and compose
    let mut glue = match &self.quill.glue {
        Some(s) if !s.is_empty() => Glue::new(s.to_string()),
        _ => Glue::new_auto(),
    };
    self.backend.register_filters(&mut glue);
    let glue_output = glue.compose(parsed_with_defaults.fields().clone())
        .map_err(|e| RenderError::TemplateFailed {
            diag: Diagnostic::new(Severity::Error, e.to_string())
                .with_code("template::compose".to_string()),
        })?;
    Ok(glue_output)
}
```

#### 4. Fix Validation Logic Bug (quillmark-core/src/validation.rs)

Fix line 57 which incorrectly checks `default.is_none()` twice:

```rust
// Current (INCORRECT):
if field_schema.default.is_none() && field_schema.default.is_none() {
    required_fields.push(field_name.clone());
}

// Fixed:
if field_schema.default.is_none() {
    // If no default, field is required
    required_fields.push(field_name.clone());
}
```

**Note:** The `required` field in FieldSchema is not currently used in the code. The schema spec states that if a default is present, the field is optional regardless of the required flag. So we only need to check if default is absent.

## Testing Strategy

### Unit Tests

1. **ParsedDocument::with_defaults()** (in parse.rs tests)
   - Test applying defaults to empty document
   - Test defaults don't override existing values
   - Test multiple defaults applied correctly
   - Test with no defaults (no-op)
   - Test with partial defaults

2. **Workflow default application** (in orchestration tests)
   - Test document with missing optional field gets default
   - Test document with all fields doesn't get defaults applied
   - Test validation passes with defaults applied
   - Test validation fails when required field missing (no default)

### Integration Tests

1. End-to-end rendering with defaults
2. Quill with field schemas containing defaults
3. Markdown missing optional fields that have defaults

## Edge Cases and Considerations

1. **Null vs Missing Fields**
   - A field explicitly set to null should remain null
   - Only truly missing fields get defaults

2. **Array/Object Defaults**
   - Default values can be complex types (arrays, objects)
   - QuillValue::clone() handles this correctly

3. **Performance**
   - Cloning ParsedDocument is acceptable for typical document sizes
   - Most documents have < 50 fields, HashMap clone is O(n)

4. **JSON Schema Files**
   - If using `json_schema_file`, we won't have FieldSchema objects
   - Solution: Don't apply defaults for json_schema_file quills (or extract from schema)
   - Initial implementation: Only support defaults from `[fields]` section

5. **Backward Compatibility**
   - This is pre-1.0, no backward compatibility required
   - Behavior change: Documents will now have default values applied
   - This is a feature addition, not a breaking change

## Implementation Sequence

1. Fix validation bug in `validation.rs`
2. Add `field_schemas` to `Quill` struct and populate during construction
3. Add `with_defaults()` method to `ParsedDocument`
4. Modify `Workflow::process_glue()` to apply defaults
5. Add unit tests for each component
6. Add integration tests
7. Update documentation if needed

## Summary

This plan provides a clean, KISS-compliant solution for applying default values:

- **Location**: Near validation in the workflow (before validation, after parsing)
- **Approach**: Immutable transformation via `with_defaults()` method
- **Mutability**: Minimal - returns new ParsedDocument, doesn't mutate original
- **Complexity**: Low - simple HashMap operations and cloning
- **Robustness**: Handles edge cases, preserves existing behavior for fields without defaults

The implementation follows the principle of least surprise and maintains clear separation of concerns between parsing, default application, validation, and rendering.
