# Proposal: Canonical Document Model and Modular Parsing

**Status:** Draft — design only.
**Scope:** `quillmark-core`, `quillmark-typst` (Rust glue only),
`quillmark-wasm`, `quillmark-python`. Pre-release: we remove, we do
not deprecate.

---

## 1. Problem

Downstream consumers of `@quillmark/wasm` treat Markdown as the
canonical storage format. To edit a field, add a card, or reorder
cards, they parse to JSON, mutate, and reconstruct the Markdown by
hand. That round-trip is where correctness breaks: YAML emission is
non-trivial (quoting, Norway, multiline scalars), fence
reconstruction is non-trivial (leading-blank rule, closer alignment,
body boundaries), and there is no shared library code for any of it.

Root cause: `quillmark-core` exposes only a one-way parser.
`ParsedDocument` is read-only with a single constructor. There is no
inverse, no editor, and no schema-aware projection.

## 2. Goal

Make `quillmark-core` the source of truth for reading, editing, and
writing Quillmark Markdown documents. Mirror the surface through
`quillmark-wasm` and `quillmark-python`.

Non-goals:

- Preserving original formatting, comments, quoting style, or tags
  on re-emit. This proposal commits to **canonical re-emission**.
- Changing the Quillmark Markdown standard or the schema model.

## 3. Core idea

**The canonical type is `Document`.** A typed, in-memory Quillmark
document — not a "parsed markdown payload". Markdown is one import
format and one export format; the structure is primary.

```rust
pub struct Document {
    quill_ref: QuillReference,
    frontmatter: IndexMap<String, QuillValue>,  // no CARDS, no BODY
    body: String,
    cards: Vec<Card>,
    warnings: Vec<Diagnostic>,
}

pub struct Card {
    tag: String,
    fields: IndexMap<String, QuillValue>,
    body: String,
}
```

`ParsedDocument` is renamed to `Document` and the old name is removed.

## 4. Module layout

Replace `parse.rs` (2,925 lines) with a module tree. Each submodule
is independently usable and has a single job.

```
quillmark_core::document
├── mod.rs      // public Document / Card / errors
├── fences.rs   // line-oriented fence scanner
├── sentinel.rs // QUILL / CARD extraction, reserved-name checks
├── assemble.rs // fences + sentinels → Document
├── emit.rs     // Document → canonical Markdown
├── edit.rs     // typed mutators
└── limits.rs   // size/depth/count constants
```

The split is mechanical: each submodule already exists as a
contiguous section of `parse.rs`.

## 5. Canonical emission

### 5.1 Contract

`Document::to_markdown() -> String` guarantees:

1. **Type-fidelity round-trip.** `from_markdown(doc.to_markdown())`
   returns a `Document` equal to `doc` by value **and by type**.
   `QuillValue::String("on")` survives as a string, never as a bool.
   `QuillValue::String("01234")` survives as a string, never as an
   integer. This is the whole point of owning emission.
2. **Emit-idempotent.** `to_markdown` is a pure function of `doc`;
   two calls return byte-equal strings.

We do **not** guarantee byte-equality with the original source.

### 5.2 Rules

Opinionated, uniform, no heuristics:

- Line endings: `\n`. CRLF normalized on import.
- Frontmatter: `---\n`, `QUILL: <ref>` first, remaining fields in
  `IndexMap` insertion order, `---\n`, blank line.
- Cards: `---\nCARD: <tag>\n<fields>\n---\n<body>` in `cards` order,
  preceded by one blank line.
- Body: emitted verbatim.
- Mappings and sequences: block style only.
- Booleans: `true` / `false`. Never yes/no/on/off.
- Null: `null`.
- Numbers: bare literals.
- **Strings: always double-quoted**, JSON-style escaping (`\"`,
  `\\`, `\n`, `\t`, `\u00XX`). This is the load-bearing rule.
  Unconditional quoting makes it impossible for YAML to
  re-interpret the value as any other type. No plain-scalar
  heuristic means no Norway / numeric-string / date-string bugs.
