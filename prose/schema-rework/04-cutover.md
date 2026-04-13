# Phase 4: Cutover

Flip all internal callers and language bindings to use the new config-native
APIs. After this phase, nothing in the active code path depends on
`schema.rs`'s JSON Schema generation. Everything in `schema.rs` becomes dead
code, ready for deletion in phase 5.

## Goal

One atomic cutover per call site. The new implementations from phases 1-3
already exist and are tested; this phase just redirects callers. By the end
of this phase:

- `dry_run` and the render pipeline use `QuillConfig::validate()` and
  `QuillConfig::coerce()`
- Defaults and examples surfaces read from `QuillConfig::defaults()` and
  `examples()`
- WASM binding exposes `getQuillSchema(name) -> String` (YAML)
- Python binding exposes `quill.schema -> str` (YAML)
- `getStrippedSchema` is removed
- `QuillInfo.schema` is a YAML `String`

## Why this phase

Phases 1-3 built new, additive APIs alongside the old ones. The parity tests
in phase 2 give us confidence that the new extractors produce equivalent
results. Phase 1's validator tests cover the validation surface. Phase 3
emits the new public contract.

With that foundation, cutover is mechanical — search for old call sites,
replace with new ones, run tests.

## Deliverables

### Part A: Internal call-site cutover

#### Locate callers

Grep the codebase for these symbols:

```
build_schema_from_config
build_schema
project_schema
SchemaProjection
validate_document                  # free function in schema.rs
extract_defaults_from_schema
extract_examples_from_schema
coerce_document
```

Expected call sites (non-exhaustive; verify):

- `crates/core/src/workflow.rs` or wherever `dry_run` lives
- `crates/core/src/` render pipeline entry points
- Binding code that reads defaults/examples from `QuillInfo`

#### Replace validation

Before:

```rust
let schema = schema::build_schema_from_config(&quill.config)?;
schema::validate_document(&schema, &parsed.fields)?;
```

After:

```rust
quill.config.validate(&parsed.fields)
    .map_err(|errs| RenderError::Validation(errs))?;
```

Update `RenderError` if needed to carry `Vec<ValidationError>` instead of
`jsonschema::ValidationError`. If the old error type was public and shaped
specifically for JSON Schema errors, replace it. This is a breaking change
to `RenderError` — acceptable pre-release.

#### Replace coercion

Before:

```rust
let schema = schema::build_schema_from_config(&quill.config)?;
let coerced = schema::coerce_document(&parsed.fields, &schema)?;
```

After:

```rust
let coerced = quill.config.coerce(&parsed.fields)?;
```

#### Replace defaults / examples

Before:

```rust
let schema = schema::build_schema_from_config(&quill.config)?;
let defaults = schema::extract_defaults_from_schema(&schema);
```

After:

```rust
let defaults = quill.config.defaults();
```

### Part B: WASM binding cutover

**File:** `crates/bindings/wasm/src/engine.rs`

Remove `getStrippedSchema()` (lines 187-211). Replace with:

```rust
#[wasm_bindgen(js_name = getQuillSchema)]
pub fn get_quill_schema(&self, name: &str) -> Result<String, JsValue> {
    let quill = self.inner.get_quill(name)
        .ok_or_else(|| JsValue::from_str(&format!("quill `{name}` not found")))?;
    quill.config.public_schema_yaml()
        .map_err(|e| JsValue::from_str(&format!("schema serialization: {e}")))
}
```

**File:** `crates/bindings/wasm/src/types.rs`

Change `QuillInfo.schema` from `serde_json::Value` to `String`:

```rust
#[derive(Serialize)]
pub struct QuillInfo {
    pub name: String,
    pub description: Option<String>,
    pub schema: String,   // YAML string
    pub defaults: serde_json::Value,
    pub examples: serde_json::Value,
    // ... other fields
}
```

Construct it with `quill.config.public_schema_yaml()?` instead of JSON
schema building.

`defaults` and `examples` on `QuillInfo` are constructed from
`QuillConfig::defaults()` and `examples()` directly, serialized to
`serde_json::Value` for JS consumption (since they're structured data, not
a schema document).

### Part C: Python binding cutover

**File:** `crates/bindings/python/src/types.rs` (line ~273)

```rust
#[getter]
fn schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let yaml = self.inner.config.public_schema_yaml()
        .map_err(|e| PyValueError::new_err(format!("schema: {e}")))?;
    Ok(yaml.into_pyobject(py)?.into_any())
}
```

Returns `str` (YAML text). The previous `dict` return type is gone.

Update any `Py*` method that wraps the schema (e.g. `PyQuillInfo`).

### Part D: Test updates

Update tests that called the old APIs:

