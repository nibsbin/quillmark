# WASM Type Surface & Body-Separator Refactor Tasking

**Audience:** Quillmark engine + WASM binding maintainer
**Consumer feedback source:** `@quillmark/quiver` authors (post-integration review)
**Branch:** `claude/review-consumer-feedback-TRloK`
**wasm-bindgen version:** 0.2.118 (supports `unchecked_param_type` /
`unchecked_return_type`; added in 0.2.95).

## Background

A downstream consumer reviewed the generated `.d.ts` for `@quillmark/wasm` and
flagged four friction points where the binding forces runtime assertions that
should be compile-time checks. Three are binding-level typing fixes. The
fourth, initially framed as a binding concern (`trim_body` scattered across
output paths), turned out to be a symptom of a core storage decision and is
promoted to a core refactor.

Scope is bounded: no change to `render`, `parseMarkdown`, selector resolution,
or plate wire format. No new public surface area beyond what's listed.

## Tasks

### 1a. `Quillmark.quill(tree)` — narrow the declared input type

Today `quill(tree: any): Quill` in the generated `.d.ts`. The runtime strictly
requires `js_sys::Map` (`crates/bindings/wasm/src/engine.rs:502`). Narrow the
declared type to match runtime truth:

```rust
#[wasm_bindgen(js_name = quill, unchecked_param_type = "Map<string, Uint8Array>")]
pub fn quill(&self, tree: JsValue) -> Result<Quill, JsValue>
```

Zero runtime change. Anyone currently passing a `Map` keeps compiling.

### 1b. `Quillmark.quill(tree)` — also accept `Record` (deferred)

Accepting `Record<string, Uint8Array>` is a separate *runtime* change that adds
a code path for consumer convenience (`new Map(Object.entries(x))` is the
current workaround). The ergonomic win is small relative to the maintenance
cost, and it would be a silent behaviour change for callers who used to get
a clear error on plain-object input.

**Deferred.** Revisit only if concrete consumer friction demands it.

### 2. `pushCard` / `insertCard` — typed input via TS-only `CardInput`

Today both methods accept `card: any`. The runtime shape is already defined by
the function-local `CardInput` struct in `js_value_to_card`
(`crates/bindings/wasm/src/engine.rs:430`): `tag` required, `fields` and
`body` optional with serde defaults.

**Deliberately rejected:** promoting the local `CardInput` to a public
tsify-derived binding struct. That would export a nominal type consumers must
import, add a `TryFrom` indirection, and is overkill for two call sites. The
function-local struct stays.

**Change:**

1. Add a `typescript_custom_section`:

   ```rust
   #[wasm_bindgen(typescript_custom_section)]
   const CARD_INPUT_TS: &'static str = r#"
   export interface CardInput {
     tag: string;
     fields?: Record<string, unknown>;
     body?: string;
   }
   "#;
   ```

2. Annotate the two methods:

   ```rust
   #[wasm_bindgen(js_name = pushCard, unchecked_param_type = "CardInput")]
   pub fn push_card(&mut self, card: JsValue) -> Result<(), JsValue>

   #[wasm_bindgen(js_name = insertCard, unchecked_param_type = "CardInput")]
   pub fn insert_card(&mut self, index: usize, card: JsValue) -> Result<(), JsValue>
   ```

`js_value_to_card` stays unchanged. TS-only type, no Rust-side plumbing.

### 3. Typed output getters — `cards`, `frontmatter`, `warnings`

Today all three return `JsValue` (emitted as `any`). Each method rebuilds a
`serde_json::Value` and hands it to `serde_wasm_bindgen` manually — a
hand-rolled reimplementation of what the tsify-derived `Card` and `Diagnostic`
types in `crates/bindings/wasm/src/types.rs` already do automatically.

**Change:** return typed values and delete the JSON scaffolding.

1. `cards()` returns `Vec<Card>`:

   ```rust
   #[wasm_bindgen(getter, js_name = cards)]
   pub fn cards(&self) -> Vec<Card> {
       self.inner.cards().iter().map(Card::from).collect()
   }
   ```

   Add a `From<&quillmark_core::Card> for Card` impl in `types.rs` that
   constructs the tsify struct from core fields.

2. `warnings()` returns `Vec<Diagnostic>`:

   ```rust
   #[wasm_bindgen(getter, js_name = warnings)]
   pub fn warnings(&self) -> Vec<Diagnostic> {
       self.parse_warnings.iter().cloned().map(Into::into).collect()
   }
   ```

3. `frontmatter()` returns a tsify newtype wrapping `serde_json::Value` with
   `#[tsify(type = "Record<string, unknown>")]`, matching the convention
   already used on `Card.fields` (`types.rs:164`).

4. **Delete** `card_to_js_value` (`engine.rs:457`). This also changes
   `removeCard`: today it returns `JsValue` (typed as `any`, semantically
   `Card | undefined`). Change the return type to `Option<Card>` so the
   generated `.d.ts` declares `Card | undefined` explicitly.

   **This is a public API change** — the runtime shape is unchanged (object
   or `undefined`), but the declared TS type narrows. Consumers who relied
   on `any` lose the implicit-any escape hatch.

### 4. Remove F2 separator storage from core (deeper fix)

The binding-level `trim_body` helper (`engine.rs:476`) is applied in three
output paths (`body`, `cards`, `card_to_js_value`). Its own doc comment
(`engine.rs:471-479`) names the issue: the trailing newline characters are
"structural separators, not part of what the document author wrote" — yet
they live in `Card.body` and `Document.body` storage and every consumer-facing
read has to strip them.

