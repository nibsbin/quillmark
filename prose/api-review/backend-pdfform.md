# Backend: pdfform

## Surface

`crates/backends/pdfform` declares its five internals as private modules
(`mod bind; mod flatten; mod form; mod resolve; mod typography;` — `lib.rs:17-21`),
none re-exported. Every `pub` item inside those files (`BoundWidget`, `BindError`,
`FormSpec`, `BoundField`, `WidgetKind`, `Rect`, `FormParseError`, `bind_widgets`,
`bind`, `project_kind`, `flatten`, `field_spec`, `SCHEMA_PREFIX`, `SCHEMA_VERSION`,
…) is consequently unreachable from outside the crate — `pub` there means
crate-visible, not `quillmark_pdfform::`-visible. `typography.rs`'s constants are
`pub(crate)`, doubly private.

The crate's entire externally-visible API is:

```rust
pub struct PdfformBackend;                      // unit struct, Debug + Default (lib.rs:54-55)

impl Backend for PdfformBackend {
    fn id(&self) -> &'static str;                                              // "pdfform"
    fn supported_formats(&self) -> &'static [OutputFormat];                    // [Pdf, Svg, Png]
    fn open(&self, source: &Quill, json_data: &serde_json::Value)
        -> Result<LiveSession, RenderError>;
}
```

No feature flags — `Cargo.toml` has no `[features]` table; `hayro`/`hayro-svg`
(the SVG/PNG raster path) are unconditional dependencies, per the Cargo.toml
comment explaining WASM `core`'s no-features build excludes the whole crate
rather than gating within it.

`open` returns `LiveSession` (an opaque `Box<dyn SessionHandle>` from
`quillmark-core`) — no `quillmark-pdf` type (`FieldSpec`, `PdfError`,
`StampOptions`, `CHECKBOX_ON_STATE`) appears in any public signature. The
backend consumes `quillmark-pdf` as pure implementation plumbing and none of it
leaks. Same for `quillmark-content` (used only inside `resolve.rs` to lower
richtext to plaintext) and `hayro`/`hayro-svg` (rasterization, `lib.rs` only).

This is the tightest possible opacity for a `Backend` impl — no findings on
surface bloat. Everything below is behavioral/diagnostic, not shape.

## Findings

### `RenderOptions.pages` is silently ignored across every output format
**Severity: High** — `lib.rs:152-192` (`render`), `lib.rs:269-294` (`render_svg`), `lib.rs:299-333` (`render_png`)

`SessionHandle::render` never reads `opts.pages`. `core`'s own doc comment on
`RenderOptions::pages` (`crates/core/src/types.rs:139-146`) specifies the
contract every backend is expected to honor: `None` renders all pages, `Some`
selects a subset, an out-of-range index fails with an error code, and a format
that can't do partial output (PDF) fails when `pages` is `Some`. The Typst
backend implements this exactly (`crates/backends/typst/src/compile.rs:83-122`:
`typst::pdf_page_selection_not_supported` for PDF+`Some`, `typst::page_index_out_of_bounds`
for an out-of-range index, real filtering for SVG/PNG). `pdfform` does none of
it: `stamp()` always stamps every page, `render_svg`/`render_png` always
iterate `pdf.pages().iter()` in full, and an out-of-range index is never
checked. A caller that requests `pages: Some(vec![0])` against a multi-page
pdfform quill silently gets every page back — no error, no truncation, no
diagnostic. No test in `tests/*.rs` exercises `opts.pages` against this
backend, so the gap has no regression coverage either. Caller impact: a UI
that paginates a large stamped form (or that assumes the documented
out-of-bounds/PDF-selection errors fire, per the shared `RenderOptions` doc)
gets silently wrong output instead of an error or the requested subset.

### `format_not_supported` is a different code on each backend
**Severity: Medium** — `lib.rs:160`

pdfform's unsupported-format diagnostic carries `pdfform::format_not_supported`.
Typst's equivalent check (`crates/backends/typst/src/lib.rs:297`) carries
`backend::format_not_supported` — and a second, seemingly-dead copy in
`crates/backends/typst/src/compile.rs:189` carries a third code,
`typst::format_not_supported`. `ERROR.md` lists `backend::*` as a namespace
precisely for backend-generic conditions like this one, so a caller that
matches on `backend::format_not_supported` to build one "requested format X,
this backend doesn't do X" UI path works for Typst but silently falls through
for pdfform, which never emits that code. Either both backends should emit
`backend::format_not_supported` for this shared, generic condition, or the
canon should stop listing it as the generic case.

