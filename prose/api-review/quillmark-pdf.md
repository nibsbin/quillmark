# quillmark-pdf

## Surface

Crate root (`lib.rs`), re-exported:

- `pub use error::PdfError;`
- `pub use stamp::{regions_of, stamp, StampOptions, CHECKBOX_ON_STATE};`
- `pub use update::PdfUpdate;`
- `pub mod reader;` / `pub mod writer;` — both fully public modules; every non-`pub(crate)` item inside is part of the crate's API, not just internal wiring.
- `pub fn page_media_boxes(base: &[u8]) -> Result<Vec<[f32; 4]>, PdfError>` (lib.rs:41)
- `pub struct FieldSpec { name: String, schema_field: Option<String>, page: usize, rect: [f32; 4], field_type: FieldType, value: Option<String>, tooltip: Option<String> }` — `Debug, Clone, PartialEq` (lib.rs:52)
- `pub enum FieldType { Text { multiline: bool }, Checkbox, Choice { options: Vec<String> }, Signature }` — `Debug, Clone, PartialEq` (lib.rs:79)

`error.rs`:
- `pub struct PdfError { pub code: &'static str, pub message: String }` — `Debug, Clone, PartialEq, Eq, thiserror::Error` (error.rs:13)
- `pub fn PdfError::new(code: &'static str, message: impl Into<String>) -> Self` (error.rs:22)

`stamp.rs`:
- `pub const CHECKBOX_ON_STATE: &str = "Yes"` (stamp.rs:36)
- `pub struct StampOptions { pub producer: Option<String> }` — `Debug, Clone, Default` (stamp.rs:44)
- `pub fn stamp(base: Vec<u8>, fields: &[FieldSpec], opts: &StampOptions) -> Result<Vec<u8>, PdfError>` (stamp.rs:60)
- `pub fn regions_of(fields: &[FieldSpec]) -> Vec<RenderedRegion>` (stamp.rs:171) — `RenderedRegion` from `quillmark_core`.

`update.rs`:
- `pub struct PdfUpdate { pub catalog_id: u32, pub next_id: u32, pub objects: Vec<UpdatedObject>, /* private: xref_offset, extra_info_ref */ }` (update.rs:25)
- `pub fn PdfUpdate::begin(pdf: &[u8], producer: Option<&str>) -> Result<Self, PdfError>` (update.rs:45)
- `pub fn PdfUpdate::resolve_pages(&self, pdf: &[u8], fields: &[FieldSpec]) -> Result<Vec<u32>, PdfError>` (update.rs:95)
- `pub fn PdfUpdate::finish(self, pdf: Vec<u8>) -> Result<Vec<u8>, PdfError>` (update.rs:131)

`reader.rs` (module is `pub`; these are the non-`pub(crate)` items, so all reachable at `quillmark_pdf::reader::*`):
- `pub fn err(code: &'static str, msg: impl Into<String>) -> PdfError` (reader.rs:25)
- `pub struct UpdatedObject { pub id: u32, pub bytes: Vec<u8> }` (reader.rs:97)
- `pub fn find_object_bytes(pdf: &[u8], id: u32) -> Option<(usize, usize)>` (reader.rs:180)
- `pub fn find_dict_value<'a>(dict_bytes: &'a [u8], key: &str) -> Option<&'a [u8]>` (reader.rs:288)
- `pub fn splice_dict_value(dict: &[u8], key: &[u8], value: &[u8], new_value: &[u8]) -> Vec<u8>` (reader.rs:327)
- `pub fn extract_outer_dict(obj_bytes: &[u8]) -> Option<&[u8]>` (reader.rs:545)
- Everything else (`find_startxref`, `assert_traditional_xref`, `find_trailer_dict`, `append_incremental_update`, `object_generation`, `assert_overwrite_gen_zero`, `parse_indirect_ref`, `resolve_page_ids`, `assert_unrotated_page`, `parse_ref_array`, `page_media_boxes`) is `pub(crate)`.

