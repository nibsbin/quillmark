# UI Schema Properties Implementation Plan

> **⚠️ DEPRECATED**: The `ui.placeholder` feature implemented in this plan has been removed from the codebase. See [REMOVE_PLACEHOLDER_AND_JSON_SCHEMA_FILE.md](../REMOVE_PLACEHOLDER_AND_JSON_SCHEMA_FILE.md) for details.

**Status**: ~~✅ Completed~~ → Superseded (feature removed)
**Goal**: ~~Add `ui.placeholder` property to Quill field schemas (`ui.tooltip` is already implemented)~~ Feature removed
**Related**: [SCHEMAS.md](../designs/SCHEMAS.md), [QUILL_ANNOTATION_PHASE_2.md](completed/QUILL_ANNOTATION_PHASE_2.md)

---

## Overview

This plan addresses the issue requesting `ui.tooltip` and `ui.placeholder` properties in addition to `ui.group`.

**Findings from codebase exploration:**
- `ui.group` - ✅ **Already implemented** (groups fields into UI sections)
- `ui.tooltip` - ✅ **Already implemented** (short hint text for fields)
- `ui.order` - ✅ **Already implemented** (auto-generated from TOML field position)
- `ui.placeholder` - ❌ **Not implemented** (documented in SCHEMAS.md but missing implementation)

This plan describes the implementation of `ui.placeholder` for Quill field schemas. The property provides placeholder text for UI input components (e.g., "e.g., John Doe" in a text field).

---

## Current State Analysis

### Already Implemented

The following `ui` properties are fully implemented:

| Property | Description | Location |
|----------|-------------|----------|
| `ui.group` | Groups fields into UI sections | `UiSchema.group` |
| `ui.tooltip` | Short hint text for fields | `UiSchema.tooltip` |
| `ui.order` | Field display order (auto-generated) | `UiSchema.order` |

### Not Yet Implemented

| Property | Description | Status |
|----------|-------------|--------|
| `ui.placeholder` | Input placeholder text | Documented in SCHEMAS.md, not implemented |
| `ui.component` | Recommended UI component | Documented in SCHEMAS.md, not implemented |

### Implementation Gaps

1. **`UiSchema` struct** (`crates/core/src/quill.rs:11-19`) lacks `placeholder` field
2. **`FieldSchema::from_quill_value()`** (`crates/core/src/quill.rs:91-126`) does not parse `placeholder`
3. **`build_schema_from_fields()`** (`crates/core/src/schema.rs:51-68`) does not serialize `placeholder` to `x-ui`

---

## Desired State

### TOML Configuration

```toml
[fields.author]
description = "The full name of the document author"
type = "str"

[fields.author.ui]
group = "Author Info"
tooltip = "Your full name"
placeholder = "e.g., John Doe"
```

### Generated JSON Schema

```json
{
  "properties": {
    "author": {
      "type": "string",
      "description": "The full name of the document author",
      "x-ui": {
        "group": "Author Info",
        "tooltip": "Your full name",
        "placeholder": "e.g., John Doe",
        "order": 0
      }
    }
  }
}
```

---

## Implementation Plan

### Step 1: Extend UiSchema Struct

**File**: `crates/core/src/quill.rs`

**Change**: Add `placeholder` field to `UiSchema`

```rust
pub struct UiSchema {
    pub group: Option<String>,
    pub tooltip: Option<String>,
    pub placeholder: Option<String>,  // Add this line
    pub order: Option<i32>,
}
```

### Step 2: Update TOML Parsing

**File**: `crates/core/src/quill.rs`

**Change**: Parse `placeholder` in `FieldSchema::from_quill_value()`

Add to the UI parsing block (around line 94):

```rust
let placeholder = ui_obj
    .get("placeholder")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());
```

Update the known-key validation (around line 105):

```rust
match key.as_str() {
    "group" | "tooltip" | "placeholder" => {}  // Add placeholder
    _ => {
        eprintln!("Warning: Unknown UI property '{}'. Only 'group', 'tooltip', and 'placeholder' are supported.", key);
    }
}
```

Update `UiSchema` construction (around line 115):

```rust
Some(UiSchema {
    group,
    tooltip,
    placeholder,  // Add this line
    order: None,
})
```

### Step 3: Update JSON Schema Generation

**File**: `crates/core/src/schema.rs`

**Change**: Serialize `placeholder` to `x-ui` in `build_schema_from_fields()`

Add after the tooltip handling (around line 63):

```rust
if let Some(ref placeholder) = ui.placeholder {
    ui_obj.insert("placeholder".to_string(), Value::String(placeholder.clone()));
}
```

### Step 4: Update QuillConfig Default UI

**File**: `crates/core/src/quill.rs`

**Change**: Include `placeholder: None` in default `UiSchema` construction (around line 620):

```rust
schema.ui = Some(UiSchema {
    group: None,
    tooltip: None,
    placeholder: None,  // Add this line
    order: Some(order),
});
```

---

## Testing Strategy

### Unit Tests

Add to `crates/core/src/schema.rs`:

```rust
#[test]
fn test_build_schema_with_placeholder() {
    let mut fields = HashMap::new();
    let mut schema = FieldSchema::new(
        "author".to_string(),
        "Document author name".to_string(),
    );
    schema.r#type = Some("str".to_string());
    schema.ui = Some(UiSchema {
        group: Some("Author Info".to_string()),
        tooltip: Some("Your full name".to_string()),
        placeholder: Some("e.g., John Doe".to_string()),
        order: Some(0),
    });
    fields.insert("author".to_string(), schema);

    let json_schema = build_schema_from_fields(&fields)
        .unwrap()
        .as_json()
        .clone();

    let x_ui = &json_schema["properties"]["author"]["x-ui"];
    assert_eq!(x_ui["placeholder"], "e.g., John Doe");
    assert_eq!(x_ui["tooltip"], "Your full name");
    assert_eq!(x_ui["group"], "Author Info");
}
```

### Integration Tests

Add to `crates/core/src/quill.rs`:

```rust
#[test]
fn test_quill_with_placeholder() {
    let toml_content = r#"[Quill]
name = "placeholder-test"
backend = "typst"
description = "Test placeholder"

[fields.name]
description = "Your name"
type = "str"

[fields.name.ui]
placeholder = "e.g., Jane Smith"
"#;

    let config = QuillConfig::from_toml(toml_content).unwrap();
    
    let name_field = &config.fields["name"];
    assert_eq!(
        name_field.ui.as_ref().unwrap().placeholder,
        Some("e.g., Jane Smith".to_string())
    );
}
```

---

## Backward Compatibility

**Breaking Changes**: None

- `placeholder` is optional in `UiSchema`
- Existing Quills without `placeholder` continue to work
- JSON Schema `x-ui.placeholder` is ignored by standard validators
- WASM API surface unchanged (metadata embedded in existing `schema` field)

---

## Files Changed

| File | Change |
|------|--------|
| `crates/core/src/quill.rs` | Add `placeholder` to `UiSchema`, update parsing |
| `crates/core/src/schema.rs` | Serialize `placeholder` to `x-ui` |

---

## Implementation Checklist

- [x] Add `placeholder: Option<String>` to `UiSchema` struct
- [x] Update `FieldSchema::from_quill_value()` to parse `placeholder`
- [x] Update UI property validation to include `placeholder`
- [x] Update `UiSchema` construction to include `placeholder`
- [x] Update `build_schema_from_fields()` to serialize `placeholder`
- [x] Add unit test for schema generation with placeholder
- [x] Add integration test for TOML parsing with placeholder
- [x] Verify WASM bindings expose `x-ui.placeholder` (automatic via existing schema generation)
