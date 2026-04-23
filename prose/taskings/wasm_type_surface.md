# WASM Type Surface & Body-Separator Refactor Tasking

**Audience:** Quillmark engine + WASM binding maintainer
**Consumer feedback source:** `@quillmark/quiver` authors (post-integration review)
**Branch:** `claude/review-consumer-feedback-TRloK`

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

### 1. `Quillmark.quill(tree)` — narrow the input type and accept `Record`

Today `quill(tree: any): Quill` in the generated `.d.ts`. The runtime strictly
requires `js_sys::Map` (`crates/bindings/wasm/src/engine.rs:502`), so
`Record<string, Uint8Array>` throws — a shape most JS consumers naturally
produce.

**Change:**

1. Broaden `js_tree_entries` to also accept plain objects. If the value is not
   a `Map`, fall through to a branch that iterates `Object.keys(value)` (not
   `for...in` — avoid prototype chain). Symbol keys ignored.
2. Annotate the wasm method with a TypeScript parameter type:

   ```rust
   #[wasm_bindgen(js_name = quill, unchecked_param_type = "Map<string, Uint8Array> | Record<string, Uint8Array>")]
   pub fn quill(&self, tree: JsValue) -> Result<Quill, JsValue>
   ```

No new exported TS type. The union is inline on the method signature.

**Error messages:** update the "quill requires a Map<string, Uint8Array>"
message in `js_tree_entries` to mention both accepted shapes.

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

1. Add a single `typescript_custom_section`:

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

4. **Delete** `card_to_js_value` (`engine.rs:457`). The `removeCard` method
   currently calls it — rewrite to return `Option<Card>` instead of `JsValue`.

### 4. Remove body-separator storage from core (deeper fix)

The binding-level `trim_body` helper (`engine.rs:476`) is applied in three
output paths (`body`, `cards`, `card_to_js_value`). Its own doc comment
(`engine.rs:471-479`) names the issue: the trailing `\n`/`\r` characters are
"structural separators, not part of what the document author wrote" — yet
they live in `Card.body` and `Document.body` storage and every consumer-facing
read has to strip them.

The separator is a function of structural context — is another card next? is
this EOF? — so it's an emitter concern, not content. Storing it is redundant
state that has to be defended at every read.

**Change:** bodies in core never contain the F2 structural separator.

1. **Parser side** (`crates/core/src/document/assemble.rs`,
   `crates/core/src/document/fences.rs`): when constructing `Card` and
   `Document`, strip trailing `\n`/`\r` from body segments before storing.
   Apply at the point of construction, not as a post-pass.

2. **Edit side** (`crates/core/src/document/edit.rs:158`, `:315`): `set_body`
   / `replace_body` already accept arbitrary strings from consumers.
   Normalise on entry by stripping trailing `\n`/`\r` so the invariant holds
   uniformly regardless of origin (parsed vs. edited).

3. **Emit side** (`crates/core/src/document/emit.rs:116-138`): already has
   logic to ensure `"\n\n"` precedes each metadata fence. Verify it still
   works when the stored body has no trailing newline — it should, since the
   existing `ensure_blank_line_terminator` appends as needed. Adjust if any
   branch assumed the separator was already present.

4. **Binding side**: delete `trim_body` entirely. `Document.body` getter
   forwards `self.inner.body().to_string()`. The `From<&core::Card> for Card`
   impl from Task 3 needs no trim.

5. **Round-trip property:** `fromMarkdown(md).toMarkdown()` must remain
   byte-equal to canonical output of `md`. This is the load-bearing test —
   if it passes before and after, the refactor is correct.

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

- **Task 1:** in `crates/bindings/wasm/tests/` (or the existing
  `basic.test.js`), add cases that pass a plain object, a `Map`, and a
  `Map`/object mix — verify all three load identically.
- **Task 2:** no runtime test change needed (shape is unchanged). Add a
  `.d.ts` snapshot test or a `tsc --noEmit` fixture that imports
  `CardInput` and `pushCard` with an object literal.
- **Task 3:** existing tests that assert shape of `cards` / `warnings` /
  `frontmatter` should keep passing. Add a TS compilation fixture that
  relies on the narrowed types (e.g. `doc.cards[0].tag` should type-check
  without a cast).
- **Task 4:** in `crates/core/src/document/tests/assemble_tests.rs`, update
  `test_body_with_trailing_newlines` to assert the stored body has **no**
  trailing newline. Add a round-trip test: parse a document with multi-card
  bodies, serialise back with `toMarkdown`, assert byte equality with the
  input's canonical form.

## Done when

- `.d.ts` for `@quillmark/wasm` contains no `any` on `quill`, `pushCard`,
  `insertCard`, `cards`, `frontmatter`, or `warnings`.
- `CardInput` is exported from the `.d.ts` and accepts object literals
  without a nominal import.
- `quill({ "Quill.toml": bytes })` works from JS with a plain object.
- `trim_body` is deleted from `crates/bindings/wasm/src/engine.rs`.
- Core `Card::body()` and `Document::body()` return content with no trailing
  structural separator, for both parsed and edited documents.
- Markdown round-trip tests (`fromMarkdown`/`toMarkdown`) pass unchanged.
- No existing wasm or core tests regress.
