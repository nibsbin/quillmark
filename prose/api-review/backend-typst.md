# Backend: typst

## Surface

`quillmark-typst` exports two things at the crate root. Everything else
(`compile`, `error_mapping`, `helper`, `overlay`, `world`) is `mod`, not `pub
mod` — every `pub fn`/`pub struct` inside those files (`QuillWorld::new`,
`helper::generate_lib_typ`, `map_typst_errors`, …) is unreachable from outside
the crate despite the `pub` marker, matching the crate doc's own claim
(`lib.rs:15-16`): "The `compile` and `error_mapping` modules are internal and
not part of the public API. The public lowering surface is `emit`."

**`Backend` impl** (`lib.rs:506-587`)
```rust
pub struct TypstBackend;                                    // lib.rs:42, derives Debug
impl Backend for TypstBackend {
    fn id(&self) -> &'static str;                            // "typst"
    fn supported_formats(&self) -> &'static [OutputFormat];  // &[Pdf, Svg, Png]
    fn open(&self, source: &Quill, json_data: &serde_json::Value) -> Result<LiveSession, RenderError>;
}
impl Default for TypstBackend { fn default() -> Self { Self } }
```
No inherent methods beyond the trait. No fields. Not `Clone`/`PartialEq`. This
matches `pdfform::PdfformBackend` exactly in shape (unit struct, `Backend`
impl, `Default`) — see Cross-cutting for the one stylistic wrinkle.

**`pub mod emit`** (`lib.rs:24`, `emit.rs`) — the content→Typst-markup
lowering, exposed so `crates/fuzz/src/convert_fuzz.rs` can fuzz it directly
(confirmed: it is the crate's only external consumer — `grep` across the
workspace finds no other `use quillmark_typst::emit`).

```rust
pub fn escape_markup(s: &str) -> String;                     // emit.rs:57
pub fn escape_string(s: &str) -> String;                     // emit.rs:78
pub enum EscapeCtx { Markup, StringLit }                     // emit.rs:125, Debug+Clone+Copy+PartialEq+Eq
pub struct SegmentMap {                                      // emit.rs:222, Debug+Clone+PartialEq+Eq
    pub content: Range<usize>,
    pub gen: Range<usize>,
    pub runs: Vec<(Range<usize>, Range<usize>, EscapeCtx)>,
}
pub struct Emission {                                        // emit.rs:236, Debug+Clone
    pub markup: String,
    pub segments: Vec<SegmentMap>,
}
pub enum EmitError { NestingTooDeep { depth: usize, max: usize } }  // emit.rs:247, thiserror
pub fn emit_content(rt: &Content) -> Result<Emission, EmitError>;  // emit.rs:259
```
`Content` here is `quillmark_content::Content`, a workspace type — no
`typst::*` type appears in any of these signatures. `emit_content_inline`,
`invert_gen_offset`, `forward_content_offset` are `pub(crate)`, not exported.

**Feature flags**: `crates/backends/typst/Cargo.toml` declares no
`[features]` table at all. Nothing in `src/` is `#[cfg(feature = ...)]`-gated
(the only `cfg` in the tree is `cfg(target_arch = "wasm32")`, for `world.rs`'s
`today()`). So the answer to "are default features coherent / does disabling
one silently degrade output" is moot for this crate: there is nothing to
disable, and the whole public surface (`TypstBackend`, `emit::*`) is
unconditionally present on every target.

## Findings

### 1. Package/asset/font-directory load failures are swallowed to stderr, never reach a `Diagnostic`
**File**: `crates/backends/typst/src/world.rs:237-241, 294-298, 347-352, 380-384`
**Severity**: High

`load_assets_from_quill`, `load_packages_from_quill`, and
`load_package_files_from_quill` all hit real failure modes — an asset path
that isn't a valid Typst virtual path, a `typst.toml` that fails to parse, a
package file with an invalid path, a declared `entrypoint` that doesn't
exist — and handle every one of them with `eprintln!(...)` followed by
`continue`, returning `Ok(())` regardless. None of these become a
`Diagnostic`, a warning, or an `Err`.

