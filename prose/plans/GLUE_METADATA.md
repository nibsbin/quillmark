# Implementation Plan: Glue Metadata Access

This plan outlines the steps to implement the `__metadata__` field feature described in [designs/GLUE_METADATA.md](../designs/GLUE_METADATA.md).

## Overview

Expose a `__metadata__` field in MiniJinja template contexts containing all parsed document fields except `body`. This provides template authors with convenient access to metadata without body content.

## Prerequisites

- Review [designs/GLUE_METADATA.md](../designs/GLUE_METADATA.md) for design rationale
- Understand current template composition in `quillmark-core/src/templating.rs`
- Understand parsed document structure in `quillmark-core/src/parse.rs`

## Implementation Steps

### 1. Add Import for BODY_FIELD Constant

**File**: `quillmark-core/src/templating.rs`

Add import at the top of the file:
```rust
use crate::parse::BODY_FIELD;
```

**Rationale**: Per design decision, directly import the constant from `parse.rs` for clean dependency and avoiding duplication.

### 2. Update `convert_quillvalue_to_minijinja` Helper

**File**: `quillmark-core/src/templating.rs`

This helper currently converts all fields to MiniJinja values. No changes needed - it will be reused for both full context and metadata subset.

### 3. Update `TemplateGlue::compose` Method

**File**: `quillmark-core/src/templating.rs` (lines ~169-203)

**Changes**:
1. Before converting fields to MiniJinja, separate metadata from body
2. Convert metadata fields separately
3. Create a MiniJinja object/map from metadata fields
4. Add `__metadata__` to the context before rendering

**Implementation**:
```rust
fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError> {
    // Separate metadata from body
    let metadata_fields: HashMap<String, QuillValue> = context
        .iter()
        .filter(|(key, _)| key.as_str() != BODY_FIELD)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Convert QuillValue to MiniJinja values
    let mut minijinja_context = convert_quillvalue_to_minijinja(context)?;
    let metadata_minijinja = convert_quillvalue_to_minijinja(metadata_fields)?;
    
    // Add __metadata__ field as a MiniJinja Value
    minijinja_context.insert(
        "__metadata__".to_string(), 
        minijinja::value::Value::from_serialize(&metadata_minijinja)
    );

    // Create environment and render (existing code continues)
    // ...
}
```

### 4. Update `AutoGlue::compose` Method

**File**: `quillmark-core/src/templating.rs` (lines ~224-246)

**Changes**:
1. Add `__metadata__` field to JSON output
2. Ensure `__metadata__` contains all fields except body

**Implementation**:
```rust
fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError> {
    // Separate metadata from body
    let metadata_fields: HashMap<String, QuillValue> = context
        .iter()
        .filter(|(key, _)| key.as_str() != BODY_FIELD)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    
    // Convert context to JSON
    let mut json_map = serde_json::Map::new();
    for (key, value) in &context {
        json_map.insert(key.clone(), value.as_json().clone());
    }
    
    // Add __metadata__ object
    let mut metadata_json = serde_json::Map::new();
    for (key, value) in &metadata_fields {
        metadata_json.insert(key.clone(), value.as_json().clone());
    }
    json_map.insert(
        "__metadata__".to_string(), 
        serde_json::Value::Object(metadata_json)
    );

    let json_value = serde_json::Value::Object(json_map);
    // ... rest of existing code
}
```

### 5. Add Unit Tests

**File**: `quillmark-core/src/templating.rs`

Add tests in the `mod tests` section:

**Test Cases**:
1. `test_metadata_field_excludes_body` - Verify `__metadata__` doesn't contain body
2. `test_metadata_field_includes_frontmatter` - Verify all non-body fields are in `__metadata__`
3. `test_metadata_field_empty_when_only_body` - Verify empty `__metadata__` when no frontmatter
4. `test_backward_compatibility_top_level_access` - Verify existing field access still works
5. `test_metadata_iteration_in_template` - Test iterating over `__metadata__` in template
6. `test_auto_glue_metadata_field` - Verify `__metadata__` in JSON output

**Example Test**:
```rust
#[test]
fn test_metadata_field_excludes_body() {
    let template = "{% for key in __metadata__ %}{{ key }},{% endfor %}";
    let mut glue = Glue::new(template.to_string());
    
    let mut context = HashMap::new();
    context.insert("title".to_string(), QuillValue::from_json(json!("Test")));
    context.insert("author".to_string(), QuillValue::from_json(json!("John")));
    context.insert("body".to_string(), QuillValue::from_json(json!("Body content")));
    
    let result = glue.compose(context).unwrap();
    
    // Should contain title and author, but not body
    assert!(result.contains("title"));
    assert!(result.contains("author"));
    assert!(!result.contains("body"));
}
```

### 6. Add Integration Tests (Optional)

**File**: Create `quillmark/tests/metadata_integration.rs`

Test end-to-end workflow with backends to ensure `__metadata__` works correctly in real templates.

### 7. Update Documentation

**Files to Update**:

1. **prose/designs/ARCHITECTURE.md**
   - Add cross-reference to GLUE_METADATA.md in Template System Design section
   - Mention `__metadata__` in the template context description

2. **quillmark-core/src/templating.rs** (module documentation)
   - Add example showing `__metadata__` usage
   - Document the field in the module overview

3. **README.md** (if applicable)
   - Add mention of `__metadata__` in template features

## Testing Strategy

### Manual Testing

1. Create test markdown with frontmatter:
```markdown
---
title: Test Document
author: Test Author
tags: [test, example]
---

# Body Content

This is the body.
```

2. Create test template using `__metadata__`:
```jinja
Metadata fields: {{ __metadata__ | length }}

{% for key, value in __metadata__ %}
  {{ key }}: {{ value }}
{% endfor %}

Body: {{ body }}
```

3. Run through orchestration and verify output

### Automated Testing

Run existing test suite to ensure no regressions:
```bash
cargo test --package quillmark-core
cargo test --package quillmark
```

## Rollout Considerations

### Backward Compatibility

- Existing templates continue to work without modification
- Top-level field access remains unchanged
- New `__metadata__` field is purely additive

### Performance Impact

- Minimal: only adds one HashMap filter operation and one context insert
- No impact on templates that don't use `__metadata__`
- Small overhead for templates that use it (acceptable trade-off for convenience)

### Future Extensibility

The `__` prefix establishes a convention for system-generated fields:
- `__metadata__`: Current implementation
- `__schema__`: Future enhancement for schema access
- `__quill__`: Future enhancement for quill metadata
- `__version__`: Future enhancement for version info

## Dependencies

### Modified Files
- `quillmark-core/src/templating.rs` - Core implementation
- `prose/designs/GLUE_METADATA.md` - Design document (created)
- `prose/designs/ARCHITECTURE.md` - Updated references

### No Changes Required
- `quillmark-core/src/parse.rs` - Only importing constant
- Backend implementations - Work transparently with new field
- Existing templates - Backward compatible

## Success Criteria

- [ ] `__metadata__` field accessible in MiniJinja templates
- [ ] `__metadata__` contains all non-body fields
- [ ] `body` field excluded from `__metadata__`
- [ ] All existing tests pass
- [ ] New unit tests for `__metadata__` pass
- [ ] Documentation updated
- [ ] No performance regression
- [ ] Backward compatibility maintained

## References

- [designs/GLUE_METADATA.md](../designs/GLUE_METADATA.md) - Design document
- [designs/PARSE.md](../designs/PARSE.md) - Document structure
- [designs/ARCHITECTURE.md](../designs/ARCHITECTURE.md) - System architecture
