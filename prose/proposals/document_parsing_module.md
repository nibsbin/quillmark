# Proposal: Canonical Document Model and Modular Parsing

**Status:** Draft — design only, not scheduled.
**Author:** (session proposal)
**Scope:** `quillmark-core`, `quillmark-wasm`, `quillmark-python`. Backend Rust glue in `quillmark-typst` is refactored to consume the typed `Document` (§7.2); the plate wire format and the `.typ.template` are unchanged. No schema-model changes.

---

## 1. Problem

Downstream consumers of `@quillmark/wasm` — including the form editor —
treat Markdown as the canonical storage format. To edit a field, add a
card, or reorder cards, they parse to JSON, mutate, and then reconstruct
the Markdown by hand. That round-trip is where correctness breaks:

- YAML emission is non-trivial (quoting, multiline scalars, escaping,
  booleans-as-strings, Norway problem, empty mappings).
- Metadata-fence reconstruction is non-trivial (leading blank rule,
  closer alignment, CARD ordering, body boundaries).
- There is no shared library code for any of it. Each consumer reinvents
  it. The form editor's implementation is "not robust" because this
  reinvention has no home.

The root cause is that `quillmark-core` exposes only a one-way parser.
`ParsedDocument` is a read-only inspection surface with a single
constructor (`from_markdown`). There is no inverse, no editor, and no
schema-aware projection that a form UI can consume directly. The
2,925-line `parse.rs` monolith makes it hard to share intermediate
stages between the current parser and a would-be emitter.

## 2. Goal

Make **`quillmark-core`** the single source of truth for reading,
editing, and writing Quillmark Markdown documents. Re-export the same
surface through **`quillmark-wasm`** so downstream consumers never
touch YAML strings or fence markers directly.

Non-goals:

- Preserving the author's original formatting, comments, key order,
  anchors, or tags on re-emit. This proposal commits to **canonical
  re-emission** (see §5).
- Changing the Quillmark Markdown standard itself. The standard is
  defined in `MARKDOWN.md`; this proposal is purely about how we
  consume and produce documents that conform to it.
- Changing the `QuillConfig` / schema model. Coercion, validation, and
  defaults stay where they are.

## 3. Core idea

**The canonical type is `Document`.** It is a typed, in-memory
Quillmark document — not a "parsed markdown payload". Markdown is one
import format and one export format; the structure is primary.
Everything else in this proposal follows.

Read this as the new contract:

> A `Document` is a typed, in-memory Quillmark document: a quill
> reference, a set of frontmatter fields, a body, and an ordered list
> of cards. It can be built from Markdown, mutated through a typed
> editor surface, and serialized back to Markdown. The Markdown
> emitted is canonical: semantically equivalent to any Markdown that
> would parse to the same `Document`, byte-equal across round-trips of
> a `Document` that was constructed without re-parsing.

**Naming: we rename `ParsedDocument` → `Document`.** `ParsedDocument`
remains as a `#[deprecated]` type alias for one release so downstream
consumers can upgrade at their own pace, then is removed.

## 4. Module layout

Replace the single `parse.rs` with a module tree under
`quillmark-core::document`. Each submodule is independently usable by
Rust consumers and has a single job.

```
quillmark_core::document
├── mod.rs           // public Document / DocumentBuilder / errors
├── fences.rs        // line-oriented fence scanner → Vec<RawBlock>
├── sentinel.rs      // QUILL / CARD extraction, reserved-name checks
├── assemble.rs      // RawBlock[] → Document
├── emit.rs          // Document → String (canonical Markdown)
├── edit.rs          // typed mutators on Document
└── limits.rs        // size/depth/count constants (from error.rs today)
```

The existing `parse` module becomes a thin compatibility shim:

```rust
// crates/core/src/parse.rs
pub use crate::document::{Document as ParsedDocument, ParseOutput, BODY_FIELD};
```

Why split it this way:

- **`fences`** is pure layout scanning (no YAML knowledge). Today it's
  `find_metadata_blocks` + the `Lines` helper + `code_fence_on_line`.
  Isolating it makes the fence rules testable in isolation and reusable
  by the emitter (which needs to know what shapes count as "legal" to
  produce).
- **`sentinel`** owns the QUILL / CARD rules, reserved-name rejection
  (BODY, CARDS), and tag-name validation. Currently scattered across
  `extract_sentinels`, `is_valid_tag_name`, and inline checks in
  `decompose_with_warnings`.
- **`assemble`** is the current `decompose_with_warnings` body minus
  the fence scanning and sentinel extraction — just the "stitch
  RawBlocks and YAML values into a `Document`" step.
- **`emit`** is new. See §5.
- **`edit`** is new. See §6.

This split is mechanical relative to the current code: each submodule
already exists as a contiguous section of `parse.rs`.

## 5. Canonical emission

### 5.1 Contract

`Document::to_markdown() -> String` produces Markdown such that:

1. **Parse-idempotent:** `Document::from_markdown(doc.to_markdown())`
   returns a `Document` equal to `doc` (by value).
2. **Emit-idempotent:** `doc.to_markdown()` is a pure function of
   `doc`. Calling it twice returns byte-equal strings.
3. **Round-trip stable on pure-editor paths:** if a `Document` was
   produced by `from_markdown` followed only by `edit::*` mutators,
   `to_markdown` produces a string that survives another full
   round-trip unchanged.

Notably, **we do not guarantee byte-equality with the original source.**
A document parsed from human-authored Markdown may be re-emitted with
different whitespace, quoting, key order, or comment removal. This is
the explicit tradeoff accepted in §2.

### 5.2 Canonical form

For the initial version the emitter applies these rules. Each rule is
deliberately conservative and can be tightened later without breaking
the contract above.

- **Line endings:** `\n`. CRLF is normalized on import.
- **Frontmatter block:**
  - Opener `---\n` at byte 0.
  - `QUILL: <name>[@<selector>]` on the first content line, unquoted
    when the grammar allows (it always does for valid quill refs).
  - Remaining fields in **insertion order** (`HashMap` becomes
    `IndexMap` — see §7.1). This is the "canonicalization" contract:
    the emitter does not sort, but it does honor insertion order
    deterministically.
  - Closer `---\n` followed by one blank line.
- **Body:** emitted verbatim. `BODY` is a `String`; what's stored is
  what's written. The parser already guarantees `BODY` begins with a
  newline when non-empty, per current behavior.
- **Cards:** each card is emitted as `---\nCARD: <name>\n<fields>\n---\n<body>`
  in the order they appear in the `CARDS` array. A single blank line
  precedes each opener to satisfy the F2 leading-blank rule.
- **YAML scalar emission:** handled by a single
  `emit::yaml::emit_mapping` helper — no ad-hoc YAML string building
  elsewhere. Emitter implementation is an open question; see §5.2.1.

### 5.2.1 YAML emitter choice — pre-implementation spike

The workspace today uses `serde-saphyr` for YAML **parsing only** (the
crate is parse-side). There is no emitter in the dependency tree. This
is a concrete technical risk, not a footnote:

- `serde_yaml` is archived / unmaintained upstream but still functional
  and widely used; known issues around booleans-as-strings (the
  "Norway problem"), `y` / `yes` / `on` as booleans, and unquoted
  numeric-looking strings. All tractable with `Tag::force_quote` or
  equivalent, but every rule has to be verified against our
  round-trip test corpus.
- `saphyr-emitter` (companion to `saphyr-parser`) exists but is
  separately versioned and may not match the parse dialect
  `serde-saphyr` accepts. Symmetry with the parser is desirable but
  not required — the contract is round-trip equality of `Document`,
  not byte-equality of YAML.
- Hand-rolling emission against the Quillmark schema is feasible
  because frontmatter values are constrained (`QuillValue`, no anchors
  or tags survive round-trip, §5.3). Only needed as a fallback.

