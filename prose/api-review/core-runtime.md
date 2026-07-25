# Core: runtime (backend, session, error, value, path, io)

## Surface

Re-exported at crate root (`lib.rs`):

- `backend`: `Backend` (trait), `formats_support_canvas(&[OutputFormat]) -> bool`.
- `session`: `LiveSession`, `ChangeSet { page_count, dirty_pages }`, `ApplyError`, `Assoc`, `Delta { ops }`, `LineOp`, `MarkOp`, `Op` (last five are `quillmark_content` re-exports).
- `error`: `Diagnostic`, `Location`, `ParseError`, `RenderError`, `RenderResult`, `Severity`. Also `MAX_NESTING_DEPTH`, `MAX_YAML_DEPTH` re-exported from other crates; `MAX_INPUT_SIZE`, `MAX_YAML_SIZE`, `MAX_CARD_COUNT`, `MAX_FIELD_COUNT` local consts (all used internally, not dead).
- `types`: `Artifact { bytes, output_format }`, `OutputFormat` (+ `ALL`, `as_str`, `mime_type`, `FromStr`), `RenderOptions`. `ParseOutputFormatError` is defined `pub` in `types.rs` but **not** re-exported at crate root — reachable only as `quillmark_core::types::ParseOutputFormatError`.
- `region`: `doc_path_to_plate_addr`, `plate_addr_to_doc_path`, `field_boxes`, `ContentHit`, `HitGranularity`, `RenderedRegion` (+ `contains`).
- `value`: `QuillValue`, `PathSegment`, `json_depth_exceeds`.
- `path`: `DocPath`, `DocSeg`. `DocPathParseError` defined `pub` but **not** re-exported at root — reachable only as `quillmark_core::path::DocPathParseError`.
- `version`: `Version`, `VersionSelector`, `QuillReference`, `quill_ref_hint()`.
- `writer`/`reader`: `TypedWriter`, `CardWriter`, `TypedReader`, `CardReader`, `ReadValue`.

Module-reachable but **not** re-exported at crate root:

- `session::SessionHandle` — `#[doc(hidden)] pub trait`, the trait a backend actually implements to produce a `LiveSession`.
- `session::LiveSession::new` — `#[doc(hidden)] pub fn`, the only constructor for `LiveSession`.
- `normalize::normalize_document`, `normalize::normalize_field_name` — `pub mod normalize;` with no `pub use`; used cross-crate today only via the full module path (`core/tests/spec_conformance_probe.rs`).
- `types::ParseOutputFormatError`, `path::DocPathParseError` (see above).

## Findings

### `Backend`'s only implementation path is hidden from rustdoc
`crates/core/src/session.rs:22-23`, `:180-181`; `crates/backends/typst/src/lib.rs:578`; `crates/backends/pdfform/src/lib.rs:112`. Severity: **High**.

`Backend::open` must return a `LiveSession`, and the only way to build one is `LiveSession::new(Box<dyn SessionHandle>)` — both `SessionHandle` and `LiveSession::new` carry `#[doc(hidden)]`. Both in-tree backends reach into `quillmark_core::session::SessionHandle` (module path, not the crate-root re-export list) to satisfy this. `#[doc(hidden)]` suppresses the item from generated rustdoc (e.g. docs.rs), so an out-of-tree implementor reading the published API docs for `quillmark-core` sees `Backend` and `LiveSession` but not the trait or constructor needed to produce one — the extension point exists and is exercised by both shipped backends, but is invisible to anyone who doesn't already know to grep the source or read `prose/canon/ARCHITECTURE.md` (which does document `SessionHandle` as the sanctioned mechanism). Either re-export and un-hide `SessionHandle`/`LiveSession::new` as supported API, or state explicitly (in `Backend`'s doc comment) that implementing it requires the internal, doc-hidden `session` module path.

### Canon documents a `LiveSession::handle()` / `SessionHandle::as_any` downcast that doesn't exist
`prose/canon/ARCHITECTURE.md:71`; `crates/core/src/session.rs` (whole file). Severity: **Medium**.

