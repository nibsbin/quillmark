# Migration: Rust Library Consumers

Audience: Rust crates depending on `quillmark-core` or `quillmark`
directly. Applies to both third-party integrators and internal
workspace crates vendoring these outside the current tree.

## Summary

- The entire `quillmark_core::schema` module is **deleted**.
- `QuillConfig` is now the schema source of truth; all extraction,
  validation, coercion, and emission live on it (or in
  `quill::validation`).
- The `jsonschema` crate is no longer a dependency.
- The `schema` field on `Quill` is gone — use `Quill::config` (which is
  `QuillConfig`) directly.

## Dependency changes

```toml
# before (Cargo.toml)
[dependencies]
quillmark-core = { version = "…" }
jsonschema = { version = "0.37", default-features = false }  # if you were using it alongside
```

```toml
# after
[dependencies]
quillmark-core = { version = "…" }
# drop jsonschema if you only pulled it to validate Quill schemas
# add serde_yaml if you need to parse the public schema yourself
serde_yaml = "0.9"
```

Nothing we ship re-exports `jsonschema` anymore. If you were relying on
a transitive pull, add it explicitly — but see below for why you
probably don't need it.

## Module / item map

| Old path | New path |
|---|---|
| `quillmark_core::schema::build_schema_from_config` | — (deleted; no replacement needed) |
| `quillmark_core::schema::build_schema` | — (deleted) |
| `quillmark_core::schema::build_schema_from_fields` | — (deleted) |
| `quillmark_core::schema::project_schema` | — (deleted; no projection) |
| `quillmark_core::schema::SchemaProjection` | — (deleted) |
| `quillmark_core::schema::extract_defaults_from_schema` | `QuillConfig::defaults()` |
| `quillmark_core::schema::extract_examples_from_schema` | `QuillConfig::examples()` |
| `quillmark_core::schema::extract_card_item_defaults` | `QuillConfig::card_defaults(name)` |
| `quillmark_core::schema::apply_card_item_defaults` | handled by `Workflow` pipeline; or call `card_defaults` + merge yourself |
| `quillmark_core::schema::validate_document` | `QuillConfig::validate(fields)` |
| `quillmark_core::schema::coerce_document` | `QuillConfig::coerce(fields)` |
| `Quill.schema` field (`QuillValue`) | removed — use `Quill.config` |

All replacement items are re-exported from `quillmark_core` via
`quillmark_core::{Quill, QuillValue}` and `quillmark_core::quill::{QuillConfig, CoercionError, …}`.

## Call-site rewrites

### Extracting defaults / examples

```rust
// before
use quillmark_core::schema::{build_schema_from_config, extract_defaults_from_schema};

let schema = build_schema_from_config(&quill.config)?;
let defaults = extract_defaults_from_schema(&schema);
```

```rust
// after
let defaults = quill.config.defaults();
// examples:
let examples = quill.config.examples();
// card-scoped:
let card_defaults = quill.config.card_defaults("indorsement");
let card_examples = quill.config.card_examples("indorsement");
```

Return types are unchanged: `HashMap<String, QuillValue>` and
`HashMap<String, Vec<QuillValue>>` respectively. `card_*` variants
return `Option<…>` and resolve to `None` for unknown card names.

### Validation

```rust
// before
use quillmark_core::schema::{build_schema_from_config, validate_document};

let schema = build_schema_from_config(&quill.config)?;
let errors = validate_document(&schema, &fields);  // Vec of JSON-Schema errors
```

```rust
// after
match quill.config.validate(&fields) {
    Ok(()) => {}
    Err(errors) => {
        for err in errors {
            eprintln!("{err}");  // ValidationError impls Display via thiserror
        }
    }
}
```

Error type: `Vec<quillmark_core::quill::validation::ValidationError>`.
Note: the `validation` module is `pub(crate)` in `quillmark-core` today;
concrete variants are reachable only through the `QuillConfig::validate`
return type, matched with `Debug` / `Display`. If you need to pattern-
match on variants from outside the crate, we'll need to promote the
module to `pub` — open a ticket.

Variants (for reference — see `crates/core/src/quill/validation.rs:12`):

- `MissingRequired { path }`
- `TypeMismatch { path, expected, actual }`
- `EnumViolation { path, value, allowed }`
- `FormatViolation { path, format }`
- `UnknownCard { path, card }`
- `MissingCardDiscriminator { path }`

`path` is a field path (e.g. `cards.indorsement[0].signature_block`),
not a JSON Pointer.

### Coercion

```rust
// before
use quillmark_core::schema::{build_schema_from_config, coerce_document};

let schema = build_schema_from_config(&quill.config)?;
let coerced = coerce_document(&schema, &fields);  // infallible
```

```rust
// after
use quillmark_core::quill::CoercionError;

let coerced = quill.config.coerce(&fields)?;
// CoercionError is a thiserror enum with one variant today:
//   Uncoercible { path, value, target, reason }
```

