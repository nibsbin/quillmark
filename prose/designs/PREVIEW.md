# Canvas Preview (WASM, Typst)

## TL;DR

A Typst-only, WASM-only path that paints rasterized pages directly into a
`CanvasRenderingContext2d`. Sits alongside the existing byte-output verbs
(`render` for PDF/PNG/SVG); does not replace them. Both paths share the
cached `PagedDocument` produced by `Backend::open`, so one compile feeds
both.

## Why

For live previews of long documents, the byte-output formats are
sub-optimal:

- **Iframed SVG**: each iframe is its own browser document. N pages → N
  documents; teardown and memory cost grow linearly.
- **Inline SVG**: scales with content complexity (every glyph is a DOM
  node); long, dense documents produce huge DOM trees.
- **PNG**: pays zlib encode + decode on every render, and you typically
  hold N decoded bitmaps.

A canvas painter skips the encode/decode round-trip entirely:

```
typst-render → tiny_skia::Pixmap → unpremultiply → ImageData → putImageData
```

Pixels go straight from the rasterizer into the canvas backing store. No
PNG compression, no SVG XML parse, no second layout pass in the browser.

For long documents, the consumer can keep memory bounded to the visible
viewport — only paint pages near the viewport, repaint as the user
scrolls.

## Non-goals

- Native (CLI / Python) exposure. Capability is WASM-only.
- Text selection, find-in-page, accessibility. Canvas has none of these by
  design — if you need them, keep an SVG/PDF export path alongside.
- Click-to-jump or cursor-to-region mapping. Investigated as a Typst spike
  (jump_from_click / jump_from_cursor + an OriginMap) but deferred — not
  needed for the preview itself.

## API

### Rust

```rust
// quillmark-core
pub trait SessionHandle: Any + Send + Sync {
    fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError>;
    fn page_count(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
}

impl RenderSession {
    pub fn page_count(&self) -> usize;
    pub fn warnings(&self) -> Vec<Diagnostic>;
    pub fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError>;
    #[doc(hidden)]
    pub fn handle(&self) -> &dyn SessionHandle;
}
```

```rust
// quillmark-typst
pub struct TypstSession { /* PagedDocument + page_count */ }

impl TypstSession {
    pub fn page_size_pt(&self, page: usize) -> Option<(f32, f32)>;
    pub fn render_rgba(&self, page: usize, scale: f32) -> Option<(u32, u32, Vec<u8>)>;
}

pub fn typst_session_of(s: &RenderSession) -> Option<&TypstSession>;
```

### TypeScript (WASM)

```ts
class RenderSession {
  readonly pageCount: number;
  readonly backendId: string;
  readonly warnings: Diagnostic[];

  render(opts?: RenderOptions): RenderResult;
  pageSize(page: number): PageSize;     // { widthPt, heightPt } in pt
  paint(ctx: CanvasRenderingContext2D, page: number, scale: number): void;
}
```

`scale` multiplies Typst's natural 72 ppi (1 pt → 1 device pixel at
`scale = 1`). Caller computes `scale = devicePixelRatio * userZoom` and
sizes the canvas before calling `paint`.

## Architecture

The canvas path is a typed side channel — `core` stays output-format-only,
the typst crate owns the typed surface, the WASM binding wires it to
`web-sys`.

```
core::RenderSession            ← Box<dyn SessionHandle>
  └─ TypstSession              ← typst-only; holds PagedDocument
       └─ typst-render::render ← PagedDocument + ppi → tiny_skia::Pixmap
            └─ Pixmap.demultiply() → RGBA8 buffer
                 └─ ImageData → ctx.putImageData
```

The seam in `core` is minimal: `SessionHandle: Any + as_any(&self)` plus a
`#[doc(hidden)]` `RenderSession::handle()` accessor. The typst crate owns
the downcast in one place (`typst_session_of`). Native bindings never
link the WASM side and never call the typed accessor; their behavior is
byte-identical.

## Lifecycle and consumer flow

