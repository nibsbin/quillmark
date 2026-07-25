# quillmark-content

## Surface

Crate root (`lib.rs`), re-exported at `quillmark_content::*`:

- `pub use delta::{diff_import, Assoc, Delta, Op};`
- `pub use export::{to_markdown, to_plaintext};`
- `pub use import::{from_markdown, from_plaintext};`
- `pub use island::KnownIslandType;`
- `pub use model::{Container, Invariant, Island, Line, LineKind, Loss, Mark, MarkKind, Content, Usv};`
- `pub use normalize::normalize_markdown;`
- `pub use ops::{change_bundle_from_value, line_op_from_value, line_op_to_value, mark_op_from_value, mark_op_to_value, ApplyError, LineOp, MarkOp};`
- `pub use serial::ParseError;`
- `pub const MAX_NESTING_DEPTH: usize = 100;` (lib.rs:65) — shared with the typst backend via `quillmark_core::error::MAX_NESTING_DEPTH`.
- All nine submodules (`delta`, `export`, `import`, `island`, `model`, `normalize`, `ops`, `serial`, `usv`) are `pub mod`, so every non-`pub(crate)` item inside is independently reachable at `quillmark_content::<module>::*`, not just through the root re-export.

`model.rs`:
- `pub type Usv = usize;` (model.rs:19)
- `pub const ISLAND_SLOT: char = '\u{FFFC}';` (model.rs:24)
- `pub struct Content { pub text: String, pub lines: Vec<Line>, pub marks: Vec<Mark>, pub islands: Vec<Island> }` — `Debug, Clone, PartialEq`, all fields public (model.rs:33-46).
- `pub struct Line { pub kind: LineKind, pub containers: Vec<Container>, pub continues: bool }` — `Debug, Clone, PartialEq, Eq` (model.rs:50-65).
- `pub enum LineKind { Para, Heading{level:u8}, Code{lang:Option<String>}, Island, Rule }` — `Debug, Clone, PartialEq, Eq` (model.rs:71-88).
- `pub enum Container { ListItem{ordered:bool,start:u64,ordinal:u64}, Quote }` — `Debug, Clone, PartialEq, Eq` (model.rs:92-111).
- `pub struct Mark { pub start: Usv, pub end: Usv, pub kind: MarkKind }` — `Debug, Clone, PartialEq` (no `Eq`) (model.rs:116-120).
- `pub enum MarkKind { Strong, Emph, Underline, Strike, Code, Link{url}, Anchor{id}, Unknown{tag,attrs} }` — `Debug, Clone, PartialEq` (no `Eq`) (model.rs:127-152).
  - `pub fn MarkKind::is_formatting(&self) -> bool` (model.rs:189)
  - `pub fn MarkKind::ord(&self) -> u8` (model.rs:203)
  - `pub fn MarkKind::attrs_key(&self) -> String` (model.rs:220)
- `pub struct Island { pub id: String, pub island_type: String, pub props: JsonValue, pub loss: Loss }` — `Debug, Clone, PartialEq` (model.rs:157-173).
- `pub enum Loss { Lossless, Degraded, Unrepresentable }` — `Debug, Clone, Copy, PartialEq, Eq` (model.rs:177-184).
- `pub enum Invariant { CarriageReturn, BidiControl(char), IslandSlotMismatch{..}, LineCountMismatch{..}, MarkOutOfRange{..}, ZeroWidthFormatting{..}, BadHeadingLevel(u8), FirstLineContinues, ReservedUnknownTag(String), MarkEdgeOnNewline{..}, TableAlignsMismatch{..}, TableRaggedRow{..}, TableCellNewline{..}, IslandIdCollision{..}, AnchorIdCollision{..}, TableHeaderNotArray }` — `Debug, Clone, PartialEq, Eq`, no `Display`/`Error` impl (model.rs:305-355).
- `impl Content`: `empty() -> Self`, `len_usv(&self) -> Usv`, `is_inline(&self) -> bool`, `is_plain(&self) -> bool`, `is_blank(&self) -> bool`, `segment_count(&self) -> usize`, `normalize(&mut self)`, `pub const RESERVED_MARK_TYPES: [&'static str; 7]`, `validate(&self) -> Result<(), Invariant>` (model.rs:358-604).

