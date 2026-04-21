# Plan: Canonical `Document` Model & Modular Parsing

**Tracks proposal:** [`prose/proposals/document_parsing_module.md`](../proposals/document_parsing_module.md)
**Scope:** `quillmark-core` (Rust), `quillmark-typst` Rust glue, `quillmark-wasm`, `quillmark-python`.
**Mode:** Pre-release — remove, do not deprecate. No compatibility shims.
**Sequencing:** Five PRs, merged in order. Each PR is green on its own and leaves the tree shippable.

---

## Ground rules that apply to every phase

- **One behavior-change per PR.** Refactors land without API changes; API changes land without emitter behavior; emitter behavior lands in its own PR.
- **No API drift between bindings and core.** Every public surface added to `Document` / `Card` / `Quill` gets mirrored in WASM and Python in the same PR that adds it. If a phase can't realistically touch a binding, call that out in the PR description.
- **Fixture corpus is authoritative.** `crates/fixtures/resources/` is the regression truth. Every phase runs the full existing fixture suite; nothing may regress. New fixtures land with the phase that motivates them.
- **No speculative extension points.** The proposal is explicit about v2-candidate features (block scalars, layout-preserving round-trip, nested-map ordering preservation). Do not write scaffolding for them.
- **Documentation updates ride with the PR.** Rustdoc, the WASM `README.md`, the Python `__init__.pyi`, and any affected pages in `prose/designs/` change in the same PR that changes behavior.

---

## Phase 1 — Module split (no behavior change)

**Goal:** turn `crates/core/src/parse.rs` (2,925 lines) into `crates/core/src/document/` with named responsibilities. Zero observable change.

### Work

1. Create `crates/core/src/document/` with:
   - `mod.rs` — re-exports today's public surface (`ParseOutput`, `ParsedDocument`, `BODY_FIELD`). The rename to `Document` is **Phase 2**, not here.
   - `fences.rs` — the `Lines` / `is_fence_marker_line` / `first_content_key` / fence-scanner block (roughly `parse.rs:193–400` range — confirm exact boundaries when splitting).
   - `sentinel.rs` — `is_valid_tag_name`, reserved-name constants, near-miss linting.
   - `assemble.rs` — `decompose`, `decompose_with_warnings`, `MetadataBlock`, and the top-level glue that stitches fences + sentinels into a `ParsedDocument`.
   - `limits.rs` — `MAX_YAML_DEPTH` (currently in `error.rs`) plus any other budget constants. `error.rs` re-exports what external callers already import.
2. `crates/core/src/lib.rs` replaces `pub mod parse;` with `pub mod document;`. Re-export the same three names (`ParseOutput`, `ParsedDocument`, `BODY_FIELD`) from `document::` so every downstream `use quillmark_core::ParsedDocument;` still compiles unchanged.
3. Delete `parse.rs` once all sections are migrated. Move the inline `#[cfg(test)] mod tests` blocks to `document/tests/` (one test file per submodule) rather than collapsing them into `assemble.rs`.
4. No changes to `quillmark-wasm`, `quillmark-python`, backends, or the orchestration layer beyond trivial import path updates if anything references `quillmark_core::parse::…` by internal path. Prefer keeping everything on the re-exports.

### Tests