```js
const session = quill.open(doc);              // compiles once, caches PagedDocument
const dpr = window.devicePixelRatio || 1;
const scale = dpr * userZoom;                 // userZoom is a UI control
const { widthPt, heightPt } = session.pageSize(page);

canvas.width  = Math.round(widthPt  * scale); // backing store, device px
canvas.height = Math.round(heightPt * scale);
canvas.style.width  = `${widthPt  * userZoom}px`;  // CSS box, layout px
canvas.style.height = `${heightPt * userZoom}px`;

session.paint(canvas.getContext('2d'), page, scale);
```

Setting `canvas.width` / `canvas.height` clears the backing store, which
is the recommended way to handle page-to-page transitions. If a consumer
reuses a canvas without resizing, `clearRect` first.

## Decisions and rationale

- **Method on `RenderSession`, not a sub-handle.** Earlier drafts had a
  `Preview` sub-handle returned by `session.preview()`. Justified only if
  paint shipped with `click()` and `locate_cursor()` (they share state).
  With paint alone, the sub-handle is ceremony — keep the verb on
  `RenderSession`.
- **Not an `OutputFormat`.** Canvas is a side-effecting paint into a JS
  object, not a serializable byte stream. Forcing it into the enum
  either leaks `wasm_bindgen` into `core` or makes `Artifact` dishonest.
- **Coalesce at the session, not at the format.** One compile feeds
  bytes (`render`), pixels (`paint`), and metadata (`pageSize`,
  `warnings`).
- **`Any` downcast over a generic capability registry.** Canvas is
  Typst-only and WASM-only; pushing it through a generic core trait would
  force every backend to implement or stub it and would drag `web-sys`
  toward `core`. The downcast is the standard escape hatch.
- **`scale` is a multiplier on 72 ppi, not a ppi value.** Matches how
  canvas/DPR consumers think (`scale = dpr * zoom`), and makes
  `scale = 1` the natural "1 pt = 1 device pixel" baseline.
- **Required `scale` (not optional).** Real consumers always compute it
  from `devicePixelRatio`. A default of `1.0` would produce non-retina
  output by accident. Forcing the parameter is better DX.
- **Unpremultiplied RGBA on the wire.** `tiny_skia` produces premultiplied
  alpha; `ImageData` expects non-premultiplied. We unpremultiply pixel-by-
  pixel before constructing `ImageData`. One allocation per repaint;
  fine for typical edit cadence.
- **`warnings` accessor on `RenderSession`.** The session-level diagnostic
  attached at `Backend::open` time is otherwise invisible to canvas
  consumers (it was only surfaced via `render()`'s `RenderResult`).

## Crate layout

```
crates/
├── core/src/session.rs              extended  — Any + handle()
├── backends/typst/src/lib.rs        extended  — TypstSession is pub;
│                                                page_size_pt, render_rgba;
│                                                typst_session_of accessor
└── bindings/wasm/
    ├── Cargo.toml                   extended  — web-sys features
    │                                            (CanvasRenderingContext2d,
    │                                             ImageData)
    └── src/engine.rs                extended  — paint, pageSize,
                                                  backendId, warnings
                                                  (calls typst_session_of
                                                  directly; no separate
                                                  adapter file)
```

## Future work (not in V1)

- **OffscreenCanvas / Worker support.** The current painter accepts only
  `CanvasRenderingContext2d`. Multi-page documents will jank typing on
  the host thread. Add `OffscreenCanvas` features and an overload taking
  `OffscreenCanvasRenderingContext2d`, or document the main-thread-only
  constraint loudly so consumers route pixels through `postMessage` if
  they want a worker.
- **Direct `CanvasRenderingContext2d` adapter.** V1 allocates an RGBA
  `Vec<u8>` per repaint. A direct path that hands tiny_skia's pixmap to
  the canvas (or a typed-array view backed by linear memory) would
  remove the allocation. Optimize only if profiling demands.
- **Click → editor and cursor → preview mapping.** Out of scope for the
  preview itself. If/when added, would slot in via the same
  `TypstSession` accessor by exposing `IdeWorld` + an `OriginMap` from
  MD→Typst conversion.
- **Cargo feature gate for the canvas path.** Consumers who want only
  PDF/SVG output and no canvas dependency could opt out of `web-sys`.
  Bundle-size impact is small relative to typst itself; defer until
  there's real demand.
