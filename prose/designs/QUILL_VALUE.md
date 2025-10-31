## QuillValue - Centralized Value Type

**Problem**: Conversion logic for TOML/YAML/JSON values is duplicated across Python bindings, WASM bindings, and templating code.

**Solution**: Single canonical value type (`QuillValue`) backed by `serde_json::Value` in `quillmark-core`.

### Design Principles

**Use `serde_json::Value` as underlying representation** because:
- Simple and well-understood
- Maps naturally to JS/WASM (no extra conversions)
- Easy to convert to/from TOML/YAML at boundaries
- Excellent interop with templating engines and bindings

**Conversion boundary rule**: 
- TOML and YAML only for specialized deserialization
- Convert to `QuillValue` immediately after parsing
- Never pass `serde_yaml::Value` or `toml::Value` around the codebase

### Implementation

**File**: `quillmark-core/src/value.rs`
- `pub struct QuillValue(serde_json::Value)` - newtype wrapper
- Conversion helpers: `from_toml()`, `from_yaml()`, `to_minijinja()`, `as_json()`
- Handle edge cases: non-string keys, YAML tags, numeric coercion

**Migration points**:
- `Quill.metadata`: Change to `HashMap<String, QuillValue>`
- `ParsedDocument.fields`: Change to `HashMap<String, QuillValue>`
- `FieldSchema`: Use `QuillValue` for example and default fields
- Bindings: Use `QuillValue::as_json()` for Python/WASM conversions