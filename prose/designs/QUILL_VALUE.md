# QuillValue

Status: **Implemented** (2026-03-22)  
Source: `crates/core/src/value.rs`

`QuillValue` is the single value type used across parsing, schemas, metadata, and FFI. It wraps `serde_json::Value` with a small convenience surface.

## API
- Constructors: `from_json(serde_json::Value)`, `from_yaml_str(&str)` (via `serde_saphyr`), `into_json()`, `as_json()`.
- Accessors: `as_str`, `as_bool`, `as_i64`, `as_u64`, `as_f64`, `as_array`/`as_sequence`, `as_object`, `get(key)`, `is_null`.
- `Deref<Target = serde_json::Value>` for direct JSON use.

## Usage
- Parsed frontmatter fields (`ParsedDocument.fields`).
- Quill metadata, typst config, schemas, defaults/examples.
- Serialized diagnostics for Python/WASM bindings.
