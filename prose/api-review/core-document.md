# Core: document

Scope: `crates/core/src/document/**` (`mod.rs`, `edit.rs`, `emit.rs`, `assemble.rs`,
`prescan.rs`, `payload.rs`, `dto.rs`, `wire.rs`, `fences.rs`, `meta.rs`,
`yaml_hints.rs`, `limits.rs`).

## Surface

Blessed (re-exported from `crates/core/src/lib.rs`):

```
Card, CardWire, Document, EditError, Parsed, Payload, PayloadItem,
PayloadItemWire, RichtextDecodeError, SeedOverlay, WireError
```

Re-exported one level down, at `document::` but not promoted to crate root
(`quillmark_core::document::X`, not `quillmark_core::X`):

```
peek_schema_version, StorageError, StoredDocument, SCHEMA_V0_93_0   (dto)
is_valid_kind_name, validate_composable_kind, CardKindError          (meta)
MetaKey                                                              (payload)
FORMAT_RULES, blueprint_instruction                                  (mod.rs)
```

Public but reachable only via the full module path (not re-exported at any
level — `quillmark_core::document::<module>::X`):

```
edit::{is_valid_field_name, validate_field, FieldViolation}
wire::PathStepWire
prescan::{PreItem, PreScan, NestedComment, prescan_fence_content}
limits::MAX_YAML_DEPTH
```

### `Card` (mod.rs)

```rust
pub struct Card { .. }                                          // fields private
fn from_parts(payload: Payload, body: Content) -> Self          // unvalidated
fn quill(&self) -> Option<&QuillReference>
fn kind(&self) -> Option<&str>
fn id(&self) -> Option<&str>
fn ext(&self) -> Option<&serde_json::Map<String, Value>>
fn payload(&self) -> &Payload
fn payload_mut(&mut self) -> &mut Payload
fn body(&self) -> &Content
fn body_markdown(&self) -> String
fn field_richtext(&self, name: &str) -> Option<Result<Content, RichtextDecodeError>>
fn field_markdown(&self, name: &str) -> Option<Result<String, RichtextDecodeError>>
fn field_plaintext(&self, name: &str) -> Option<Result<String, RichtextDecodeError>>
// edit.rs
fn new(kind: impl Into<String>) -> Result<Self, EditError>
fn store_field(&mut self, name: &str, value: impl Into<QuillValue>) -> Result<(), EditError>
fn store_fill(&mut self, name: &str, value: impl Into<QuillValue>) -> Result<(), EditError>
fn store_fields<K,V,I>(&mut self, fields: I) -> Result<(), Vec<(String, EditError)>>
fn remove_field(&mut self, name: &str) -> Result<Option<QuillValue>, EditError>
fn store_ext(&mut self, value: Map) -> Result<(), EditError>
fn remove_ext(&mut self) -> Option<Map>
fn store_ext_namespace(&mut self, namespace: impl Into<String>, value: Value) -> Result<(), EditError>
fn remove_ext_namespace(&mut self, namespace: &str) -> Option<Value>
fn seed(&self) -> Option<&Map>
fn store_seed_namespace(&mut self, card_kind: impl Into<String>, value: Value) -> Result<(), EditError>
fn remove_seed_namespace(&mut self, card_kind: &str) -> Option<Value>
fn install_body(&mut self, content: Content)
fn install_field(&mut self, name: &str, content: Content) -> Result<(), EditError>
fn commit_field(&mut self, name: &str, value: impl Into<QuillValue>, schema: &FieldSchema) -> Result<(), EditError>
fn revise_body(&mut self, body: impl Into<String>) -> Result<Delta, EditError>
fn revise_field(&mut self, name: &str, body: impl Into<String>) -> Result<Delta, EditError>
fn revise_field_checked(&mut self, name: &str, body: impl Into<String>, schema: &FieldSchema) -> Result<Delta, EditError>
fn apply_body_change(&mut self, text_delta: &Delta, line_ops: &[LineOp], mark_ops: &[MarkOp]) -> Result<(), EditError>
fn apply_field_richtext_change(&mut self, name: &str, text_delta: &Delta, line_ops: &[LineOp], mark_ops: &[MarkOp]) -> Result<(), EditError>
```

### `Document` (mod.rs / edit.rs / emit.rs)