ARCHITECTURE.md: "A backend with a different richer typed surface can still downcast via `LiveSession::handle()` + `SessionHandle::as_any`." Neither `handle()` nor `as_any` exists anywhere in `session.rs` or `SessionHandle`. A backend author who needs a richer surface than the seam methods (`page_size_pt`, `render_rgba`, `regions`, `position_at`, `locate`, `warnings`) has no documented-and-real escape hatch — canon promises one that was removed or never landed. Either add the downcast or fix the doc.

### `RenderOptions::pages` doc bakes in Typst-only error codes and is silently wrong for pdfform
`crates/core/src/types.rs:139-147`; `crates/backends/typst/src/lib.rs:304`; `crates/backends/pdfform/src/lib.rs:152-206`. Severity: **Medium**.

The doc comment on the backend-neutral `RenderOptions::pages` field states the contract in terms of two Typst-namespaced diagnostic codes (`typst::page_index_out_of_bounds`, `typst::pdf_page_selection_not_supported`) as if that were the universal behavior. It isn't: `PdfformBackend::render` never reads `opts.pages` at all — a caller who sets `pages: Some(vec![0])` against a pdfform-backed quill gets **all** pages back with no error, not the documented out-of-bounds/unsupported failure. A `Backend`-generic caller (the orchestration crate, a binding) cannot rely on the documented contract without special-casing which backend it's talking to. Either enforce `pages` uniformly at the `LiveSession`/orchestration layer (so every backend gets the same behavior for free) or scope the doc comment to "Typst-specific; other backends may ignore this" and give pdfform's actual behavior a home.

### `Diagnostic::source_chain` is captured but never surfaced by any formatter; canon's `fmt_pretty_with_source` doesn't exist
`crates/core/src/error.rs:168-196` (`fmt_pretty`); `prose/canon/ERROR.md:214`. Severity: **Medium**.

`Diagnostic::with_source` eagerly flattens a `std::error::Error` cause chain into `source_chain` specifically so it can be displayed. `fmt_pretty()` prints severity, message, code, location, path, hint — never `source_chain`. Canon documents a second method, `Diagnostic::fmt_pretty_with_source()`, that "appends each cause in the source chain as `cause N: <message>`" — it isn't in the code. As shipped, the only way to see `source_chain` is to read the public field directly; every diagnostic pretty-printer in the crate (`fmt_pretty`, `print_errors`) silently drops it.

### `MarkOp`/`LineOp` are re-exported without the types needed to construct or match them
`crates/core/src/lib.rs:55`; `crates/core/src/document/edit.rs:905-906` (consumer requiring these ops); `crates/content/src/model.rs:71,92,127` (`LineKind`, `Container`, `MarkKind` — none re-exported by core). Severity: **Medium**.

