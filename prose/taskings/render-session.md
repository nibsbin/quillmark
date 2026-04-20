# RenderSession: First-Class Iterative Rendering

**Audience:** Quillmark engine, backend, and bindings maintainers  
**Affects:** `quillmark-core`, `crates/backends/typst`, `crates/quillmark`, `crates/bindings/wasm`, `crates/bindings/python`

## Background

The current design exposes `CompiledDocument` as a public type and splits page-selective rendering across two optional `Backend` trait methods (`compile_to_document`, `render_pages`). Both methods have default implementations that return "not supported" errors, making page-selective rendering feel like an afterthought that may or may not work depending on the backend.

This creates three problems:

**Leaky abstraction.** `CompiledDocument` wraps a `Box<dyn Any + Send + Sync>` — a type-erased blob that only the originating backend knows how to use. It is meaningless to consumers and exists solely to thread internal backend state across two API calls. It should never have been public.

**Wrong trait design.** Optional methods with default error implementations are a code smell. A method either belongs on the trait or it doesn't. `compile_to_document` and `render_pages` being optional has let the Backend trait accumulate a split between a "real" required path and a "bonus" optional path. Iterative rendering is a first-class use case; its support should be required, not optional.

**Redundant code paths.** `Quill::render()` and `Quill::compile()` share identical validation and data-compilation logic (`compile_data_internal`), then diverge only at the final backend call. This redundancy exists because the two operations were designed independently rather than as one unified pipeline.

## New Design

The public API surface changes to:

```
quill.render(parsed, opts)      → RenderResult          // one-shot
quill.open(parsed)              → RenderSession         // iterative
session.page_count              → usize
session.render(opts)            → RenderResult          // all or selected pages
```

`RenderOptions` gains an optional `pages` field. `session.render(opts)` respects it; `quill.render()` does not accept page selection (it is always all-pages; callers who need page selection use `open()`).

`CompiledDocument` disappears from every public surface. Each backend holds its compiled state in a private struct that implements a sealed internal `SessionHandle` trait. `RenderSession` wraps a `Box<dyn SessionHandle>` — type-erased at the core boundary, concrete inside each backend.

`Backend::compile_to_document` and `Backend::render_pages` are deleted from the trait. They are replaced by a single required `Backend::open()` method that returns a `RenderSession`. `Backend::compile()` is also deleted — `Quill::render()` becomes a thin wrapper over `quill.open(parsed)?.render(opts)`. The backend trait is left with one render-related required method: `open()`.

---

## Tasks

### 1. Add `pages` to `RenderOptions`

**File:** `crates/core/src/types.rs`

Add one field to `RenderOptions`:

```rust
pub struct RenderOptions {
    pub output_format: Option<OutputFormat>,
    pub ppi: Option<f32>,
    pub pages: Option<Vec<usize>>,   // None = all pages
}
```

`pages` is `None` by default. `Quill::render()` ignores this field (documents this in the doc comment). `RenderSession::render()` respects it. Update `RenderOptions::default()` — `pages` defaults to `None`.

Update the WASM `RenderOptions` type in `crates/bindings/wasm/src/types.rs` to include an optional `pages?: number[]` field. Update the Python `RenderOptions` equivalent if one exists.

### 2. Define the internal `SessionHandle` trait

**File:** `crates/core/src/session.rs` (new file)

```rust
pub(crate) trait SessionHandle: Send + Sync {
    fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError>;
    fn page_count(&self) -> usize;
}
```

This trait is `pub(crate)` — it must not appear in any public re-export. Backends implement it on their private compiled-state structs. The `crates/core` crate boundary is the only place that knows about it.

### 3. Define the public `RenderSession` struct

**File:** `crates/core/src/session.rs`

```rust
pub struct RenderSession {
    inner: Box<dyn SessionHandle>,
}

impl RenderSession {
    pub(crate) fn new(inner: Box<dyn SessionHandle>) -> Self {
        Self { inner }
    }

    pub fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    pub fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
        self.inner.render(opts)
    }
}
```

`RenderSession::new` is `pub(crate)` — only core and backend implementations construct one. Consumers receive `RenderSession` from `Quill::open()` and can call `page_count()` and `render()`. They cannot inspect or construct the inner state.

Re-export `RenderSession` from `crates/core/src/lib.rs`.

### 4. Rework the `Backend` trait