```rust
pub struct Document { .. }                                       // fields private
fn new(quill: QuillReference) -> Self
fn from_main_and_cards(main: Card, cards: Vec<Card>) -> Self      // debug_assert only, infallible
fn parse(markdown: &str) -> Result<Parsed, ParseError>
fn main(&self) -> &Card
fn main_mut(&mut self) -> &mut Card
fn quill_reference(&self) -> QuillReference                       // owned, .expect()-panics if absent
fn cards(&self) -> &[Card]
fn cards_mut(&mut self) -> &mut [Card]
fn card(&self, index: usize) -> Option<&Card>
fn card_mut(&mut self, index: usize) -> Option<&mut Card>         // edit.rs
fn find_card(&self, id: &str) -> Option<(usize, &Card)>
fn to_plate_json(&self) -> serde_json::Value
// edit.rs
fn set_quill_ref(&mut self, reference: QuillReference)
fn push_card(&mut self, card: Card) -> Result<(), EditError>
fn insert_card(&mut self, index: usize, card: Card) -> Result<(), EditError>
fn set_card_id(&mut self, index: usize, id: impl Into<String>) -> Result<(), EditError>
fn remove_card_id(&mut self, index: usize) -> Option<String>
fn remove_card(&mut self, index: usize) -> Option<Card>
fn set_card_kind(&mut self, index: usize, new_kind: impl Into<String>) -> Result<(), EditError>
fn move_card(&mut self, from: usize, to: usize) -> Result<(), EditError>
// emit.rs
fn to_markdown(&self) -> String
```

### `Payload` / `PayloadItem` (payload.rs)

```rust
pub enum MetaKey { Ext, Seed }
impl MetaKey { fn as_str(self) -> &'static str; fn from_key_str(&str) -> Option<Self>; fn is_root_only(self) -> bool }

pub enum PayloadItem {
    Quill { reference: QuillReference },
    Kind { value: String },
    Id { value: String },
    Meta { key: MetaKey, value: Map, nested_comments: Vec<NestedComment> },
    Field { key: String, value: QuillValue, fill: bool, nested_comments: Vec<NestedComment> },
    Comment { text: String, inline: bool },
}
impl PayloadItem {
    fn field(key: impl Into<String>, value: QuillValue) -> Self       // unvalidated
    fn nested_comments(&self) -> &[NestedComment]
    fn comment(text: impl Into<String>) -> Self
    fn comment_inline(text: impl Into<String>) -> Self
}

pub struct Payload { .. }                                             // fields private
fn new() -> Self
fn from_index_map(map: IndexMap<String, QuillValue>) -> Self
fn from_items(items: Vec<PayloadItem>) -> Self
fn items(&self) -> &[PayloadItem]
fn items_mut(&mut self) -> &mut [PayloadItem]                         // unchecked backdoor, documented
fn quill(&self) -> Option<&QuillReference>
fn kind(&self) -> Option<&str>
fn id(&self) -> Option<&str>
fn ext(&self) -> Option<&Map>
fn seed(&self) -> Option<&Map>
fn set_quill(&mut self, reference: QuillReference)
fn set_kind(&mut self, kind: impl Into<String>)
fn set_id(&mut self, id: impl Into<String>)
fn take_id(&mut self) -> Option<String>
fn set_ext(&mut self, value: Map)
fn set_seed(&mut self, value: Map)
fn take_ext(&mut self) -> Option<Map>
fn take_seed(&mut self) -> Option<Map>
fn iter(&self) -> impl Iterator<Item = (&String, &QuillValue)>
fn keys(&self) -> impl Iterator<Item = &String>
fn len(&self) -> usize                                                // Field items only
fn is_empty(&self) -> bool                                            // Field items only
fn get(&self, key: &str) -> Option<&QuillValue>
fn contains_key(&self, key: &str) -> bool
fn is_fill(&self, key: &str) -> bool
fn insert(&mut self, key: impl Into<String>, value: QuillValue) -> Result<Option<QuillValue>, FieldViolation>
fn insert_fill(&mut self, ..) -> Result<Option<QuillValue>, FieldViolation>
fn remove(&mut self, key: &str) -> Option<QuillValue>
fn to_index_map(&self) -> IndexMap<String, QuillValue>
impl<'a> IntoIterator for &'a Payload
```

### Wire (`wire.rs`) and storage DTO (`dto.rs`)

