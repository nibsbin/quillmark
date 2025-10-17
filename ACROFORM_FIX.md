# Acroform Backend Fix: Blank Fields Issue

## Problem

When using the acroform backend with markdown that has missing or null data fields (especially common in WASM usage), all PDF form fields were being filled with empty/blank strings instead of preserving the original template text.

### Example

With minimal markdown:
```markdown
---
QUILL: usaf_form_8
---
```

**Before Fix**: All 60+ form fields were filled with empty strings or minimal placeholder text like `", "`, `"/"`, etc.

**After Fix**: Only fields with actual non-empty data are updated. Fields without data keep their original template text (e.g., `{{examinee.first}}`).

## Root Cause

The acroform backend uses PDF field values as MiniJinja templates (e.g., `{{examinee.first}}`). The issue occurred in this flow:

1. User provides markdown with missing data fields
2. MiniJinja renders templates with `UndefinedBehavior::Chainable`, which renders missing fields as empty strings
3. Backend's `should_update` logic checked if `rendered_value != source`
4. Since `""` != `"{{examinee.first}}"`, the field was updated to `""`
5. Result: ALL fields got filled with blank strings

## Solution

Modified the `should_update` condition in `quillmark-acroform/src/lib.rs` (lines 117-123):

```rust
let rendered_value_is_non_empty = !rendered_value.trim().is_empty();
let should_update = using_tooltip_template
    || (rendered_value != source && rendered_value_is_non_empty);
```

Fields are now only updated when:
1. **Using a tooltip template** (explicit override - always applied), OR
2. **The rendered value differs from source AND is non-empty**

This preserves the original template text in fields when data is missing, while still properly filling fields that have data.

## Impact

### No Breaking Changes
- All existing tests pass
- Full usaf_form_8 example with complete data works identically
- Tooltip templates (explicit overrides) still work as before

### Behavioral Changes
- **Empty data**: Fields with templates that render to empty strings are no longer updated (they keep the template text)
- **Missing data**: Fields referencing missing context data keep their template text instead of being blanked
- **Partial data**: Only fields with actual data are filled, others remain as templates

### WASM Usage
This fix is particularly important for WASM users who might:
- Pass minimal markdown without all required fields
- Use default/example templates to understand required fields
- Want to see which fields need data (templates remain visible)

## Testing

Added comprehensive test suite in `quillmark-wasm/tests/wasm_bindings.rs`:

1. **test_usaf_form_8_glue_output**: Verifies glue JSON contains correct data from markdown
2. **test_usaf_form_8_from_json_matches_from_path**: Validates JSON-loaded quills match filesystem-loaded quills
3. **test_usaf_form_8_render_via_json_workflow**: Tests full render workflow via JSON (simulating WASM)
4. **test_usaf_form_8_with_minimal_markdown**: Verifies fields are NOT filled when data is missing

All tests pass, including existing acroform integration tests.

## Migration Guide

No migration needed - this is a bug fix that improves the user experience. The behavior change is:

**Before**: Missing data → all fields filled with empty strings  
**After**: Missing data → fields keep template text, showing what data is needed

If you want the old behavior (filling with empty strings), use tooltip templates:
```
Field tooltip: description__{{myfield | default("")}}
```

The tooltip template syntax explicitly overrides the field and will always apply, even if rendering to an empty string.
