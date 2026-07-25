# quillmark (orchestration) + fixtures

## Surface

`quillmark/src/lib.rs` re-exports from `quillmark_core`:
`Artifact, Backend, Card, ChangeSet, Delta, Diagnostic, Document, LiveSession, Location, OutputFormat, ParseError, Parsed, Quill, RenderError, RenderOptions, RenderResult, Content, Severity`.

Own surface (2 files, ~145 lines total):

- `load.rs:17` — `pub fn quill_from_path<P: AsRef<Path>>(path: P) -> Result<Quill, RenderError>`. Filesystem-only; honours `.quillignore`, skips symlinks, caps files at 50 MiB.
- `orchestration/engine.rs` — `pub struct Quillmark { backends: HashMap<String, Arc<dyn Backend>> }`:
  - `Quillmark::new() -> Self` (no `Result`; auto-registers `TypstBackend`/`PdfformBackend` per feature flag)
  - `Default for Quillmark`
  - `register_backend(&mut self, backend: Box<dyn Backend>)`
  - `registered_backends(&self) -> Vec<&str>`
  - `open(&self, quill: &Quill, doc: &Document) -> Result<LiveSession, RenderError>`
  - `render(&self, quill: &Quill, doc: &Document, opts: &RenderOptions) -> Result<RenderResult, RenderError>`
  - `supported_formats(&self, quill: &Quill) -> Result<&'static [OutputFormat], RenderError>`
  - `supports_canvas(&self, quill: &Quill) -> bool`

Happy path (disk → PDF), per `README.md` and `lib.rs` doctest:

```rust
use quillmark::{quill_from_path, Document, OutputFormat, Quillmark, RenderOptions};
let quill = quill_from_path("path/to/quill")?;
let engine = Quillmark::new();
let doc = Document::parse(markdown)?.document;
let result = engine.render(&quill, &doc, &RenderOptions { output_format: Some(OutputFormat::Pdf), ..Default::default() })?;
let pdf_bytes = &result.artifacts[0].bytes;
```

5 lines, one crate import, no `quillmark_core` needed. This is the crate's chief asset: it is a thin, correctly-scoped shell (backend registry + fs loader) over `quillmark_core`, not a re-implementation — matches `ARCHITECTURE.md`'s description exactly (engine does not construct quills, resolves backend at render time, fs walking kept out of core).

`crates/fixtures/src/lib.rs`: 4 free functions (`resource_path`, `quills_path`, `example_output_dir`, `write_example_output`), all `PathBuf`/`io::Result<()>` — no test-only types leak. `Cargo.toml` has `publish = false`, so it never reaches crates.io; not a released-surface concern.

## Findings

### `register_backend`'s custom-backend path requires a non-re-exported, submodule-nested core type

**Severity: High**

`orchestration/engine.rs:45` (`register_backend`) is the one documented extension point for a native Rust caller — plug in a third backend. To use it, a caller must produce a `LiveSession`, whose only public constructor is `LiveSession::new(Box<dyn SessionHandle>)`. `SessionHandle` is defined in `crates/core/src/session.rs:23` but is not in core's own `pub use session::{...}` list (`crates/core/src/lib.rs:54-55` re-exports only `ApplyError, Assoc, ChangeSet, Delta, LineOp, LiveSession, MarkOp, Op`) and is absent from `quillmark`'s re-export list (`lib.rs:22-26`). The only way to name it is the submodule path `quillmark_core::session::SessionHandle` — confirmed by `quillmark/tests/backend_registration_test.rs:4`, which imports exactly that to build a mock backend.

