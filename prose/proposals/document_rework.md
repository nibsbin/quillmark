# Proposal: Unify Main Document as First Card

## Problem

`QuillConfig` has two separate fields for what is structurally the same thing:

```rust
pub struct QuillConfig {
    pub document: CardSchema,                    // main document
    pub cards: HashMap<String, CardSchema>,      // card definitions
    // ...
}
```

Both are `CardSchema`. The separation creates unnecessary complexity:
- Two parsing paths in `from_yaml` (one for root `fields:`, one for `cards:`)
- Consumers must handle `config.document` and `config.cards` as distinct concepts
- The `HashMap` for cards loses definition order

Additionally, the JSON Schema built from this config conflates validation structure with authoring guidance. The `CARDS` array property describes the parsed output shape, but AI consumers generating Quillmark markdown don't produce that shape — they write separate YAML frontmatter blocks. The schema misleads rather than guides.

## Decisions

### 1. Naming: `main`

The main document card uses the `main` semantic throughout.

- `main` is unambiguous in code (`config.main()`) with no collision against existing types (`ParsedDocument`, etc.)
- Terse and clean in YAML: `main:`
- Familiar pattern: `main()` entry point

Rejected alternatives: `document` (collides with `ParsedDocument` and overloaded usage), `root` (too developer-oriented), `base` (vague), `primary` (verbose).

### 2. Quill.yaml: explicit `main:` section

Replace the loose root-level `fields:` section with an explicit `main:` section.

**Before:**
```yaml
Quill:
  name: usaf_memo
  backend: typst
  description: USAF memorandum template

fields:
  sender:
    type: string
  date:
    type: date

cards:
  indorsement:
    fields:
      from:
        type: string
```

**After:**
```yaml
Quill:
  name: usaf_memo
  backend: typst
  description: USAF memorandum template

main:
  fields:
    sender:
      type: string
    date:
      type: date

cards:
  indorsement:
    fields:
      from:
        type: string
```

The `main:` section supports `fields:` and optional `ui:` (e.g., `hide_body`). It does not support `title` (implicitly `"main"`) or `description` (inherited from `Quill.description`).

### 3. Internal model: unified `cards: Vec<CardSchema>`

```rust
pub struct QuillConfig {
    pub cards: Vec<CardSchema>,  // cards[0] = main, cards[1..] = card definitions
    pub backend: String,
    // ...
}

impl QuillConfig {
    pub fn main(&self) -> &CardSchema { &self.cards[0] }
    pub fn card_definitions(&self) -> &[CardSchema] { &self.cards[1..] }
    pub fn card_definitions_map(&self) -> HashMap<String, CardSchema> { /* rebuild */ }
}
```

Future optimization: replace `Vec<CardSchema>` with `IndexMap<String, CardSchema>` to eliminate the `card_definitions_map()` rebuild. Bookmarked, not required initially.

### 4. Markdown parsing: no change

The first frontmatter block with `QUILL:` is implicitly the main card. No `CARD:` directive needed.

```markdown
---
QUILL: usaf_memo@0.2
sender: John
date: 2026-03-29
---

Body text here...

---
CARD: indorsement
from: ORG/SYMBOL
---

Indorsement body...
```

This gives users a guaranteed card for initial configuration without additional syntax.

### 5. Parsed document output: no change

Main card fields remain top-level in `ParsedDocument`. Card instances remain in the `CARDS` array. Templates continue to access `sender`, `date`, `BODY` as top-level variables and iterate `CARDS` for card instances.

```json
{
  "sender": "John",
  "date": "2026-03-29",
  "BODY": "Body text here...",
  "CARDS": [
    { "CARD": "indorsement", "from": "ORG/SYMBOL", "BODY": "..." }
  ]
}
```

### 6. JSON Schema: QuillConfig is the schema, JSON Schema is a serialization format

`QuillConfig` is the single source of truth for the schema. JSON Schema is one projection — a serialization format chosen for its ecosystem benefits (validation libraries, AI familiarity, UI tooling).