- Full `cargo test --workspace` passes with zero modifications to test bodies.
- `cargo doc --workspace --no-deps` builds; every existing doctest in `parse.rs` survives (relocate into the new module's rustdoc).
- Manual diff review: no production code inside the module change behaves differently — strings, error messages, budgets, and diagnostic codes are byte-identical.

### Exit criteria

- `parse.rs` removed.
- `document/` tree present with the six files above.
- Workspace builds, tests pass, docs build.
- No binding-facing change; WASM and Python pass their test suites untouched.

### Risks

- **Accidental behavior drift.** Large mechanical moves are where subtle `pub`/`pub(crate)` and visibility leaks happen. Mitigation: run `cargo public-api` (or diff rustdoc JSON) before/after and attach the diff to the PR. It must be empty.
- **Test imports breaking.** Any test that does `use crate::parse::…` needs to switch to `use crate::document::…`. Grep before merging.

---

## Phase 2 — Reshape `Document` (typed internal model)

**Goal:** `ParsedDocument` → `Document`. Fields split into typed `frontmatter` / `body` / `cards`. Internal consumers migrate. Bindings mirror the new shape. `to_plate_json()` becomes the wire contract for the Typst plate layer.

This is the destructive phase. It is also the phase where we pay the most migration tax across the repo. Plan it carefully.

### Work

1. **Type definition** in `document/mod.rs`:
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
   Add `indexmap` to `crates/core/Cargo.toml` if not already present. `IndexMap` preserves insertion order for deterministic emit in Phase 4.
2. **`Document::from_markdown`** constructs the typed shape directly in `assemble.rs`. The current code path assembles a `HashMap` with `CARDS` and `BODY` string keys — replace that branch with explicit construction of `frontmatter` + `body` + `cards`. Delete every error arm that says `"CARDS is not an array"` / `"BODY is not a string"` — those cases are unrepresentable in the new type.
3. **Accessors:**
   ```rust
   pub fn quill_reference(&self) -> &QuillReference;
   pub fn frontmatter(&self) -> &IndexMap<String, QuillValue>;
   pub fn body(&self) -> &str;
   pub fn cards(&self) -> &[Card];
   pub fn warnings(&self) -> &[Diagnostic];
   ```
   `ParsedDocument::get_field`, `::fields`, and `BODY_FIELD` are **removed**. Callers must use `frontmatter()` / `body()` / `cards()`.
4. **`Document::to_plate_json(&self) -> serde_json::Value`** — assembles the JSON shape the plate layer expects:
   ```json
   { "QUILL": "...", "<key>": "...", "CARDS": [...], "BODY": "..." }
   ```
   This is the only place in core that knows about the plate wire format.
5. **Internal consumer migration** (expect broad churn — audit each file, do not just search-and-replace):
   - `crates/quillmark/src/orchestration/workflow.rs` — `compile_data` stops iterating `parsed.fields()`. It calls `doc.to_plate_json()` for the backend payload. Coercion/validation/defaults apply against `frontmatter` + per-card `fields` separately.
   - `crates/core/src/quill/config.rs` — `coerce`, `defaults`, `apply_schema_defaults` take typed inputs. Split into `coerce_frontmatter` / `coerce_card` rather than handling both through one `HashMap`.
   - `crates/core/src/quill/validation.rs` — same split. Delete CARDS/BODY-shape error arms.
   - `crates/backends/typst/src/lib.rs` — the code at lines ~184, 226, 321, 418, 438, 521 (markdown-field transform over CARDS items) now operates on the JSON produced by `to_plate_json()`, not directly on `fields`. Confirm the Typst template (`lib.typ.template`) still sees `CARDS` / `BODY` at the top level.
   - `crates/core/src/normalize.rs` — `normalize_document` / `normalize_fields` take typed inputs. `normalize_markdown` unchanged.
6. **Binding mirror:**
   - **WASM** (`crates/bindings/wasm/src/types.rs`, `lib.rs`): `Document` class with `fromMarkdown` / `toMarkdown` (stub — returns `unimplemented!()` or throws until Phase 4), read-only `quillRef`, `frontmatter`, `body`, `cards`, `warnings`. `Card` interface with `tag`, `fields`, `body`. Render/open signatures are unchanged — internally they hold a `Document` and pass `to_plate_json()` to core.
   - **Python** (`crates/bindings/python/src/types.rs`, `__init__.py`, `__init__.pyi`): same shape, snake_case. Remove `fields` / `get_field` from Python API.
   - Update `crates/bindings/wasm/basic.test.js`, `crates/bindings/python/tests/test_parse.py`, `test_render.py`, `test_api_requirements.py` to use the new surface.
7. **Rename** `ParsedDocument` → `Document` across `grep -r ParsedDocument`. At the time of writing this hits: `quillmark/src/`, `quillmark/tests/`, `core/tests/`, bindings, fixtures examples, CLI, fuzz. Do the rename in a single commit inside this PR so reviewers can separate it from semantic changes.

### Tests

- Every existing `get_field("CARDS")` call in tests converts to `doc.cards()`.
- Every existing `get_field("BODY")` / `doc.body()` call converts to `doc.body()` (now `&str`, not `Option<&str>` — update the Option unwrap paths).
- Add round-trip test: `to_plate_json` on a fixture produces the same JSON shape the old `compile_data` produced against the same fixture. Capture that JSON as a snapshot.
- WASM JS test: `Document.fromMarkdown(md)` exposes `.cards`, `.frontmatter`, `.body`, `.warnings` with the right shapes.
- Python: `test_parse.py` asserts `doc.frontmatter`, `doc.body`, `doc.cards`.

### Exit criteria

- `ParsedDocument` not referenced anywhere in the tree.
- No `fields.get("CARDS")` / `fields.get("BODY")` anywhere in core or backends.
- Typst template (`lib.typ.template`) receives identical JSON (`to_plate_json`). Fixture rendering produces byte-identical PDFs/SVGs for every `crates/fixtures/resources/quills/*` example.
- Bindings (WASM, Python) expose the typed surface. `toMarkdown` / `to_markdown` exist but are stubs that panic/throw with a clear "not yet implemented (phase 4)" message.

### Risks

- **Hidden HashMap iteration order.** Anything that relied on `HashMap` iteration order was already broken; migrating to `IndexMap` may make a latent bug visible as a test diff. If a test starts passing or failing because of order, that is a real bug — do not paper over it.
- **Typst template coupling.** Validate early that `to_plate_json()` output exactly matches today's `compile_data` output shape for a representative document. Any divergence breaks plate authors.
- **Card-body type.** Today, card `BODY` may be absent (`Option<String>`). Decision: `Card.body` is `String`, empty string when absent. Document this in the rustdoc. Validate against fixtures.

---

## Phase 3 — Editor surface

**Goal:** add mutators to `Document` and `Card` with invariant enforcement.

### Work

1. New submodule `crates/core/src/document/edit.rs` containing:
   - `EditError` enum — distinct from `ParseError`, no source-location fields.
     Variants: `ReservedName(String)`, `InvalidFieldName(String)`, `InvalidTagName(String)`, `IndexOutOfRange { index: usize, len: usize }`.
   - Name validation helpers that reuse `is_valid_tag_name` (now in `sentinel.rs`) and add `is_valid_field_name` (`[a-z_][a-z0-9_]*`, NFC-normalized).
   - Reserved-name set: `{"BODY", "CARDS", "QUILL", "CARD"}` — a single `const` pulled from `sentinel.rs`.
2. `impl Document` methods per proposal §6:
   - `set_field(name, value) -> Result<(), EditError>`
   - `remove_field(name) -> Option<QuillValue>`
   - `set_quill_ref(QuillReference)`
   - `replace_body(impl Into<String>)`
   - `card_mut(index) -> Option<&mut Card>`
   - `push_card(Card) -> Result<(), EditError>` (currently trivial — reserved for future cross-card invariants)
   - `insert_card(index, Card) -> Result<(), EditError>`
   - `remove_card(index) -> Option<Card>`
   - `move_card(from, to) -> Result<(), EditError>`
3. `impl Card` methods:
   - `Card::new(tag) -> Result<Self, EditError>` — validates tag.
   - `set_field(name, value) -> Result<(), EditError>`
   - `remove_field(name) -> Option<QuillValue>`
   - `set_body(impl Into<String>)`
4. **WASM surface** (`crates/bindings/wasm/src/types.rs`): `setField`, `removeField`, `setQuillRef`, `replaceBody`, `pushCard`, `insertCard`, `removeCard`, `moveCard`, `updateCardField`, `updateCardBody`. Errors surface as JS exceptions carrying the `EditError` variant name and message.
5. **Python surface**: same shape, snake_case. `EditError` maps to a Python exception class (`quillmark.EditError`).
6. Rustdoc on every mutator states: (a) which invariants it enforces, (b) that the document remains a valid `Document` after any successful call, (c) that mutators never touch `warnings`.

### Tests

- One unit test per `EditError` variant triggering path.
- Reserved-name test matrix: each of `{BODY, CARDS, QUILL, CARD}` rejected by `set_field` and `Card::set_field`.
- Move-card boundary tests: `move_card(0, 0)` is a no-op; `move_card(last, 0)` rotates; out-of-range returns `IndexOutOfRange`.
- Invariant property test (quickcheck-style if already in deps, else hand-rolled): after any sequence of mutations, `doc.frontmatter()` contains no reserved key and every `card.tag` passes `is_valid_tag_name`.
- WASM: `basic.test.js` round-trip — parse, mutate via binding, assert new state visible on subsequent reads.
- Python: parallel tests in `test_api_requirements.py`.

### Exit criteria

- All mutators callable from Rust, WASM, Python.
- Every invariant documented in proposal §6 is either enforced by the type system (e.g. card ordering is a `Vec<Card>`) or by an `EditError` return path covered by a test.
- No mutator can produce a `Document` that would fail to re-parse in Phase 4 emit+parse round-trip.

### Risks

- **WASM ownership model.** WASM `Document` must be mutable on the JS side but the underlying Rust object is boxed. Confirm the existing `wasm_bindgen` setup supports `&mut self` methods on the exported class; if not, design the mutation interface to clone-on-write or to expose mutation via owned values. Check `crates/bindings/wasm/src/engine.rs:100` area for the existing pattern.
- **Python GIL and mutation.** `pyo3` mutable methods require `PyRefMut`. Prototype one mutator end-to-end before scaling out.

---

## Phase 4 — Emitter (`Document::to_markdown`)

**Goal:** canonical, type-fidelity, emit-idempotent Markdown emission. This is the load-bearing PR.

### Pre-PR spike (merge as a throwaway or discard)

The proposal §5.3 leaves one decision open: does `serde-saphyr::SerializerOptions` expose a hook to force every string scalar to double-quoted, or do we need a `ForceQuoted(&str)` newtype?

- Read `serde-saphyr`'s public API at the pinned version (`~0.0`).
- Write a 30-line spike emitting `{"key": "on", "num": 42, "nested": {"a": "01234"}}` and verify the output quotes `"on"` and `"01234"` but leaves `42` bare.
- If `SerializerOptions` suffices: great, use it.
- If not: implement `ForceQuoted(&str)` whose `Serialize` impl writes pre-escaped bytes. Wrap every `QuillValue::String` at serialize time by walking the value tree.

The spike informs the actual PR; do not land the spike itself. Document the finding in the PR description of Phase 4.

### Work

1. New submodule `crates/core/src/document/emit.rs` and `crates/core/src/document/limits.rs` updates if the spike revealed any new constants.
2. `impl Document { pub fn to_markdown(&self) -> String; }` implementing the rules from proposal §5.2:
   - `\n` line endings only; CRLF normalization happens on import (already in `normalize.rs`).
   - Frontmatter order: `QUILL` first, then remaining frontmatter in `IndexMap` insertion order.
   - Block-style mappings and sequences.
   - Booleans emitted as `true` / `false`. Null as `null`. Numbers bare.
   - **Every string scalar double-quoted**, JSON-escaped (`\"`, `\\`, `\n`, `\t`, `\u00XX` for control chars). This is what buys type fidelity.
   - Multi-line strings: double-quoted with `\n` escapes. No block scalars in v1.
   - Cards: preceded by one blank line, fence `---\nCARD: <tag>\n<fields>\n---\n<body>`.
3. Rustdoc on `to_markdown`: contract (§5.1), rules (§5.2), and a "what is lost" section (§5.4: comments, custom tags, original quoting).
4. **Binding wiring**: the stubs planted in Phase 2 (`toMarkdown` / `to_markdown`) now call through to the real implementation. Remove the "not yet implemented" error path.
5. Decide the two open questions from proposal §11:
   - **Nested map ordering.** If the spike shows `serde-saphyr` preserves a serde-side `IndexMap`, extend `QuillValue::Object` to use `IndexMap` and preserve insertion order. Otherwise, **sort nested-map keys on emit** and document that choice in the rustdoc. Either is deterministic; the decision is diff-aesthetics.
   - **Empty containers.** Fix one rule: empty map → omit key entirely from emit; empty sequence → `key: []`. Document in rustdoc.

### Tests — this phase is test-heavy by design

1. **Type-fidelity round-trip** (the whole point). For every fixture in `crates/fixtures/resources/quills/*` and every `.md` in `crates/fixtures/resources/`:
   ```rust
   let a = Document::from_markdown(src)?;
   let b = Document::from_markdown(&a.to_markdown())?;
   assert_eq!(a, b);  // by value AND by type
   ```
   `PartialEq` on `Document` / `Card` / `QuillValue` compares type variants strictly. `QuillValue::String("on") != QuillValue::Bool(true)`.
2. **Ambiguous-strings regression corpus.** New fixture `crates/fixtures/resources/ambiguous_strings.md` exercising:
   `"on"`, `"off"`, `"yes"`, `"no"`, `"true"`, `"false"`, `"null"`, `"~"`, `"01234"`, `"1e10"`, `"0x1F"`, `"2024-01-15"`, `""` (empty), `" "` (single space), `"\n"` (literal newline inside), `"\""` (embedded quote), `"\\"` (embedded backslash), `"key: value"` (looks like YAML), `"- item"` (looks like a sequence marker), `"#comment"`, `"&anchor"`, `"*alias"`, `"!tag"`.
   Each survives a round-trip as a string with identical bytes.
3. **Emit idempotence.** `doc.to_markdown() == doc.to_markdown()` byte-for-byte. Run on 50+ fixture documents.
4. **Parse ∘ Emit ∘ Parse ∘ Emit** equality: `to_markdown` applied twice to the re-parsed document produces the same bytes as applied once. Catches non-deterministic emitter bugs.
5. **Lossiness documentation tests.** Explicitly test (and document) that: YAML comments disappear, custom tags (`!fill`) lose the tag but keep the value, original quoting style is not preserved.
6. **Fuzzing.** Extend `crates/fuzz/src/parse_fuzz.rs` with an `emit_roundtrip_fuzz` target: for any parser-accepted input, `from_markdown → to_markdown → from_markdown` is stable by type.
7. **WASM/Python integration tests.** End-to-end: parse MD in binding, mutate a field, emit, re-parse, assert shape.

### Exit criteria

- Type-fidelity round-trip passes for the entire fixture corpus.
- Ambiguous-strings corpus passes every entry.
- Emit idempotence holds across fixtures.
- Fuzz target runs 5 minutes locally without crashes or round-trip mismatches.
- Downstream consumer story closes: a consumer calls `fromMarkdown → setField → toMarkdown` and gets back a valid Quillmark Markdown document without writing their own YAML emitter.

### Risks

- **`serde-saphyr` emitter gaps.** This is the single biggest schedule risk. If the spike shows the crate cannot be coerced into forced-quoting, the fallback (a `ForceQuoted` newtype wrapper emitted via its `Serialize` impl writing pre-escaped bytes) needs real design work — it interacts with every nested `QuillValue` type. Budget an extra day if the fallback is needed.
- **Nested-map order.** Picking "sort keys on emit" is the safe default. If authors care about author-order preservation in nested maps, that is a v2 feature and a separate PR.
- **Numbers at the edge of precision.** `1e10`, `0x1F`, large integers — confirm `QuillValue::Number` and the emitter agree on the string representation. Add explicit tests.

---

## Phase 5 — Schema-aware form projection

**Goal:** `Quill::project_form(&Document) -> FormProjection`, giving form editors exactly the shape they need. Piggy-backs on existing `coerce` / `defaults` / `validate`.

### Work

1. New types in `crates/core/src/quill/` (probably `query.rs` or a new `form.rs`):
   ```rust
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
       pub source: FormFieldSource,
   }
   pub enum FormFieldSource { Document, Default, Missing }
   ```
2. `impl Quill { pub fn project_form(&self, doc: &Document) -> FormProjection; }` implementation:
   - For the main card schema, walk each declared field; pull value from `doc.frontmatter()`, fall back to schema default, otherwise `Missing`.
   - For each `doc.cards()`, find the matching `CardSchema` by tag; missing tags append a `Diagnostic` to `diagnostics`.
   - `validate` runs over the projection and merges its diagnostics into `diagnostics`.
3. **WASM**: `Quill.projectForm(doc)` returning a plain JS object (no class). Serializable via `JSON.stringify` for trivial UI integration.
4. **Python**: `quill.project_form(doc)` returning dataclasses or `dict` — match the existing `quillmark-python` convention for complex return types.
5. Rustdoc spells out that `FormProjection` is a read-only snapshot; subsequent edits to `doc` require re-projecting.

### Tests

- Projection over a fixture with missing fields: those fields show `FormFieldSource::Default` or `Missing` correctly.
- Projection over a document with an unknown card tag: diagnostic emitted, card appears in `cards` with its schema as `None`-equivalent (decide: either drop the card or surface it with a sentinel schema; document the choice).
- Validation diagnostics appear in `FormProjection.diagnostics`.
- WASM/Python: smoke tests that `JSON.stringify(projection)` / `json.dumps(projection)` round-trip cleanly.

### Exit criteria

- `project_form` exists on `Quill` in Rust, WASM, Python.
- Every schema field in a fixture produces a deterministic `FormFieldValue` with the correct `source`.
- No duplication of coercion / default / validation logic — `project_form` composes the existing functions.

### Risks

- **Shape drift from consumers' expectations.** Downstream consumers probably already have a hand-rolled form shape. Before landing, circulate the `FormProjection` shape with those consumers and adjust. Changing the shape after merge is a breaking change.

---

## Overall sequencing & parallelism

```
Phase 1 (module split)
      ↓
Phase 2 (reshape Document + bindings + to_plate_json)
      ↓              ↓
Phase 3 (editor)   Phase 4 spike (serde-saphyr emitter)
      ↓              ↓
     → Phase 4 (emitter)
              ↓
           Phase 5 (form projection)
```

- Phase 3 and the Phase 4 spike can run in parallel after Phase 2 merges.
- Phase 4 proper depends on both Phase 3 (editor surface exists, so `toMarkdown` is paired with mutation in integration tests) and the spike outcome.
- Phase 5 is independent of Phase 4 once Phase 2 is in. Could land before Phase 4 if editor/emitter slip.

## Non-goals (explicit — do not do these)

- Preserving original YAML formatting, comments, quoting, or tags on re-emit (proposal §2, §5.4).
- Changing the Quillmark Markdown standard or schema model (proposal §2).
- Adding block-scalar emission (`|`, `>`) — deferred to v2 (proposal §5.2).
- Layout-preserving round-trip — separate proposal if ever needed (proposal §11).
- Any compatibility shim or deprecation path — pre-release, we delete (proposal status line).

## Open decisions captured (resolve during the phases indicated)

- **Nested-map order preservation vs. key-sort on emit.** Decide during Phase 4 spike.
- **Empty containers.** Decide during Phase 4; the plan's default is "omit empty map, emit `[]` for empty sequence".
- **Card.body type (`String` vs `Option<String>`).** Plan chooses `String`, empty when absent; revisit in Phase 2 if a fixture or consumer breaks.
- **WASM mutation ergonomics.** Prototype one mutator in Phase 3 before scaling.
