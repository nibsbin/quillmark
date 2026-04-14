# Phase 2: Config-Native Extractors

Move defaults, examples, and coercion from JSON Schema-driven functions to
methods on `QuillConfig` that read directly from the config tree. Ship
alongside existing schema-based versions.

## Goal

Eliminate the "schema as intermediate" pattern for the three extraction
functions currently living in `schema.rs`:

- `extract_defaults_from_schema(&QuillValue) -> HashMap<...>` (line 436)
- `extract_examples_from_schema(&QuillValue) -> HashMap<...>` (line 472)
- `coerce_document(&fields, &schema) -> Result<HashMap, RenderError>` (line 908)

After this phase, all three will be available as methods on `QuillConfig`
reading from `FieldSchema` directly.

## Why this phase

Once the native validator (phase 1) exists, defaults/examples/coerce are the
only remaining internal consumers of `build_schema_from_config`. Porting them
to config-native methods isolates all remaining JSON Schema dependency to
the public emission path (phase 3) and dead-consumer cleanup (phase 5).

This phase is additive. Old functions continue to work; new methods exist
beside them.

## Deliverables

### New file: `crates/core/src/quill/extract.rs`

Or add as `impl` block methods on `QuillConfig` in existing module — pick
the location that best matches the codebase's convention (check how `main()`
and `cards()` accessors are organized). For this plan assume a new file.

```rust
use std::collections::HashMap;
use crate::quill::types::{QuillConfig, FieldSchema, CardSchema, FieldType};
use crate::quill_value::QuillValue;

impl QuillConfig {
    /// Extract default values from the main field schemas.
    ///
    /// Returns a map from field name to default `QuillValue`. Fields without
    /// an explicit `default` are omitted. Does not include card defaults
    /// (cards are instance-based; defaults apply per-instance).
    pub fn defaults(&self) -> HashMap<String, QuillValue>;

    /// Extract example values from the main field schemas.
    ///
    /// Returns a map from field name to the list of examples declared on
    /// that field's `examples:` array. Fields without examples are omitted.
    pub fn examples(&self) -> HashMap<String, Vec<QuillValue>>;

    /// Extract defaults for a specific card type by name.
    ///
    /// Returns defaults for the fields defined on that card's schema.
    pub fn card_defaults(&self, card_name: &str) -> Option<HashMap<String, QuillValue>>;

    /// Extract examples for a specific card type by name.
    pub fn card_examples(&self, card_name: &str) -> Option<HashMap<String, Vec<QuillValue>>>;

    /// Coerce a flat field map's values to the types declared in the main
    /// schema. Used after markdown parsing to normalize values before
    /// validation.
    ///
    /// Returns an error if coercion fails for any field (e.g. a string that
    /// can't be parsed as a date).
    pub fn coerce(
        &self,
        fields: &HashMap<String, QuillValue>,
    ) -> Result<HashMap<String, QuillValue>, CoercionError>;
}
```

### Coercion rules

Port the existing rules from `schema::coerce_document` (line 908 of
`crates/core/src/schema.rs`). The only functional change is the source of
type information: read `FieldSchema.r#type` directly instead of walking
`Value::Object { "type": ... }` in a JSON Schema document.

Expected behaviors (verify against existing tests):

- `String` → identity
- `Number` → parse if given a string that looks like a number; identity if
  already a number
- `Boolean` → parse `"true"` / `"false"` strings; identity if already bool
- `Date` → parse from `"YYYY-MM-DD"` or already-normalized value
- `Datetime` → parse from ISO 8601
- `Markdown` → identity (treated as string)
- `Array` → coerce each element using `items` schema
- `Object` → coerce each property using `properties` schema

### `CoercionError`

New error type, similar structure to `ValidationError`:

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum CoercionError {
    #[error("cannot coerce `{value}` to type `{target}` at `{path}`: {reason}")]
    Uncoercible { path: String, value: String, target: String, reason: String },
}
```

Alternatively, reuse/extend `ValidationError` if a `Coercion` variant feels
cleaner there. Pick whichever reduces the number of error types flowing
through `dry_run`.

### Tests

New test module: `crates/core/src/quill/extract_tests.rs` (or inline).

Port the following test scenarios from `schema.rs`:

- `test_extract_defaults_from_schema` → `test_config_defaults_simple`
- Multi-field defaults with mix of defaulted and non-defaulted fields
- Nested object with nested defaults (flatten or keep structured — match
  existing behavior)
- `test_extract_examples_from_schema` → `test_config_examples_simple`
- Fields with multiple examples
- Cards: defaults and examples per card type
- Coercion: string → number, string → bool, string → date, string → datetime
- Coercion: identity for already-correct types
- Coercion: array and object element-wise coercion
- Coercion: error on unparseable date
- Coercion: error on unparseable number

Aim for ~15-20 test functions.

### Parity check

Add a **parity test module**: `crates/core/tests/extract_parity.rs`.

For each fixture quill (`usaf_memo` and any others), assert:

```rust
#[test]
fn defaults_parity_usaf_memo() {
    let quill = load_fixture("usaf_memo");
    let schema = schema::build_schema_from_config(&quill.config).unwrap();

    let old = schema::extract_defaults_from_schema(&schema);
    let new = quill.config.defaults();

    assert_eq!(old, new);
}
```

Similarly for examples and coerce. This guards against regression during the
port and gives phase 4 confidence when cutting over.

## Non-goals

- No removal of `extract_defaults_from_schema`, `extract_examples_from_schema`,
  or `coerce_document` from `schema.rs`. They stay until phase 5.
- No changes to call sites. Old functions remain in use.
- No bindings changes.

## Acceptance criteria

- [ ] `crates/core/src/quill/extract.rs` compiles
- [ ] `QuillConfig::defaults()`, `examples()`, `card_defaults()`,
      `card_examples()`, `coerce()` methods implemented
- [ ] All port tests pass (~15-20 functions)
- [ ] Parity tests pass for all fixture quills
- [ ] `cargo test -p quillmark-core` passes (existing tests untouched)
- [ ] `schema.rs` is unmodified

## Implementation notes

### Defaults and structured values

The existing `extract_defaults_from_schema` walks JSON Schema looking for
`"default"` keys. On `QuillConfig` directly, this becomes:

```rust
for (name, field) in &self.main().fields {
    if let Some(default) = &field.default {
        out.insert(name.clone(), default.clone());
    }
}
```

Substantially simpler. Nested object defaults may need recursion if the
existing behavior supports them — check `schema.rs` tests for the expected
shape.

### Card defaults

Cards are polymorphic; a single "defaults map" doesn't apply. The old
schema-based extractor may have emitted defaults under `$defs.<card>_card` —
check how existing consumers use this. The new API splits defaults per card
type (`card_defaults(name)`), which is cleaner and avoids the confusion of
flattening card defaults with main defaults.

If the old API conflated these, add `all_defaults()` as a convenience if
needed by any existing caller.

### What calls coerce?

Grep for `coerce_document` callers. Typically the render pipeline calls it
after parsing markdown. In phase 4, those callers switch to
`config.coerce(fields)`.

### `QuillValue` serialization

`QuillValue` already has serde support. The default/example values on
`FieldSchema` should already be `QuillValue` — confirm during implementation.
If they're raw `serde_yaml::Value` or `serde_json::Value`, that's a separate
cleanup (probably out of scope for this phase; file a follow-up).

## Estimated scope

~400 LOC new, ~300 LOC tests including parity tests. Two new files at most.
