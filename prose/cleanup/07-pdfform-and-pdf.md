# Cleanup review: pdfform backend + quillmark-pdf

Scope: `crates/backends/pdfform/src/{bind,flatten,form,lib,resolve,typography}.rs` (2430 LOC) + `tests/{sample_form,richtext_form,canvas_conformance,render_formats}.rs` (610 LOC); `crates/quillmark-pdf/src/{reader,stamp,writer,update,error,lib}.rs` (1820 LOC) + `tests/stamp.rs` (652 LOC). Total ~5510 LOC reviewed.

## Findings

### F1: `decode_pdf_text` + `widget` test helpers copy-pasted across two integration-test files
- **Category**: duplicate-helper
- **Location**: `crates/backends/pdfform/tests/sample_form.rs:48-75` and `crates/backends/pdfform/tests/richtext_form.rs:20-44`
- **Evidence**: `diff` on the two spans shows byte-identical bodies for `decode_pdf_text` (UTF-16BE-vs-Latin1 decode) and `widget` (linear scan of `/AcroForm/Fields` by `/T` name) — the only difference is a 3-line doc comment present in `sample_form.rs` and absent in `richtext_form.rs`. Both files independently import `lopdf::Document as PdfDoc` to support them. No third copy exists (`canvas_conformance.rs` and `render_formats.rs` don't reparse with lopdf).
- **Recommendation**: extract both functions into a shared `crates/backends/pdfform/tests/support.rs` (or `common.rs`, matching the precedent at `crates/bindings/wasm/tests/common.rs`) and `include!`/`mod`-import it from both test files.
- **Est. LOC removable**: ~24
- **Confidence**: high
- **Risk if removed**: none — pure test-helper consolidation, no behavior change.

### F2: Unbound `checkbox`/`choice` widgets are a documented wire-format feature with zero exercised call path
- **Category**: dead-code / speculative-feature
- **Location**: `crates/backends/pdfform/src/bind.rs:158-170` (`bind_unbound`), `:345-356` (`widget_type`, `Checkbox`/`Choice` arms); `crates/backends/pdfform/src/form.rs:106-116` (`WidgetKind::Checkbox`/`Choice`)
- **Evidence**: `form.json`'s `widgets` population supports `type: text|checkbox|choice|signature` per `docs/quills/pdfform-backend.md:129-141`. Grepped the whole workspace (fixtures, docs, migration guide, tests) for any `"widgets"` JSON array: the only occurrence anywhere — `crates/fixtures/resources/quills/sample_form/0.1.0/form.json`, `docs/quills/pdfform-backend.md`, `docs/migrations/0.93-to-0.94.md` — declares exactly one widget, `type: "signature"`. `bind_unbound`/`widget_type`'s `Checkbox` and `Choice` arms are reached by no fixture, no integration test, and no unit test in `bind.rs` (its test module binds only schema-bound `fields`, never a `widgets` array). `form.rs`'s `unbound_widgets_carry_every_kind` test only exercises JSON *parsing* of all four kinds, not the bind→resolve→stamp pipeline. `Text` unbound is likewise never bound end-to-end (only ever seen as a *bound* field in every fixture).
- **Recommendation**: either add a fixture/test exercising an unbound checkbox or choice widget through `bind_widgets` + `stamp`/`flatten`, or — if no real quill needs a signer-filled checkbox/dropdown — narrow the documented contract to `signature` (the only proven need) until a concrete use case appears.
- **Est. LOC removable**: ~15 (match arms + `WidgetKind` variants), but removal reduces a documented public contract — treat as a coverage gap first, a removal candidate only if product confirms no real use case.
- **Confidence**: medium (verified unexercised; not verified as unwanted)
- **Risk if removed**: breaks any external/future quill's `form.json` declaring an unbound checkbox/choice widget; the format is public and versioned (`form@0.2.0`).

### F3: `RenderSettings` construction duplicated between `render_rgba` and `render_png`
- **Category**: duplicate-helper
- **Location**: `crates/backends/pdfform/src/lib.rs:213-219` (`render_rgba`) and `:308-314` (`render_png`)
- **Evidence**: both build an identical `RenderSettings { x_scale: scale, y_scale: scale, bg_color: WHITE, ..Default::default() }` plus a `standard_font_settings()` call and a `HayroPdf::new(self.flat_pdf.clone())` parse — same four lines, twice, differing only in single-page-by-index (`render_rgba`) vs. all-pages-iterated (`render_png`).
- **Recommendation**: factor a small private helper (e.g. `fn render_settings(scale: f32) -> RenderSettings`) shared by both call sites.
- **Est. LOC removable**: ~6
- **Confidence**: medium
- **Risk if removed**: none — mechanical extraction.

### F4: `render_formats.rs`'s SVG/PNG tests check format shape, not that field values reached the artifact
- **Category**: low-value-test
- **Location**: `crates/backends/pdfform/tests/render_formats.rs:40-64` (`renders_svg_per_page`, `renders_png_per_page`)
- **Evidence**: `renders_svg_per_page` only asserts the bytes decode as UTF-8 and contain `"<svg"`; `renders_png_per_page` only checks the 8-byte PNG magic. Neither inspects whether `full_name`/`comments`/etc. values are actually baked into the raster/vector. `canvas_conformance.rs:99-121` already proves, for the *same* pre-flattened `self.flat_pdf` these two formats also render from, that the flatten pipeline bakes non-white ink into a field's region box — so the risk these two tests exist to catch (flatten silently producing a blank page) is already covered at the `render_rgba` seam. `png_ppi_controls_raster_size` (same file, :67-77) is a stronger test — it threads `ppi` through to byte-size — and should stay.
- **Recommendation**: either strengthen `renders_svg_per_page`/`renders_png_per_page` to assert field-value text/ink appears (e.g. reuse `canvas_conformance.rs`'s region-box ink check for PNG), or accept them as thin format-shape smoke tests and drop the redundant assumption that they validate content — no action strictly required, but don't add more tests of this shape to this file.
- **Est. LOC removable**: 0 (not proposing deletion — noting the coverage overlap)
- **Confidence**: low
- **Risk if removed**: losing them would drop the only test asserting the SVG document tag / PNG magic bytes for this backend specifically.

### F5: Two near-duplicate `BadSchema` tests
- **Category**: test-consolidation
- **Location**: `crates/backends/pdfform/src/form.rs:337-344` (`rejects_foreign_schema_tag`) and `:346-353` (`rejects_unknown_form_version_as_bad_schema`)
- **Evidence**: identical assertion shape (`matches!(FormSpec::parse(json), Err(FormParseError::BadSchema(_)))`) against two different bad `schema` strings (`"something/else@1"` vs `"quillmark/form@9.9.9"`) — same branch, same outcome, no distinguishing behavior tested.
- **Recommendation**: merge into one test iterating both strings (matching the table-driven style already used a few lines away in `dangling_root_and_segment_error` and `object_and_object_array_are_unbindable`).
- **Est. LOC removable**: ~6
- **Confidence**: medium
- **Risk if removed**: none.

### F6: Page-`/Annots`-splice (spine) and page-`/Contents`/`/Resources`-splice (pdfform) share a "find-key, splice-into-array-or-create" shape across the crate boundary
- **Category**: cross-crate-duplication (observation, not a clean removal)
- **Location**: `crates/quillmark-pdf/src/stamp.rs:278-320` (`rewrite_page_with_annots`) vs. `crates/backends/pdfform/src/flatten.rs:231-260` (`add_content_stream`) and `:272-335` (`add_font_resource`)
- **Evidence**: all three follow the same three-way branch — key absent → append fresh `[…]`; key present as an inline array → splice via `splice_dict_value`; key present as something else → reject/wrap. `add_content_stream` and `rewrite_page_with_annots` are near-identical in structure (~25 lines each); `add_font_resource` extends the pattern one level deeper into a nested `/Resources/Font` dict.
- **Recommendation**: not a clean unification — `/Annots` rejects an indirect reference outright (`pdf::indirect_annots`), while `/Contents` legitimately wraps a bare indirect ref into an array (a single-stream `/Contents` is common and valid), so the three-way branches don't fully agree. A shared "append ref to array-valued key" helper in `quillmark-pdf::writer` could still remove the array-splice half (~15 LOC) if someone touches this code again, but it isn't a standalone win today.
- **Est. LOC removable**: ~15 (only if bundled with other work in this area)
- **Confidence**: low
- **Risk if removed**: the semantic difference (annots-indirect-is-an-error vs. contents-indirect-is-fine) must survive any consolidation, or a real base PDF with a single-stream `/Contents` breaks.

## Load-bearing (looks redundant, is not)

- **The `quillmark-pdf` / `pdfform` two-crate split** — the review's central question. `quillmark-pdf` is *not* an internal-only layer for `pdfform`: `crates/backends/typst/src/{overlay/mod.rs, compile.rs, lib.rs}` independently import `quillmark_pdf::{FieldSpec, FieldType, CHECKBOX_ON_STATE, stamp, PdfError, StampOptions, regions_of}` to stamp Typst's own `form-field` overlay widgets onto compiled PDFs. Both backends genuinely collapse to the same `&[FieldSpec] -> stamped PDF` seam described in `quillmark-pdf/src/lib.rs`'s module doc, and `prose/canon/ARCHITECTURE.md:48-50` documents this explicitly. Removing the split would either duplicate the ~1000-line byte-level reader/incremental-update writer into two backends, or force the Typst backend to depend on `pdfform`'s bind/resolve/flatten machinery it doesn't need. Keep as-is.
- **`PdfUpdate` (`quillmark-pdf/src/update.rs`) shared open/close envelope** — used identically by `stamp()` (this crate) and `flatten()` (`pdfform/src/flatten.rs`), exactly to prevent the two incremental-update paths from drifting; this is the point of the crate, not incidental overlap.
- **`writer.rs`'s `dict_object`/`alloc_id`/`pdf_escape`/`winansi_encode`** — all consumed by both the stamp path (this crate) and the flatten path (`pdfform`); this is the documented single source of truth for PDF byte serialization, not a forwarding wrapper.
- **`resolve.rs`'s `lookup`/`descend` (data-path descent) vs. `bind.rs`'s `bind`/`descend` (schema-path descent)** — structurally parallel but operate on different types (`serde_json::Value` vs. `FieldSchema`) at different phases (per-render value extraction vs. once-at-load validation); this is the intentional two-phase "validate shape at load, extract value at render" design stated in both files' module docs, not duplicated logic.
- **`bind.rs`'s `SchemaType::Enum` arm in `project_kind`** — the comment correctly notes it's unreachable in practice (the loader guarantees `enum_values.is_some()` whenever `r#type == Enum`, and that case is caught earlier by the `enum_values` check), but it's required for match exhaustiveness — not dead code to delete.
