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

Two problems:

**Ergonomics.** The `Quillmark` engine is required even for the simplest case — one quill, one document, one artifact. Consumers must hold and thread multiple objects (`Quill`, `Quillmark`, `ParsedDocument`) before getting any output. The engine's role is meaningful for multi-quill, version-resolved workflows; it is ceremony for everyone else.

**Mental model.** The engine owns rendering, but it is the *quill* that defines what a document becomes — its schema, its template, its backend. Having render live on the engine rather than the quill inverts the natural ownership.

The overhaul makes `Quill.render()` the primary surface. The engine remains available as an explicit advanced path for consumers who need version resolution or multi-quill management. `parseMarkdown` is promoted to a static factory on `ParsedDocument` (it already is in Python; WASM follows suit).

The Python binding's `Workflow` type is not removed — it stays as the mechanism for dynamic asset and font injection. `Quill.render()` becomes the happy path; `engine.workflow()` + `workflow.render()` remains the escape hatch for mutable runtime concerns.

---

## Tasks

### 1. Add `render()` and `compile()` to core `Quill`

**File:** `crates/core/src/quill/mod.rs` (or wherever `Quill` is defined in core)

The core `Quill` struct must gain the ability to drive a render without going through `Quillmark`. This requires `Quill` to carry a resolved `Arc<dyn Backend>` after construction. The backend is known at quill-load time from the `backend` field in `Quill.yaml`; store it on the struct.

Add to core `Quill`:

```rust
impl Quill {
    pub fn render(
        &self,
        input: QuillInput,        // enum: Markdown(String) | Parsed(ParsedDocument)
        opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError>;

    pub fn compile(
        &self,
        input: QuillInput,
    ) -> Result<CompiledDocument, RenderError>;
}
```

`QuillInput` is a new enum in core:

```rust
pub enum QuillInput {
    Markdown(String),
    Parsed(ParsedDocument),
}
```

When the input variant is `Markdown`, `render()` parses it internally (calling `ParsedDocument::from_markdown`) before proceeding through the existing `compile_data` pipeline. When the input is `Parsed`, it proceeds directly.

The implementation follows the same pipeline as `Workflow::render` today: `compile_data` (coerce → validate → normalize → transform → defaults → serialize), load plate, merge assets, `backend.compile`.

### 2. QUILL field mismatch warning

When `Quill::render()` (or `Quill::compile()`) receives a `QuillInput::Parsed` whose `quill_ref` does not match `self.name`, append a warning diagnostic to the returned `RenderResult`:

```
code:    "quill::ref_mismatch"
message: "document declares QUILL '{doc_ref}' but was rendered with '{quill_name}'"
hint:    "remove the QUILL field or use the engine to resolve quills dynamically"
```

This is a warning, not an error. Rendering proceeds. The intent is a loud footgun: the consumer gets an artifact and a clear signal that something is probably wrong, but is not blocked.

### 3. WASM: move `parseMarkdown` to `ParsedDocument`

**File:** `crates/bindings/wasm/src/engine.rs`

Currently `parseMarkdown` is a static method on `Quillmark`. Move it to a static on `ParsedDocument`:

```typescript
// Before
Quillmark.parseMarkdown(markdown: string): ParsedDocument

// After
ParsedDocument.fromMarkdown(markdown: string): ParsedDocument
```

Implementation: extract the existing body of `Quillmark::parse_markdown` into a `#[wasm_bindgen(js_name = fromMarkdown)]` static on the `ParsedDocument` struct in `engine.rs`.

Keep `Quillmark.parseMarkdown` as a deprecated thin wrapper that calls `ParsedDocument.fromMarkdown` and emits a `console.warn`. Remove it in a follow-up once downstream call sites are migrated.

### 4. WASM: add `render()` and `compile()` to the WASM `Quill` binding

**File:** `crates/bindings/wasm/src/engine.rs`

