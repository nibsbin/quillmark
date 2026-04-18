# Proposal: Add `float` Field Type Distinct from `number`

## Problem

The single `FieldType::Number` variant accepts both integers and floats without distinction. Validation (`validation.rs:147`) only checks `is_number()`; coercion (`config.rs:299`) opportunistically prefers `i64` but silently falls back to `f64`. Schema output emits `"type": "number"` regardless.

Consumers (form UIs, backend templates, validators) cannot tell from the schema whether a field expects a whole count (e.g., `page_count: 3`) or a decimal quantity (e.g., `tax_rate: 0.075`). Authors have no way to communicate that intent in `Quill.yaml`.

Typst plates, in particular, distinguish `int` and `float` at the type system level. Passing a coerced `f64` where an `int` is expected causes runtime errors the schema could have prevented.

## Decisions

### 1. Add `FieldType::Float` as a distinct variant

Not an alias of `Number`. Two separate types with different semantics:

| Quill.yaml  | Accepts                          | Coerces to        | JSON Schema emit                  |
|-------------|----------------------------------|-------------------|-----------------------------------|
| `number`    | integers only                    | `i64`             | `"type": "integer"`               |
| `float`     | integers and decimals            | `f64`             | `"type": "number"`                |

`number` tightens to integer-only. `float` is the new "any numeric including decimal" type. A `float` field accepts integer input and promotes it to `f64`; a `number` field rejects decimal input with a coercion error.

### 2. Tightened coercion for `number`

`FieldType::Number` coercion (`config.rs:299`):
- Integer JSON value → pass through
- Decimal JSON value → **reject** with `Uncoercible { target: "number" }` (today: accepted)
- String `"5"` → `i64(5)`
- String `"5.0"` → **reject** (today: accepted as `f64`)
- Bool → `0` / `1` (unchanged)

### 3. New coercion for `float`

`FieldType::Float`:
- Any JSON numeric → `f64`
- String parseable as `f64` → `f64`
- Bool → `0.0` / `1.0`

### 4. Schema emit maps to JSON Schema conventions

`number` emits `"type": "integer"` (JSON Schema's integer type).
`float` emits `"type": "number"` (JSON Schema's any-numeric type).

This aligns the public schema with JSON Schema / OpenAPI conventions without exposing a `float` keyword that isn't in those standards — the distinction is visible to consumers via `integer` vs. `number`.

### 5. No alias

`"float"` in `Quill.yaml` is a first-class type name. No `float` → `number` alias. `FieldType::from_str` gets a new arm, not an alias entry.

## Scope

### In scope
- Add `FieldType::Float` variant in `types.rs`
- `FieldType::from_str`: add `"float" => Float` arm
- `FieldType::as_str`: add `Float => "float"` arm
- Validation in `validation.rs:147`: `Number` requires `is_i64() || is_u64()`; `Float` requires `is_number()`
- Coercion in `config.rs:299`: split `Number` and `Float` branches per rules above
- Schema emit in `schema_yaml.rs` / `schema.rs`: `Number` → `integer`, `Float` → `number`
- Tests: add float coercion tests, tighten number coercion tests, schema emit tests
- Update docs: `creating-quills.md`, `quill-yaml-reference.md`, `SCHEMAS.md`
- Audit and migrate existing quills/fixtures that used `number` for decimal values

### Out of scope
- `integer` as an alias for `number` — deferred; consumers get the signal via schema emit
- Numeric bounds (`minimum`, `maximum`, `multipleOf`) — separate proposal
- Unsigned integer type — deferred until proven need

## Migration

Breaking change for any quill that used `number` with decimal values. Migration is mechanical:

1. Scan fixtures and example data for decimal values under `number` fields.
2. For each, either change the field type to `float` or round the example to an integer, depending on author intent.
3. Coercion errors surface at quill-load time with field path, making migration failures loud.

## Files affected

| File                                      | Change                                                        |
|-------------------------------------------|---------------------------------------------------------------|
| `crates/core/src/quill/types.rs`          | Add `Float` variant, update `from_str`/`as_str`               |
| `crates/core/src/quill/validation.rs`     | Split `Number`/`Float` validation, update type-name mapping   |
| `crates/core/src/quill/config.rs`         | Split `Number`/`Float` coercion branches                      |
| `crates/core/src/quill/schema_yaml.rs`    | `Number` → `integer`, `Float` → `number` in emitted schema    |
| `crates/core/src/schema.rs`               | Same mapping in JSON Schema builder; coercion recursion       |
| `crates/core/src/quill/tests.rs`          | New float tests; update number tests for tightened semantics  |
| `docs/guides/creating-quills.md`          | Document `float`; clarify `number` = integer                  |
| `docs/guides/quill-yaml-reference.md`     | Add `float` row to type table                                 |
| `prose/designs/SCHEMAS.md`                | Update type mapping table                                     |
| Example quills / fixtures                 | Migrate decimal-valued `number` fields to `float`             |