`import.rs`:
- `pub enum ImportError { NestingTooDeep{depth,max} }` — `Display + Error` (import.rs:64-78).
- `pub fn from_markdown(markdown: &str) -> Result<Content, ImportError>` (import.rs:81)
- `pub fn from_plaintext(s: &str) -> Content` (import.rs:116)

`export.rs`:
- `pub fn to_markdown(rt: &Content) -> String` (export.rs:35)
- `pub fn to_plaintext(rt: &Content) -> String` (export.rs:68)
- `pub struct Segment { pub start, end, byte_start, byte_end, slots_before: usize }` — `Debug, Clone, Copy, PartialEq, Eq` (export.rs:81-92)
- `pub fn line_segments(rt: &Content) -> Vec<Segment>` (export.rs:95) — consumed by `backends/typst/src/emit.rs:335`.
- `pub fn clip_range_to_atomic(start: &mut usize, end: &mut usize, atomics: &[(usize, usize)])` (export.rs:709) — consumed by `backends/typst/src/emit.rs:772`.

`serial.rs`:
- `pub enum ParseError { Shape(&'static str), Json(String), Invalid(Invariant) }` — `Display + Error` (serial.rs:23-41).
- `impl Content`: `to_canonical_json(&self) -> String`, `from_canonical_json(s: &str) -> Result<Content, ParseError>` (serial.rs:49-61).
- `pub fn to_canonical_value(rt: &Content) -> Value`, `pub fn from_canonical_value(v: &Value) -> Result<Content, ParseError>` (serial.rs:116-132).
- `pub fn line_kind_to_value/line_kind_from_value`, `container_to_value/container_from_value`, `mark_to_value/mark_from_value` (serial.rs:146-356) — the shared wire vocabulary `ops.rs` reuses for the op wire and the typst backend reuses for `parse_cell`.
- `pub fn parse_cell(v: &Value) -> (String, Vec<Mark>)` (serial.rs:371) — consumed by `backends/typst/src/emit.rs:972`.

`island.rs`:
- `pub enum KnownIslandType { Table, Image }` — `Debug, Clone, Copy, PartialEq, Eq` (island.rs:26-32).
  - `as_str(self) -> &'static str`, `parse(s: &str) -> Option<Self>`, `default_loss(self) -> Loss`, `cell_marks(self, props: &Value) -> Vec<(String, Vec<Mark>)>`, `normalize_props(self, props: &mut Value)`, `shape_error(self, props: &Value) -> Option<Invariant>` (island.rs:36-92).

`delta.rs`:
- `pub struct Delta { pub ops: Vec<Op> }` — `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` (delta.rs:51-54).
- `pub enum Op { Retain(usize), Insert(String), Delete(usize) }` — same derives, externally tagged lowercase (delta.rs:58-67).
- `pub enum Assoc { Before, After }` (delta.rs:71-78).
- `pub struct BaseLengthMismatch { pub expected: usize, pub actual: usize }` — `Debug, Clone, Copy, PartialEq, Eq`, no `Display`/`Error` (delta.rs:82-86).
- `impl Delta`: `expected_base_len(&self) -> Usv`, `apply(&self, base: &str) -> String` (**panics** on over-long delta), `try_apply(&self, base: &str) -> Result<String, BaseLengthMismatch>`, `map_pos(&self, pos: Usv, assoc: Assoc) -> Usv` (delta.rs:91-190).
- `pub fn diff(base: &str, new: &str) -> Delta` (delta.rs:263)
- `pub fn diff_import(base: &Content, new_markdown: &str) -> Result<(Content, Delta), ImportError>` (delta.rs:357)