```rust
pub enum PayloadItemWire { Field { key, value, fill, nested_fills: Vec<Vec<PathStepWire>> }, Comment { text, inline } }
pub enum PathStepWire { Index(usize), Key(String) }                   // not re-exported anywhere
pub struct CardWire { kind, quill, id, ext, seed, payload_items, body }
pub enum WireError { InvalidQuillReference { .. }, InvalidField { .. } }
impl From<&Card> for CardWire
impl TryFrom<CardWire> for Card                                        // no $kind validation

pub const SCHEMA_V0_93_0: &str
pub fn peek_schema_version(json: &str) -> Option<String>
pub enum StoredDocument { V0_93_0(..), V0_92_0(..) }
pub enum StorageError { InvalidQuillReference { .. }, Malformed(String) }
pub struct DocumentV0_93_0 / CardV0_93_0 / CanonicalContent(pub Content)
pub struct DocumentV0_92_0 / CardV0_92_0 / PayloadV0_92_0
pub enum PayloadItemV0_92_0 / CommentPathSegmentV0_92_0
pub struct NestedCommentV0_92_0
// From/TryFrom conversion impls both directions, per DOCUMENT_STORAGE.md
```

### `meta.rs`, `prescan.rs`, `limits.rs`

```rust
pub fn is_valid_kind_name(name: &str) -> bool
pub fn validate_composable_kind(kind: &str) -> Result<(), CardKindError>
pub enum CardKindError { InvalidName, Reserved }

pub enum PreItem { Field { .. }, Comment { .. } }                      // no external consumer
pub struct NestedComment { container_path, position, text, inline }    // leaks through PayloadItem, not re-exported
pub struct PreScan { .. }
pub fn prescan_fence_content(content: &str) -> PreScan

pub const MAX_YAML_DEPTH: usize = 100
```

### `EditError` / `RichtextDecodeError` / `SeedOverlay`

`EditError` (edit.rs): 13 variants, each with `variant_name()`, `code()`,
`doc_path(&DocPath) -> Option<DocPath>`. `RichtextDecodeError` (mod.rs): 2
variants (`NotContent`, `BadMarkdown`), `into_message(self) -> String`.
`SeedOverlay` (mod.rs): plain pub-field struct (`fields`, `body`), one
constructor `from_json(&Value) -> Option<Self>`, no `Serialize`/write path
(by design — the write door is `Card::store_seed_namespace`, which takes a
raw `Value`, not a `SeedOverlay`).

## Findings

### `Document::from_main_and_cards` only `debug_assert!`s its invariants — release builds can construct a corrupt `Document`, and `quill_reference()` then panics on it

**Severity: High** — `mod.rs:385-406`, consequence at `mod.rs:428-433`

`from_main_and_cards` is the only public constructor from a pre-built
`(Card, Vec<Card>)` pair. Every other document-building entry —
`Document::new`, `Card::new`, `push_card`, `insert_card`, the storage DTO's
`TryFrom<StoredDocument>`, `CardWire`'s `TryFrom` — enforces its invariants
with `Result`/`?` and a typed error. `from_main_and_cards` instead runs four
`debug_assert!`s (main carries `$quill`; composable cards don't carry
`$quill`/`$seed`; composable `$id`s are non-empty and unique) and is
otherwise infallible. `debug_assert!` compiles to nothing in a release
build, so in a `--release` binary this constructor accepts any input
silently — a duplicate card `$id`, a composable card smuggling `$quill`, all
without a single check.

This directly contradicts PROGRAMMATIC.md's stated guarantee ("every
mutator enforces the same … invariants … so a constructed document cannot
be invalid") for the one constructor that is not gated at all in release.
It is also live, not theoretical: `Document::quill_reference()` (`mod.rs:428`)
is documented "Always present on parsed documents" and backs that with
`.expect("root block's $quill is validated at parse time")`. A caller who
builds a `Document` via `from_main_and_cards(main_without_quill, vec![])` in
a release build gets a value that type-checks and constructs fine, then
panics the first time anything calls `quill_reference()`, `to_plate_json()`
(which calls it), or `to_markdown()`.