**Mandatory spike before step 4:** verify that the chosen emitter
round-trips the full fixture corpus under the §5.1 contract. Concretely:

1. Pick one of the three candidates above.
2. For every `.md` fixture in `crates/fixtures`, run
   `from_markdown → to_markdown → from_markdown` and assert the two
   `Document` values are equal.
3. For every failure, classify as (a) legitimate information loss we
   accept (comments, custom tags per §5.3), or (b) an emitter bug that
   blocks this proposal.

If the chosen emitter has blocker-class bugs, escalate before starting
step 4 — the emitter choice constrains the canonical-form rules in
§5.2, and switching later is expensive.

### 5.3 What survives

| Input trait                                      | After round-trip |
|--------------------------------------------------|------------------|
| Field values (strings, numbers, arrays, objects) | Preserved by value |
| Insertion order of frontmatter fields            | Preserved |
| CARD ordering                                    | Preserved |
| Body text (including interior `---` not on fences) | Byte-preserved |
| YAML comments                                    | **Lost** |
| Custom YAML tags (e.g., `!fill`)                 | **Lost** (value preserved per `QuillValue`) |
| Explicit quoting style                           | Normalized |
| Key order within nested mappings                 | Preserved if we switch nested maps to `IndexMap` too; otherwise lost |

Documented on the `to_markdown` rustdoc with a pointer to this section.

## 6. Editor surface

The editor is a set of **free functions on `Document`** (or methods,
style TBD), each of which enforces the same invariants the parser
enforces. Invariants are the non-negotiable part; the exact method
shape is open to bikeshed.

### 6.1 Frontmatter

```rust
impl Document {
    pub fn set_field(&mut self, name: &str, value: QuillValue) -> Result<(), EditError>;
    pub fn remove_field(&mut self, name: &str) -> Option<QuillValue>;
    pub fn set_quill_ref(&mut self, reference: QuillReference);
    pub fn replace_body(&mut self, body: impl Into<String>);
}
```

`set_field` rejects reserved names (`BODY`, `CARDS`, `QUILL`, `CARD`)
and rejects keys that do not match the Quillmark field-name grammar
(`[a-z_][a-z0-9_]*`, NFC-normalized — same rule the normalizer applies).
`set_quill_ref` takes a pre-validated `QuillReference` so the grammar
check happens at parse time at the edge.

### 6.2 Cards

```rust
impl Document {
    pub fn cards(&self) -> &[Card];
    pub fn card(&self, index: usize) -> Option<&Card>;
    pub fn card_mut(&mut self, index: usize) -> Option<&mut Card>;
    pub fn push_card(&mut self, card: Card) -> Result<(), EditError>;
    pub fn insert_card(&mut self, index: usize, card: Card) -> Result<(), EditError>;
    pub fn remove_card(&mut self, index: usize) -> Option<Card>;
    pub fn move_card(&mut self, from: usize, to: usize) -> Result<(), EditError>;
}

pub struct Card { /* tag, fields (IndexMap), body */ }
impl Card {
    pub fn new(tag: impl Into<String>) -> Result<Self, EditError>;  // validates tag name
    pub fn set_field(&mut self, name: &str, value: QuillValue) -> Result<(), EditError>;
    pub fn remove_field(&mut self, name: &str) -> Option<QuillValue>;
    pub fn set_body(&mut self, body: impl Into<String>);
}
```

`Card` is the typed view of an entry in the `CARDS` array. Internally
the `CARDS` entry in `fields` is still a `QuillValue::Array` — see §7.2
for whether we keep that redundancy or promote cards to a first-class
field on `Document`.

### 6.3 Errors

A single `EditError` enum with variants for each invariant (reserved
name, invalid name, quill-ref parse error, card index out of range).
`EditError` is distinct from `ParseError`; editor errors do not
produce parse-style diagnostics with source locations because there is
no source at that point.