`writer.rs` (module is `pub`):
- `pub fn dict_object(id: u32, inner: &[u8]) -> UpdatedObject` (writer.rs:18)
- `pub fn alloc_id(next: &mut u32) -> Result<u32, PdfError>` (writer.rs:28)
- `pub fn pdf_escape(out: &mut Vec<u8>, bytes: &[u8])` (writer.rs:40)
- `pub fn winansi_encode(s: &str) -> Vec<u8>` (writer.rs:169)
- `pdf_text_string`, `upsert_producer`, `apply_producer_stamp`, `winansi_byte` are `pub(crate)`, not on the surface.

## Findings

### `reader::find_dict_value`/`splice_dict_value`/`extract_outer_dict`/`find_object_bytes`/`UpdatedObject`/`err` are unrestricted `pub`, turning an internal byte scanner into published API
`reader.rs:25,97,180,288,327,545`. `writer.rs:18,28,40,169`. Severity: **Medium**.

The crate doc (lib.rs:1-22) frames this crate as the stamp spine with one operation; reader.rs's own module doc calls itself "a deliberately small scanner," explicitly not a general parser. But because `reader`/`writer` are `pub mod` (not `pub(crate) mod` with a narrower re-export), every non-`pub(crate)` item inside is exported to any external Cargo consumer of `quillmark-pdf` (the crate is published — `documentation = "https://docs.rs/quillmark-pdf"`). This is deliberate for one specific consumer: `crates/backends/pdfform/src/flatten.rs` imports exactly this set (`reader::{err, extract_outer_dict, find_dict_value, find_object_bytes, splice_dict_value, UpdatedObject}`, `writer::{alloc_id, dict_object, pdf_escape, winansi_encode}`) to build a third, sibling incremental-update path without duplicating the byte scanner. That in-workspace reuse is a legitimate reason to share code, but exposing it via ordinary `pub` rather than a crate-scoped visibility (`pub(crate)` + a `pub(in crate::…)` friend path, or a `#[doc(hidden)] pub` with a stability note) means any external user of this crate now has "primitive PDF dict/xref scanner" as part of its semver-covered contract, with no documented invariants for outside callers (e.g. `find_dict_value` returns a raw undecoded value slice; the parsers that would make that slice useful — `parse_indirect_ref`, `parse_ref_array` — are `pub(crate)` and *not* reachable, so an external caller who finds a `/Pages` value via the public `find_dict_value` cannot resolve it without reimplementing indirect-ref parsing). Caller impact: docs.rs will list a "reader"/"writer" module as first-class API; anyone depending on it beyond the intended sibling crate has no contract to code against and no way to actually parse most of what they can locate.

### `map_pdf_err` is duplicated verbatim in both backends despite `quillmark-pdf` already depending on `quillmark-core`
`crates/backends/typst/src/compile.rs:24-28` and `crates/backends/pdfform/src/lib.rs:355-359`. Severity: **Medium**.

Both functions are byte-identical:
```rust
fn map_pdf_err(e: PdfError) -> RenderError {
    RenderError::from_diag(Diagnostic::new(Severity::Error, e.message).with_code(e.code.to_string()))
}
```
`error.rs:1-7`'s doc comment justifies the separate `PdfError` type by saying a `From<PdfError> for RenderError` conversion "would invert the dependency (a leaf crate shaping a core type)" — but `quillmark-pdf` already depends on `quillmark-core` (Cargo.toml:14, used for `RenderedRegion`), and `PdfError` is a local type to this crate, so `impl From<PdfError> for RenderError` here is a legal, non-orphan-violating impl that adds no new dependency edge. The stated rationale doesn't hold; the actual effect is this exact 5-line mapping function hand-copied into every backend, with no compiler-enforced guarantee the two stay in sync. Caller impact: a third backend (or a future binding) needing this conversion writes it a third time; a future `Diagnostic` field (e.g. `path`, `location`) added at one call site and not the other silently diverges.