- Multi-line strings: same rule — double-quoted with `\n` escapes.
  (Block scalar `|` is a possible v2 enhancement; v1 stays uniform.)

### 5.3 Emitter: `serde-saphyr`

Use `serde-saphyr` for emission. We already use it for parsing; same
crate on both sides eliminates dialect drift. The required feature
is narrow: force every string scalar to double-quoted. Achievable
via `SerializerOptions` if exposed, or a local `ForceQuoted(&str)`
newtype whose `Serialize` impl writes pre-escaped bytes. Rejected:
`serde_yaml` (archived), `yaml_serde` / `serde-yaml-ng` (would
duplicate YAML crate in deps), `serde_yml` (RUSTSEC-2025-0068).

### 5.4 What's lost

YAML comments, custom tags (`!fill` — value preserved, tag dropped),
original quoting style. Documented on the `to_markdown` rustdoc.

## 6. Editor surface

```rust
impl Document {
    pub fn set_field(&mut self, name: &str, value: QuillValue) -> Result<(), EditError>;
    pub fn remove_field(&mut self, name: &str) -> Option<QuillValue>;
    pub fn set_quill_ref(&mut self, reference: QuillReference);
    pub fn replace_body(&mut self, body: impl Into<String>);

    pub fn cards(&self) -> &[Card];
    pub fn card_mut(&mut self, index: usize) -> Option<&mut Card>;
    pub fn push_card(&mut self, card: Card) -> Result<(), EditError>;
    pub fn insert_card(&mut self, index: usize, card: Card) -> Result<(), EditError>;
    pub fn remove_card(&mut self, index: usize) -> Option<Card>;
    pub fn move_card(&mut self, from: usize, to: usize) -> Result<(), EditError>;
}

impl Card {
    pub fn new(tag: impl Into<String>) -> Result<Self, EditError>;  // validates tag name
    pub fn set_field(&mut self, name: &str, value: QuillValue) -> Result<(), EditError>;
    pub fn remove_field(&mut self, name: &str) -> Option<QuillValue>;
    pub fn set_body(&mut self, body: impl Into<String>);
}
```

Mutators enforce parser invariants:

- Reserved names rejected (`BODY`, `CARDS`, `QUILL`, `CARD`).
- Field names match `[a-z_][a-z0-9_]*`, NFC-normalized.
- Tag names validated via existing `is_valid_tag_name`.
- Index out of range on card methods.

`EditError` is distinct from `ParseError` (no source locations at
edit time).

## 7. Two layers: internal model vs. plate wire format

Today `CARDS` and `BODY` live inside `fields` as string keys on a
`HashMap<String, QuillValue>`. That conflates two different things:

- **Layer A — internal Rust consumers** (`validation.rs`,
  `config.rs`, `workflow.rs`, `typst/lib.rs` Rust glue) read
  `fields.get("CARDS")` because the representation is stringly-typed.
  Historical, not a contract.
- **Layer B — plate wire format.** `typst/lib.typ.template` reads
  `d.at("CARDS")` and `d.at("BODY")` at Typst runtime. Plate authors
  code against this shape. Genuine public API.

Resolution:

- **Layer A goes typed.** `Document` stores `frontmatter` (no
  CARDS/BODY), `body`, `cards`. Consumers iterate `doc.cards()` and
  read `doc.body()`. Type-mismatch error arms (`"CARDS is not an
  array"`) are deleted — they can't arise.
- **Layer B stays as-is.** A new method:

  ```rust
  impl Document {
      /// JSON payload expected by backend plates. CARDS and BODY
      /// appear as top-level keys (wire contract).
      pub fn to_plate_json(&self) -> serde_json::Value;
  }
  ```

  `Workflow::compile_data` calls `to_plate_json()` instead of
  assembling a map by hand. The `.typ.template` is unchanged.

## 8. Schema-aware form projection

New method on `Quill`:

