# Remove ui.placeholder and json_schema_file from Quill.toml

**Status**: ✅ Completed
**Goal**: Remove the `ui.placeholder` field from Quill field schemas and the `json_schema_file` field from `QuillConfig`. Both features are being removed with extreme prejudice—no backward compatibility.
**Related**: [SCHEMAS.md](../designs/SCHEMAS.md), [UI_SCHEMA_PROPERTIES.md](completed/UI_SCHEMA_PROPERTIES.md)

---

## Overview

This plan describes the complete removal of two features from the Quillmark codebase:

1. **`ui.placeholder`**: A property that provides placeholder text for UI input components (e.g., "e.g., John Doe" in a text field). Currently part of the `UiSchema` struct.

2. **`json_schema_file`**: A property in `QuillConfig` that allows users to specify an external JSON schema file that overrides the `[fields]` section. We want to remove this so users can never configure the JSON schema directly.

Both are **breaking changes** by design. No backward compatibility considerations are needed.

---

## Current State Analysis

### Part A: ui.placeholder Implementation Locations

The `ui.placeholder` feature is implemented in the following locations:

| File | Location | Description |
|------|----------|-------------|
| `crates/core/src/quill.rs` | Line 18 | `placeholder: Option<String>` field in `UiSchema` struct |
| `crates/core/src/quill.rs` | Lines 106-109 | Parsing `placeholder` from TOML in `FieldSchema::from_quill_value()` |
| `crates/core/src/quill.rs` | Line 114 | `placeholder` in UI property validation (allowed keys) |
| `crates/core/src/quill.rs` | Line 117 | Warning message mentioning `placeholder` |
| `crates/core/src/quill.rs` | Line 125 | `UiSchema` construction includes `placeholder` |
| `crates/core/src/quill.rs` | Line 631 | Default `UiSchema` construction with `placeholder: None` |
| `crates/core/src/quill.rs` | Lines 2228-2249 | Test `test_quill_with_placeholder` |
| `crates/core/src/quill.rs` | Lines 2252-2276 | Test `test_quill_with_all_ui_properties` uses `placeholder` |
| `crates/core/src/schema.rs` | Lines 63-68 | Serializing `placeholder` to `x-ui` in `build_schema_from_fields()` |
| `crates/core/src/schema.rs` | Lines 1014-1033 | Test `test_build_schema_with_placeholder` |

### Part B: json_schema_file Implementation Locations

The `json_schema_file` feature is implemented in the following locations:

| File | Location | Description |
|------|----------|-------------|
| `crates/core/src/quill.rs` | Line 483 | `json_schema_file: Option<String>` field in `QuillConfig` struct |
| `crates/core/src/quill.rs` | Lines 558-561 | Parsing `json_schema_file` from TOML in `QuillConfig::from_toml()` |
| `crates/core/src/quill.rs` | Lines 798-826 | Schema loading from file in `Quill::from_config()` with conditional logic |
| `crates/core/src/quill.rs` | Lines 2073-2149 | Test `test_json_schema_file_override` |

### Documentation Locations (Both Features)

| File | Description |
|------|-------------|
| `prose/designs/SCHEMAS.md` | Documents `placeholder` as implemented UI property; documents `json_schema_file` as optional field |
| `prose/plans/completed/UI_SCHEMA_PROPERTIES.md` | Completed plan that added `placeholder` |
| `prose/plans/completed/QUILL_ANNOTATION_PHASE_2.md` | Mentions `placeholder` in examples |
| `docs/guides/creating-quills.md` | Documents `placeholder` in UI configuration example; lists `json_schema_file` as optional field |

---

## Desired State

After implementation:

### ui.placeholder
- The `UiSchema` struct will only contain: `group`, `tooltip`, and `order`
- TOML parsing will not recognize `placeholder` as a UI property
- JSON Schema generation will not include `placeholder` in `x-ui`
- All tests referencing `placeholder` will be removed or updated

### json_schema_file
- The `QuillConfig` struct will not have a `json_schema_file` field
- TOML parsing will not recognize `json_schema_file` in the `[Quill]` section
- Schema loading will always use `build_schema_from_fields()` from the `[fields]` section
- All tests referencing `json_schema_file` will be removed
- Users can never bypass the `[fields]` section by providing an external JSON schema

### Documentation
- All references to both features will be removed from documentation

---

## Implementation Plan

### Part A: Remove ui.placeholder

#### Step A1: Remove placeholder from UiSchema Struct

**File**: `crates/core/src/quill.rs`

**Change**: Remove the `placeholder` field from the `UiSchema` struct.