## 7. Internal changes that fall out

### 7.1 `HashMap<String, QuillValue>` → `IndexMap<String, QuillValue>`

`fields` becomes an `IndexMap` so insertion order is preserved through
edits and emission. This is a breaking change to the return type of
`ParsedDocument::fields()`. We can either:

- (a) change the return type to `&IndexMap<_, _>` and let consumers
  upgrade (both maps iterate as `(&K, &V)`), or
- (b) introduce a new accessor `fields_ordered()` and leave the old one
  returning a `HashMap` view built on demand.

(a) is cleaner and the rest of the crate already treats the map as
opaque. I'd default to (a) unless we find a consumer that relies on the
concrete type.

### 7.2 Two layers: internal model vs. plate wire format

Today `CARDS` lives inside `fields` as a `QuillValue::Array` and `BODY`
as a `QuillValue::String`. An audit of consumers shows the string keys
are doing two very different jobs, which the first draft of this
proposal conflated.

**Layer A — Rust-side internal consumers.** These read `CARDS` / `BODY`
as string keys out of a `HashMap<String, QuillValue>` because the
underlying representation *is* a stringly-typed map. They are
historical accidents, not contracts:

| Site | Pattern | Simpler if typed |
|---|---|---|
| `core/src/quill/validation.rs:50` | `fields.get("CARDS").and_then(as_array)` | `for card in doc.cards()` |
| `core/src/quill/config.rs:173` | Same | `doc.cards()` |
| `backends/typst/src/lib.rs:227` | Same (Rust glue before JSON serialization) | `doc.cards()` / `doc.body()` |
| `bindings/python/src/types.rs` | `fields` exposes CARDS/BODY to Python via PyDict | `doc.cards` / `doc.body` typed accessors |

All four branch on "what if CARDS isn't an array?" — a case that can't
arise once cards are typed.

**Layer B — the plate data contract.** This is the JSON payload
backends receive at render time:

- `backends/typst/src/lib.typ.template:61-88` reads `d.at("CARDS")` at
  Typst runtime, and `d.at("BODY")` elsewhere. Plate authors code
  against this shape.

Layer B is a genuine public API — the wire format that crosses the
Rust ↔ backend boundary. It does **not** follow the internal model.

### Resolution

Draw the line between the two layers explicitly.

**Internal model (Layer A) goes typed.** `Document` stores:

```rust
pub struct Document {
    quill_ref: QuillReference,
    frontmatter: IndexMap<String, QuillValue>,  // no CARDS, no BODY
    body: String,
    cards: Vec<Card>,
}
```

Accessors: `doc.quill_ref()`, `doc.frontmatter()`, `doc.body()`,
`doc.cards()`, plus the editor methods from §6.

Consumers migrate:

- `validation.rs` iterates `doc.cards()` directly. The type-mismatch
  branch ("CARDS must be an array") is deleted.
- `config.rs` coerce walks `doc.cards()`. Same branch deleted.
- `typst/src/lib.rs` Rust glue calls `doc.cards()` / `doc.body()` /
  `doc.frontmatter()` and assembles the plate JSON from those.
- Python `fields` becomes the frontmatter dict only (no CARDS/BODY).
  Python gains typed `cards` and `body` accessors (Python already has
  `body()` — good; we make it consistent and add `cards`).

**Plate wire format (Layer B) stays as-is, explicitly.** A new method
on `Document`:

```rust
impl Document {
    /// Serialize this document into the JSON payload format expected
    /// by backend plates. The payload is the wire contract authors
    /// code against: `CARDS` and `BODY` appear as top-level keys.
    pub fn to_plate_json(&self) -> serde_json::Value;
}
```

`Workflow::compile_data` calls `to_plate_json()` instead of manually
assembling a map from `fields()`. The typst Rust glue and the
`.typ.template` are unchanged because the JSON they see is unchanged.

### Consequence: no derived-`fields()` method

