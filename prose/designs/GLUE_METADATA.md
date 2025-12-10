# Plate Metadata Access

This document details the design for exposing document metadata to MiniJinja templates through a dedicated `__metadata__` field.

> **Implementation**: `quillmark-core/src/templating.rs`

## Overview

The plate templating system provides template authors with access to parsed document fields via MiniJinja expressions. Currently, all fields (including `body`) are available at the top level of the template context. This design introduces a special `__metadata__` field that aggregates all non-body fields for convenient metadata-only access.

**Key capabilities:**
- Access all frontmatter fields through `__metadata__` variable
- Separate metadata from body content for clearer template semantics
- Maintain backward compatibility with existing top-level field access
- Enable metadata iteration and inspection without including body content

## Design Principles

### 1. Convenience Without Breaking Changes

The `__metadata__` field is **additive**: existing templates continue to work as-is. Template authors can choose between:
- Top-level access: `{{ title }}`, `{{ author }}`, `{{ body }}`
- Metadata object access: `{{ __metadata__.title }}`, `{{ __metadata__.author }}`
- Body-only access: `{{ body }}` (not in `__metadata__`)

### 2. Semantic Clarity

The `__metadata__` field makes template intent clearer:
```jinja
{% for key, value in __metadata__ %}
  {# Iterate over all metadata fields, excluding body #}
  {{ key }}: {{ value }}
{% endfor %}
```

This is more semantically clear than iterating over all fields and manually filtering out `body`.

### 3. Reserved Naming Convention

The `__metadata__` name uses double underscore prefix to signal:
- System-generated field (not from user frontmatter)
- Reserved namespace (future system fields will use `__` prefix)
- Unlikely collision with user-defined fields

## Implementation Design

### Data Flow

```
ParsedDocument.fields()
    ↓
    ├─ body field → context["body"]
    └─ all other fields → context["__metadata__"]
                    └─ also in top-level context (backward compat)
```

### Architectural Considerations

**Key Question**: Should `templating.rs` import `parse.rs` to access `BODY_FIELD` constant?

**Options Considered:**

1. **Direct Import** (Simplest)
   - `templating.rs` imports `crate::parse::BODY_FIELD`
   - Clean, minimal code duplication
   - Creates dependency from templating → parse
   
2. **Duplicate Constant** (Current Independence)
   - Define `BODY_FIELD = "body"` in both modules
   - Keeps modules independent
   - Risk of drift if constant changes
   
3. **Move Constant to Common Module** (Refactor)
   - Create `constants.rs` or `fields.rs` module
   - Both `parse.rs` and `templating.rs` import from there
   - Cleanest separation of concerns
   - Requires module restructuring

**Recommendation**: **Option 1 - Direct Import**

**Rationale:**
- The `BODY_FIELD` constant is semantically owned by the parsing module - it defines the contract for how parsed documents are structured
- `templating.rs` already depends on `parse.rs` conceptually (it processes parsed documents)
- The dependency is one-way and clean: templating consumes parse output
- No circular dependency risk
- Minimal implementation complexity
- The constant is unlikely to change (stable API contract)

### Implementation in `compose()`

The `compose()` method in `TemplatePlate` and `AutoPlate` will be updated to:

1. Create a metadata object containing all fields except `body`
2. Add this object to the context under `__metadata__` key
3. Continue adding all fields (including `body`) to top-level context

**Pseudocode:**
```rust
fn compose(context: HashMap<String, QuillValue>) -> Result<String, TemplateError> {
    // Separate metadata from body
    let metadata_fields: HashMap<String, QuillValue> = context
        .iter()
        .filter(|(key, _)| key.as_str() != BODY_FIELD)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    
    // Convert to MiniJinja values
    let mut minijinja_context = convert_quillvalue_to_minijinja(context)?;
    let metadata_minijinja = convert_quillvalue_to_minijinja(metadata_fields)?;
    
    // Add __metadata__ field as a Value from the metadata HashMap
    // MiniJinja's Value::from_serialize or Value::from_serializable handles HashMap
    minijinja_context.insert(
        "__metadata__".to_string(), 
        minijinja::value::Value::from_serialize(&metadata_minijinja)
    );
    
    // Render template with enhanced context
    // ...
}
```

## Usage Examples

### Iterating Over Metadata

```jinja
{% for key, value in __metadata__ %}
  {{ key }}: {{ value }}
{% endfor %}
```

### Conditional Metadata Rendering

```jinja
{% if __metadata__.author %}
  Author: {{ __metadata__.author }}
{% endif %}
```

### Metadata Inspection

```jinja
{# Count metadata fields #}
Metadata fields: {{ __metadata__ | length }}
```

### Separating Content from Metadata

```jinja
{# Header with all metadata #}
{% for key, value in __metadata__ %}
  #set document({{ key }}: {{ value | String }})
{% endfor %}

{# Body content separately #}
{{ body | Content }}
```

## Edge Cases

### Empty Metadata
- If document has only `body` field: `__metadata__` is an empty object `{}`
- Templates can check: `{% if __metadata__ | length > 0 %}`

### Extended YAML Metadata Standard
- SCOPE-based collections appear in `__metadata__` (not filtered out)
- Only the `body` field is excluded
- Example: `{ title: "...", products: [...] }` → `__metadata__` contains both `title` and `products`

### Reserved Field Name Collision
- User cannot define `__metadata__` in frontmatter (double underscore is reserved)
- Parser validation may be added to reject `__metadata__` in user YAML
- If collision occurs, system-generated `__metadata__` takes precedence

## Future Enhancements

### Additional System Fields

Future system-generated fields could include:
- `__schema__`: Quill schema definition
- `__quill__`: Quill template name and metadata
- `__version__`: Quillmark version information

### Nested Metadata Groups

Could introduce additional groupings:
- `__frontmatter__`: Only top-level global fields
- `__scoped__`: Only SCOPE-based collections
- `__all__`: Everything including body (for completeness)

## Testing Strategy

### Unit Tests

1. Test `__metadata__` contains all non-body fields
2. Test `__metadata__` excludes body field
3. Test backward compatibility (top-level access still works)
4. Test empty metadata scenario
5. Test extended metadata with SCOPE collections
6. Test iteration over `__metadata__` in templates

### Integration Tests

1. Render templates using `__metadata__` with real backends
2. Verify output correctness with various field combinations
3. Test error handling with malformed templates using `__metadata__`

## Migration Guide

**For Template Authors:**

No migration required - this is a purely additive feature. Existing templates work unchanged.

**Optional Enhancement:**

Templates that iterate over fields to build headers/preambles can simplify:

**Before:**
```jinja
{% for key, value in context %}
  {% if key != "body" %}
    #set document({{ key }}: {{ value | String }})
  {% endif %}
{% endfor %}
```

**After:**
```jinja
{% for key, value in __metadata__ %}
  #set document({{ key }}: {{ value | String }})
{% endfor %}
```

## References

- [PARSE.md](PARSE.md) - Document parsing and field structure
- [ARCHITECTURE.md](ARCHITECTURE.md) - Template system architecture
- [Extended YAML Metadata Standard](PARSE.md#extended-yaml-metadata-standard)