**File:** `crates/core/src/backend.rs`

Remove:
- `fn compile(...)` — the one-shot backend method
- `fn compile_to_document(...)` — the optional compile-to-handle method
- `fn render_pages(...)` — the optional page-selective render method

Add one required method:

```rust
fn open(
    &self,
    plate_content: &str,
    quill: &Quill,
    json_data: &serde_json::Value,
) -> Result<RenderSession, RenderError>;
```

`open()` compiles the plate + data into a backend-specific internal representation and returns an opaque `RenderSession`. The session's `render(opts)` method selects pages and produces artifacts. Backends that cannot support page selection (e.g., a plain-text backend with no page concept) implement `open()` by compiling immediately and storing the result; `page_count()` returns 1; `render()` ignores `opts.pages` and returns the stored artifact.

The trait now has four methods total: `id()`, `supported_formats()`, `plate_extension_types()`, `open()`. All are required.

Remove the `transform_fields` method from this audit only if it is already planned elsewhere — do not change it here.

### 5. Implement `open()` in the Typst backend

**File:** `crates/backends/typst/src/lib.rs`

Add a private struct:

```rust
struct TypestSession {
    document: typst::Document,
    page_count: usize,
}
```

Implement `SessionHandle` for `TypestSession`:

- `page_count()` returns `self.page_count`
- `render(opts)` selects pages from `self.document` per `opts.pages`, then renders to the requested format (PDF, SVG, PNG) and ppi. This is the logic currently split between `compile_to_document` and `render_pages` — consolidate it here.

Implement `Backend::open()` for the Typst backend by calling the existing `compile::compile_to_document()` helper in `crates/backends/typst/src/compile.rs`, wrapping the resulting `typst::Document` in a `TypestSession`, and returning `RenderSession::new(Box::new(session))`.

The functions `compile_to_pdf`, `compile_to_svg`, `compile_to_png` in `compile.rs` can be removed or made private if they are no longer called from the trait impl — they were only there to serve the old `Backend::compile()` and `Backend::render_pages()`.

### 6. Replace `Quill::render()` and `Quill::compile()` with `render()` and `open()`

**File:** `crates/core/src/quill/render.rs`

`Quill::render()` becomes a thin convenience wrapper:

```rust
pub fn render(&self, parsed: ParsedDocument, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
    let all_pages_opts = RenderOptions { pages: None, ..opts.clone() };
    self.open(parsed)?.render(&all_pages_opts)
}
```

The separate `compile_data_internal` call is eliminated — it moves inside `open()`.

`Quill::compile()` is renamed to `Quill::open()`:

```rust
pub fn open(&self, parsed: ParsedDocument) -> Result<RenderSession, RenderError> {
    let backend = self.require_backend()?;
    let warning = self.ref_mismatch_warning(&parsed);
    let json_data = self.compile_data_internal(&parsed, backend)?;
    let plate_content = self.plate.clone().unwrap_or_default();
    let session = backend.open(&plate_content, self, &json_data)?;
    // Attach the ref-mismatch warning to the session so it surfaces on first render.
    // See note below.
    Ok(session)
}
```

**Ref-mismatch warning and `open()`:** Currently `ref_mismatch_warning` attaches to `RenderResult`. Since `open()` does not produce a `RenderResult`, the warning must either be deferred to the first `session.render()` call or dropped. Prefer deferring: store it in `RenderSession` alongside `inner`, and append it to every `RenderResult` returned by `session.render()`. This preserves the existing behavior without leaking it to a separate return type.

`render_default()` in the same file can be deleted — it was already a thin wrapper and callers can use `render()` with an appropriate `RenderOptions` directly.

### 7. Remove `CompiledDocument`

**Files:** `crates/core/src/types.rs`, `crates/core/src/lib.rs`, `crates/quillmark/src/lib.rs`

Delete the `CompiledDocument` struct and its `new()` constructor. Remove it from every `pub use` re-export chain. Confirm no public API surface references it after this change (`rg 'CompiledDocument' crates/` should return hits only in git history and this tasking).

### 8. Update `Workflow::compile()` → `Workflow::open()`

**File:** `crates/quillmark/src/orchestration/workflow.rs`

Rename `Workflow::compile()` to `Workflow::open()`. Its return type changes from `Result<CompiledDocument, RenderError>` to `Result<RenderSession, RenderError>`. The implementation delegates to `backend.open()` via the prepared render context, identically to how it previously delegated to `backend.compile_to_document()`.