The first draft proposed a synthesized `fields()` view that would
include `CARDS` and `BODY` for backward compatibility. That's dropped.
The clearer split is:

- `frontmatter()` — typed internal map, frontmatter only.
- `to_plate_json()` — wire-format JSON, includes `CARDS`/`BODY`.

Callers that want the flat map shape today get it from
`to_plate_json()` (plus `.as_object()`). Callers that want frontmatter
get `frontmatter()`. No single accessor does both badly.

### Migration cost

Concrete, bounded. This is larger than the first draft's "keep
derived" resolution, but cleaner:

- `crates/core/src/quill/validation.rs` — one function (`validate_document`)
  rewritten to iterate typed cards. Simpler, fewer error paths.
- `crates/core/src/quill/config.rs` — one function (`coerce`)
  rewritten the same way.
- `crates/backends/typst/src/lib.rs:189-235` — `transform_markdown_fields`
  takes `&Document` (or structured pieces) instead of `HashMap`, and
  builds the plate JSON at the end. Scope: one file.
- `crates/quillmark/src/orchestration/workflow.rs:164-170`
  (`fields_to_json`) — replaced by `doc.to_plate_json()`. Deletion.
- `crates/bindings/python/src/types.rs:366-371` — `fields` property
  changes shape. **Breaking change for Python consumers.** Document
  in the crate's changelog; Python minor version bump. Mitigation: we
  can keep `fields` returning CARDS/BODY for one release with a
  deprecation warning and make it opt-in to the new shape, but I'd
  rather do the clean break at the same time as the `Document`
  rename.
- `crates/bindings/wasm/src/engine.rs` — `parsed_document_impl` and
  `to_core_parsed` no longer round-trip through a JSON `fields`
  object; they ferry typed `Document`. WASM bindings expose
  `frontmatter` and `cards` directly per §9.

## 8. Schema-aware form projection

New method on `Quill` (not `Document`):

```rust
impl Quill {
    pub fn project_form(&self, doc: &Document) -> FormProjection;
}

pub struct FormProjection {
    pub main: FormCard,         // fields from main schema, merged with doc
    pub cards: Vec<FormCard>,   // one entry per CARD in doc, keyed to card schema
    pub diagnostics: Vec<Diagnostic>,  // validation errors/warnings at projection time
}

pub struct FormCard {
    pub schema: CardSchema,      // from QuillConfig
    pub values: IndexMap<String, FormFieldValue>,
}

pub struct FormFieldValue {
    pub value: Option<QuillValue>,    // current value from doc, if any
    pub default: Option<QuillValue>,  // schema default, if any
    pub source: FormFieldSource,      // Document | Default | Missing
}
```

`FormProjection` is the view a form editor actually wants: for every
field in the schema, what's the current value, what's the default,
what's missing, and what validation errors exist. It's a read-only
projection; the form editor mutates the `Document` directly and can
re-project whenever it wants.

The implementation piggybacks on the existing `QuillConfig::coerce`,
`defaults`, and `validate` — it just packages their outputs in a
shape a form consumer can render without writing its own merge logic.
`Workflow` continues to call those functions directly for rendering; it
does not need `FormProjection`.

## 8.1 Python surface

`quillmark-python` (`crates/bindings/python/src/types.rs`) currently
exposes `ParsedDocument` as read-only: `from_markdown`, `body()`,
`get_field()`, `fields()` (returning `PyDict` including CARDS/BODY),
and `warnings`.

The new Python surface:

```python
class Document:  # renamed from ParsedDocument
    @staticmethod
    def from_markdown(markdown: str) -> Document: ...
    def to_markdown(self) -> str: ...

    # read
    quill_ref: str
    frontmatter: dict[str, Any]       # frontmatter only; no CARDS, no BODY
    body: str                         # typed, was get_field("BODY")
    cards: list[Card]                 # typed, was get_field("CARDS")
    warnings: list[Diagnostic]

    # write
    def set_field(self, name: str, value: Any) -> None: ...
    def remove_field(self, name: str) -> None: ...
    def set_quill_ref(self, ref: str) -> None: ...
    def replace_body(self, body: str) -> None: ...
    def push_card(self, card: Card) -> None: ...
    def insert_card(self, index: int, card: Card) -> None: ...
    def remove_card(self, index: int) -> None: ...
    def move_card(self, from_: int, to: int) -> None: ...

class Quill:
    def project_form(self, doc: Document) -> FormProjection: ...
```

Errors raise `ValueError` (existing PyO3 convention for this crate).
Note the **breaking change to `fields`**: it is renamed `frontmatter`
and no longer includes `CARDS` / `BODY`. Python consumers reading
`doc.fields["BODY"]` migrate to `doc.body`; `doc.fields["CARDS"]` to
`doc.cards`. Python minor-version bump; documented in the crate's
changelog alongside the `Document` rename.

## 9. WASM surface

Every public `Document` method and `Quill::project_form` gets a WASM
wrapper. Concretely:

```typescript
class Document {                        // renamed from ParsedDocument
  static fromMarkdown(markdown: string): Document;
  toMarkdown(): string;

  // read — frontmatter, body, cards are separate surfaces
  readonly quillRef: string;
  readonly frontmatter: Record<string, unknown>;  // no CARDS, no BODY
  readonly body: string;
  readonly cards: Card[];

  // write
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

interface Card {
  tag: string;                           // e.g. "indorsement"
  fields: Record<string, unknown>;
  body: string;
}

class Quill {
  // existing: render, open — signatures unchanged, accept Document
  projectForm(doc: Document): FormProjection;
}

interface FormProjection { /* mirrors the Rust type */ }
```

Design notes:

- Everything that can fail throws a structured error (reuse existing
  `WasmError` path; add `EditError` variants).
- `frontmatter` is a plain read-only object. Writes go through typed
  methods only — we do **not** make it a `Proxy` that intercepts
  assignments.
- `Card` on the JS side is a plain value object; `pushCard` takes a
  `{ tag, fields, body }` and validates. No opaque JS handle needed.
- The existing `ParsedDocument` class name is kept as a deprecated
  alias in the WASM surface for one release. `ParsedDocument.fromMarkdown`
  returns a `Document`; existing `fields` access on it is removed
  because we can't ship a half-typed shim that doesn't match either
  old or new — see §10 for why.
- The `parsed: Document` arguments to `Quill.render` and `Quill.open`
  continue to work unchanged — internally they call `to_plate_json()`
  (not `to_markdown`) and feed the result through the existing
  pipeline. No markdown round-trip at render time.

## 10. Migration

This release is **deliberately breaking** on the `fields()` surface.
The two-layer split in §7.2 is the headline change, and splitting it
across releases (keeping CARDS/BODY in `frontmatter` for one release,
then removing them) would ship a half-typed intermediate state that
matches neither old nor new consumers.

**Rust:**
- `Document::from_markdown` works unchanged.
- `Document::fields()` is removed. Callers use `frontmatter()`,
  `body()`, `cards()`, or `to_plate_json()` depending on intent.
- `ParsedDocument` remains as a `#[deprecated]` type alias for one
  release, with a compiler note pointing at `Document`.
- `with_defaults` stays on `Document` with the same signature.

**WASM:**
- `Document` is the primary class. `ParsedDocument` is kept as a
  deprecated alias of `Document` (via `#[wasm_bindgen(js_name = ...)]`
  or re-export).
- `.fields` on the returned object is removed. Consumers that read
  `fields.BODY` / `fields.CARDS` migrate to `.body` / `.cards`; others
  migrate to `.frontmatter`.
- `quill.render(doc)` and `quill.open(doc)` accept the new `Document`
  unchanged. The form editor does parse → mutate → `toMarkdown` (for
  storage) or parse → mutate → `quill.render` (for preview) without
  touching YAML strings.