### Leaf `pdf::*` codes cross the backend boundary unwrapped
**Severity: Medium** — `lib.rs:356-360` (`map_pdf_err`)

`map_pdf_err` forwards `PdfError.code` verbatim onto the surfaced `Diagnostic`
(`.with_code(e.code.to_string())`). `quillmark-pdf`'s own codes are namespaced
`pdf::*` (`pdf::xref_stream`, `pdf::flatten_parse` — `crates/quillmark-pdf/src/error.rs:14`,
`flatten.rs:35`), which is not in `ERROR.md`'s enumerated namespace list
(`parse::*, validation::*, quill::*, edit::*, typst::*, pdfform::*, backend::*,
engine::*`). A malformed `form.pdf` background, or a stamp/flatten failure at
render time, therefore reaches the caller as `pdf::xref_stream` or
`pdf::flatten_parse` rather than a `pdfform::*`-prefixed code, breaking the
"one namespace per producing stage" contract every other pdfform error
(`bind::BindError::code()`, `form::FormParseError::code()`) honors. This is
identical, not asymmetric, in the Typst backend (`compile.rs:22-28` does the
same forward-as-is), so it's a shared-boundary defect, not pdfform-specific —
see Cross-cutting.

### Diagnostics never carry `Location` or `path`, only message text
**Severity: Low** — `lib.rs:363-366` (`engine_err`), `bind.rs:81-109` (`BindError::fmt`)

Every pdfform diagnostic is built via `Diagnostic::new(severity, message).with_code(...)`
— none call `.with_location(...)` or `.with_path(...)`, both of which
`quillmark_core::Diagnostic` supports and which the Typst backend's
`error_mapping.rs` uses (span → file/line/column via `.with_location`). This
is defensible where there's genuinely no source position (a binary PDF byte
offset isn't a useful "location" for a quill author) — but `BindError::Dangling`
and `BindError::Unbindable` already carry a structured `path`/`segment` a
`schema_field` string that names an exact, addressable location in
`Quill.yaml`'s field tree. That structured data is stringified straight into
the message rather than riding `.with_path()`, so tooling built against
`Diagnostic.path` (the canon's document-model anchor) gets nothing back for a
pdfform bind failure. Low severity: these are quill-authoring-time errors
(surfaced from `open`, not from binding live document data), so the cost is
borne by the quill author reading a message, not a runtime document editor.

## Cross-cutting

- The `pdf::*`-code-leak finding (`map_pdf_err`) is byte-identical between
  `crates/backends/pdfform/src/lib.rs:356-360` and
  `crates/backends/typst/src/compile.rs:22-28`. Fixing it means either
  reworking `quillmark-pdf::PdfError` to hand back a namespace-free code the
  backend wraps (`format!("pdfform::{}", e.code)` / `format!("typst::{}", e.code)`),
  or adding `pdf::*` to `ERROR.md`'s documented namespace list and accepting it
  crosses both backends verbatim. Either fix belongs at the `quillmark-pdf` /
  `ERROR.md` level, not in this crate alone.
- The `RenderOptions.pages` gap is pdfform-only in the code, but the contract
  it violates is owned by `quillmark_core::types::RenderOptions` — its doc
  comment (`crates/core/src/types.rs:139-146`) is written in Typst-specific
  terms (`typst::page_index_out_of_bounds`, `typst::pdf_page_selection_not_supported`)
  as if it were the only backend, which likely let pdfform's non-implementation
  go unnoticed. Worth flagging to whoever owns `core`'s `RenderOptions` docs
  and to the pdfform maintainer together.
- `format_not_supported`'s three-way code split (`pdfform::format_not_supported`,
  `backend::format_not_supported`, `typst::format_not_supported`) is a small
  taxonomy cleanup that touches both backend crates plus (if `backend::*` is
  kept as the intended shared code) a one-line follow-up in pdfform's `lib.rs:160`.