The raw schema returned by `build_schema()` is the **validation schema** — the full JSON Schema with `CARDS` array property, `$defs`, `oneOf` dispatch. This is the default. No projection needed for internal validation.

External consumers get **projections** — curated views derived from the raw schema:

- **AI projection:** Strips the `CARDS` property and `x-ui` extensions. Keeps `$defs` so AI consumers can reference card type field definitions. The markdown authoring format (separate `---` blocks with `CARD:` discriminator) is communicated via the prompt engineering layer, not the schema.
- **UI projection:** Strips `CARDS` property. Keeps `$defs` and `x-ui` for form generation.

### 7. Schema projection API: projection-oriented, not strip-oriented

Replace the current `strip_schema_fields` (subtractive, caller-driven) with a projection-oriented API (intent-driven, centralized):

```rust
/// Schema projections for external consumers.
/// The raw schema (from build_schema) is the validation view — no projection needed.
pub enum SchemaProjection {
    AI,  // strips CARDS, x-ui
    UI,  // strips CARDS, keeps x-ui
}

pub fn project_schema(schema: &QuillValue, projection: SchemaProjection) -> QuillValue {
    // ...
}
```

**Why:** The current `strip_schema_fields(&mut schema, &["x-ui"])` requires every call site to know which fields to remove. Adding a new internal field or a new consumer means updating scattered call sites. A projection enum centralizes the logic — "I need the AI view" — and the filtering rules live in one place.

### 8. Defaults and coercion: operate on QuillConfig directly

Today defaults and coercion round-trip through JSON Schema:

```
QuillConfig → build_schema() → JSON Schema → extract_defaults() → defaults
                                            → with_coercion(&schema)
```

This is unnecessary. `QuillConfig` already has all the information — field types, defaults, enum constraints. The JSON Schema is just a serialization of what `CardSchema` and `FieldSchema` already express.

**Proposed data flow:**

```
QuillConfig ─→ extract_defaults()  → HashMap<String, QuillValue>
            ─→ with_coercion()     → coerced fields
            ─→ build_schema()      → JSON Schema (for validation + export only)
```

- `with_defaults()` takes `&QuillConfig` — iterates `main().fields` directly
- `with_coercion()` takes `&QuillConfig` — reads `FieldSchema.r#type` directly
- `build_schema()` becomes a pure serialization function — renders `QuillConfig` as JSON Schema
- `jsonschema` validation remains, operating on the serialized JSON Schema. This is the one load-bearing use of JSON Schema. External consumers don't validate against it independently, so this can be revisited later if custom validation proves worthwhile.

**Why:** Eliminates the round-trip. Faster, one code path for defaults, and divergence between config and schema is caught by validation as a safety net.

## Scope

### In scope
- `QuillConfig` struct change (`document` + `cards HashMap` -> `cards: Vec<CardSchema>`)
- `main()`, `card_definitions()`, `card_definitions_map()` accessors
- Quill.yaml parsing: `main:` section replaces root `fields:`
- Schema projection API replacing `strip_schema_fields`
- Defaults and coercion extracted from `QuillConfig` directly
- `build_schema()` refactored to pure serialization
- Update all consumers: `load.rs`, `schema.rs`, CLI `validate`, tests
- Update fixture Quill.yaml files to use `main:` format

### Out of scope (future work)
- `IndexMap` optimization for `cards`
- Changes to markdown parsing or `ParsedDocument` structure
- Changes to template/plate variable access patterns
- Prompt engineering layer for AI authoring guidance
- Replacing `jsonschema` crate with custom validation

## Files affected

| File | Change |
|------|--------|
| `crates/core/src/quill/config.rs` | Struct change, `main:` parsing, accessors |
| `crates/core/src/quill/load.rs` | `config.document` -> `config.main()` |
| `crates/core/src/schema.rs` | `build_schema` signature, add projection functions |
| `crates/bindings/cli/src/commands/validate.rs` | Use new accessors |
| `crates/core/src/quill/tests.rs` | Update all config access patterns |
| `crates/fixtures/resources/*/Quill.yaml` | `fields:` -> `main:` format |
