# Proposal: Add `integer` Field Type Distinct from `number`

## Problem

The single `FieldType::Number` variant accepts both integers and floats without distinction. Validation (`validation.rs:147`) only checks `is_number()`; coercion (`config.rs:299`) opportunistically prefers `i64` but silently falls back to `f64`. Schema output emits `"type": "number"` regardless.

Consumers (form UIs, backend templates, validators) cannot tell from the schema whether a field expects a whole count (e.g., `page_count: 3`) or a decimal quantity (e.g., `tax_rate: 0.075`). Authors have no way to communicate that intent in `Quill.yaml`.

Typst plates, in particular, distinguish `int` and `float` at the type system level. Passing a coerced `f64` where an `int` is expected causes runtime errors the schema could have prevented.

## Decisions

### 1. Add `FieldType::Integer` as a distinct variant

Not an alias of `Number`. Two separate types with different semantics:

| Quill.yaml  | Accepts                       | Coerces to | JSON Schema emit      |
|-------------|-------------------------------|------------|-----------------------|
| `number`    | integers and decimals         | `f64`      | `"type": "number"`    |
| `integer`   | integers only                 | `i64`      | `"type": "integer"`   |

`number` is unchanged in behavior — it remains the "any numeric" type. `integer` is the new strict type that rejects decimal values. This matches the JSON Schema / OpenAPI convention exactly.

### 2. `number` coercion unchanged

`FieldType::Number` (`config.rs:299`) keeps its current behavior:
- Any JSON numeric → pass through
- String `"5"` → `i64(5)`
- String `"5.0"` → `f64(5.0)`
- Bool → `0` / `1`

### 3. New coercion for `integer`

`FieldType::Integer`:
- Integer JSON value → `i64`
- Decimal JSON value → **reject** with `Uncoercible { target: "integer" }`
- String `"5"` → `i64(5)`
- String `"5.0"` → **reject**
- Bool → `0` / `1`

### 4. Schema emit maps to JSON Schema conventions

`number` continues to emit `"type": "number"`.
`integer` emits `"type": "integer"`.

Both are standard JSON Schema / OpenAPI type keywords — no custom extensions needed.

### 5. No alias

`"integer"` in `Quill.yaml` is a first-class type name. No alias for `int` or similar. `FieldType::from_str` gets a new arm.

## Scope

### In scope
- Add `FieldType::Integer` variant in `types.rs`
- `FieldType::from_str`: add `"integer" => Integer` arm
- `FieldType::as_str`: add `Integer => "integer"` arm
- Validation in `validation.rs`: `Integer` requires `is_i64() || is_u64()`; `Number` unchanged
- Coercion in `config.rs`: add `Integer` branch per rules above; `Number` branch unchanged
- Schema emit in `schema_yaml.rs` / `schema.rs`: `Integer` → `"integer"`
- Tests: integer coercion tests (accept int, reject decimal), schema emit test
- Update docs: `creating-quills.md`, `quill-yaml-reference.md`, `SCHEMAS.md`

### Out of scope
- Any changes to existing `number` behavior — non-breaking
- Numeric bounds (`minimum`, `maximum`, `multipleOf`) — separate proposal
- Unsigned integer type — deferred until proven need

## Migration

Non-breaking for existing quills. `number` behavior is unchanged. Authors opt in to `integer` for fields where decimal values should be rejected.

## Files affected

| File                                      | Change                                                        |
|-------------------------------------------|---------------------------------------------------------------|
| `crates/core/src/quill/types.rs`          | Add `Integer` variant, update `from_str`/`as_str`             |
| `crates/core/src/quill/validation.rs`     | Add `Integer` validation, update type-name mapping            |
| `crates/core/src/quill/config.rs`         | Add `Integer` coercion branch                                 |
| `crates/core/src/quill/schema_yaml.rs`    | `Integer` → `"integer"` in emitted schema                     |
| `crates/core/src/schema.rs`               | Same mapping in JSON Schema builder                           |
| `crates/core/src/quill/tests.rs`          | New integer coercion and schema emit tests                    |
| `docs/guides/creating-quills.md`          | Document `integer`; clarify `number` = any numeric            |
| `docs/guides/quill-yaml-reference.md`     | Add `integer` row to type table                               |
| `prose/designs/SCHEMAS.md`                | Update type mapping table                                     |
