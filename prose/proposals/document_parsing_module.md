# Proposal: Canonical Document Model and Modular Parsing

**Status:** Draft — design only, not scheduled.
**Author:** (session proposal)
**Scope:** `quillmark-core`, `quillmark-wasm`. No backend changes. No schema-model changes.

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

**`ParsedDocument` becomes a canonical in-memory document representation
— not a "parsed markdown payload".** Markdown is one import format and
one export format; the structure is primary. Everything else in this
proposal follows from that rename of intent.

Read this as the new contract:

> A `Document` is a typed, in-memory Quillmark document: a quill
> reference, a set of frontmatter fields, a body, and an ordered list
> of cards. It can be built from Markdown, mutated through a typed
> editor surface, and serialized back to Markdown. The Markdown
> emitted is canonical: semantically equivalent to any Markdown that
> would parse to the same `Document`, byte-equal across round-trips of
> a `Document` that was constructed without re-parsing.

The type may keep the name `ParsedDocument` for binary compatibility, or
we rename it to `Document` and keep `ParsedDocument` as a type alias for
one release. That's a call to make during implementation, not here.

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

### 7.2 Cards: array-in-fields vs. first-class field — RESOLVED

Today `CARDS` lives inside `fields` as a `QuillValue::Array` and `BODY`
as a `QuillValue::String`. Audit shows these are not just convention —
real callers depend on the shape:

- `crates/backends/typst/src/lib.rs:227` reads `result.get("CARDS")`
  to run card-field transformation.
- `crates/backends/typst/src/lib.typ.template:61-88` reads
  `d.at("CARDS")` at runtime inside the Typst plate.
- `crates/core/src/quill/validation.rs`, `crates/core/src/quill/config.rs`
  both consume `CARDS` from `fields` during validation and coercion.
- `crates/bindings/python/src/types.rs:366-371` exposes `fields` as a
  `PyDict` to Python consumers — removing `CARDS`/`BODY` there is a
  visible API break.

**Resolution: commit to "keep derived."** `Document` stores
`cards: Vec<Card>` and `body: String` as first-class fields. `fields()`
returns a view that synthesizes `CARDS` (from `cards`) and `BODY` (from
`body`) on access, so backends, templates, validation, and Python
bindings are unchanged. The editor mutates only the typed sides; there
is no possible desync because the view is derived, not cached.

If `fields()` returning a synthesized `IndexMap` on every call is a
performance concern in the render hot path, we can memoize it behind
`&self` with interior mutability or expose a separate `fields_view()`
method that takes `&self` and returns `impl Iterator`. That's an
implementation detail, not a design question.

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
`get_field()`, `fields()` (returning `PyDict`), and `warnings`.

The proposal mirrors the WASM surface here:

```python
class ParsedDocument:
    @staticmethod
    def from_markdown(markdown: str) -> ParsedDocument: ...
    def to_markdown(self) -> str: ...

    # read (existing)
    quill_ref: str
    fields: dict[str, Any]
    body: str
    # new
    cards: list[Card]
    warnings: list[Diagnostic]

    # write (new)
    def set_field(self, name: str, value: Any) -> None: ...
    def remove_field(self, name: str) -> None: ...
    def set_quill_ref(self, ref: str) -> None: ...
    def replace_body(self, body: str) -> None: ...
    def push_card(self, card: Card) -> None: ...
    def insert_card(self, index: int, card: Card) -> None: ...
    def remove_card(self, index: int) -> None: ...
    def move_card(self, from_: int, to: int) -> None: ...

class Quill:
    def project_form(self, doc: ParsedDocument) -> FormProjection: ...
```

Errors raise `ValueError` (existing `PyO3` convention for this crate).
`fields` stays a `PyDict` view including synthesized `CARDS`/`BODY` per
§7.2. Python is not a form-editor host today, but mirroring the surface
keeps the two bindings from drifting and costs little beyond a PyO3
method wrapper per method.

## 9. WASM surface

Every public `Document` method and `Quill::project_form` gets a WASM
wrapper. Concretely:

```typescript
class ParsedDocument {
  static fromMarkdown(markdown: string): ParsedDocument;
  toMarkdown(): string;

  // read
  readonly quillRef: string;
  readonly fields: Record<string, unknown>;    // synthesized, includes CARDS/BODY
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

class Quill {
  // existing: render, open
  projectForm(doc: ParsedDocument): FormProjection;
}

interface FormProjection { /* mirrors the Rust type */ }
```

Design notes:

- Everything that can fail throws a structured error (reuse existing
  `WasmError` path; add `EditError` variants).
- `fields` stays a plain object (a `Record`) for the read path because
  consumers already depend on that shape. Writes go through typed
  methods only — we do **not** make `fields` a `Proxy` that intercepts
  assignments.
- `Card` on the JS side is a plain value object; `pushCard` takes a
  `{ tag, fields, body }` and validates. No opaque JS handle needed.
- The `parsed: ParsedDocument` arguments to `Quill.render` and
  `Quill.open` continue to work unchanged — they call `toMarkdown` (or
  an internal equivalent) and feed the result through the existing
  pipeline. TBD whether we keep going through the markdown round-trip
  or pass the typed document straight through; former is simpler for
  the initial implementation, latter avoids a round-trip per render.

## 10. Migration