`ops.rs`:
- `pub enum MarkOp { Add{start,end,kind}, Remove{start,end,kind}, RemoveAnchor{id} }` — `Debug, Clone, PartialEq` (no `Eq`) (ops.rs:21-45).
- `pub enum LineOp { Split{at}, Join{line}, SetKind{line,kind}, SetContainers{line,containers}, SetContinues{line,continues} }` — `Debug, Clone, PartialEq` (no `Eq`) (ops.rs:50-72).
- `pub fn mark_op_to_value/mark_op_from_value`, `line_op_to_value/line_op_from_value` (ops.rs:92-234).
- `pub fn change_bundle_from_value(v: &Value) -> Result<(Delta, Vec<LineOp>, Vec<MarkOp>), String>` (ops.rs:245) — consumed by `bindings/wasm/src/engine.rs:1901`.
- `pub enum ApplyError { MarkOutOfRange{..}, LineOutOfRange{..}, SplitPositionOutOfRange{..}, SplitAtNewline{..}, LineCountMismatch{..}, FirstLineContinues, DeltaBaseMismatch{..}, IslandSlotInInsert, AnchorIdCollision{..}, EmptyAnchorId }` — `Debug, Clone, PartialEq, Eq`, no `Display`/`Error` impl (ops.rs:286-333).
- `impl Content`: `apply_text_delta(&mut self, delta: &Delta) -> Result<(), ApplyError>`, `apply_mark_ops(&mut self, ops: &[MarkOp]) -> Result<(), ApplyError>`, `apply_line_ops(&mut self, ops: &[LineOp]) -> Result<(), ApplyError>`, `apply_field_change(&mut self, text_delta: &Delta, line_ops: &[LineOp], mark_ops: &[MarkOp]) -> Result<(), ApplyError>` (ops.rs:351-614) — consumed by `crates/core/src/document/edit.rs:909,947`.

`normalize.rs`:
- `pub fn strip_bidi_formatting(s: &str) -> String` (normalize.rs:34)
- `pub fn fix_html_comment_fences(s: &str) -> String` (normalize.rs:48)
- `pub fn normalize_markdown(markdown: &str) -> String` (normalize.rs:115) — the only one of the three re-exported at the crate root; `strip_bidi_formatting`/`fix_html_comment_fences` are public but not re-exported, reachable only via `quillmark_content::normalize::*`.

`usv.rs`:
- `pub fn char_to_byte(text: &str, char_idx: usize) -> usize` (usv.rs:13)

No `pub` item in this crate looks like a stray internal helper: every public function outside `lib.rs`'s curated re-export list (`line_segments`, `clip_range_to_atomic`, `parse_cell`, the `serial` wire converters, `strip_bidi_formatting`, `fix_html_comment_fences`) is confirmed consumed by the typst backend, a binding, or `ops.rs` itself — see Cross-cutting.

## Findings

### `export::to_markdown` has no container-nesting depth guard; `import`'s `MAX_NESTING_DEPTH` check has no counterpart on the way out
`export.rs:145-226` (`emit_block`/`emit_container` mutually recurse once per container-path depth); `model.rs:487-604` (`Content::validate` has no nesting-depth `Invariant` variant). Severity: **High**.

`import::Builder::check_depth` (import.rs:423-434) rejects markdown whose container+mark nesting exceeds `MAX_NESTING_DEPTH` (100) with `ImportError::NestingTooDeep`, and the typst backend's emitter defensively re-checks the same constant against a hand-built `Content` (`backends/typst/src/emit.rs:266`, comment: "fires only on a hand-built content" since import already capped it). `Content::validate()` has no equivalent invariant, so a `Content` built by hand, or round-tripped through `Content::from_canonical_json`/`from_canonical_value` (both of which call `validate()` but never check depth), can carry an arbitrarily deep `Line::containers` path. `to_markdown`'s `emit_block`/`emit_container` pair recurses one stack frame per depth level with no cap. A caller that loads untrusted or generated JSON straight into `Content` (the exact shape `from_canonical_json` is meant to accept) and calls `to_markdown` on it can stack-overflow the process, where the typst backend on the identical input degrades gracefully to `ConversionError::NestingTooDeep`. This is the one place the crate's own two producers of a validated `Content` (import, and canonical-JSON parse) disagree on what "validated" bounds, and the one exporter that doesn't check.