Caller impact: a quill author who typos an asset filename, ships a malformed
package `typst.toml`, or misspells `entrypoint` gets a document that compiles
"successfully" — possibly with a missing image, a package silently absent
(so `#import` fails downstream with an unrelated "file not found" the author
can't connect back to the typo), or nothing at all if the fallback happens to
paper over it. Diagnosing this requires reading process stderr, which no
binding surfaces: the WASM build has no stderr channel a JS caller reads, the
Python binding never wires `sys.stderr` into `QuillmarkError`, and the CLI
only prints it if the invocation happens to inherit the parent's stderr.
`ERROR.md`'s stated contract — "every consumer... handles all rendering
errors through this single \[`Diagnostic`\] shape" — is not honored by these
four sites.

Compare `quillmark-pdfform`, which has zero `eprintln!`/`println!` in its
entire `src/` tree (verified by grep) — every structural problem
(`missing_form_pdf`, `missing_form_json`, an unbindable `schema_field`, an
out-of-range page) becomes a coded `Diagnostic` via `engine_err`/`map_pdf_err`.
The two backends diverge sharply here: one backend's malformed-input class
fails loud, the other's fails silent.

### 2. `RenderOptions.pages` is honored by this backend but silently ignored by `pdfform`
**File**: `crates/backends/typst/src/compile.rs:83-122` vs `crates/backends/pdfform/src/lib.rs:152-192, 269-333`
**Severity**: High (cross-backend contract violation)

This backend's `render_document_pages` reads `opts.pages`: it rejects a
`pages` selection for PDF with `typst::pdf_page_selection_not_supported`, and
for SVG/PNG it filters to exactly the requested indices (bounds-checked,
erroring on an out-of-range index). This is the correct, spec-shaped
behavior for the `pages` field's documented contract
(`crates/core/src/types.rs:139-147`: "`None` renders all pages... Any index
`>= page_count` fails...").

`PdfformSession::render` (`pdfform/src/lib.rs:152`) never reads `opts.pages`
at all — `render_svg`/`render_png` take no options and always emit one
`Artifact` per page in the document. A caller who sets
`RenderOptions { pages: Some(vec![2]), .. }` against a `pdfform`-backed quill
silently gets every page back, with no error and no indication the selection
was ignored, while the identical call against a `typst`-backed quill gets
exactly page 2. `Backend` is meant to be swappable behind one contract; this
field is not portable across the two shipped implementations.

### 3. Typst-native diagnostic `code` is a message-text heuristic, not a stable identifier
**File**: `crates/backends/typst/src/error_mapping.rs:30-33`
**Severity**: Medium (documented, but still fragile)

```rust
let code = Some(format!(
    "typst::{}",
    error.message.split(':').next().unwrap_or("error").trim()
));
```
This is called out in `prose/canon/ERROR.md` ("Error codes:
`\"typst::<message-prefix>\"` (the diagnostic message text up to the first
`:`)"), so it is not an unintentional bug — but it is the one place in this
backend where `code`, the field `ERROR.md` says consumers should "route on...
not on \[message\] type," is derived from prose rather than authored. Every
other code this backend emits is a hand-picked constant
(`typst::pdf_page_selection_not_supported`, `typst::world_creation`,
`backend::format_not_supported`, …), and `pdfform`'s codes are 100% curated.
A Typst compile error whose message happens to contain a colon before its
real subject (e.g. "unknown variable: general" → `typst::unknown variable`,
note the embedded space, unlike every hand-authored `snake_case` code) yields
a code a caller cannot safely build a stable `match` against, and the crate
is pinned to `typst = "0.15.0"` at the workspace root — a Typst point
release that reword a diagnostic message silently reshapes these codes with
no compile-time signal.

### 4. Multiple Typst hints collapse to one, with no signal that truncation happened
**File**: `crates/backends/typst/src/error_mapping.rs:27`
**Severity**: Low

```rust
let hint = error.hints.first().map(|h| h.v.to_string());
```
`typst::diag::SourceDiagnostic::hints` is a list; Typst can and does attach
more than one hint to a diagnostic. Only the first survives into
`Diagnostic.hint`. This is really a shape limit of
`quillmark_core::Diagnostic` (`hint: Option<String>`, singular) rather than
something this file could fix unilaterally, but it's worth flagging here
because this is the site where the drop happens and nothing (no `+N more`
suffix, no `source_chain` fallback) indicates data was discarded.

### 5. Dead `OutputFormat::Txt` arm in `render_document_pages`
**File**: `crates/backends/typst/src/compile.rs:184-190`
**Severity**: Low

`TypstSession::render` (`lib.rs:291-300`) already rejects any format outside
`SUPPORTED_FORMATS` (`[Pdf, Svg, Png]`) before calling
`render_document_pages`, so the `OutputFormat::Txt => Err(...)` arm here is
unreachable through the only call site. Harmless, but it's the kind of
defensive-but-dead branch that makes the function's real contract ("pages,
format, ppi, in — one of three artifact shapes out") slightly harder to read
than it is.

### 6. `TypstBackend`'s hand-written `Default` vs `PdfformBackend`'s derive
**File**: `crates/backends/typst/src/lib.rs:582-587`
**Severity**: Low (cosmetic)

`TypstBackend` is `pub struct TypstBackend;` with a manual
`impl Default for TypstBackend { fn default() -> Self { Self } }` plus a doc
comment. `PdfformBackend` gets the identical result from
`#[derive(Debug, Default)]`. Since both are bare unit structs directly
constructible as `TypstBackend`/`PdfformBackend`, `Default` adds nothing
callers couldn't already do — the two backends could (and, for consistency,
should) spell this identically.

## Cross-cutting

- Finding 2 (`RenderOptions.pages` ignored by `pdfform`) is a defect in
  `crates/backends/pdfform/src/lib.rs`, not this crate; flagging it here
  because the asymmetry is only visible by reading both backends side by
  side. The pdfform reviewer should independently flag
  `PdfformSession::render`/`render_svg`/`render_png` for not consulting
  `opts.pages`.
- Finding 3's root cause (`code` derived from message text) is scoped by
  `ERROR.md` as accepted Typst-backend behavior; if it's ever revisited, the
  fix belongs in `quillmark_core` (giving `Diagnostic` a real enum/namespace
  for backend-native codes) rather than in this file alone.
- Finding 4's real fix is in `quillmark_core::Diagnostic` (`crates/core/src/error.rs:96-120`):
  `hint: Option<String>` structurally cannot hold more than one hint. Any
  backend mapping a multi-hint upstream diagnostic (Typst today; conceivably
  others later) hits the same ceiling.
- The workspace root `Cargo.toml:72-73` pins
  `quillmark-typst = { ..., default-features = false }` and the same for
  `quillmark-pdfform`, but neither crate declares a `[features]` table — the
  declaration is a no-op. Not a `backend-typst`-scoped fix (the file lives at
  the workspace root), but worth a follow-up to confirm it isn't standing in
  for a feature that was meant to exist (e.g. gating the embedded Figtree
  fallback fonts) and got dropped.
- `emit`'s public surface (Surface, above) has exactly one consumer today
  (`crates/fuzz`). That's a legitimate reason for it to be `pub` rather than
  `pub(crate)` — cross-crate visibility requires real `pub` — but it does
  mean any future change to `Emission`/`SegmentMap`/`EscapeCtx`/`EmitError`'s
  shape is now a semver-relevant change to `quillmark-typst`, for the benefit
  of a dev-only, non-published-facing consumer. Worth keeping in mind if this
  crate is ever published to crates.io on its own cadence.