**Semantic correction from initial draft.** `trim_body`'s current
implementation (`trim_end_matches(|c| c == '\n' || c == '\r')`) strips **all**
trailing newlines, conflating two distinct things:

- The F2 *blank line* required before the next fence (exactly one line ending
  at the tail of a body that's followed by another block). This is structural.
- Any line-ending or trailing whitespace that's part of the author's content
  (e.g. a code block that ends with `\n`). This is content.

Moving `trim_body`-as-written into core would propagate the conflation — every
reader would see content-ending newlines silently dropped. The correct model
is narrower.

**Correct model.**

- **F2 separator = exactly one line ending at the tail of a body slice that is
  followed by another metadata block.** For `\n` line endings, that's one
  `\n`. For `\r\n`, that's one `\r\n`.
- Bodies at end-of-document have no F2 separator to strip.
- Author content after the strip is preserved verbatim, including its own line
  terminators.

Worked example: `...---\nalpha\n\n---\nCARD: x...`
- Raw body slice = `"alpha\n\n"` (seven bytes).
- The tail `\n` is the F2 blank line; the first `\n` terminates the `alpha`
  line — that's content.
- Stored body = `"alpha\n"`.
- Emit side already ensures `\n\n` before the next fence (`emit.rs:140`), so
  round-trip byte equality holds.

**Change:**

1. **Parse side** (`crates/core/src/document/assemble.rs`): when extracting a
   body segment that is followed by another block (i.e. `idx + 1 < blocks.len()`
   for card bodies, or a CARD block exists for the global body), strip exactly
   one trailing line ending. Bodies at EOF are stored as-is.

   Implement as a private helper in `assemble.rs`:

   ```rust
   fn strip_f2_separator(body: &str) -> &str {
       if let Some(rest) = body.strip_suffix("\r\n") { rest }
       else if let Some(rest) = body.strip_suffix('\n') { rest }
       else { body }
   }
   ```

2. **Edit side** (`crates/core/src/document/edit.rs:158`, `:315`): no change.
   `set_body` / `replace_body` continue to accept arbitrary strings as-is; the
   emitter already adds the F2 separator on output, so a consumer setting
   `replace_body("x")` still round-trips correctly. **Not** normalising on
   input means `get_body` returns what was set.

3. **Emit side** (`crates/core/src/document/emit.rs:140`): no change needed.
   `ensure_blank_line_before_fence` already handles bodies ending in `\n`,
   `\n\n`, or no newline. Verify with tests; adjust only if a round-trip case
   fails.

4. **Binding side**: delete `trim_body` entirely. `Document.body` getter
   forwards `self.inner.body().to_string()`. The `From<&core::Card> for Card`
   impl from Task 3 copies the body without trimming.

**Load-bearing tests.**

- Round-trip byte equality: for every fixture in
  `crates/core/src/document/tests/fixtures/`, `emit(decompose(src)) == src`
  (modulo documented canonicalisation).
- **Content fidelity:** a body ending in `\n` as author content survives
  round-trip. Construct a case: `---\nCARD: x\n---\ncode line\n\n---\nCARD: y\n---\n`
  — the first card's body should be `"code line\n"` (not `"code line"`), and
  re-emitting should restore `\n\n` before the next fence.

**Scope estimate:** medium. Touches parser + emitter audit + test fixtures +
binding cleanup. ~4 distinct commits: parse-side strip, binding trim removal,
test updates, cross-crate verification.

## Out of scope

- Changes to plate wire format or `to_plate_json`. That format is a backend
  contract separate from the Document representation.
- Any new public API beyond `CardInput` (TS-only) and the return-type
  changes on existing getters.
- Reworking `updateCardField` / `setField` — those take a `JsValue` value
  because field values are genuinely dynamic (`unknown` on the TS side).
  They're already typed reasonably; a future pass can tighten to `unknown`
  if `any` is still showing through.

## Test updates

- **Task 1a:** `tsc --noEmit` fixture that rejects `quill({})` at compile
  time and accepts `quill(new Map<string, Uint8Array>())`.
- **Task 2:** no runtime test change. Add a `.d.ts` snapshot or `tsc`
  fixture importing `CardInput` and calling `pushCard({ tag: "foo" })`.
- **Task 3:** existing tests that assert shape of `cards` / `warnings` /
  `frontmatter` should keep passing. Add a TS fixture that relies on the
  narrowed types (e.g. `doc.cards[0].tag` type-checks without a cast).
- **Task 4:**
  - Update `test_body_with_trailing_newlines`
    (`assemble_tests.rs:1572`) to match the new model — at EOF, trailing
    content newlines are preserved; followed by a fence, exactly one line
    ending is stripped.
  - Add an explicit F2-strip test covering `\n`, `\r\n`, and multi-newline
    cases for bodies followed by a fence.
  - Add a content-fidelity test asserting that a body ending in `\n` as
    content survives round-trip.

## Done when

- `.d.ts` for `@quillmark/wasm` contains no `any` on `quill`, `pushCard`,
  `insertCard`, `removeCard`, `cards`, `frontmatter`, or `warnings`.
- `CardInput` is exported from the `.d.ts` and accepts object literals
  without a nominal import.
- `trim_body` is deleted from `crates/bindings/wasm/src/engine.rs`.
- Core `Card::body()` / `Document::body()` return exactly what was authored
  (or set), with the F2 structural separator removed on parse and re-added
  on emit.
- Markdown round-trip byte equality holds for all existing fixtures.
- Content-fidelity test (body ending in `\n`) passes.
- No existing wasm or core tests regress.
