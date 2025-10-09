## Task: Centralize Value Conversions with QuillValue

**Problem**: Conversion logic for TOML/YAML/JSON values is duplicated across Python bindings, wasm bindings, and templating code.

**Solution**: Create a single canonical value type (`QuillValue`) backed by `serde_json::Value` in `quillmark-core` and centralize all conversions.

### Design Principles

**Use `serde_json::Value` as the underlying representation** because:
- Simple and well-understood
- Maps naturally to JS/wasm (no extra conversions needed)
- Easy to convert to/from TOML/YAML at boundaries
- Excellent interop with templating engines, typst filters, and bindings

**Conversion boundary rule**: 
- TOML and YAML should **only** be used for specialized deserialization:
  - `Quill.toml` parsing → immediately convert to `QuillValue`
  - Markdown frontmatter parsing → immediately convert to `QuillValue`
- After deserialization, everything uses `QuillValue` internally
- Never pass `serde_yaml::Value` or `toml::Value` around the codebase

### What to Build

**New file**: `quillmark-core/src/value.rs`
- `pub struct QuillValue(serde_json::Value)` — newtype wrapper
- Conversion helpers:
  - `from_toml()` — convert TOML → `QuillValue`
  - `from_yaml()` — convert YAML → `QuillValue`
  - `to_minijinja()` — convert `QuillValue` → `minijinja::Value` for templating
  - `as_json()` / `into_json()` — expose underlying `serde_json::Value` for bindings
- Handle edge cases: non-string keys, YAML tags, numeric coercion

### What to Change

**Core structs** (all backed by `QuillValue`):
- `Quill.metadata`: `HashMap<String, serde_yaml::Value>` → `HashMap<String, QuillValue>`
- `Quill.field_schemas`: `HashMap<String, serde_yaml::Value>` → `HashMap<String, QuillValue>`
- `ParsedDocument.fields`: `HashMap<String, serde_yaml::Value>` → `HashMap<String, QuillValue>`
- Any `FieldSchema` types using values

**Conversion points** (deserialize at boundaries):
- `Quill::from_path()`: Parse TOML → convert to `QuillValue` immediately
- `ParsedDocument::from_markdown()`: Parse YAML frontmatter → convert to `QuillValue` immediately

**Consumers**:
- `templating.rs`: Replace `yaml_to_minijinja_value()` with `QuillValue::to_minijinja()`
- `quillmark-python/types.rs`: Replace `yaml_value_to_py()` with new helper using `QuillValue::as_json()`
- `quillmark-wasm/quill.rs`: Use `QuillValue::as_json()` + `serde_wasm_bindgen::to_value()` for JsValue conversions

### Migration Order
1. Add `value.rs` with `QuillValue` and conversion helpers + tests
2. Update `ParsedDocument`, `Quill`, and `FieldSchema` structs
3. Update parsing functions to convert TOML/YAML → `QuillValue` at boundaries
4. Update templating to use `QuillValue::to_minijinja()`
5. Update Python and wasm bindings to use `QuillValue::as_json()`
6. Update all tests

**Why newtype**: Centralized conversion policy, future flexibility, zero runtime cost, self-documenting API.