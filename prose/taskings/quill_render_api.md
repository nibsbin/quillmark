# Render API Overhaul: `Quill.render()` as Primary Surface

**Audience:** Quillmark engine and bindings maintainers  
**Affects:** `quillmark-core`, `crates/bindings/wasm`, `crates/bindings/python`

## Background

The current render flow for WASM consumers requires four sequential steps:

```javascript
const quill = Quill.fromTree(fileTree)         // 1. build
engine.registerQuill(quill)                    // 2. register
const parsed = Quillmark.parseMarkdown(md)     // 3. parse
const result = engine.render(parsed, opts)     // 4. render
```

Three problems:

**Ergonomics.** The `Quillmark` engine is required even for the simplest case — one quill, one document, one artifact. Consumers must hold and thread multiple objects (`Quill`, `Quillmark`, `ParsedDocument`) before getting any output.

**Mental model.** The engine owns rendering, but it is the *quill* that defines what a document becomes — its schema, its template, its backend. Having render live on the engine rather than the quill inverts the natural ownership.

**The registry is the wrong layer.** The engine's internal quill registry (`VersionedQuillSet`, `HashMap<String, VersionedQuillSet>`) is an application-level concern — tracking which quills are available, resolving version selectors, deciding which template to use for a given document. That belongs in a higher layer (a router, a registry service, a framework like Quiver) that is outside the engine's scope. The engine's job is to execute a render given a quill and a document, not to manage a quill collection.

The overhaul has two parts:

1. **`Quill.render()` becomes the primary surface.** A quill renders documents. The engine is no longer in the render path for typical consumers.

2. **The engine sheds its quill registry.** It becomes a backend registry and quill factory: given a file tree, it resolves the declared backend and returns a `Quill` ready to render. Version management, quill selection, and QUILL-field routing are caller concerns.

`parseMarkdown` is promoted to a static factory on `ParsedDocument` (it already is in Python; WASM follows suit). The Python binding's `Workflow` type is not removed — it stays as the mechanism for dynamic asset and font injection, with `Quill.render()` as the simpler happy path.

The new canonical flow for WASM consumers:

```javascript
const engine = new Quillmark()
const quill = engine.quillFromTree(fileTree)  // factory: load + attach backend
const result = quill.render(markdown, opts)
```

---

## Tasks

### 1. Remove the quill registry from core `Quillmark`

**File:** `crates/quillmark/src/orchestration/engine.rs`

Delete:

- The `quills: HashMap<String, VersionedQuillSet>` field from `Quillmark`
- `VersionedQuillSet` (the `BTreeMap<Version, Quill>` wrapper) — the entire type
- `Quillmark::register_quill()` — the public registration method
- `Quillmark::has_quill()` — existence check on the registry
- `Quillmark::workflow(quill_ref: &str)` — the registry-lookup path of `workflow()`

`Quillmark::workflow()` currently accepts a quill reference string and resolves it against the registry. That string-based resolution path is gone. Replace it with a direct method that accepts a `&Quill`:

```rust
impl Quillmark {
    pub fn workflow(&self, quill: &Quill) -> Result<Workflow, RenderError>;
}
```

This creates a `Workflow` from a caller-provided quill and the registered backend, without any registry lookup. The `Workflow` type itself is unchanged.

`VersionSelector`, `QuillReference`, and `Version` parsing in `crates/core/src/version.rs` are not deleted — they remain available for callers (like Quiver) that manage their own registries above the engine. Do not touch them.

### 2. Add `Quill::with_backend()` and make `Quillmark` a quill factory

**Files:** `crates/core/src/quill/mod.rs`, `crates/quillmark/src/orchestration/engine.rs`

`Quill` must carry a resolved `Arc<dyn Backend>` so it can render without an engine. Add to core `Quill`:

```rust
pub struct Quill {
    // existing fields ...
    pub(crate) backend: Option<Arc<dyn Backend>>,
}

impl Quill {
    pub fn with_backend(mut self, backend: Arc<dyn Backend>) -> Self;
    pub fn backend(&self) -> Option<&Arc<dyn Backend>>;
}
```

`Quill::from_tree` and `Quill::from_path` continue to produce quills with `backend: None`. The engine attaches a backend via `with_backend` when acting as a factory. Calling `Quill::render()` on a quill with no backend is an error: `RenderError::NoBackend`.

Add to `Quillmark`:

```rust
impl Quillmark {
    pub fn load_quill(&self, tree: FileTreeNode) -> Result<Quill, RenderError>;
}
```