`Workflow::render_pages()` is deleted. Page selection is now expressed through `session.render(opts)` with `opts.pages` set.

### 9. WASM bindings

**File:** `crates/bindings/wasm/src/engine.rs`

Remove the `CompiledDocument` WASM struct and its `#[wasm_bindgen]` impl (the `page_count` getter and `renderPages` method).

Add a `RenderSession` WASM struct:

```rust
#[wasm_bindgen]
pub struct RenderSession {
    inner: quillmark_core::RenderSession,
}

#[wasm_bindgen]
impl RenderSession {
    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> usize { ... }

    #[wasm_bindgen(js_name = render)]
    pub fn render(&self, opts: RenderOptions) -> Result<RenderResult, JsValue> { ... }
}
```

Update `Quill::compile()` → `Quill::open()` in the WASM `Quill` binding, returning `RenderSession` instead of `CompiledDocument`.

The JavaScript surface becomes:

```typescript
class Quill {
  render(parsed: ParsedDocument, opts?: RenderOptions): RenderResult
  open(parsed: ParsedDocument): RenderSession
}

class RenderSession {
  readonly pageCount: number
  render(opts?: RenderOptions): RenderResult
}
```

### 10. Python bindings

**File:** `crates/bindings/python/src/types.rs`

Remove the `PyCompiledDocument` wrapper and its `render_pages` method.

Add a `PyRenderSession` wrapper:

```python
class RenderSession:
    @property
    def page_count(self) -> int: ...
    def render(self, format: OutputFormat | None = None, pages: list[int] | None = None) -> RenderResult: ...
```

Python's `pages` argument maps to `RenderOptions::pages`. The method signature is more explicit than passing a `RenderOptions` object, which follows the existing Python binding convention (Python bindings expand options into keyword arguments rather than passing a struct).

Update `PyQuill::compile()` → `PyQuill::open()` to return `PyRenderSession`. Update `PyWorkflow::compile()` → `PyWorkflow::open()` likewise.

### 11. Update tests

**Core and integration tests** (`crates/core/`, `crates/quillmark/tests/`):

- Replace any `quill.compile(parsed)` calls with `quill.open(parsed)`.
- Replace `compiled.render_pages(pages, format, ppi)` with `session.render(opts)` where `opts.pages` is set.
- Add a test that `quill.render(parsed, opts)` ignores `opts.pages` (renders all pages regardless).
- Add a test that `quill.open(parsed)` followed by `session.render(opts)` with specific pages returns only those pages.
- Confirm `session.render()` surfaces the ref-mismatch warning when applicable.

**WASM tests** (`crates/bindings/wasm/tests/`):

- Replace `quill.compile(parsed)` → `quill.open(parsed)`.
- Replace `compiled.renderPages(pages, opts)` → `session.render(opts)` with `opts.pages` set.
- Add a test for `session.pageCount`.

**Python tests** (`crates/bindings/python/tests/`):

- Replace `quill.compile(parsed)` → `quill.open(parsed)`.
- Replace `compiled.render_pages(pages, format, ppi)` → `session.render(format, pages)`.

---

## Out of scope

- CLI binding — the CLI operates on paths and does not use page-selective rendering. No changes needed.
- `transform_fields` on the `Backend` trait — unrelated to this change.
- Dynamic asset/font injection via `Workflow` — the workflow internals are unchanged; only `compile` is renamed to `open`.
- Adding new backends. The refactor affects all existing backends; new backends are a separate concern.

---

## Done when

- `rg 'CompiledDocument' crates/` returns zero hits outside of test fixtures and this file.
- `rg 'compile_to_document\|render_pages' crates/` returns zero hits in any public module path.
- `rg 'fn compile\b' crates/core/src/backend.rs` returns zero hits.
- `Backend` trait has exactly four methods: `id`, `supported_formats`, `plate_extension_types`, `open`.
- `quill.open(parsed).render(opts)` with `opts.pages = Some(vec![0])` returns a single-page artifact in both WASM and Python.
- `quill.render(parsed, opts)` is a one-liner wrapper over `open` + `render` with no separate backend call.
- `cargo test --workspace` passes clean.
- WASM and Python binding test suites pass clean.
- The ref-mismatch warning surfaces on `session.render()` when the parsed document names a different quill.