### `to_markdown` silently discards `MarkKind::Unknown`, unlike the deliberate, documented omission of `MarkKind::Anchor`
`export.rs:465-479` (`render_inline`'s per-mark classification), `export.rs:762-777` (`render_cell_md`, the table-cell twin), `export.rs:790-811` (`delim_open`/`delim_close`). Severity: **Medium**.

`MarkKind::Anchor` is explicitly filtered out of the markdown projection with a one-line, documented reason ("omitted from the projection" — export.rs:474, and the module doc at export.rs:1-7 states this by design). `MarkKind::Unknown` — the crate's other identity-class, non-formatting mark kind (`is_formatting()` at model.rs:189 excludes both) and the one the model doc calls "round-tripped opaque" (model.rs:147-152) — has no such arm. It falls through the catch-all `_ => fmt.push((s, e, &m.kind))` (export.rs:477, and identically at export.rs:775), which treats it as a nestable formatting wrap for the mark sweep. `delim_open`/`delim_close` then return `String::new()` for it (the `_` arm at export.rs:799/809), so the wrapped text is emitted with no delimiters at all — the mark's range/kind/attrs vanish from the markdown with no comment placeholder (unlike an unrepresentable island, which emits `<!-- island:… -->`) and no `Loss` signal. Because the mark still occupies a slot in the sweep's open/close stack (it participates in `opening.sort_by`'s longest-span-first ordering, export.rs:592-593), its mere presence can also reorder the delimiters of neighboring `Strong`/`Emph`/etc. marks it overlaps, even though it renders nothing itself. `MarkKind::Unknown` is exactly the type `serial.rs` round-trips byte-for-byte through storage (`unknown_mark_round_trips_opaque`, serial.rs:695), so a caller that persists a `Content` carrying an `Unknown` mark, exports it to markdown for preview, then re-imports, loses the mark — silently, with no test in this crate exercising that path.

### `ApplyError`, `BaseLengthMismatch`, and `Invariant` don't implement `Display`/`std::error::Error`, unlike `ImportError` and `ParseError` in the same crate
`ops.rs:286-333`, `delta.rs:82-86`, `model.rs:304-355` vs. `import.rs:63-78`, `serial.rs:22-41`. Severity: **Medium**.

Two of the crate's five public error types (`ImportError`, `ParseError`) hand-implement `Display` and `std::error::Error`. The other three — `ApplyError` (the type every mutator in `ops.rs` returns), `BaseLengthMismatch` (returned by `Delta::try_apply`), and `Invariant` (returned by `Content::validate`, and wrapped — not forwarded — inside `ParseError::Invalid` via `{inv:?}` Debug formatting rather than a real `Display`) derive only `Debug`/`PartialEq`/`Eq`. A caller in a binding (`bindings/wasm`, `bindings/python`) that wants one uniform `Box<dyn std::error::Error>` boundary for every fallible call into this crate must special-case three of the five error enums, and any caller that wants a human-readable message for an `ApplyError` (the most common runtime failure — bad range, wrong revision, first-line-continues) has to hand-roll its own match rather than `.to_string()`.

