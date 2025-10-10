# Changes for Python API Consumers

This document summarizes the API changes for Python consumers since version 0.1.14 (commit 6a99057).

## Version 0.1.15+

### 🔄 Breaking Changes

#### FieldSchema Structure
Field schemas returned by `quill.field_schemas` now have a standardized structure. Each field schema is a dictionary with the following keys:

- `description` (str, required): Description of the field
- `required` (bool): Whether the field is required (default: `false`)
- `type` (str, optional): Type hint (e.g., "string", "number", "boolean", "object", "array"). May be `None` if not specified in the schema.
- `example` (optional): Example value for the field
- `default` (optional): Default value for the field

**Before (v0.1.14)**:
```python
# Field schemas were unstructured YAML values
schema = quill.field_schemas["title"]
# schema could be any YAML structure
```

**After (v0.1.15+)**:
```python
# Field schemas have a defined structure
schema = quill.field_schemas["title"]
# schema = {
#     "description": "Document title",
#     "required": True,
#     "type": "string",
#     "example": "My Document"
# }
```

#### Field Type Property Renamed
Within field schemas, the `field_type` property has been renamed to `type`:

**Before**:
```python
field_type = schema.get("field_type")  # ❌ No longer works
```

**After**:
```python
field_type = schema.get("type")  # ✅ Use this instead
```

#### Quill.toml Format Changes
If you're creating Quill templates, note that `description` is now a required field in the `[Quill]` section of `Quill.toml`:

```toml
[Quill]
name = "my_template"
backend = "typst"
description = "A template description"  # Now required
glue = "template.typ"
```

### 🔧 Internal Changes

These changes should be transparent to Python users but are worth noting:

#### Value Representation Changed from YAML to JSON
Internally, all metadata and field values now use `QuillValue`, which is backed by JSON instead of YAML. This change:

- **Does not affect the Python API surface** - values are still returned as Python dicts, lists, strings, etc.
- Improves consistency across language bindings
- May affect edge cases with YAML-specific types (e.g., YAML tags are no longer supported)

**Migration**: No code changes required. The conversion to Python objects remains the same.

#### Unsigned Integer Support
The conversion from internal values to Python now properly handles unsigned integers (u64):

```python
# Large unsigned integers are now correctly converted
value = parsed.get_field("large_number")  # Works correctly for u64 values
```

### 📋 Summary

**What you need to do**:
1. If you access field schemas, update any code that expects `field_type` to use `type` instead
2. If you access field schemas, be aware they now have a standardized structure
3. If you create Quill templates, add a `description` field to your `Quill.toml` files

**What's backwards compatible**:
- All other Quill properties (`name`, `glue_template`, `example`, `metadata`)
- ParsedDocument API (`from_markdown()`, `body()`, `get_field()`, `fields()`, `quill_tag()`)
- Workflow API (`render()`, `render_source()`, etc.)
- Quillmark engine API (`register_quill()`, `workflow_from_quill_name()`)
- Error handling and diagnostic structures

### Migration Checklist

- [ ] Review any code that accesses `quill.field_schemas`
- [ ] Replace references to `field_type` with `type` in field schema access
- [ ] Update Quill.toml files to include required `description` field
- [ ] Test with edge cases involving large numbers or complex data structures
