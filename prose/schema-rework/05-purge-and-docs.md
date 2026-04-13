# Phase 5: Purge and Documentation

Delete the now-unreachable JSON Schema generation code. Drop the `jsonschema`
crate dependency. Purge every mention of `x-ui` from the codebase. Rewrite
the design docs to describe the new architecture.

## Goal

Leave the codebase with exactly one schema representation: `QuillConfig`.
No projection logic, no JSON Schema intermediates, no `x-ui` namespace
gymnastics, no dead `SchemaProjection` enum. Documentation reflects reality.

## Why this phase last

Phase 4 made all JSON Schema code unreachable but did not delete it. That
separation was deliberate: deletion and cutover as one combined change is
harder to review and harder to revert if an unexpected call site turns up.
With phase 4 merged and the test suite green, deletion is safe.

## Deliverables

### Part A: Delete `schema.rs` code

**File:** `crates/core/src/schema.rs`

Delete the following functions entirely:

- `build_schema_from_config`
- `build_schema`
- `build_field_property`
- `build_card_def`
- `project_schema`
- `strip_schema_fields`
- `validate_document` (the JSON-Schema one)
- `extract_defaults_from_schema`
- `extract_examples_from_schema`
- `coerce_document` (the JSON-Schema one)

Delete the `SchemaProjection` enum.

Delete every test in `schema.rs` that references the above (~70+ tests).
Tests that were relocated to `validation.rs` / `extract.rs` / `schema_yaml.rs`
in earlier phases stay where they are.

**Outcome:** `schema.rs` should be empty or removed entirely. If empty,
delete the file and remove `pub mod schema;` from `crates/core/src/lib.rs`.

### Part B: Drop `jsonschema` crate dependency

**Files:**
- `crates/core/Cargo.toml` — remove the `jsonschema` dependency
- Workspace `Cargo.toml` if `jsonschema` is a workspace dep — remove it
- `Cargo.lock` — regenerate via `cargo build`

Confirm no other crate in the workspace still pulls `jsonschema` transitively
in a way that matters. `cargo tree -p quillmark-core | grep jsonschema`
should return nothing.

### Part C: Purge `x-ui` references

Grep for `x-ui` / `x_ui` everywhere in the codebase. Expected locations:

- Remove from any remaining test assertions
- Remove from doc comments
- Remove from `docs/format-designer/quill-yaml-reference.md`
- Remove from `prose/designs/SCHEMAS.md` (which is being rewritten anyway)
- Remove from any example YAML or JSON in `docs/`

Replace references with plain `ui:` where they described the shape of the
emitted output.

### Part D: Delete JSON Schema documentation

**Files to delete:**
- None strictly required, but check `prose/designs/` for any file
  specifically about JSON Schema generation. If `SCHEMAS.md` is majority
  about the JSON Schema output, rewrite rather than delete (Part E).

### Part E: Rewrite `prose/designs/SCHEMAS.md`

The current file documents:
- Type mapping Quill.yaml → JSON Schema
- `x-ui` extension
- `SchemaProjection` (AI vs UI views)
- Validation semantics

Rewrite to document:
- Quill.yaml DSL (types, constraints, ui hints)
- Native validation walker (one section; summarize `01-native-validator.md`)
- Public schema YAML emission (one section; summarize `03-public-schema-emission.md`)
- How `QuillConfig` is the sole source of truth

Keep type reference table (it's still useful, just drop the JSON Schema
column):

| Quill.yaml Type | Meaning |
|---|---|
| `string` | UTF-8 text |
| `number` | Numeric value |
| `boolean` | `true` / `false` |
| `array` | Ordered list; use `items:` |
| `object` | Structured map; use `properties:` |
| `date` | `YYYY-MM-DD` |
| `datetime` | ISO 8601 |
| `markdown` | Rich text; backends handle conversion |

### Part F: Write `prose/designs/PUBLIC_SCHEMA.md`

New design doc describing the external contract:

- What it is (YAML subset with name/description/example/fields/cards)
- Who consumes it (LLMs, form UI, third parties)
- Exact shape (copy the schema from `00-overview.md`)
- How it relates to `Quill.yaml` (projection by exclusion of `Quill:` internals)
- Why YAML string, not JSON object
- The output-shape contract: example demonstrates it; no separate spec

Link to this from `prose/designs/INDEX.md`.

### Part G: Update consumer-facing docs

**File:** `docs/format-designer/quill-yaml-reference.md`

- Remove all `x-ui` mentions
- Document the `example:` field on the `Quill:` block
- Note that `ui:` hints are part of the public schema (no renaming)

