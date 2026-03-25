# Proposal: Selective Page Rendering

## Problem

The current `render()` API always compiles and rasterizes all pages. For web-based document previews (e.g. a 30-page document), this means the full compilation and rasterization cost is paid even when only a few pages are visible. There is no way to render a subset of pages without rendering everything.

## Solution

Split the rendering pipeline into two explicit phases:

1. **Compile** — runs Typst layout, produces an opaque `CompiledDocument`. Expensive; done once.
2. **Render pages** — rasterizes specific pages from the compiled document. Cheap; called on demand.

This matches how Typst works internally: `typst::compile()` always lays out all pages, but `typst_render::render()` is already per-page and independent.

## API (WASM consumer)

```javascript
// Phase 1: compile once
const doc = engine.compile(parsed, { quillRef: 'my-template' });
doc.pageCount; // total pages

// Phase 2: rasterize on demand
const visible = doc.renderPages([4, 5, 6], { format: 'png', ppi: 80 });
visible.artifacts; // 3 Artifacts, in requested order

// null/undefined = all pages
const all = doc.renderPages(null, { format: 'png', ppi: 80 });

// explicit cleanup
doc.free();
```

### Semantics

- **Page indices** are 0-based. Out-of-bounds returns an error.
- **Ordering** of returned artifacts matches the order of the requested indices. Duplicates are allowed.
- **`null`/`undefined`** means all pages in document order. An empty array `[]` returns zero artifacts.
- **Format** is specified per `renderPages` call, so the same compiled document can produce PNG at 80 PPI for preview and SVG for export without recompiling.
- The existing `render()` method is unchanged.

## Layer Changes

### `quillmark-core`
- New `CompiledDocument` struct — opaque wrapper (`Box<dyn Any + Send + Sync>`) with a `page_count: usize` field.
- Two new default methods on the `Backend` trait: `compile_to_document()` and `render_pages()`. Default implementations return a "not supported" error, so existing backends (e.g. acroform) require no changes.

### `quillmark-typst`
- Implement `compile_to_document()`: runs `QuillWorld::new_with_data` + `typst::compile::<PagedDocument>()`, wraps the result in `CompiledDocument`.
- Implement `render_pages()`: downcasts the opaque inner to `&PagedDocument`, rasterizes the requested page indices via `typst_render::render` (PNG) or `typst_svg::svg` (SVG).

### `quillmark` (orchestration)
- `Workflow::compile()` — returns `CompiledDocument`.
- `Workflow::render_pages(doc, pages, format, ppi)` — thin wrapper over the backend method.

### `quillmark-wasm`
- New `#[wasm_bindgen]` struct `CompiledDocument` wrapping the core type, held on the JS side.
- `engine.compile(parsed, opts)` → `CompiledDocument`.
- `CompiledDocument::renderPages(pages, opts)` → `RenderResult`.
- `CompiledDocument::pageCount` getter.
- `CompiledDocument::free()` is handled automatically by wasm-bindgen's drop.

## What Does Not Change

- `render()`, `render_with_options()`, `dryRun()`, `compileData()` — unchanged.
- Python binding — uses the `render()` convenience path, no changes needed.
- Acroform backend — no changes needed.
