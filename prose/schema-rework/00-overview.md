# Schema Rework: Overview

Replace the JSON Schema external contract with a **YAML subset of Quill.yaml**.
Delete the JSON Schema generation machinery entirely. Replace internal
validation with a native walker over `QuillConfig`.

This is a pre-release refactor. There is no backward compatibility to preserve.
Legacy code should be deleted aggressively at the end of each phase.

## Why

JSON Schema serves three distinct concerns today:

1. **External contract** — what LLMs and the form UI receive
2. **Internal validation** — `dry_run`, coercion, defaults/examples extraction
3. **Serialization format** — consumer-facing wire format

The impedance mismatch between Quill.yaml's native DSL and JSON Schema forced
the `x-ui` namespace hack, a misleading top-level `CARDS` array, dual
`SchemaProjection` views (AI vs UI), and a recursive `strip_schema_fields`
walker to paper over the translation. All of that machinery exists to serve
concern #1.

By making the public contract a **YAML subset of the author's own Quill.yaml**,
we:

- Eliminate projections, `x-ui`, `strip_schema_fields`, dual AI/UI views
- Remove the misleading `CARDS` top-level property
- Give LLMs a denser, more natural format (same shape as the source file)
- Let our form builder parse one canonical format
- Drop the `jsonschema` crate dependency entirely
- Collapse the "schema as intermediate for validation/defaults/examples" pattern
  into direct reads from `QuillConfig`

## The new public contract

A YAML string containing:

```yaml
name: <string>
description: <string>
example: <markdown string>    # full end-to-end document example

fields:
  <field_name>:
    type: string | number | boolean | array | object | date | datetime | markdown
    title: <string>
    description: <string>
    required: <bool>
    default: <value>
    examples: [<value>, ...]
    enum: [<value>, ...]
    properties: { ... }       # for object types
    items: { ... }            # for array types
    ui: { group, order, compact, multiline }

cards:
  <card_name>:
    title: <string>
    description: <string>
    fields: { ... }
    ui: { hide_body, default_title }
```

`name`, `description`, and `example` are promoted from the `Quill:` block.
Everything else (`version`, `backend`, `glue`) stays internal.

Consumers (LLMs, our form builder) receive this as a **YAML string**, not a
parsed object. LLMs pass it straight through to prompts; the form builder
parses once.

## Architectural principles

1. **`QuillConfig` is the sole source of truth.** No intermediate serialization
   format. No JSON Schema. Validation, coercion, defaults, and examples all
   read directly from `QuillConfig`.

2. **The public contract is a projection by exclusion.** The subset emission
   drops `Quill:` metadata (except name/description/example) and nothing else
   needs transformation. Author writes `ui:`, consumer reads `ui:`. No rename.

3. **Output shape is demonstrated, not prescribed.** The `example` field in
   the subset shows LLMs how to produce a conforming document. No separate
   output-format spec needed in the public contract.

4. **Native validation, not generic JSON Schema.** Walk `QuillConfig` directly.
   Field-path error messages. No jsonschema keyword translation.

## Phase sequence

| Phase | File | Scope |
|---|---|---|
| 1 | `01-native-validator.md` | Add `QuillConfig`-native validator alongside existing JSON Schema validation |
| 2 | `02-config-native-extractors.md` | Move defaults, examples, coercion to `QuillConfig` methods |
| 3 | `03-public-schema-emission.md` | Add `example` field to Quill.yaml; emit public YAML subset from `QuillConfig` |
| 4 | `04-cutover.md` | Flip internal callers + bindings to new API; old JSON Schema code becomes dead |
| 5 | `05-purge-and-docs.md` | Delete all JSON Schema generation code; drop `jsonschema` dep; update docs |

Phases are strictly additive through phase 3. Phase 4 switches the active
code path. Phase 5 removes the dead code.

## Non-goals

- **No changes to the parser.** Markdown → `ParsedDocument` is out of scope.
- **No changes to backends.** Typst/other backends receive the same internal
  data structure (main fields + CARDS array + BODY).
- **No changes to `Quill.yaml` authoring syntax** except adding the optional
  `example:` field to the `Quill:` block.
- **No new form-builder work.** The public YAML contract is defined here;
  consumers of it are out of scope.

## Success criteria

- `crates/core/src/schema.rs` is deleted or reduced to a thin re-export shim
- `jsonschema` crate no longer appears in `Cargo.toml`
- WASM and Python bindings expose `quill.schema -> String` (YAML text)
- `getStrippedSchema()` and `SchemaProjection` are gone
- All existing fixture-based integration tests pass
- `x-ui` no longer appears anywhere in the codebase or docs
- The subset emitted for `usaf_memo` fixture round-trips through `serde_yaml`
  as expected and contains `name`, `description`, `example`, `fields`, `cards`

## Files that will be modified or deleted

**Deleted:**
- `crates/core/src/schema.rs` (~2700 lines; possibly replaced by ~200-line
  native validator + subset emitter)

**Modified:**
- `crates/core/src/quill/types.rs` — add `example: Option<String>` to the
  Quill block representation
- `crates/core/src/quill/mod.rs` (or wherever `QuillConfig` lives) — add
  validation/extraction/emission methods
- `crates/core/src/lib.rs` — adjust module exports
- `crates/bindings/wasm/src/engine.rs` — replace `getStrippedSchema` with
  `getQuillSchema` returning YAML string
- `crates/bindings/wasm/src/types.rs` — `QuillInfo.schema` becomes `String`
- `crates/bindings/python/src/types.rs` — `quill.schema` returns `str`
- `Cargo.toml` files — drop `jsonschema` dependency
- `prose/designs/SCHEMAS.md` — rewrite to describe new architecture
- `docs/format-designer/quill-yaml-reference.md` — remove `x-ui`, document
  `example:` field
- `docs/integration/validation.md` — update LLM validation loop example

**Added:**
- `crates/core/src/quill/validation.rs` (phase 1)
- `crates/core/src/quill/extract.rs` or methods on `QuillConfig` (phase 2)
- `crates/core/src/quill/schema_yaml.rs` or methods on `QuillConfig` (phase 3)
- `prose/designs/PUBLIC_SCHEMA.md` — new design doc for the YAML subset contract