In-crate call sites (`quill/seed.rs`, `quill/compose.rs`,
`quill/blueprint.rs`, `quill/validation.rs`, `document/dto.rs`,
`normalize.rs`) all appear to pre-validate before calling, but the function
itself carries none of that as a compile-time or run-time guarantee — a
future in-crate caller, or any external caller (the function is `pub`, and
`Document`/`Card` are both blessed re-exports), can violate it with no
diagnostic. Compare `Card::from_parts` (`mod.rs:186`), which is equally
unvalidated but says so explicitly ("without validation … For user-facing
construction … use `Card::new`"); `from_main_and_cards`'s doc reads like a
checked constructor ("main must carry `$quill`; composable cards must
not") with no hint that the check is debug-only.

**Fix shape**: either return `Result<Self, EditError>` (or a dedicated
error) and check for real, or rename/document it unambiguously as an
unchecked builder on par with `from_parts` and `items_mut`.

### `PathStepWire` and `NestedComment` are unreachable through any re-export, despite being exposed as public fields of blessed, root-exported types

**Severity: Medium-High** — `wire.rs:58,74`; `payload.rs:53,125,139` / `prescan.rs:34,53`

`PayloadItemWire::Field` (blessed — `PayloadItemWire` is re-exported at the
crate root in `lib.rs`) carries `nested_fills: Vec<Vec<PathStepWire>>`.
`PathStepWire` is `pub enum` in `wire.rs`, but `document::mod.rs`'s
re-export list is `pub use wire::{CardWire, PayloadItemWire, WireError};` —
`PathStepWire` is left out, at every level (not even reachable via
`quillmark_core::document::PathStepWire`). The only way to name it is the
full private-looking path `quillmark_core::document::wire::PathStepWire`.
A binding author who does `use quillmark_core::PayloadItemWire;` and
pattern-matches `Field { nested_fills, .. }` cannot name the element type
without spelunking into `wire.rs`.

The same gap exists one level down: `PayloadItem::Field`/`Meta` (also
blessed) carry `nested_comments: Vec<NestedComment>`, and `NestedComment`
(`prescan.rs:53`) is public but `prescan` is never `pub use`'d from
`document::mod.rs` — so it's reachable only via
`quillmark_core::document::prescan::NestedComment`.

Contrast with the rest of the payload-adjacent surface, where every type
that shows up in a public field signature does get a top-level or
one-level re-export (`CardWire`, `WireError`, `MetaKey`, `Payload`,
`PayloadItem` all do). These two are the only field-referenced types that
don't.

### `TryFrom<CardWire> for Card` never validates `$kind`, unlike every other card-construction path

**Severity: Medium** — `wire.rs:218-282`, contrast `edit.rs:479-491` and `dto.rs:456-471`

`Card::new(kind)` validates via `validate_composable_kind` (rejects a
malformed name and the reserved `"main"`). The storage DTO's
`TryFrom<DocumentV0_93_0> for Document` validates every composable card's
kind the same way before calling `from_main_and_cards`. `TryFrom<CardWire>
for Card`, by contrast, does `payload.set_kind(wire.kind)` with no grammar
or reserved-word check at all — confirmed by `WireError` itself, which has
no variant for it (`InvalidQuillReference` / `InvalidField` only). A
binding round-tripping a `CardWire` can mint a `Card` whose kind is
`"Bad-Kind!"` or `"main"`; nothing catches it until (if ever) it's passed to
`push_card`/`insert_card`. Fed to `from_main_and_cards` instead (finding
above), it's never caught at all in a release build.

This may be intentional layering (wire is a raw carrier, the door is
`push_card`), matching `Card::from_parts`'s stance — but unlike
`from_parts`, `CardWire`'s doc doesn't say the kind is unchecked, and unlike
`from_parts` the type is meant for exactly this data path (bindings), where
a missing check is more likely to be reached by real, externally-controlled
input.

### `$ext` gets a full read/write quartet on `Card`; `$seed` only gets the namespace-scoped half

**Severity: Medium** — `edit.rs:583-689`

`Card` exposes `store_ext` / `remove_ext` (wholesale, depth-checked) *and*
`store_ext_namespace` / `remove_ext_namespace` (namespace-scoped,
depth-checked) — four methods covering both granularities. `$seed` (called
"the structural twin of `$ext`" in CARDS.md) only gets the namespace pair:
`store_seed_namespace` / `remove_seed_namespace`. There is no
`Card::store_seed` / `Card::remove_seed` wholesale pair, even though
`Payload::set_seed` / `Payload::take_seed` exist and are directly reachable
via `card.payload_mut().set_seed(map)` — bypassing `check_meta_depth`, the
§8 depth-bound check every other `Card`-level `$ext`/`$seed` writer applies.
A caller that wants to replace the whole `$seed` map atomically and
depth-checked has no method to call; they either loop per-kind through
`store_seed_namespace`, or drop to the unchecked `payload_mut()` escape
hatch that skips validation the sibling `$ext` methods perform.

### The "quill" concept is spelled four different ways across four sibling get/set pairs

**Severity: Low-Medium** — `mod.rs:428` (`quill_reference`), `edit.rs:314` (`set_quill_ref`), `mod.rs:190` (`Card::quill`), `payload.rs:408/360` (`Payload::set_quill`/`quill`)

- `Card::quill(&self) -> Option<&QuillReference>` — borrow, short name
- `Payload::quill(&self)` / `Payload::set_quill(&mut self, ..)` — short name, symmetric
- `Document::quill_reference(&self) -> QuillReference` — long name, **owned** (clones), panics if absent
- `Document::set_quill_ref(&mut self, ..)` — different abbreviation again (`_ref`, not `_reference`)

Four names for one concept on directly adjacent types in the same object
graph (`quill`, `set_quill`, `quill_reference`, `set_quill_ref`), and the
one place the name grows longest (`quill_reference`) is also the one that
silently changes calling convention (owned clone vs. `Option<&_>` borrow
everywhere else) and adds a panic path. A `Document::quill(&self) ->
Option<&QuillReference>` mirroring `Card::quill` would be more consistent
and let a caller check presence without an `.expect()` landmine.

### `Payload::len()` / `is_empty()` count only `Field` items, silently diverging from `items().len()`

**Severity: Low** — `payload.rs:337-347` vs `538-548`

`items()` returns every item (`$` entries, fields, comments); `len()` /
`is_empty()` count `Field` items only ("`$` entries and comments
excluded" — correctly documented, but easy to miss). `Payload` doesn't
implement `Deref<Target = [PayloadItem]>` or `Index`, so nothing forces
`len()` to agree with `items()`'s cardinality, but the two methods sitting
a few lines apart in the same "item-level access" vs "user-field access"
split, with the same name a `Vec`/slice user expects to mean "how many
things are in here," is a natural trip point for a caller who reaches for
`payload.len()` expecting the fence's raw line count.

### `prescan.rs` is fully `pub` with no consumer outside the crate; `yaml_hints.rs`, its structural twin, is correctly `pub(crate)`

**Severity: Low** — `prescan.rs:34,53,62,91` vs `yaml_hints.rs:22,35`

Both modules are internal parser helpers recovering YAML features
`serde_saphyr` drops (comments/tags vs. error enrichment). `yaml_hints.rs`
marks its `EnrichedYamlError` and `enrich_yaml_error` `pub(crate)` — correct,
since nothing outside `document/` uses them (confirmed: no external
references). `prescan.rs` marks `PreItem`, `PreScan`, and
`prescan_fence_content` fully `pub`, and a repo-wide search turns up no
consumer outside `crates/core/src/document` and one in-crate use in
`quill/blueprint.rs` — both of which only need `pub(crate)`. `NestedComment`
is the one item here that must stay externally visible (it leaks through
`PayloadItem`, see the finding above), but `PreItem`/`PreScan`/
`prescan_fence_content` don't need to.

## Cross-cutting

- `from_main_and_cards`'s debug-only invariant (finding 1) is only as safe
  as its callers in `crates/core/src/quill/{seed,compose,blueprint,validation}.rs`
  pre-validating every time — worth the `quill/` reviewer double-checking
  each call site actually does, since the constructor itself no longer will
  in a release build.
- `CardWire`'s missing `$kind` validation (finding 3) is inherited by
  whichever binding (`crates/bindings/{wasm,python}`) round-trips a bare,
  detached `CardWire` before placing it with `push_card`/`insert_card` —
  worth the bindings reviewer confirming there's no code path that skips
  that placement step (e.g., building a `Card` from wire and rendering it
  standalone).
- `EditError::ContentApply` / `apply_field_richtext_change` and
  `apply_body_change` assume `Content::apply_field_change` is all-or-nothing
  (edit.rs:899-901, 946-948) — that contract lives in `quillmark-content`,
  outside this scope; worth the content-crate reviewer confirming it holds.
- The frozen DTO tree in `dto.rs` (`CommentPathSegmentV0_92_0`,
  `PathStepWire`, `crate::value::PathSegment`) is three parallel
  encodings of "a path segment" by design (DOCUMENT_STORAGE.md's
  frozen-DTO-per-version rule) — flagged here only for completeness, not as
  a defect.