The new `coerce` is **fallible**. If you were downstream of the old
infallible `coerce_document`, wrap with `?` or `.unwrap_or_else(...)`.
Unparseable strings for `number` / `date` / `datetime` / `boolean`
targets now surface as `Err(CoercionError::Uncoercible { ... })`
instead of silently retaining the original value.

### Reading the public schema

```rust
// before — JSON Schema for external consumers
let schema = build_schema_from_config(&quill.config)?;
let json_schema_str = serde_json::to_string(&schema)?;
```

```rust
// after — YAML subset
let yaml = quill.config.public_schema_yaml()?;  // Result<String, serde_yaml::Error>
```

See `crates/core/src/quill/schema_yaml.rs:137` for the API and
`crates/fixtures/resources/quills/usaf_memo/0.1.0/__golden__/public_schema.yaml`
for a complete example.

### `Quill.schema` field removal

```rust
// before
let schema_json = &quill.schema;      // QuillValue holding JSON Schema
```

```rust
// after — no equivalent stored on Quill; rebuild on demand if needed:
let yaml = quill.config.public_schema_yaml()?;
// or access underlying structure via Quill.config:
let main_fields = quill.config.main().fields.iter();
```

If you only need the emitted YAML, call `public_schema_yaml()` — it is
cheap enough (one serde_yaml pass) that we did not cache it. If you
need the structural model, walk `QuillConfig` directly:

```rust
// QuillConfig surface
impl QuillConfig {
    pub fn main(&self) -> &CardSchema;                 // cards[0]
    pub fn card_definitions(&self) -> &[CardSchema];   // everything else
    pub fn card_definitions_map(&self) -> HashMap<String, CardSchema>;
    pub fn card_definition(&self, name: &str) -> Option<&CardSchema>;
    pub fn defaults(&self) -> HashMap<String, QuillValue>;
    pub fn examples(&self) -> HashMap<String, Vec<QuillValue>>;
    pub fn card_defaults(&self, name: &str) -> Option<HashMap<String, QuillValue>>;
    pub fn card_examples(&self, name: &str) -> Option<HashMap<String, Vec<QuillValue>>>;
    pub fn validate(&self, fields: &HashMap<String, QuillValue>)
        -> Result<(), Vec<ValidationError>>;
    pub fn coerce(&self, fields: &HashMap<String, QuillValue>)
        -> Result<HashMap<String, QuillValue>, CoercionError>;
    pub fn public_schema_yaml(&self) -> Result<String, serde_yaml::Error>;
    pub fn from_yaml(yaml_content: &str)
        -> Result<Self, Box<dyn StdError + Send + Sync>>;
}
```

`CardSchema` and `FieldSchema` are re-exported at
`quillmark_core::quill::{CardSchema, FieldSchema, FieldType, UiContainerSchema, UiFieldSchema}`.

## `Workflow` API is unchanged

The user-facing `Workflow` methods in the `quillmark` crate kept their
signatures:

- `Workflow::new(backend, quill)`
- `Workflow::render(&parsed, &opts)`
- `Workflow::dry_run(&parsed)`
- `Workflow::validate_schema(&parsed)`

`dry_run` and `validate_schema` internally went from `jsonschema`-backed
validation to `QuillConfig::validate`. Return type is still
`Result<…, RenderError>`; `RenderError` variant shapes covering
validation failures may differ in their wrapped messages — check your
`match` arms.

## What happens if I keep using `jsonschema`?

Nothing — you can still pull it yourself and write your own JSON Schema
from `QuillConfig` if you must. We just don't ship that path. The
recommendation is to consume the YAML public schema directly with
`serde_yaml` or use the typed `QuillConfig` accessors.

## Test-surface breaks

If your tests asserted on `build_schema_from_config(...)` output shape
(e.g., counted `properties` keys), rewrite them to walk
`quill.config.main().fields`. If you had golden JSON Schemas checked
into your repo, delete them — there is no JSON Schema anymore.

## Quick checklist

- [ ] Delete `use quillmark_core::schema::…` imports
- [ ] Replace `extract_defaults_from_schema(&schema)` →
      `quill.config.defaults()`
- [ ] Replace `extract_examples_from_schema(&schema)` →
      `quill.config.examples()`
- [ ] Replace `validate_document(&schema, &fields)` →
      `quill.config.validate(&fields)`
- [ ] Replace `coerce_document(&schema, &fields)` →
      `quill.config.coerce(&fields)?` (now fallible)
- [ ] Replace any read of `quill.schema` with `quill.config` or
      `quill.config.public_schema_yaml()?`
- [ ] Drop `jsonschema` from `Cargo.toml` if it was only for us
- [ ] Add `serde_yaml` if you parse the public schema in-process
- [ ] Update error matching: new variants on
      `ValidationError` and `CoercionError`; old `jsonschema::ValidationError`
      type is gone
- [ ] Audit tests that asserted on JSON Schema output shape