`load_quill` reads the `backend` field from `Quill.yaml` inside the tree, looks it up in `self.backends`, and returns `Quill::from_tree(tree)?.with_backend(arc_backend)`. This is the canonical way to produce a render-ready `Quill`.

### 3. Add `render()` and `compile()` to core `Quill`

**File:** `crates/core/src/quill/mod.rs`

```rust
pub enum QuillInput {
    Markdown(String),
    Parsed(ParsedDocument),
}

impl Quill {
    pub fn render(
        &self,
        input: QuillInput,
        opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError>;

    pub fn compile(
        &self,
        input: QuillInput,
    ) -> Result<CompiledDocument, RenderError>;
}
```

When `input` is `Markdown`, parse internally via `ParsedDocument::from_markdown` before entering the pipeline. When `input` is `Parsed`, proceed directly.

The pipeline is unchanged from `Workflow::render` today: `compile_data` (coerce → validate → normalize → transform → defaults → serialize), load plate, merge assets, `backend.compile`. Both methods error with `RenderError::NoBackend` if `self.backend` is `None`.

### 4. QUILL field mismatch warning

When `Quill::render()` or `Quill::compile()` receives a `QuillInput::Parsed` whose `quill_ref` does not match `self.name`, append a warning to `RenderResult`:

```
code:    "quill::ref_mismatch"
message: "document declares QUILL '{doc_ref}' but was rendered with '{quill_name}'"
hint:    "the QUILL field is informational; ensure you are rendering with the intended quill"
```

Warning, not an error. Rendering proceeds. The intent is a loud footgun: the consumer gets an artifact and a clear signal that something is probably wrong, but is not blocked.

### 5. WASM: remove registry-dependent engine methods

**File:** `crates/bindings/wasm/src/engine.rs`

Delete the following from the WASM `Quillmark` binding:

- `registerQuill(quill: Quill): QuillInfo`
- `hasQuill(ref: string): boolean`
- `getQuillInfo(name: string): QuillInfo` (the registry-lookup variant)
- `render(parsed: ParsedDocument, opts: RenderOptions): RenderResult`
- `compile(parsed: ParsedDocument): CompiledDocument`
- `dryRun(parsed: ParsedDocument): void`
- `compileData(parsed: ParsedDocument): object`

These methods all presuppose a quill registry. With the registry gone they have no implementation.

Add the factory method:

```typescript
class Quillmark {
  quillFromTree(tree: Map<string, Uint8Array> | Record<string, Uint8Array>): Quill
}
```

Delegates to `Quillmark::load_quill` from task 2.

### 6. WASM: move `parseMarkdown` to `ParsedDocument`

**File:** `crates/bindings/wasm/src/engine.rs`

```typescript
// Before
Quillmark.parseMarkdown(markdown: string): ParsedDocument

// After
ParsedDocument.fromMarkdown(markdown: string): ParsedDocument
```

Extract the existing body of `Quillmark::parse_markdown` into a `#[wasm_bindgen(js_name = fromMarkdown)]` static on the `ParsedDocument` struct.

Keep `Quillmark.parseMarkdown` as a deprecated thin wrapper that calls `ParsedDocument.fromMarkdown` and emits a `console.warn`. Remove it in a follow-up once downstream call sites are migrated.

### 7. WASM: add `render()` and `compile()` to the WASM `Quill` binding

**File:** `crates/bindings/wasm/src/engine.rs`

```typescript
class Quill {
  render(input: string | ParsedDocument, opts?: RenderOptions): RenderResult
  compile(input: string | ParsedDocument): CompiledDocument
}
```

Accept `JsValue` for `input` and branch on whether it is a string (`QuillInput::Markdown`) or `ParsedDocument` instance (`QuillInput::Parsed`). Use `js_sys::JsString::instanceof` / `JsCast` for the branch.

`RenderOptions` and `CompiledDocument.renderPages()` are unchanged.

### 8. Python: remove registry-dependent engine methods

**Files:** `crates/bindings/python/src/lib.rs`, `crates/bindings/python/python/quillmark/__init__.pyi`

Remove from `Quillmark`:

- `register_quill(quill: Quill)` — registration is gone
- `registered_quills() -> list[str]` — no registry to enumerate
- `workflow(quill_ref: str | Quill | ParsedDocument)` — replace with `workflow(quill: Quill)` only (see below)

The `workflow` method narrows to accept only a `Quill`:

```python
def workflow(self, quill: Quill) -> Workflow: ...
```

The `str` and `ParsedDocument` overloads relied on registry lookup. They are gone. Callers that previously passed a quill ref string are responsible for resolving it to a `Quill` themselves before calling `workflow`.

Add a quill factory to `Quillmark`:

```python
def quill_from_path(self, path: str | Path) -> Quill: ...
```

This replaces `Quill.from_path()` as the canonical way to get a render-ready quill. It reads the backend declaration from `Quill.yaml` and attaches the resolved backend. `Quill.from_path()` is not deleted — it still works for inspecting quill metadata — but it returns a quill with no backend attached and cannot render.

### 9. Python: add `render()` to `Quill`

**Files:** `crates/bindings/python/src/lib.rs`, `crates/bindings/python/python/quillmark/__init__.pyi`

```python
def render(
    self,
    input: str | ParsedDocument,
    format: OutputFormat | None = None,
) -> RenderResult: ...
```

When `input` is `str`, parse internally. When `ParsedDocument`, proceed directly. Emit the ref-mismatch warning (task 4) into `RenderResult.warnings` when applicable. Raises `QuillmarkError` with a clear message if the quill has no backend attached (i.e., was created via `Quill.from_path()` rather than `engine.quill_from_path()`).

`ParsedDocument.from_markdown` is already a static in Python — no change needed.

The existing `engine.workflow(quill)` → `workflow.render(parsed)` path remains the correct path when dynamic assets or fonts must be injected at render time. Document this in the stub:

```python
class Quill:
    def render(self, input: str | ParsedDocument, format: OutputFormat | None = None) -> RenderResult:
        """Render a document using this quill.

        For dynamic asset or font injection, use engine.workflow(quill) instead.
        Raises QuillmarkError if this quill was not created via engine.quill_from_path().
        """
```

### 10. Update tests

**WASM Rust tests** (`crates/bindings/wasm/tests/wasm_bindings.rs`):

- Replace all `engine.registerQuill()` setup with `engine.quillFromTree()`.
- Add tests for `Quill::render` with a `&str` input and with a `ParsedDocument` input.
- Add a test for the ref-mismatch warning: render a `ParsedDocument` whose `quill_ref` names a different quill, assert one warning with code `"quill::ref_mismatch"`.
- Add a test that a `Quill` built via `Quill::from_tree` (no backend) errors on `render` with `NoBackend`.

**WASM JS tests** (`crates/bindings/wasm/basic.test.js`):

- Replace `engine.registerQuill(Quill.fromTree(...))` setup with `engine.quillFromTree(...)`.
- Add a test for `quill.render(markdownString, opts)` — the new happy path.
- Add a test for `ParsedDocument.fromMarkdown(markdown)` as a standalone static call.
- Verify `Quillmark.parseMarkdown` still works (deprecated wrapper) and logs a `console.warn`.

**Python tests** (`crates/bindings/python/tests/test_render.py`, `test_quill.py`, `test_engine.py`):

- Replace `engine.register_quill()` + `engine.workflow(str)` setup with `engine.quill_from_path()`.
- Add tests for `quill.render(markdown_str)`, `quill.render(parsed)`, and the ref-mismatch warning.
- Add a test that `Quill.from_path()` without engine raises on `render`.
- Confirm `engine.workflow(quill)` still works for the dynamic-asset path.

---

## Out of scope

- Removing `Quillmark.parseMarkdown` from WASM (deprecated in this tasking, removed later).
- Changes to `Workflow` in Python beyond narrowing `engine.workflow()` to accept only `Quill`.
- CLI binding changes — the CLI operates on files and paths, not the in-memory object graph.
- Deleting `VersionSelector`, `QuillReference`, or `Version` from core — they are available for higher-layer consumers.
- The `compile()` + `renderPages()` selective-page path — `Quill.compile()` is added in task 3/7, but `renderPages` behavior on `CompiledDocument` is unchanged.

---

## Done when

- `engine.quillFromTree(tree).render(markdown, opts)` produces a valid `RenderResult` in WASM.
- `engine.quill_from_path(path).render(markdown)` produces a valid `RenderResult` in Python.
- `ParsedDocument.fromMarkdown(markdown)` works as a WASM static with no engine.
- Rendering a `ParsedDocument` with a mismatched `quill_ref` produces one warning with code `"quill::ref_mismatch"` and still returns an artifact.
- `Quillmark` in core has no `quills` field, no `register_quill`, no `has_quill`, no string-based `workflow`.
- `registerQuill`, `hasQuill`, and registry-dependent `render`/`compile` are absent from the WASM `Quillmark` TypeScript surface.
- `register_quill` and string-based `workflow` are absent from the Python `Quillmark` stub.
- `engine.workflow(quill)` in Python still produces a working `Workflow` for dynamic-asset renders.
- `cargo test --workspace` and the WASM JS test suite pass clean.