`lib.rs` re-exports `LineOp`/`MarkOp` (and documents the parallel `Content` re-export explicitly as "so consumers ... can name the type without depending on `quillmark-content` directly"), but `MarkOp::Add`/`Remove` carry a `kind: MarkKind` field, `LineOp::SetKind` carries `kind: LineKind`, and `LineOp::SetContainers` carries `containers: Vec<Container>` — none of `MarkKind`, `LineKind`, `Container` are re-exported anywhere in `quillmark_core`. A downstream crate that wants to build a `MarkOp::Add` to feed the document mutator (`document/edit.rs`'s `line_ops: &[LineOp], mark_ops: &[MarkOp]` parameter) cannot do so without adding `quillmark-content` as a direct dependency and reaching into its internal `model` module — exactly the coupling the `Content` re-export's own doc comment says core is trying to spare callers from. Either re-export the three missing types or drop the misleading justification comment.

### `Version`/`VersionSelector`/`QuillReference::from_str` return a bare `String`, unlike every other parser in scope
`crates/core/src/version.rs:32,121,213` (`type Err = String`); contrast `crates/core/src/path.rs:184-196` (`DocPathParseError`, structured, `Display` + `Error`) and `crates/core/src/types.rs:57-72` (`ParseOutputFormatError`, structured, `Display` + `Error`). Severity: **Medium**.

Three `FromStr` impls in the same crate return `Result<Self, String>`. `String` does not implement `std::error::Error`, so a caller cannot `?` one of these into a `Box<dyn Error>`-returning function without an extra `.map_err`, cannot match on a variant (there are none), and gets no error code. `path.rs` and `types.rs` in the same scope both demonstrate the crate's own preferred shape (a named struct implementing `Display` + `Error`, carrying the offending input). Internally this is masked because `ParseError::InvalidQuillReference` just stores the string as `reason` — but any binding or downstream crate calling `Version::from_str`/`VersionSelector::from_str`/`QuillReference::from_str` directly (both WASM and Python bindings call `QuillReference::from_str` — `bindings/wasm/src/engine.rs:762`, `bindings/python/src/types.rs:275`) gets the weaker shape.

### `QuillValue` has no fill-preserving array-index accessor
`crates/core/src/value.rs:436-441` (`get`, object-only); contrast `as_array()` at `:426-428` (returns `&Vec<serde_json::Value>`, fill-free). Severity: **Low**.

`QuillValue::get(&self, key: &str)` walks into an object and returns a child `QuillValue`, preserving that child's `!must_fill` annotation — the one navigation primitive that keeps the "annotated tree is authoritative" property the module doc leads with. There is no array-index counterpart (`get_index(usize)` or similar): the only way to reach an array element is `as_array()`, which returns plain `serde_json::Value`s with fill already stripped. A consumer walking a `QuillValue` tree by hand to inspect fill state on an array element (rather than via the whole-tree `fill_paths()`/`set_fill_at()`) cannot do it — `get` and `as_array` are not peers over the two container kinds the type supports.

### `normalize` module's public functions aren't re-exported at crate root, unlike every sibling module
`crates/core/src/lib.rs:74` (`pub mod normalize;`, no `pub use`); contrast every other `pub mod` in the file. Severity: **Low**.

Every other top-level module in `lib.rs` gets its key items hoisted to the crate root (`pub use writer::{...}`, `pub use path::{...}`, etc.). `normalize` is the one exception — `normalize_document`/`normalize_field_name` are `pub fn` but reachable only via `quillmark_core::normalize::...`. The crate's own integration test (`core/tests/spec_conformance_probe.rs:7`) already depends on this module path externally, so it is de facto public API; the omission from the root re-export list looks like an oversight rather than a deliberate "internal" marking (nothing hides it — no `#[doc(hidden)]`, unlike the `SessionHandle` case above).

### `ParseOutputFormatError` / `DocPathParseError` are public but root-unreexported
`crates/core/src/types.rs:57-72`; `crates/core/src/path.rs:184-196`. Severity: **Low**.

Both are the `Err` type of a `FromStr` impl whose `Ok` type (`OutputFormat`, `DocPath`) *is* re-exported at the crate root. A caller who does `"x".parse::<quillmark_core::OutputFormat>()` gets back an error type they can only name via `quillmark_core::types::ParseOutputFormatError`, not `quillmark_core::ParseOutputFormatError`. Minor asymmetry between a re-exported success type and its own error type's path.

## Cross-cutting

- The `MarkOp`/`LineOp`/`MarkKind`/`LineKind`/`Container` gap (finding above) affects any consumer that wants to build content ops from outside core — most concretely the WASM/Python bindings' editor-facing surfaces (`bindings/wasm`, `bindings/python`), which are out of this review's scope but are the actual callers of `Card::apply_body_change`'s `line_ops`/`mark_ops` parameters in `document/edit.rs` (owned by the `document/**` review).
- The pdfform-vs-Typst `RenderOptions::pages` behavioral gap is a `backends/{typst,pdfform}` concern as much as a `core` one; flagging here because the misleading contract lives in `core::types::RenderOptions`'s doc comment, but the fix likely also touches both backend crates' `render()` — worth cross-referencing with whichever review owns `backends/pdfform` and `backends/typst`.
- `SessionHandle`'s doc-hidden status and the stale `LiveSession::handle()`/`as_any` canon reference matter most to whoever documents "how to write a third backend" — likely a `prose/canon` or top-level docs concern beyond this crate's source.
