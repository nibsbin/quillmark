# QuillValue - Centralized Value Type

> **Status**: Implemented  
> **Implementation**: `quillmark-core/src/value.rs`

## Overview

`QuillValue` is a unified value type centralizing TOML/YAML/JSON conversions. Backed by `serde_json::Value`, it provides a single canonical representation for metadata and fields.

## Design Principles

1. **JSON Foundation** - Use `serde_json::Value` for simplicity and broad ecosystem support
2. **Conversion Boundaries** - Convert TOML/YAML to `QuillValue` at system boundaries
3. **Newtype Pattern** - Wrap JSON to add domain-specific methods and control API

## Implementation

```rust
pub struct QuillValue(serde_json::Value);
```

**Conversion methods:** `from_toml()`, `from_yaml()`, `from_json()`, `to_minijinja()`, `as_json()`, `into_json()`

**Delegating methods:** `is_null()`, `as_str()`, `as_bool()`, `as_i64()`, `as_array()`, `as_object()`, `get(key)`
- `get(key)` - Field access with `QuillValue` wrapping

### Deref Implementation

Implements `Deref<Target = serde_json::Value>` for transparent access to JSON methods.

## Usage

Used throughout the system:
- Quill metadata and schemas
- Parsed document fields
- Field default and example values
- FFI boundaries (Python, WASM)