```rust
impl Quill {
    pub fn project_form(&self, doc: &Document) -> FormProjection;
}

pub struct FormProjection {
    pub main: FormCard,
    pub cards: Vec<FormCard>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct FormCard {
    pub schema: CardSchema,
    pub values: IndexMap<String, FormFieldValue>,
}

pub struct FormFieldValue {
    pub value: Option<QuillValue>,
    pub default: Option<QuillValue>,
    pub source: FormFieldSource,  // Document | Default | Missing
}
```

`FormProjection` is what a form editor wants: for every schema
field, what's the current value, what's the default, what's
missing, and what validation errors exist. Implementation
piggybacks on the existing `coerce` / `defaults` / `validate`.

## 9. Binding surfaces

**WASM** (`quillmark-wasm`) mirrors `Document` and `Card`:

```typescript
class Document {
  static fromMarkdown(markdown: string): Document;
  toMarkdown(): string;

  readonly quillRef: string;
  readonly frontmatter: Record<string, unknown>;
  readonly body: string;
  readonly cards: Card[];
  readonly warnings: Diagnostic[];

  setField(name: string, value: unknown): void;
  removeField(name: string): void;
  setQuillRef(ref: string): void;
  replaceBody(body: string): void;
  pushCard(card: Card): void;
  insertCard(index: number, card: Card): void;
  removeCard(index: number): void;
  moveCard(from: number, to: number): void;
  updateCardField(index: number, name: string, value: unknown): void;
  updateCardBody(index: number, body: string): void;
}

interface Card { tag: string; fields: Record<string, unknown>; body: string; }

class Quill {
  projectForm(doc: Document): FormProjection;
  // render / open signatures unchanged; internally call to_plate_json().
}
```

**Python** (`quillmark-python`) mirrors the same shape with
`snake_case` names. `fields` / `get_field` are removed; callers use
`frontmatter`, `body`, `cards`.

Both bindings pass `Document` through to `render` / `open`
unchanged — internally those call `to_plate_json()`, never round-trip
through Markdown.

## 10. Rollout

Five PRs, sequenced:

1. **Module split.** Move code from `parse.rs` into
   `document/{fences, sentinel, assemble, limits}.rs`. No behavior
   change; tests pass unchanged.
2. **Reshape `Document`.** Rename `ParsedDocument` → `Document`
   (remove the old name). Replace `fields: HashMap<String, QuillValue>`
   with typed `frontmatter` (`IndexMap`), `body`, `cards: Vec<Card>`.
   Add `to_plate_json()`. Migrate internal consumers
   (validation.rs, config.rs, typst glue, workflow.rs). Rewrite
   bindings.
3. **Editor surface.** `document::edit` + methods on `Document` and
   `Card`. Unit tests per invariant.
4. **Emitter.** Spike: can `serde-saphyr::SerializerOptions` force
   every string to double-quoted, or do we need the `ForceQuoted`
   newtype wrapper? Then `document::emit` and `Document::to_markdown`.
   Tests: type-fidelity round-trip against the fixture corpus; an
   ambiguous-strings regression set (`"on"`, `"off"`, `"true"`,
   `"null"`, `"01234"`, `"1e10"`, `"2024-01-15"`, empty, whitespace,
   `\n`, `"`, `\`); emit-idempotence.
5. **Form projection.** `Quill::project_form` + `FormProjection`.

Bindings (WASM, Python) re-export as each step lands.

## 11. Open questions

- **Nested map ordering.** `IndexMap` at the top level gives
  deterministic order. For nested `QuillValue::Object`, preserving
  author order needs a custom serde flow; alternative is sorting
  keys on emit (still deterministic). Defer to step 4 — the spike
  will surface whether nested-order preservation is free.
- **Empty containers.** Render empty map as `key:`, empty sequence
  as `key: []`, or skip entirely? Type fidelity holds either way;
  it's a diff-aesthetics call. Defer to step 4.
- **Comments.** Lost on round-trip per §5.4. No consumer blocks on
  this today. If a future requirement surfaces, layout-preserving
  round-trip is a separate proposal.