- Integration tests using `build_schema_from_config` in assertions → switch
  to YAML emission via `public_schema_yaml()`, parse with `serde_yaml`, and
  assert on the parsed structure
- WASM tests calling `getStrippedSchema` → `getQuillSchema`
- Python tests asserting `quill.schema["properties"]["foo"]` → parse the
  returned string as YAML first, then assert

Any test that specifically covered JSON Schema output (e.g. verifying
`$defs`, `oneOf`, `contentMediaType`) should be **deleted** as part of this
phase — they test behavior we're removing. Don't port them.

### Part E: Verification

Run the full test suite:

```
cargo test --workspace
cd crates/bindings/wasm && wasm-pack test --node    # or whichever mode
cd crates/bindings/python && pytest
```

Run a fixture-based end-to-end render for `usaf_memo` to confirm parity
with pre-cutover behavior:

```
cargo run --example render_usaf_memo     # or existing example binary
```

## Non-goals

- **No deletion of `schema.rs` code.** The functions remain but become
  unreachable. Phase 5 deletes them.
- **No `jsonschema` crate dependency removal.** Still present, still
  compiled, but no longer called from the active code path.
- **No documentation updates.** Phase 5.
- **No new features.** Strict 1:1 replacement of call sites.

## Acceptance criteria

- [ ] `dry_run` validates via `QuillConfig::validate()`
- [ ] Render pipeline coerces via `QuillConfig::coerce()`
- [ ] Defaults/examples consumers use `QuillConfig::defaults()` / `examples()`
- [ ] `getStrippedSchema` removed; `getQuillSchema` returns YAML string
- [ ] `QuillInfo.schema` is `String` in WASM and `str` in Python
- [ ] No active call to `build_schema_from_config`, `build_schema`,
      `project_schema`, or `schema::validate_document` remains in the
      workspace (grep returns zero matches outside `schema.rs` itself)
- [ ] `cargo test --workspace` passes
- [ ] WASM and Python binding tests pass
- [ ] End-to-end `usaf_memo` render produces identical output to pre-cutover

## Implementation notes

### `RenderError` shape

The old `RenderError` almost certainly has a variant like
`Validation(jsonschema::ValidationError)` or carries a string representation
of JSON Pointer paths. Replace with:

```rust
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("validation failed: {errors:?}")]
    Validation { errors: Vec<ValidationError> },
    // ... other variants
}
```

Downstream error display should format the vector nicely — one error per
line, field path prominently shown. LLM retry loops depend on readable
errors.

### Symmetric API across bindings

WASM's `QuillInfo` and Python's `Quill` class should have matching getters:

| Field | WASM | Python |
|---|---|---|
| schema | `quillInfo.schema` (string) | `quill.schema` (str) |
| defaults | `quillInfo.defaults` (object) | `quill.defaults` (dict) |
| examples | `quillInfo.examples` (object) | `quill.examples` (dict) |

If they differ today (e.g. only one has `examples`), don't fix the asymmetry
in this phase unless trivial. File a follow-up.

### Parity proof

Before deleting any call site, run it under both old and new implementations
side-by-side (feature-flagged or ad-hoc) and compare outputs. The parity
tests in phase 2 cover extractors; do a manual parity check for validation:

```rust
#[test]
fn validation_parity_usaf_memo() {
    let quill = load_fixture("usaf_memo");
    let doc = load_valid_document();

    let old_schema = schema::build_schema_from_config(&quill.config).unwrap();
    let old_result = schema::validate_document(&old_schema, &doc.fields);
    let new_result = quill.config.validate(&doc.fields);

    // Both should pass for valid doc
    assert!(old_result.is_ok());
    assert!(new_result.is_ok());
}

#[test]
fn validation_parity_invalid_doc() {
    // Both should fail for invalid doc, though error messages differ
    let quill = load_fixture("usaf_memo");
    let doc = load_invalid_document();

    let old_schema = schema::build_schema_from_config(&quill.config).unwrap();
    assert!(schema::validate_document(&old_schema, &doc.fields).is_err());
    assert!(quill.config.validate(&doc.fields).is_err());
}
```

These parity tests can be **deleted** immediately after cutover since the
old schema path is about to be removed.

### Error quality regression check

Manually render a deliberately broken `usaf_memo` (missing required field)
and inspect the error output. It should be at least as informative as the
old `jsonschema` output, ideally more. Sample expected output:

```
validation failed:
  - missing required field `memo_for`
  - field `format` value `weird` not in allowed set ["standard", "informal", "separate_page"]
```

## Estimated scope

~200-300 LOC of call-site changes, ~100-200 LOC of test updates, plus
deletions of old JSON-Schema-shaped tests. One sweep across the workspace
touching ~6-10 files.
