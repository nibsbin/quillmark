# Phase 1: Native Validator

Build a `QuillConfig`-native validator that walks the config and validates a
`ParsedDocument` without going through JSON Schema. Ship it alongside the
existing JSON Schema validator. Do not delete anything yet.

## Goal

Replace the `jsonschema` crate's generic-validator approach with a purpose-built
walker over `FieldSchema` and `CardSchema`. Produce error messages with real
field paths (e.g. `cards.indorsement[1].signature_block[0]`) instead of JSON
Pointer strings.

## Why this first

Validation is the largest consumer of the JSON Schema intermediate. Once a
native validator exists and passes the same test scenarios, every other piece
(defaults/examples/coercion extraction, public contract emission, dead-code
deletion) becomes straightforward. Starting here also catches any semantic
mismatches between our DSL and JSON Schema early.

## Deliverables

### New file: `crates/core/src/quill/validation.rs`

Public API:

```rust
use std::collections::HashMap;
use crate::quill::types::{QuillConfig, FieldSchema, FieldType, CardSchema};
use crate::quill_value::QuillValue;

/// Validation error with a structured field path.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("missing required field `{path}`")]
    MissingRequired { path: String },

    #[error("field `{path}` has type `{actual}`, expected `{expected}`")]
    TypeMismatch { path: String, expected: String, actual: String },

    #[error("field `{path}` value `{value}` not in allowed set {allowed:?}")]
    EnumViolation { path: String, value: String, allowed: Vec<String> },

    #[error("field `{path}` does not match expected format `{format}`")]
    FormatViolation { path: String, format: String },

    #[error("unknown card type `{card}` at `{path}`")]
    UnknownCard { path: String, card: String },

    #[error("card at `{path}` missing `CARD` discriminator")]
    MissingCardDiscriminator { path: String },
}

/// Validate a parsed document against the full config.
///
/// Validates main fields, all card instances, and enforces required fields.
/// Collects all errors rather than short-circuiting on the first.
pub fn validate_document(
    config: &QuillConfig,
    fields: &HashMap<String, QuillValue>,
) -> Result<(), Vec<ValidationError>>;

/// Validate a single value against a field schema at the given path.
/// Used internally; exposed for testing.
pub(crate) fn validate_field(
    field: &FieldSchema,
    value: &QuillValue,
    path: &str,
) -> Vec<ValidationError>;
```

### Type-checking rules

| `FieldType` | Accepts | Notes |
|---|---|---|
| `String` | `QuillValue::String` | |
| `Number` | `QuillValue::Number` | |
| `Boolean` | `QuillValue::Bool` | |
| `Array` | `QuillValue::Array` | Recurse into each element using `items` |
| `Object` | `QuillValue::Object` | Recurse using `properties` |
| `Date` | `QuillValue::String` matching `YYYY-MM-DD` | Parse to confirm |
| `Datetime` | `QuillValue::String` matching ISO 8601 | Parse to confirm |
| `Markdown` | `QuillValue::String` | No format constraint |

Use `chrono` (already a dependency if `date_rework.md` is adopted; confirm
during implementation and pick the crate already in the tree) for date and
datetime parsing.

### Validation algorithm

1. **Main fields:**
   - For each `field_name` in `config.main().fields`:
     - If `required` and not present → `MissingRequired`
     - If present → call `validate_field` at path `field_name`
   - Ignore extra fields not in schema (permissive by design; parser may
     surface unknown fields separately)

2. **Cards:**
   - Expect a `CARDS` key in `fields` containing a `QuillValue::Array`
   - For each card instance (with index `i`):
     - Read `CARD` discriminator → look up `CardSchema` in `config.cards()`
     - Missing `CARD` → `MissingCardDiscriminator { path: "cards[i]" }`
     - Unknown card name → `UnknownCard { path: "cards[i]", card }`
     - Validate each field in the card using path `cards.<name>[i].<field>`