**Python:**
- Same as WASM. Breaking changes to `Document.fields` → `frontmatter`
  + typed `cards` property. Minor version bump with a migration note
  in `docs/integration/python/api.md`.

**CLI:**
- Zero impact. `crates/bindings/cli` only calls `from_markdown_with_warnings`
  and hands the result to `Workflow`; no `fields()` reads.

The form editor cutover happens in a single step after this release
lands: delete its hand-rolled YAML emitter, call `Document.toMarkdown()`
and the typed editor methods instead. No staged rollout needed on the
consumer side.

## 11. Rollout plan (implementation order — not this session)

1. **Module split without behavior change.** Move code from `parse.rs`
   into `document/fences.rs`, `document/sentinel.rs`,
   `document/assemble.rs`, `document/limits.rs`. All existing tests
   pass unchanged. `parse` remains a re-export facade. No type changes.
2. **Rename `ParsedDocument` → `Document`; add deprecated alias.**
   Behavior unchanged. Internal callers migrate. This is a separable
   PR because the rename alone doesn't touch any field/card shape.
3. **Reshape `Document` into typed fields (`frontmatter`, `body`,
   `cards`).** This is the big one. Touches validation.rs, config.rs,
   typst/lib.rs, workflow.rs, Python bindings. Add `to_plate_json()`
   and remove the old `fields()`. All tests still green via the new
   accessors; no fixtures should change. **Land behind a single PR.**
4. **Switch `frontmatter` to `IndexMap`.** Small follow-up PR once the
   shape is right. Iteration order now stable for emission.
5. **Typed `Card` and editor surface.** Add `document::edit` plus the
   editor methods on `Document`. Unit tests around every invariant.
6. **Emitter spike + implementation.** Run §5.2.1 spike. Choose
   emitter. Add `document::emit` and `Document::to_markdown`.
   Golden-file tests using the fixture corpus: `from_markdown` →
   `to_markdown` → `from_markdown` returns an equal `Document`.
   `to_markdown` idempotence: emit twice, compare bytes.
7. **Form projection.** Add `Quill::project_form` and `FormProjection`.
   Tests: all-fields-present, all-defaults, partial-defaults,
   validation-error cases.
8. **WASM surface.** Wrap everything in `bindings/wasm`. TypeScript
   declarations regenerated. JS integration test: parse → mutate →
   emit → parse.
9. **Python surface.** Mirror in `bindings/python`. Breaking-change
   note in `docs/integration/python/api.md`.
10. **Form editor cutover.** Downstream replaces hand-rolled YAML
    emission with `toMarkdown`. Out of scope for this repo.

Each step is a reviewable PR. Steps 1–4 are sequenced; 5–9 can
interleave.

## 12. Open questions

Resolved:

- ~~`CARDS` / `BODY` derived vs. stored~~ — resolved in §7.2 via the
  Layer A / Layer B split. Internal model is typed; wire format keeps
  the keys explicitly via `to_plate_json()`.
- ~~`Document` rename~~ — confirmed. `ParsedDocument` becomes a
  deprecated alias.
- ~~Render path: round-trip or passthrough~~ — resolved. `Quill::render`
  calls `to_plate_json()` directly; no markdown round-trip at render
  time.
- ~~YAML emitter choice~~ — escalated to §5.2.1 as a mandatory
  pre-implementation spike.

Still open:

- **Nested map ordering.** `IndexMap` on `frontmatter` gives
  top-level deterministic order. For nested `QuillValue::Object`
  mappings, preserving author order requires a custom serde flow.
  Alternative: sort keys on emit (still deterministic, satisfies §5.1
  contracts, but visible in diffs). Defer to step 6 — the emitter
  spike will surface whether nested-order preservation is free or
  expensive.
- **Comments.** Confirmed lost on round-trip per §5.3. No known
  consumer blocks on this today. If a future requirement surfaces,
  layout-preserving round-trip (§2 non-goal) comes back on the table
  and this proposal would need a follow-up.