**Before**:
```rust
pub struct UiSchema {
    pub group: Option<String>,
    pub tooltip: Option<String>,
    pub placeholder: Option<String>,
    pub order: Option<i32>,
}
```

**After**:
```rust
pub struct UiSchema {
    pub group: Option<String>,
    pub tooltip: Option<String>,
    pub order: Option<i32>,
}
```

---

#### Step A2: Remove placeholder Parsing from TOML

**File**: `crates/core/src/quill.rs`

**Change**: Remove the `placeholder` parsing code in `FieldSchema::from_quill_value()`.

Remove these lines (approximately lines 106-109):
```rust
let placeholder = ui_obj
    .get("placeholder")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());
```

---

#### Step A3: Update UI Property Validation

**File**: `crates/core/src/quill.rs`

**Change**: Remove `placeholder` from the allowed UI property keys and update the warning message.

**Before**:
```rust
match key.as_str() {
    "group" | "tooltip" | "placeholder" => {}
    _ => {
        eprintln!("Warning: Unknown UI property '{}'. Only 'group', 'tooltip', and 'placeholder' are supported.", key);
    }
}
```

**After**:
```rust
match key.as_str() {
    "group" | "tooltip" => {}
    _ => {
        eprintln!("Warning: Unknown UI property '{}'. Only 'group' and 'tooltip' are supported.", key);
    }
}
```

**Note**: The `order` field is auto-generated from field position in `Quill.toml`, not specified by users in the `[ui]` section, so it is not listed in the warning message.

---

#### Step A4: Update UiSchema Construction

**File**: `crates/core/src/quill.rs`

**Change**: Remove `placeholder` from `UiSchema` construction in two locations.

**Location 1** (in `FieldSchema::from_quill_value()`, approximately line 122-127):

**Before**:
```rust
Some(UiSchema {
    group,
    tooltip,
    placeholder,
    order: None,
})
```

**After**:
```rust
Some(UiSchema {
    group,
    tooltip,
    order: None,
})
```

**Location 2** (in `QuillConfig::from_toml()`, approximately line 628-633):

**Before**:
```rust
schema.ui = Some(UiSchema {
    group: None,
    tooltip: None,
    placeholder: None,
    order: Some(order),
});
```

**After**:
```rust
schema.ui = Some(UiSchema {
    group: None,
    tooltip: None,
    order: Some(order),
});
```

---

#### Step A5: Remove placeholder Serialization from JSON Schema

**File**: `crates/core/src/schema.rs`

**Change**: Remove the `placeholder` serialization in `build_schema_from_fields()`.

Remove these lines (approximately lines 63-68):
```rust
if let Some(ref placeholder) = ui.placeholder {
    ui_obj.insert(
        "placeholder".to_string(),
        Value::String(placeholder.clone()),
    );
}
```

---

#### Step A6: Remove or Update ui.placeholder Tests

**File**: `crates/core/src/quill.rs`

**Change 1**: Remove the `test_quill_with_placeholder` test entirely (lines 2228-2249).

**Change 2**: Update `test_quill_with_all_ui_properties` to remove `placeholder` references.

**Before**:
```rust
#[test]
fn test_quill_with_all_ui_properties() {
    let toml_content = r#"[Quill]
name = "full-ui-test"
backend = "typst"
description = "Test all UI properties"

[fields.author]
description = "The full name of the document author"
type = "str"

[fields.author.ui]
group = "Author Info"
tooltip = "Your full name"
placeholder = "e.g., John Doe"
"#;

    let config = QuillConfig::from_toml(toml_content).unwrap();

    let author_field = &config.fields["author"];
    let ui = author_field.ui.as_ref().unwrap();
    assert_eq!(ui.group, Some("Author Info".to_string()));
    assert_eq!(ui.tooltip, Some("Your full name".to_string()));
    assert_eq!(ui.placeholder, Some("e.g., John Doe".to_string()));
    assert_eq!(ui.order, Some(0));
}
```

**After**:
```rust
#[test]
fn test_quill_with_all_ui_properties() {
    let toml_content = r#"[Quill]
name = "full-ui-test"
backend = "typst"
description = "Test all UI properties"

[fields.author]
description = "The full name of the document author"
type = "str"

[fields.author.ui]
group = "Author Info"
tooltip = "Your full name"
"#;

    let config = QuillConfig::from_toml(toml_content).unwrap();

    let author_field = &config.fields["author"];
    let ui = author_field.ui.as_ref().unwrap();
    assert_eq!(ui.group, Some("Author Info".to_string()));
    assert_eq!(ui.tooltip, Some("Your full name".to_string()));
    assert_eq!(ui.order, Some(0));
}
```