3. **Recursion:**
   - Array `items` with `properties` → treat as object row; recurse
   - Object `properties` → recurse into keys
   - Nested required fields enforced at recursion depth

### Error accumulation

Collect all errors into `Vec<ValidationError>` and return `Err` only if
non-empty. Do **not** short-circuit on the first error — authors and LLMs
benefit from seeing all issues at once.

### Tests

New file: `crates/core/src/quill/validation_tests.rs` (or inline `#[cfg(test)]`
module in `validation.rs`).

Port each of these scenarios from `crates/core/src/schema.rs` tests, translating
JSON Schema assertions into `ValidationError` variant checks:

- Simple string field — pass and fail
- Required field missing
- Required field present with wrong type
- Enum — valid value, invalid value
- Date — valid `YYYY-MM-DD`, invalid format
- Datetime — valid ISO 8601, invalid format
- Markdown — accepts any string
- Array of strings — valid, invalid element type
- Array of objects (typed rows) — valid, missing required inner field
- Object with nested properties — valid, missing required nested field
- Multiple errors accumulate — two missing required fields both reported
- Card with valid discriminator
- Card with unknown discriminator
- Card missing discriminator
- Multiple card instances of same type
- Multiple card types mixed

Aim for ~20-25 test functions. Reference the existing schema.rs tests
(`test_validate_document_success`, `test_validate_document_failure`, etc.)
for input scenarios.

### Integration stub

Do **not** wire the new validator into `dry_run` or the render pipeline in
this phase. It lives alongside the old one. Phase 4 performs the cutover.

## Non-goals

- No changes to `FieldSchema` or `CardSchema` type definitions
- No changes to `QuillConfig` struct shape
- No changes to the `ParsedDocument` structure
- No bindings changes
- No removal of `jsonschema` crate usage
- No changes to `schema.rs`

## Acceptance criteria

- [ ] `crates/core/src/quill/validation.rs` compiles
- [ ] `ValidationError` enum has all variants listed above
- [ ] `validate_document` collects all errors, does not short-circuit
- [ ] All 20+ test scenarios pass
- [ ] `cargo test -p quillmark-core` passes (existing tests untouched)
- [ ] No new public exports surface outside `crates/core` yet — keep the
      module `pub(crate)` at the `lib.rs` level, or behind a `#[doc(hidden)]`
      re-export. Phase 4 promotes it.
- [ ] `jsonschema` crate still present and still used by `schema.rs`

## Implementation notes

### Reading `QuillConfig`

The current `schema.rs` shows how fields are accessed. Use:

- `config.main().fields` → `&HashMap<String, FieldSchema>`
- `config.cards()` → `&HashMap<String, CardSchema>`
- `FieldSchema.r#type`, `FieldSchema.required`, `FieldSchema.enum_values`,
  `FieldSchema.properties`, `FieldSchema.items`

### `QuillValue` type checking

`QuillValue` is the tree type for parsed document values. Check which variants
map to which `FieldType` — the existing coercion code in `schema.rs`
(`coerce_document` around line 908) is the reference for edge cases, but the
validator only needs to check types, not coerce.

### Path construction

Build paths with a small helper:

```rust
fn child_path(parent: &str, child: &str) -> String {
    if parent.is_empty() { child.to_string() }
    else { format!("{parent}.{child}") }
}
fn index_path(parent: &str, i: usize) -> String {
    format!("{parent}[{i}]")
}
```

### What to keep flexible

Don't over-commit to the enum variant shape in `ValidationError`. If during
implementation you discover a cleaner encoding (e.g. a single `Error { path,
kind: ErrorKind }` struct), switch. The list above is a starting point.

### What's load-bearing

The path format (`field.nested[0].deeper`) is load-bearing — downstream
consumers (LLM retry loop in `docs/integration/validation.md`) display these
to users. Keep it human-readable.

## Estimated scope

~500-700 LOC new, ~400 LOC tests. Single module, single file touched outside
tests. Comfortable one-sitting phase.