### `LineOp::SetKind` performs no validation of its payload before mutating, unlike every other op in the same module
`ops.rs:553-556` (`apply_line_ops_inner`'s `SetKind` arm) vs. `ops.rs:551,562,567-568` (`Split`, `Join`, `SetContinues`, all of which validate before touching `self`/`scratch`). Severity: **Medium**.

Every other line/mark op in `ops.rs` rejects an invalid payload before mutating: `Split` checks the position isn't adjacent to a `\n` (`SplitAtNewline`); `Join` checks the line index; `SetContinues` refuses `continues: true` on line 0 (`FirstLineContinues`) because "`normalize` does not repair it" (ops.rs:307-310); `MarkOp::Add`/`Remove` bounds-check the range and reject zero-width formatting, a colliding anchor id, or the empty id. `LineOp::SetKind { line, kind }` (ops.rs:553-556) writes `kind` straight into `self.lines[line]` with no check that, e.g., a `LineKind::Heading { level: 0 }` or `{ level: 200 }` is in the `1..=6` range `Content::validate` requires (`Invariant::BadHeadingLevel`, model.rs:557-561) — and `normalize()` (called by the public `apply_line_ops` wrapper) doesn't repair a bad heading level either, so `apply_line_ops(&[SetKind{..., kind: Heading{level: 200}}])` returns `Ok(())` over a `Content` that will fail `validate()`. The JSON wire path is actually stricter than the Rust API here: `line_kind_from_value` (serial.rs:175-196), used to decode a `SetKind` op arriving over `change_bundle_from_value`, does enforce the `1..=6` range and returns `ParseError::Shape("heading level")` for an out-of-range value — so a caller going through the wire is protected, but a caller constructing `LineOp::SetKind` directly in Rust (the primary, non-binding entry point) is not.

### Anchor id uniqueness/non-emptiness is validated only for prose marks, not for the same `MarkKind::Anchor` inside a table cell
`model.rs:518-522,548-551` (`validate`'s `seen_anchor_ids` loop scoped to `self.marks`) vs. `model.rs:582-600` (the per-island cell-mark loop, which checks `MarkOutOfRange`/`ZeroWidthFormatting`/`ReservedUnknownTag` but not anchor identity). Severity: **Low**.

This is called out as scoped-by-design in the `AnchorIdCollision` doc comment ("Scope is prose marks — cell anchors are outside the op surface," model.rs:349), so it is not a defect against the crate's own contract. It is worth flagging because nothing in the type system or in `Content::normalize`/`validate` stops a caller from placing a `MarkKind::Anchor` inside a table cell's `{text, marks}` JSON (cells accept the same `Mark` wire shape as prose, `serial.rs:358-365`) — `normalize_marks` still dedupes a byte-identical duplicate anchor within one cell (it runs the same Spike-A rules per cell), but two *different* cells (or a cell and the prose body) can carry the same anchor `id`, or the empty id, and `validate()` will accept it. `DOCUMENT_STORAGE.md`'s "Anchor-id identity" contract (uniqueness, non-empty, per-`Content`) is therefore enforced only for the subset of anchors the op surface can produce, not for the full set of anchors the wire format can carry.

## Cross-cutting

- **Import/export are otherwise a coherent pair.** Every markdown construct import produces (paragraphs, headings 1-6, code fences, lists/ordinals, quotes, tables with formatted cells, images, `strong`/`emph`/`underline`/`strike`/`code`/`link`) has a matching export arm, verified by `export.rs`'s own `round_trips` test helper and the `properties.rs` proptest suite; the two documented, intentional exceptions (`MarkKind::Anchor` omitted by design, and the two "documented codec limits" in export.rs:15-28 — a mark spanning a hard break, an empty first line of a hard-break block) are recorded in the module doc, not hidden. The `MarkKind::Unknown` gap above is the one asymmetry that isn't documented anywhere.
- **`serial`/`import`/`export`/`ops` are one wire vocabulary reused three ways, not three dialects.** `ops.rs`'s `mark_op_to_value`/`line_op_to_value` and friends explicitly build on `serial::{mark_to_value, mark_from_value, line_kind_to_value, line_kind_from_value, container_to_value, container_from_value}` (ops.rs:84-87) rather than forking the encoding; `export::render_cell_md` and the typst backend's `parse_cell` call both read the identical `{text, marks}` cell shape `serial.rs` writes. This part of the design holds together well.
- **`delta`/`ops` and `quillmark_core::session` are one edit vocabulary, not two.** `crates/core/src/session.rs:4` re-exports `quillmark_content::{ApplyError, Assoc, Delta, LineOp, MarkOp, Op}` directly rather than defining parallel core-side types; core's own `ChangeSet` (session.rs:8-15) is a disjoint concept (post-render dirty-page reporting), not a competing edit representation. `crates/core/src/document/edit.rs:909,947` calls `Content::apply_field_change` directly. No drift found here.
- **`Content`'s four fields (`text`, `lines`, `marks`, `islands`) are all `pub` with no invariant enforcement at construction** (`LineKind::Heading{level}`, `Mark{start,end,..}`, `Container::ListItem{..}` etc. are plain struct/enum literals). This is deliberate and documented ("the freeze... hand-built content should be run through it in tests," model.rs:1-10), but it means every consuming crate (`core`, both backends, both bindings) is individually responsible for calling `validate()` after hand-building or deserializing a `Content` outside `from_markdown`/`from_canonical_json` (which do call it); a consumer that skips it inherits whatever `to_markdown`/the typst emitter do with an invalid value, including the stack-overflow risk noted above.
- **`is_inline`/`is_plain`** (model.rs:377-408) are the predicates `quillmark_core`'s schema validation (`richtext(inline)` / `plaintext` field kinds, per the doc comments' own cross-reference) depends on for its coercion rules; a change to either predicate's exact semantics is a cross-crate contract change, not a local one.
