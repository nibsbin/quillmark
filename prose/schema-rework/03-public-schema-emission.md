# Phase 3: Public Schema Emission

Add the `example` field to `Quill.yaml` parsing. Emit the public YAML subset
that external consumers (LLMs, our form builder) receive in place of JSON
Schema.

## Goal

Implement `QuillConfig::public_schema_yaml() -> String` that produces the
YAML document described in `00-overview.md`:

```yaml
name: <string>
description: <string>
example: <markdown string>

fields:
  <name>: { ... }
cards:
  <name>: { ... }
```

Add support for authoring `example: <path>` in `Quill.yaml`'s `Quill:` block,
load the referenced file during quill parse, and include its contents in the
emitted subset.

## Why this phase

Phases 1 and 2 established the config-native foundations for validation and
extraction. This phase builds the third leg: the external contract. After
this phase, both the internal API (native validator + extractors) and the
external API (YAML subset) exist as additive additions. Phase 4 flips the
switches.

## Deliverables

### Part A: Parse `example:` from `Quill.yaml`

**File:** `crates/core/src/quill/types.rs`

Extend the representation of the `Quill:` block to carry an optional example
path. The field on `QuillConfig` after loading should be the **resolved
content**, not the path.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct QuillMeta {
    pub name: String,
    pub version: String,
    pub backend: String,
    pub description: Option<String>,
    pub example: Option<String>,   // path relative to quill root
    // ... other existing fields
}

#[derive(Debug, Clone)]
pub struct QuillConfig {
    // ... existing fields
    pub example_markdown: Option<String>,  // loaded file contents
}
```

The raw deserialized `QuillMeta.example` is a path; the loader resolves it
and stores the content in `QuillConfig.example_markdown`. If the `example:`
key is absent, `example_markdown` is `None`.

**Loader change:** wherever quill directories are loaded (look for where
`Quill.yaml` is parsed — likely `crates/core/src/quill/loader.rs` or
`from_path` on `Quill`), after deserializing:

```rust
let example_markdown = match &meta.example {
    Some(path) => {
        let full_path = quill_root.join(path);
        Some(std::fs::read_to_string(&full_path)
            .map_err(|e| /* LoadError::ExampleNotFound */)?)
    }
    None => None,
};
```

Path resolution rules:

- Relative to quill root (same directory as `Quill.yaml`)
- Must be within the quill directory (reject `..` traversal)
- Missing file → load error with clear message (quill is broken)

### Part B: Emit the public YAML subset

**New file:** `crates/core/src/quill/schema_yaml.rs`

```rust
use crate::quill::types::{QuillConfig, FieldSchema, CardSchema, FieldType, UiSchema};
use serde::Serialize;
use std::collections::BTreeMap;

impl QuillConfig {
    /// Emit the public schema as a YAML string.
    ///
    /// Structure:
    /// - name, description, example (top-level)
    /// - fields: main field schemas
    /// - cards: card schemas
    ///
    /// Internal `Quill:` metadata (version, backend, glue) is excluded.
    pub fn public_schema_yaml(&self) -> Result<String, serde_yaml::Error>;
}
```

Internal representation for serialization (use serde structs for stability):

```rust
#[derive(Serialize)]
struct PublicSchema<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    example: Option<&'a str>,
    fields: BTreeMap<&'a str, PublicField<'a>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    cards: BTreeMap<&'a str, PublicCard<'a>>,
}

#[derive(Serialize)]
struct PublicField<'a> {
    r#type: &'static str,  // "string", "array", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<&'a QuillValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    examples: Option<&'a [QuillValue]>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "enum")]
    enum_values: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<&'a str, PublicField<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<PublicField<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui: Option<&'a UiSchema>,
}