### `PdfUpdate`'s `next_id`/`objects` are bare public fields with no invariant enforcement
`update.rs:25-37`. Severity: **Low**.

`PdfUpdate` is the shared low-level envelope stamp.rs and `pdfform`'s flatten.rs both build on (module doc, update.rs:1-9). `catalog_id`, `next_id`, and `objects` are plain `pub` fields, not accessed through methods. `next_id` is meant to be advanced only via `writer::alloc_id(&mut next, …)`, which increments monotonically and checks for overflow — but nothing stops a caller from writing `up.next_id = 0` or pushing an `UpdatedObject` with a colliding `id` directly onto `.objects`, producing a corrupt incremental update (a duplicate object id) with no error from `PdfUpdate` itself. Caller impact: confined today to two in-workspace call sites that use the type correctly, but as a `pub` type reachable from outside the workspace it's an unguarded footgun with no documented "don't touch these" contract beyond field-level doc comments.

### `reader::err` duplicates `PdfError::new` under a second public name
`reader.rs:25`. Severity: **Low**.

```rust
pub fn err(code: &'static str, msg: impl Into<String>) -> PdfError {
    PdfError::new(code, msg)
}
```
Forwards to the constructor already on `PdfError` (error.rs:22) with no behavior difference. Two spellings — `quillmark_pdf::PdfError::new` and `quillmark_pdf::reader::err` — construct the same value; `reader::err` exists only because it's ergonomic to `use` alongside the other reader helpers `flatten.rs` imports. Not harmful, but it's public-API surface area for a rename of `PdfError::new`.

### `FieldSpec::rect` isn't validated or corner-normalized before it reaches the widget writer
`stamp.rs:255-260` (`write_widget_object`, feeding `spec.rect` straight into `pdf_writer::Rect::new`). Severity: **Low**.

`page_media_boxes`/`reader::page_media_boxes` explicitly normalizes `/MediaBox` corners (`reader.rs:734-741`, `normalize_rect`) because a source PDF can list corners in either order. `FieldSpec::rect` carries the same "PDF points, bottom-left origin, `[x0,y0,x1,y1]`" contract (lib.rs:64) but `stamp()` never checks `x0 <= x1 && y0 <= y1`, and there's no normalization on the write side. A caller-computed rect with swapped corners (e.g. a geometry-flip bug in a producer) silently yields a degenerate or inverted widget box with no diagnostic, instead of the clean rejection the crate otherwise favors (rotated pages, xref streams, encrypted PDFs are all hard errors).

## Cross-cutting

- `PdfError` (this crate) and `quillmark_core::error::{Diagnostic, RenderError}` are deliberately parallel and non-convertible by design (error.rs:1-7); every backend hand-writes its own `map_pdf_err` at the boundary. See finding above — the stated "would invert the dependency" rationale doesn't hold given the existing `quillmark-pdf → quillmark-core` edge, and the duplication is real and visible in `crates/backends/typst/src/compile.rs` and `crates/backends/pdfform/src/lib.rs`.
- `reader`/`writer` being fully `pub` (not scoped to the workspace) is consumed today only by `crates/backends/pdfform/src/flatten.rs`; any tightening of their visibility is a decision for that crate's boundary as much as this one's.
- `crates/backends/typst/src/overlay/mod.rs` and `crates/backends/pdfform/src/{resolve,bind}.rs` construct `FieldSpec`/`FieldType` directly — both backends are equally exposed to the `rect`-normalization gap noted above, since neither backend re-validates corner order before calling `stamp()`.
- Confirmed against `docs/quills/pdfform-backend.md`: the "Typst-free," "fresh AcroForm only, never reconciled," "regions is a session-level query decoupled from `RenderResult`," and "flatten is a separate lossy path, never touching the AcroForm PDF" design points are all accurately reflected in the code and are not flagged as defects.