- **Python `get_field` fate.** Keep as a thin alias over
  `frontmatter.get(name)`, or remove? Keeping costs nothing and
  smooths migration; I lean toward keep-with-deprecation. Non-blocking.

## 13. Call-site inventory for the implementation team

Locations that touch the surfaces this proposal changes, grouped by
the step that sweeps them.

### Step 3 sweep — fields reshape (biggest PR)

**Layer A Rust consumers — migrate to typed accessors:**

- `crates/core/src/quill/validation.rs:50-103` — `validate_document`.
  Replace `fields.get("CARDS").and_then(as_array)` branching with
  iteration over `doc.cards()`. Delete the "CARDS is not an array"
  `ValidationError::TypeMismatch` arm.
- `crates/core/src/quill/config.rs:173-190` — coerce. Same pattern.
  Delete the matching `CoercionError::Uncoercible` arm.
- `crates/backends/typst/src/lib.rs:189-235`
  (`transform_markdown_fields`) — take `&Document` or a typed tuple
  of `(frontmatter, body, cards)`. Build plate JSON at the end.
- `crates/quillmark/src/orchestration/workflow.rs:40, 164-170, 195, 223` —
  `coerce(parsed.fields())` becomes direct typed dispatch; the
  `fields_to_json` helper is deleted in favor of
  `doc.to_plate_json()`.
- `crates/core/src/normalize.rs:500` — `normalize_document` walks
  `doc.fields().clone()`. Migrate to typed walk over frontmatter
  entries, `body`, and each card's body (the `CARDS` / `BODY`
  normalization loops are already split in `normalize_fields`; this
  is plumbing, not logic).
- `crates/core/src/parse.rs:826, 1613` and
  `crates/fuzz/src/parse_fuzz.rs:21, 82, 143` — tests asserting
  `fields().len()`. Rewrite against `frontmatter().len()` + explicit
  card / body checks.

**Python binding — breaking change:**

- `crates/bindings/python/src/types.rs:358-377` — remove `get_field`
  (or keep as a thin alias over `frontmatter`); rename `fields` →
  `frontmatter`; add `cards` property. `body()` stays.

**Layer B — does NOT change in this step:**

- `crates/backends/typst/src/lib.rs` all `result.get("CARDS" | "BODY")`
  sites that operate on JSON (post-`to_plate_json`) stay, because they
  are reading the wire format.
- `crates/backends/typst/src/lib.typ.template:61-88` stays — it reads
  the JSON payload at Typst runtime.

### Step 4 sweep — IndexMap

After step 3, the only map remaining on `Document` is `frontmatter`.
Change its type to `IndexMap<String, QuillValue>`. Impact:

- `crates/core/src/normalize.rs` — iteration order becomes stable.
- Tests asserting iteration order can be tightened.

### Steps 5–6 sweep — editor and emitter

New code in `crates/core/src/document/edit.rs` and
`crates/core/src/document/emit.rs`. No existing call-site edits.

### Step 8–9 sweep — bindings

**WASM entry points currently handling `ParsedDocument`:**

- `crates/bindings/wasm/src/engine.rs:76-113` (`parse_markdown_impl`,
  `to_core_parsed`). The JSON round-trip (`fields_obj`) is deleted;
  wrap the typed `Document` directly.
- `crates/bindings/wasm/src/types.rs:164-174` — `ParsedDocument` tsify
  struct. Replace `fields: serde_json::Value` with typed
  `frontmatter`, `body`, `cards`.

**Python entry point:**

- `crates/bindings/python/src/types.rs:319-377` — `PyParsedDocument`.
  Rename class to `Document`, add editor methods, match §8.1.

### Out of scope

- `crates/bindings/cli/src/commands/render.rs` — parse-only; no
  changes.
- All test fixture `.md` files under `crates/fixtures` — unchanged.
  Round-trip tests in step 6 consume them as inputs.