#[derive(Serialize)]
struct PublicCard<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    fields: BTreeMap<&'a str, PublicField<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui: Option<&'a CardUiSchema>,
}
```

Serialize with `serde_yaml::to_string` — no custom formatting.

### Key serialization details

- **Key order within fields:** Use `BTreeMap` for deterministic alphabetical
  order, OR preserve insertion order via `IndexMap` if `FieldSchema` already
  preserves source order. Preserving source order is preferable for LLM
  readability (authors often order fields logically).

  Check `QuillConfig.main().fields` — if it's `HashMap`, key order is
  nondeterministic today. Switch to `IndexMap` at the `QuillConfig` level as
  part of this phase, if not already.

- **`ui` fields:** emitted as nested maps, native `ui:` key (not `x-ui`).

- **`type` field:** emit as lowercase string matching Quill.yaml DSL:
  `"string" | "number" | "boolean" | "array" | "object" | "date" | "datetime"
  | "markdown"`. Match the `FieldType::as_yaml_str()` convention (add this
  method if it doesn't exist).

- **`example` string:** use YAML block scalar (`|`) for multi-line markdown.
  `serde_yaml` does this automatically for strings containing newlines when
  using `to_string`. Verify with a test.

### Tests

**Unit tests** (`schema_yaml.rs` inline tests):

- Emit a simple config with one string field; assert contains `name:`,
  `fields: { memo_for: { type: string ... } }`
- Emit with `description` present and absent (skip_serializing_if)
- Emit with `example` present; assert block scalar formatting
- Emit with cards; assert `cards:` section present
- Emit with cards absent; assert `cards:` section omitted (or `{}`)
- Emit with enum values
- Emit with nested object properties
- Emit with array items schema
- Emit with `ui:` hints — assert they appear as `ui:`, **not** `x-ui`
- Round-trip: emit YAML, parse back with `serde_yaml::from_str` into a
  `serde_yaml::Value`, assert structure matches expectations

**Fixture golden tests** (`crates/core/tests/public_schema_snapshots.rs`):

For each fixture quill, generate `public_schema_yaml()` and compare against
a committed golden file:

- `crates/fixtures/resources/quills/usaf_memo/0.1.0/__golden__/public_schema.yaml`

Use `insta` or plain string compare — whichever the repo already uses.

### Part C: Loader error handling

`QuillLoadError` (or whatever the loader error type is) gets a new variant:

```rust
#[error("example file `{path}` referenced in Quill.yaml not found")]
ExampleNotFound { path: PathBuf },

#[error("example file `{path}` is outside the quill directory")]
ExampleOutsideQuill { path: PathBuf },
```

## Non-goals

- No bindings exposure of `public_schema_yaml()` in this phase. Bindings
  still expose the old JSON-schema API. Phase 4 switches them.
- No removal of JSON Schema emission (`build_schema_from_config`).
- No LLM prompt engineering. We emit the YAML; consumers build prompts.
- No form-builder work. Consumer of the YAML is out of scope.

## Acceptance criteria

- [ ] `Quill.yaml` `example:` field parses
- [ ] `QuillConfig.example_markdown` populated correctly for fixtures with
      an `example.md` file added
- [ ] `QuillConfig::public_schema_yaml()` returns `Result<String>`
- [ ] Unit tests pass (~10-12 functions)
- [ ] Golden snapshot test for `usaf_memo` passes with a committed
      `public_schema.yaml` golden file
- [ ] Emitted YAML uses native `ui:` (not `x-ui`)
- [ ] Emitted YAML does **not** contain `CARDS` as a top-level property
- [ ] Round-trip parseable with `serde_yaml::from_str`
- [ ] Adding `example: examples/basic.md` to the `usaf_memo` fixture loads
      a real example markdown file included in this phase

## Implementation notes

### Fixture example content

Add `crates/fixtures/resources/quills/usaf_memo/0.1.0/examples/basic.md`
with a real end-to-end example of the memo format. Update `Quill.yaml` in
that fixture to reference it:

```yaml
Quill:
  name: usaf_memo
  version: 0.1.0
  backend: typst
  description: Typesetted USAF Official Memorandum
  example: examples/basic.md
```

### Why `BTreeMap`/`IndexMap` and not `HashMap` for emission

`HashMap` iteration is nondeterministic. Golden snapshot tests require
deterministic output. Pick one and commit; `IndexMap` preserving source
order is more author-friendly.

If switching from `HashMap` to `IndexMap` in `QuillConfig` is too invasive,
use `BTreeMap` at serialization time by constructing it from the `HashMap`.
Alphabetical but deterministic.

### Why not emit from a round-trip of `Quill.yaml`?

Tempting to just read `Quill.yaml`, strip the `Quill:` block, and emit.
Don't. That bypasses the type system, loses error checking, and couples
emission to on-disk format. Emit from `QuillConfig` so future changes to
Quill.yaml syntax don't silently break the public contract.

### Subset of the Quill block to expose

Decision from conversation: emit `name`, `description`, `example`. Exclude
`version`, `backend`, `glue`, and any internal pointers. If `version`
becomes important for consumers later, re-introduce it deliberately.

### Preserving `required: false` fields

`skip_serializing_if` drops `required: false`. The effective default is
"not required," so the omission is semantically correct and reduces noise.
LLMs will assume absent means optional.

## Estimated scope

~400 LOC emission code, ~200 LOC tests, ~50 LOC loader changes. One new
source file, one new fixture file, two modified source files.