Caller impact: implementing a custom `Backend` — the feature `register_backend` exists for — cannot be done against `quillmark` alone; it forces adding `quillmark-core` as a direct dependency (contra the crate's stated goal of not requiring that for common tasks) and reaching past both crates' top-level re-export lists into an internal module path that reads as private API.

### `Quill::from_tree`'s only argument type is not re-exported — asymmetric with `quill_from_path`

**Severity: Medium**

`load.rs:17`'s doc comment and `orchestration/mod.rs:5-8` both point in-memory quill construction at `Quill::from_tree` (core). `Quill` is re-exported (`lib.rs:23`), so the call itself is reachable through `quillmark::Quill::from_tree`, but its only parameter type, `FileTreeNode`, is not — a caller must import `quillmark_core::FileTreeNode` (or `quillmark_core::quill::FileTreeNode`) to build the tree at all. Confirmed by `quillmark/tests/validate_test.rs:7`, which does exactly this to build a quill with no filesystem.

Caller impact: the disk-loading happy path is fully self-contained in `quillmark` (`quill_from_path`); the in-memory/bundle-loading happy path is not — it requires `quillmark-core` as a direct dependency for one enum, even though `ARCHITECTURE.md` frames `Quill::from_tree` as the symmetric alternative to `quill_from_path`. `QuillIgnore` (used by `load.rs` itself for `.quillignore` parsing) has the same gap if a caller wants to replicate ignore-file handling over an in-memory tree.

### `supports_canvas` silently collapses "backend not found" into "no canvas," unlike its sibling `supported_formats`

**Severity: Medium**

`orchestration/engine.rs:118-122`:
```rust
pub fn supports_canvas(&self, quill: &Quill) -> bool {
    self.resolve_backend(quill)
        .map(|b| quillmark_core::formats_support_canvas(b.supported_formats()))
        .unwrap_or(false)
}
```
vs. `supported_formats` (`engine.rs:106-108`), which returns `Result<_, RenderError>` for the identical `resolve_backend` failure. Both are pre-session capability probes over the same `Quill`; one surfaces backend-not-found as `Err` with a hint listing registered backends, the other swallows it into `false`. The doc comment (`engine.rs:110-117`) does say "`false` when the backend is unsupported," so this is deliberate, not an oversight — but it means a caller who checks `supports_canvas(quill)` before offering a canvas UI cannot distinguish "this backend has no canvas painter" from "this quill's backend id is a typo / not registered," the exact distinction `supported_formats` on the same struct does expose.

Caller impact: debugging a silently-disabled canvas UI on a misconfigured quill requires calling `supported_formats` anyway to get the real error — `supports_canvas` alone hides it.

### `RenderedRegion` / `ContentHit` / `HitGranularity` reachable through re-exported methods but not re-exported themselves

**Severity: Low**

`LiveSession::regions() -> Vec<RenderedRegion>`, `field_at`, `position_at() -> Option<ContentHit>`, and `locate()` are all callable through the re-exported `LiveSession` (`lib.rs:23`), and `quillmark/examples/pdfform_preview.rs:60-73` exercises `regions()` from a native Rust example. But `RenderedRegion`, `ContentHit`, and `HitGranularity` (`crates/core/src/region.rs`) are not in `quillmark`'s re-export list. A caller can call these methods and iterate/destructure the results inline (as the example does) without ever naming the types, but cannot write a function signature, struct field, or `let x: Vec<RenderedRegion>` without adding `quillmark-core` directly.

Caller impact: minor until a caller wants to store or pass around region data by type, at which point the crate's "no direct `quillmark-core` dependency for common tasks" property breaks for a documented, native-Rust-legitimate feature (region geometry sidecar, per `PROGRAMMATIC.md`/`PREVIEW.md` — the sidecar itself is explicitly not WASM-only, see `RenderOptions::regions`).

### `register_backend` silently replaces a same-id backend with no diagnostic

**Severity: Low**

`orchestration/engine.rs:45-48` inserts into a `HashMap` keyed by `backend.id()`; a second `register_backend` call with a colliding id silently discards the first, with no return value, log, or panic. This is exercised and apparently intentional (`quillmark/tests/backend_registration_test.rs:59-66`, `test_register_backend_replaces_existing`), but the doc comment above `register_backend` ("Register a backend with the engine.") does not say so — a reader has to find the test to learn the replace semantics.

Caller impact: a caller who registers two backends under the same id by accident (e.g. copy-pasted feature-gated registration) gets no signal; only a doc-comment fix is needed, not a behavior change.

### `Quillmark::render` re-resolves the backend twice per call

**Severity: Low**

`orchestration/engine.rs:87-102`: `render()` calls `self.supported_formats(quill)` (which calls `resolve_backend`) and then `self.open(quill, doc)` (which calls `resolve_backend` again) — two `HashMap` lookups plus one wasted `Arc` clone per render call for the sole purpose of reading the default output format. Not a correctness issue (both resolve to the same backend), just a redundant lookup on every render's hot path; folding the default-format read into `open`'s already-resolved backend would remove it.

### `quillmark-fixtures::quills_path` falls back to a possibly-nonexistent path with no error signal

**Severity: Low**

`fixtures/src/lib.rs:26-42`: when no versioned subdirectory is found (`read_dir` fails, or the directory has none), the function falls through to `quill_dir` — returning a `PathBuf` that may not exist, with no `Option`/`Result` to signal that. Since the crate is `publish = false` and only consumed by this workspace's own tests/examples, impact is limited to a confusing downstream `quill_from_path` error pointing at the wrong layer.

## Cross-cutting

- The `SessionHandle` re-export gap (finding 1) and the `FileTreeNode`/`QuillIgnore` gap (finding 2) are both `quillmark_core::lib.rs` re-export-list omissions, not `quillmark`-crate bugs per se — fixable by either crate adding the re-export. Flagging here because the caller-facing symptom (forced `quillmark-core` dependency) is felt at the `quillmark` crate boundary, which is the crate CLAUDE.md designates as the one downstream Rust users should need.
- `RenderedRegion`/`ContentHit` (finding 3) are defined in `crates/core/src/region.rs`; the region/geometry feature itself (`PREVIEW.md`) is otherwise reviewed as part of the WASM/preview surface — this note is scoped to the re-export gap only, not the feature's design.
- The `LiveSession::apply(&mut self, json: &serde_json::Value)` edit seam is reachable natively (the type is re-exported and the method is public), but `PREVIEW.md`'s "Non-goals" section states native (CLI/Python) exposure of live-preview is explicitly out of scope — so the missing `Quillmark`-level convenience that WASM's binding hand-rolls (`check_quill_reference` + `compile_data` before `apply`, see `crates/bindings/wasm/src/engine.rs:2801-2808`) is not flagged as a defect for this crate; noting it here only so a future reviewer doesn't rediscover it as one.