**File**: `crates/core/src/schema.rs`

**Change**: Remove the `test_build_schema_with_placeholder` test entirely (lines 1014-1033).

---

### Part B: Remove json_schema_file

#### Step B1: Remove json_schema_file from QuillConfig Struct

**File**: `crates/core/src/quill.rs`

**Change**: Remove the `json_schema_file` field from the `QuillConfig` struct (around line 483).

**Before**:
```rust
pub struct QuillConfig {
    // ... other fields ...
    /// JSON schema file
    pub json_schema_file: Option<String>,
    /// Field schemas
    pub fields: HashMap<String, FieldSchema>,
    // ... other fields ...
}
```

**After**:
```rust
pub struct QuillConfig {
    // ... other fields ...
    /// Field schemas
    pub fields: HashMap<String, FieldSchema>,
    // ... other fields ...
}
```

---

#### Step B2: Remove json_schema_file Parsing from TOML

**File**: `crates/core/src/quill.rs`

**Change**: Remove the `json_schema_file` parsing code in `QuillConfig::from_toml()` (around lines 558-561).

Remove these lines:
```rust
let json_schema_file = quill_section
    .get("json_schema_file")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());
```

Also remove the `json_schema_file` field from the `QuillConfig` struct initialization later in the function.

---

#### Step B3: Simplify Schema Loading in Quill::from_config()

**File**: `crates/core/src/quill.rs`

**Change**: Remove the conditional schema loading logic that checks for `json_schema_file` (around lines 798-826). Always use `build_schema_from_fields()`.

**Before**:
```rust
// Load or build JSON schema
let schema = if let Some(ref json_schema_path) = config.json_schema_file {
    // Load schema from file if specified
    let schema_bytes = root.get_file(json_schema_path).ok_or_else(|| {
        format!(
            "json_schema_file '{}' not found in file tree",
            json_schema_path
        )
    })?;

    // Parse and validate JSON syntax
    let schema_json =
        serde_json::from_slice::<serde_json::Value>(schema_bytes).map_err(|e| {
            format!(
                "json_schema_file '{}' is not valid JSON: {}",
                json_schema_path, e
            )
        })?;

    // Warn if fields are also defined
    if !config.fields.is_empty() {
        eprintln!("Warning: [fields] section is overridden by json_schema_file");
    }

    QuillValue::from_json(schema_json)
} else {
    // Build JSON schema from field schemas if no json_schema_file
    build_schema_from_fields(&config.fields)
        .map_err(|e| format!("Failed to build JSON schema from field schemas: {}", e))?
};
```

**After**:
```rust
// Build JSON schema from field schemas
let schema = build_schema_from_fields(&config.fields)
    .map_err(|e| format!("Failed to build JSON schema from field schemas: {}", e))?;
```

---

#### Step B4: Remove json_schema_file Test

**File**: `crates/core/src/quill.rs`

**Change**: Remove the `test_json_schema_file_override` test entirely (lines 2073-2149).

This test verifies that `json_schema_file` overrides the `[fields]` section, which is no longer supported behavior.

---

### Part C: Update Documentation

#### Step C1: Update SCHEMAS.md Design Document

**File**: `prose/designs/SCHEMAS.md`

**Changes**:

1. Remove `json_schema_file` from the Quill configuration list (line 31)
2. Remove `placeholder` from the UI Configuration table (line 48)
3. Update the Implementation Status table to remove the `placeholder` row (line 56)
4. Update the JSON Schema example to remove `placeholder` from `x-ui` (lines 81-98)

---

#### Step C2: Update UI_SCHEMA_PROPERTIES.md Completed Plan

**File**: `prose/plans/completed/UI_SCHEMA_PROPERTIES.md`

**Change**: Add a note at the top indicating this feature has been removed, or archive the file with a clear deprecated status.

---

#### Step C3: Update QUILL_ANNOTATION_PHASE_2.md

**File**: `prose/plans/completed/QUILL_ANNOTATION_PHASE_2.md`

**Change**: Update examples that mention `placeholder` to remove those references.

---

#### Step C4: Update Creating Quills Guide

**File**: `docs/guides/creating-quills.md`

**Changes**:

1. **Remove `json_schema_file`** from the Optional Fields section (line 57). This field should no longer be documented as an option.

**Note**: The guide uses `extra = { placeholder = "John Doe" }` which is unrelated to `UiSchema.placeholder`. The `extra` field is a pass-through for arbitrary key-value pairs, not a structured UI property. No changes needed for this usage.