This is a non-breaking change for Rust consumers **if** we choose (b)
in §7.1 and keep `BODY`/`CARDS` in `fields` as derived views (§7.2
first option). The `ParsedDocument::from_markdown` entry point keeps
working; new capabilities arrive as additional methods.

For WASM consumers, all current call sites (`ParsedDocument.fromMarkdown`
+ `quill.render(parsed, opts)`) keep working. New methods appear as
additions. The form editor switches off its hand-rolled YAML path at
its own pace.

## 11. Rollout plan (implementation order — not this session)

1. **Module split without behavior change.** Move code from `parse.rs`
   into `document/fences.rs`, `document/sentinel.rs`,
   `document/assemble.rs`, `document/limits.rs`. All existing tests
   pass unchanged. `parse` remains a re-export facade.
2. **Switch `fields` to `IndexMap`.** Update call sites. Behavior
   unchanged except iteration order is now stable.
3. **Typed `Card` and editor surface.** Add `document::edit` plus the
   typed methods on `Document`. Unit tests around every invariant.
4. **Emitter.** Add `document::emit` and `Document::to_markdown`.
   Golden-file tests using the fixture corpus: `from_markdown` +
   `to_markdown` + `from_markdown` round-trip returns an equal
   `Document`. `to_markdown` idempotence: emit twice, compare bytes.
5. **Form projection.** Add `Quill::project_form` and `FormProjection`.
   Tests covering: all-fields-present, all-defaults, partial-defaults,
   validation-error cases.
6. **WASM surface.** Wrap everything in `bindings/wasm`. TypeScript
   declarations regenerated. Add a JS integration test that does
   parse → mutate → emit → parse.
7. **Form editor cutover.** Downstream replaces its hand-rolled YAML
   emission with `toMarkdown`. Out of scope for this repo.

Each step is a reviewable PR. Steps 1 and 2 land behind no feature
flag. Step 3 onward can land incrementally as new public API.

## 12. Open questions

Resolved during sanity-check:

- ~~`CARDS` / `BODY` derived vs. stored~~ — resolved in §7.2 (derived,
  mandatory, driven by typst backend + Python bindings consumer audit).
- ~~YAML emitter choice~~ — escalated to §5.2.1 as a mandatory
  pre-implementation spike.

Still open:

- **`Document` rename.** Keep the name `ParsedDocument` for compat, or
  rename to `Document` with a deprecated alias? I lean toward the
  rename — the type's role is changing and the old name will actively
  mislead — but this is cosmetic. Safe to defer to step 1 of rollout.
- **Nested map ordering.** `IndexMap` at the top level is confirmed in
  §7.1. For nested `QuillValue::Object` mappings, preserving author
  order requires a custom serde flow. Alternative: sort keys on emit
  (still deterministic, satisfies §5.1 contracts, but visible in
  diffs). Defer to step 4 — the emitter spike will surface whether
  nested-order preservation is free or expensive.
- **Render path: round-trip or passthrough.** `Quill::render(parsed)`
  could either (a) call `parsed.to_markdown()` and re-parse, or (b)
  feed the typed `Document` to `Workflow::compile_data` directly.
  Option (b) avoids a round-trip per render; option (a) is simpler and
  makes the parse boundary the single validation gate. Defer to step 6.
- **Comments.** Confirmed lost on round-trip per §5.3. No known
  consumer blocks on this today. If a future requirement surfaces,
  layout-preserving round-trip (§2 non-goal) comes back on the table
  and this proposal would need a follow-up.

## 13. Call-site inventory for the implementation team

Locations that touch the surfaces this proposal changes, so the
implementation team can sweep them without re-grepping:

**`ParsedDocument::fields()` returning `HashMap`** (will become
`IndexMap`, ~11 call sites):

- `crates/core/src/normalize.rs:500`
- `crates/core/src/quill/render.rs:96, 116`
- `crates/quillmark/src/orchestration/workflow.rs:40, 195, 223`
- `crates/bindings/python/src/types.rs:366-371`
- `crates/fuzz/src/parse_fuzz.rs:21, 82, 143`
- `crates/core/src/parse.rs:826, 1613` (tests)

**`"CARDS"` / `"BODY"` string keys** that must keep working through
the derivation view (§7.2):

- `crates/backends/typst/src/lib.rs:227, 231, 418, 428, 438, 471, 477, 483, 521`
- `crates/backends/typst/src/lib.typ.template:61, 63, 88`
- `crates/core/src/quill/validation.rs` (validation pulls `CARDS`)
- `crates/core/src/quill/config.rs` (coerce pulls `CARDS`)

**Existing `ParsedDocument` public API** (must remain callable):

- `ParsedDocument::from_markdown`, `::from_markdown_with_warnings`,
  `::new`, `::quill_reference`, `::body`, `::get_field`, `::fields`,
  `::with_defaults` (all in `crates/core/src/parse.rs`).

**Bindings entry points** that construct `ParsedDocument`:

- `crates/bindings/wasm/src/engine.rs:76` (`parse_markdown_impl`)
- `crates/bindings/python/src/types.rs:328` (`from_markdown`)
- `crates/bindings/cli/src/commands/render.rs` (calls
  `from_markdown_with_warnings`, parse-only — no editor surface needed
  in the CLI).
