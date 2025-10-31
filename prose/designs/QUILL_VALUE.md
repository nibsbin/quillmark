# QuillValue - Centralized Value Type

> **Status**: Implemented  
> **Implementation**: `quillmark-core/src/value.rs`

## Overview

`QuillValue` is a unified value type that centralizes all TOML/YAML/JSON conversions across the Quillmark system. It serves as the canonical representation for metadata, fields, and other dynamic values.

---

## Problem Statement

**Problem**: Conversion logic for TOML/YAML/JSON values was duplicated across:
- Python bindings
- WASM bindings  
- Templating code
- Parsing logic

This led to inconsistent handling, maintenance burden, and potential bugs.

**Solution**: Single canonical value type (`QuillValue`) backed by `serde_json::Value` in `quillmark-core`.

---

## Design Principles

### 1. JSON as Foundation

**Use `serde_json::Value` as underlying representation** because:
- Simple and well-understood
- Maps naturally to JS/WASM (no extra conversions)
- Easy to convert to/from TOML/YAML at boundaries
- Excellent interop with templating engines and bindings
- Broad ecosystem support

### 2. Conversion Boundaries

**Conversion boundary rule**: 
- TOML and YAML only for specialized deserialization at system boundaries
- Convert to `QuillValue` immediately after parsing
- Never pass `serde_yaml::Value` or `toml::Value` around the codebase
- Use `QuillValue` consistently throughout the system

### 3. Newtype Pattern

Wrap `serde_json::Value` in a newtype to:
- Add domain-specific methods
- Control the API surface
- Enable future optimization or backend changes
- Provide better type safety

---

## Implementation

### Core Type

**File**: `quillmark-core/src/value.rs`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuillValue(serde_json::Value);
```

### Conversion Methods

- `from_toml(toml::Value)` - Convert from TOML
- `from_yaml(serde_yaml::Value)` - Convert from YAML  
- `from_json(serde_json::Value)` - Wrap JSON directly
- `to_minijinja()` - Convert for template rendering
- `as_json()` - Get reference to underlying JSON
- `into_json()` - Consume and extract JSON

### Delegating Methods

Convenience methods delegate to underlying `serde_json::Value`:
- `is_null()`, `as_str()`, `as_bool()`, `as_i64()`, `as_u64()`, `as_f64()`
- `as_array()`, `as_sequence()` (YAML alias)
- `as_object()`, `as_mapping()` (YAML alias)
- `get(key)` - Field access with `QuillValue` wrapping

### Deref Implementation

Implements `Deref<Target = serde_json::Value>` for transparent access to JSON methods.

---

## Usage Across System

### Quill Metadata

```rust
pub struct Quill {
    pub metadata: HashMap<String, QuillValue>,
    pub schema: QuillValue,
    pub defaults: HashMap<String, QuillValue>,
    // ...
}
```

### Parsed Documents

```rust
pub struct ParsedDocument {
    fields: HashMap<String, QuillValue>,  // Includes frontmatter + body
}
```

### Field Schemas

```rust
pub struct FieldSchema {
    pub default: Option<QuillValue>,
    pub example: Option<QuillValue>,
    // ...
}
```

### Bindings

Python and WASM bindings use `QuillValue::as_json()` for serialization across language boundaries.

---

## Edge Cases Handled

1. **Non-string keys** - Converted to strings during TOML/YAML conversion
2. **YAML tags** - Stripped and underlying value used
3. **Numeric coercion** - Preserves integer/float distinction where possible
4. **Null values** - Properly represented and detected
5. **Nested structures** - Recursive conversion for arrays and objects

---

## Benefits

1. **Single source of truth** - No conversion ambiguity
2. **Consistent behavior** - Same semantics across all subsystems
3. **Simplified maintenance** - Change conversion logic in one place
4. **Better testing** - Comprehensive tests in value.rs
5. **Type safety** - Newtype prevents mixing raw JSON values
6. **API clarity** - Intent-revealing method names

---

## Cross-References

- **Parsing**: See [PARSE.md](PARSE.md) for YAML frontmatter handling
- **Templating**: See [ARCHITECTURE.md](ARCHITECTURE.md#template-system-design) for MiniJinja integration
- **Schemas**: See [SCHEMAS.md](SCHEMAS.md) for field schema usage