**File:** `docs/integration/validation.md`

- Update the LLM validation loop example to show how to pass the YAML
  schema to the LLM (not JSON Schema)
- Reference `quill.schema` returning YAML text
- Update the error handling example to show new `ValidationError` shape

### Part H: Update proposal and architecture docs

**File:** `prose/proposals/document_rework.md`

This proposal predates the rework. Add a note at the top:

```markdown
**Superseded in part by `prose/schema-rework/`.** This proposal introduced
`SchemaProjection` (AI/UI). The schema rework eliminated projections
entirely by making the public contract a YAML subset of Quill.yaml directly.
```

**File:** `prose/designs/INDEX.md`

- Add link to new `PUBLIC_SCHEMA.md`
- Update `SCHEMAS.md` summary to reflect rewrite

### Part I: Clean up stale references

Grep the entire workspace (source + docs + prose) for:

- `SchemaProjection`
- `build_schema`
- `build_schema_from_config`
- `project_schema`
- `getStrippedSchema`
- `extract_defaults_from_schema`
- `extract_examples_from_schema`
- `coerce_document`
- `x-ui`
- `x_ui`
- `jsonschema::`
- `serde_json::Value` where it referred to a schema (context-dependent;
  check each hit)

Any hit means something was missed. Fix or delete.

## Non-goals

- **No new features.**
- **No additional API shaping.** The shape was decided in phases 1-3.
- **No performance work.**

## Acceptance criteria

- [ ] `crates/core/src/schema.rs` deleted (file does not exist) or reduced
      to ~0 lines of exported code
- [ ] `pub mod schema;` removed from `crates/core/src/lib.rs` if file is gone
- [ ] `jsonschema` crate no longer listed in any `Cargo.toml`
- [ ] `cargo tree` shows `jsonschema` is not a transitive dep of any
      workspace crate
- [ ] `cargo build --workspace` succeeds with no warnings about unused
      imports/dependencies
- [ ] `cargo test --workspace` passes
- [ ] Workspace-wide grep for `x-ui`, `x_ui` returns zero hits
- [ ] Workspace-wide grep for `SchemaProjection`, `build_schema` (as
      standalone identifier, not substring), `getStrippedSchema` returns
      zero hits
- [ ] `prose/designs/SCHEMAS.md` rewritten
- [ ] `prose/designs/PUBLIC_SCHEMA.md` exists
- [ ] `prose/designs/INDEX.md` updated
- [ ] `docs/format-designer/quill-yaml-reference.md` updated
- [ ] `docs/integration/validation.md` updated
- [ ] `prose/proposals/document_rework.md` has supersession note

## Implementation notes

### Order of operations within this phase

1. Confirm `cargo test --workspace` is green (post-phase-4 state)
2. Delete `schema.rs` functions one at a time, running `cargo check` after
   each deletion to catch any missed call site. If a call site is found,
   fix it (shouldn't happen if phase 4 was complete; if it does, the gap
   was missed and needs investigation)
3. Delete `schema.rs` file entirely
4. Drop `jsonschema` from `Cargo.toml`
5. `cargo build --workspace` — should succeed
6. Grep for the purge list above
7. Rewrite/update docs (this part has no compilation check, so slow down
   and review each doc manually)

### Review hint

The grep commands in "Part I" are the best single signal for completeness.
Run them before declaring phase done. Zero hits = clean.

### Docs voice

The existing design docs are terse and code-adjacent. Match that voice.
Don't write marketing prose. Example sentence shape:

> The public schema is emitted as YAML text by `QuillConfig::public_schema_yaml`.
> Consumers receive the string directly via `quill.schema` in Python and
> `quillInfo.schema` in WASM. No JSON intermediate.

### Changelog / migration note

Pre-release, so no formal migration guide. But include a short note in the
top-level README or CHANGELOG.md (if one exists) describing the schema
format change for any early adopters tracking the repo. One paragraph is
enough.

### What *not* to delete

- `FieldSchema`, `CardSchema`, `UiSchema`, `QuillConfig` — structural,
  keep
- `QuillValue` — foundational
- Fixture examples created in phase 3 — keep
- Any test module created in phases 1-3 — keep
- Bindings API from phase 4 — keep

## Estimated scope

- Deletion: ~2000+ lines removed from `schema.rs`, ~500-1000 test lines
- New docs: ~200 LOC in `PUBLIC_SCHEMA.md`, ~100 LOC of updates to
  `SCHEMAS.md`, ~30-50 LOC each in consumer docs
- Net: large negative line count; mostly mechanical; finicky grep-and-fix
  for the `x-ui` purge and dead reference sweep
