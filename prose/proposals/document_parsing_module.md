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
- **YAML scalar emission:** use `serde_yaml` (or equivalent) with
  settings tuned to avoid the Norway problem and to always quote
  strings that would otherwise be parsed as non-strings. A shared
  `emit::yaml::emit_mapping` helper is the single choke point; no
  ad-hoc YAML string building elsewhere.

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

### 7.2 Cards: array-in-fields vs. first-class field

Today `CARDS` lives inside `fields` as a `QuillValue::Array`. That made
sense when `ParsedDocument` was a read-only JSON-ish dump. Once we have
a `Card` type and a typed editor, the in-`fields` copy is redundant and
easy to desynchronize.

Options:

- **Keep both, derived.** `Document` stores `cards: Vec<Card>`
  separately; `fields()` synthesizes the `CARDS` entry on demand for
  backend/template consumers. Editor mutates the typed side only.
  Backend sees no change.
- **Drop the copy.** `fields()` no longer contains `CARDS`; callers
  use `cards()` explicitly. Backends and templates need updating.

First option is the safe default. Second option is cleaner long-term.
**Recommendation:** first option, with a note to revisit once we're
sure no external consumer relies on `fields().get("CARDS")`.

The same argument applies to `BODY` living inside `fields`.

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

- **`Document` rename.** Keep the name `ParsedDocument` for compat, or
  rename to `Document` with an alias? I lean toward the rename — the
  type's role is changing, and the old name will actively mislead.
- **`CARDS` / `BODY` derived vs. stored.** §7.2 leaves this open. Worth
  deciding before step 3 since the editor API differs slightly.
- **YAML emitter choice.** `serde_yaml` is the obvious default but has
  known issues around booleans-as-strings. `serde_saphyr` (already in
  the parse path) may or may not support emission; worth a half-day
  spike before step 4.
- **Canonical ordering of nested mappings.** Do we require `IndexMap`
  all the way down, or only at the top level? Preserving order in
  nested maps requires a custom serde pipeline; the alternative is
  sorted emission for nested maps (still deterministic, but not
  author-visible order).
- **Comments.** Confirmed lost on round-trip. Is there any consumer
  where this is a blocker? If so, layout-preserving round-trip (the
  option we rejected) re-enters the discussion.