Expose the new core `Quill::render` and `Quill::compile` through the WASM `Quill` struct:

```typescript
class Quill {
  render(input: string | ParsedDocument, opts?: RenderOptions): RenderResult
  compile(input: string | ParsedDocument): CompiledDocument
}
```

On the Rust side, the `input` parameter should accept `JsValue` and branch on whether it is a `string` (pass as `QuillInput::Markdown`) or a `ParsedDocument` instance (pass as `QuillInput::Parsed`). Use `js_sys::JsString::instanceof` / `JsCast` for the branch.

`RenderOptions` remains as-is. `CompiledDocument.renderPages()` remains unchanged.

### 5. Python: add `render()` to `Quill`

**Files:** `crates/bindings/python/src/lib.rs`, `crates/bindings/python/python/quillmark/__init__.pyi`

Add to the Python `Quill` class:

```python
def render(
    self,
    input: str | ParsedDocument,
    format: OutputFormat | None = None,
) -> RenderResult: ...
```

When `input` is a `str`, parse it internally. When `input` is a `ParsedDocument`, proceed directly. Emit the ref-mismatch warning (task 2) into `RenderResult.warnings` when applicable.

`ParsedDocument.from_markdown` is already a static in Python — no change needed there.

The existing `engine.workflow()` → `workflow.render()` path is not changed. It remains the correct path when dynamic assets or fonts must be injected at render time, or when quill resolution from a bare quill ref is needed. Document this distinction clearly in the stub:

```python
class Quill:
    def render(self, input: str | ParsedDocument, format: OutputFormat | None = None) -> RenderResult:
        """Render a document using this quill.

        For dynamic asset or font injection, use engine.workflow() instead.
        """
```

### 7. Update tests

**WASM Rust tests** (`crates/bindings/wasm/tests/wasm_bindings.rs`):

- Add tests for `Quill::render` with a `&str` input and with a `ParsedDocument` input.
- Add a test for the ref-mismatch warning: render a `ParsedDocument` whose `quill_ref` names a different quill, assert one warning with code `"quill::ref_mismatch"` is present in the result.

**WASM JS tests** (`crates/bindings/wasm/basic.test.js`):

- Add a test for `quill.render(markdownString, opts)` — the new happy path.
- Add a test for `ParsedDocument.fromMarkdown(markdown)` as a static call (not via engine).
- Verify `Quillmark.parseMarkdown` still works (deprecated wrapper) and logs a warning.

**Python tests** (`crates/bindings/python/tests/test_render.py`, `test_quill.py`):

- Add tests mirroring the WASM cases: `quill.render(markdown_str)`, `quill.render(parsed)`, and the ref-mismatch warning.

---

## Out of scope

- Removing `Quillmark.parseMarkdown` from WASM (deprecated in this tasking, removed later).
- Changes to `Workflow` in Python — asset/font injection stays on that type.
- CLI binding changes — the CLI operates on files and paths, not the in-memory object graph.
- Changes to `VersionedQuillSet`, selector resolution, or `QuillReference` parsing.
- The `compile()` + `renderPages()` selective-page path — `Quill.compile()` is added in task 1/4, but `renderPages` behavior on `CompiledDocument` is unchanged.

---

## Done when

- `quill.render(markdownString, opts)` produces a valid `RenderResult` in both WASM and Python without touching an engine instance.
- `ParsedDocument.fromMarkdown(markdown)` works as a static in WASM (no engine).
- Rendering a `ParsedDocument` with a mismatched `quill_ref` produces one warning with code `"quill::ref_mismatch"` and still returns an artifact.
- All existing engine-path tests continue to pass — `engine.render(parsed, opts)` and `engine.workflow(ref).render(parsed)` are unaffected.
- The Python stub (`__init__.pyi`) reflects the new `Quill.render` signature and the `ParsedDocument.from_markdown` docstring notes the `Workflow` escape hatch.
- `cargo test --workspace` and the WASM JS test suite pass clean.