---

## Files Changed Summary

| File | Change Type |
|------|-------------|
| `crates/core/src/quill.rs` | Remove `json_schema_file` field, parsing, schema loading logic; Remove `placeholder` field, parsing, validation, construction; Remove tests for both features |
| `crates/core/src/schema.rs` | Remove placeholder serialization, tests |
| `prose/designs/SCHEMAS.md` | Remove documentation for both `json_schema_file` and `placeholder` |
| `prose/plans/completed/UI_SCHEMA_PROPERTIES.md` | Mark as superseded/deprecated |
| `prose/plans/completed/QUILL_ANNOTATION_PHASE_2.md` | Update examples to remove placeholder |
| `docs/guides/creating-quills.md` | Remove `json_schema_file` from optional fields |

---

## Implementation Checklist

### Part A: ui.placeholder Code Changes (`crates/core/src/quill.rs`)

- [ ] Remove `placeholder: Option<String>` from `UiSchema` struct (line 18)
- [ ] Remove placeholder parsing lines (lines 106-109)
- [ ] Remove `"placeholder"` from allowed keys match arm (line 114)
- [ ] Update warning message to say "Only 'group' and 'tooltip' are supported" (line 117)
- [ ] Remove `placeholder` from `UiSchema` construction in `from_quill_value()` (line 125)
- [ ] Remove `placeholder: None` from default `UiSchema` construction in `from_toml()` (line 631)
- [ ] Remove `test_quill_with_placeholder` test entirely (lines 2228-2249)
- [ ] Update `test_quill_with_all_ui_properties` test (lines 2252-2276):
  - Remove `placeholder = "e.g., John Doe"` from TOML content
  - Remove assertion for `ui.placeholder`

### Part A: ui.placeholder Code Changes (`crates/core/src/schema.rs`)

- [ ] Remove placeholder serialization block (lines 63-68)
- [ ] Remove `test_build_schema_with_placeholder` test entirely (lines 1014-1033)

### Part B: json_schema_file Code Changes (`crates/core/src/quill.rs`)

- [ ] Remove `json_schema_file: Option<String>` field from `QuillConfig` struct (line 483)
- [ ] Remove `json_schema_file` parsing from `QuillConfig::from_toml()` (lines 558-561)
- [ ] Remove `json_schema_file` from `QuillConfig` struct initialization in `from_toml()`
- [ ] Simplify schema loading in `Quill::from_config()` to always use `build_schema_from_fields()` (lines 798-826)
- [ ] Remove `test_json_schema_file_override` test entirely (lines 2073-2149)

### Part C: Documentation Changes (`prose/designs/SCHEMAS.md`)

- [ ] Remove `json_schema_file -> Option[str]` from Quill configuration list (line 31)
- [ ] Remove `placeholder -> Option[str]` from UI Configuration list (line 48)
- [ ] Remove `placeholder` row from Implementation Status table (line 56)
- [ ] Remove `"placeholder": "e.g. John Doe"` from JSON Schema example (line 93)

### Part C: Documentation Changes (`prose/plans/completed/UI_SCHEMA_PROPERTIES.md`)

- [ ] Add deprecation notice at top of file indicating this feature was removed
- [ ] Update Status field to indicate superseded

### Part C: Documentation Changes (`prose/plans/completed/QUILL_ANNOTATION_PHASE_2.md`)

- [ ] Remove `placeholder = "e.g. John Doe"` from TOML example (line 45)
- [ ] Remove `"placeholder": "e.g. John Doe"` from JSON Schema example (line 69)

### Part C: Documentation Changes (`docs/guides/creating-quills.md`)

- [ ] Remove `json_schema_file` from Optional Fields list (line 57)

---

## Testing Strategy

After implementation:

1. Run `cargo test --all-features` to verify all tests pass
2. Run `cargo build --all-features` to verify compilation
3. Verify no compilation warnings related to unused fields

---

## Breaking Change Notice

Both removals are intentional breaking changes. No migration path is provided.

### ui.placeholder

Any existing `Quill.toml` files using `ui.placeholder` will:

1. Trigger a warning about unknown UI property (due to validation)
2. Have the `placeholder` value silently ignored

**Action**: Users should simply remove `placeholder` from their `Quill.toml` files.

### json_schema_file

Any existing `Quill.toml` files using `json_schema_file` will:

1. Have the `json_schema_file` field silently ignored
2. Schema will be built from the `[fields]` section only

**Action**: Users must define their field schemas using the `[fields]` section in `Quill.toml`. External JSON schema files are no longer supported.
