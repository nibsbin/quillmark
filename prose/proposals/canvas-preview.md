# Canvas Preview

**Status:** Proposal
**Scope:** WASM bindings + Typst backend
**Affected crates:** `quillmark-typst`, `quillmark-wasm`

## Intent

Give web consumers a Typst.app-style live preview: render pages onto a JS canvas
and let users click rendered text to focus the corresponding markdown input
(body or frontmatter field), and inversely highlight preview regions as the
editor cursor moves.

## Non-goals

- Character-precise jumps. Landing in the right field or body is sufficient.
- Native (CLI / Python) exposure. Capability is WASM-only.
- DOM-level selection, accessibility shadow tree, or text copy from the canvas.
  These are deferred until there is demand.
- A generic capability/extension registry across backends.

## Approach

Reuse Typst's existing layout + IDE machinery; add a thin Quillmark translation
layer.

| Concern                         | Owner                          |
|---------------------------------|--------------------------------|
| Layout, glyph spans             | `typst` (already in pipeline)  |
| Canvas painting                 | `typst-render` → `ImageData`   |
| Click → `.typ` offset           | `typst-ide::jump_from_click`   |
| Cursor → page coordinates       | `typst-ide::jump_from_cursor`  |
| `.typ` offset ↔ markdown origin | Quillmark `OriginMap` (new)    |

The `OriginMap` is a coarse range table built during MD→Typst conversion. Each
generated `.typ` byte range is tagged with its source `Origin`:

```rust
enum Origin {
    Body,
    CardBody { card_index: usize },
    Field    { card_index: Option<usize>, name: String },
    Plate,                        // template-owned; not user-editable
}
```

Forward (click → editor): Typst returns a `.typ` offset; binary-search the map
to get the `Origin`. Reverse (cursor → preview): pick any offset inside the
range for that `Origin` and call Typst's cursor jump.

## API surface (WASM)

A typed sub-handle on `RenderSession`, retrieved once per session.
`session.preview()` is the capability check; backends without preview return
`null`.

```ts
class RenderSession {
  render(opts): RenderResult;          // existing
  page_count(): number;                // existing
  preview(): Preview | null;           // new
}

class Preview {
  paint(ctx: CanvasRenderingContext2d, page: number, scale: number): void;
  click(page: number, x: number, y: number): Jump | null;
  locate_cursor(origin: Origin): DocPosition[];
}

type Jump =
  | { kind: "origin"; origin: Origin }
  | { kind: "url";    url: string }
  | { kind: "synthetic" };           // landed in plate; ignore

type DocPosition = { page: number; x: number; y: number };
```

The three preview methods always travel together (they all need the cached
`PagedDocument`), so they are bundled rather than scattered onto
`RenderSession` with per-method capability checks.

## Project organization

```
crates/
├── core/                       UNCHANGED
├── quillmark/                  UNCHANGED
├── backends/typst/
│   ├── convert.rs              extended  — emit OriginMap during MD→Typst
│   ├── compile.rs / world.rs   extended  — session retains PagedDocument + IdeWorld + OriginMap
│   └── preview.rs              NEW       — wraps typst-ide; offset ↔ Origin translation
└── bindings/wasm/
    ├── canvas.rs               NEW       — CanvasRenderingContext2d painter
    └── engine.rs               extended  — Preview class, session.preview() accessor
```

The single cross-crate seam is a public accessor on the Typst backend that the
WASM binding imports directly:

```rust
// quillmark_typst::preview
pub fn try_from_session(session: &RenderSession) -> Option<&TypstPreview>;
```

No new traits in `core`. No `Any` downcasts. No feature negotiation through
orchestration. Native bindings never link the function.

## Lifecycle

```js
const session = quill.open(doc);          // compiles once, caches PagedDocument
const preview = session.preview();        // null on non-Typst backends
preview?.paint(ctx, 0, devicePixelRatio);

canvas.addEventListener("click", e => {
  const j = preview?.click(0, e.offsetX, e.offsetY);
  if (j?.kind === "origin") editor.focus(j.origin);
  if (j?.kind === "url")    window.open(j.url);
});

editor.onCursor(origin => {
  overlay.draw(preview?.locate_cursor(origin) ?? []);
});
```

`Preview` borrows from the session; it cannot outlive it. Same `Quill.open`
lifecycle powers bytes (`render`), pixels (`paint`), and jumps.

## Decisions and rationale

- **Sub-handle (`session.preview()`), not flat methods on `RenderSession`.**
  The three methods are a unit and share a precondition. Bundling them keeps
  `RenderSession` focused on its universal verb (bytes out) and gives one-call
  capability detection. Mirrors `canvas.getContext("2d")`.
- **Not an `OutputFormat`.** Canvas is a side-effecting paint into a JS object,
  not a serializable byte stream. Forcing it into the enum either leaks
  `wasm_bindgen` types into `core` or makes `Artifact` dishonest.
- **Coalesce at the session, not at the format.** One compile feeds all three
  consumption verbs.
- **Coarse `Origin`, not byte-precise mapping.** Lands the user in the right
  field or body. Avoids building a full source map through the conversion
  layer.
- **WASM-only.** Native consumers gain nothing from canvas, so `core` and
  orchestration stay unchanged and CLI/Python builds are byte-identical.
- **No generic extension registry.** One backend, one capability set; YAGNI.

## Implementation sketch

1. Extend MD→Typst conversion to emit `OriginMap` alongside the generated `.typ`.
2. Cache `PagedDocument`, `IdeWorld`, and `OriginMap` on the Typst session.
3. Add `quillmark_typst::preview::TypstPreview` wrapping the cache; expose
   `jump_from_click` / `jump_from_cursor` translated through the map.
4. Add `crates/bindings/wasm/src/canvas.rs` painting via `typst-render` →
   `ImageData` → `ctx.put_image_data` (v1; direct context adapter later).
5. Expose `Preview` class and `RenderSession.preview()` in `engine.rs`.

V1 painting is one allocation per repaint — fine for typical edit cadence.
Optimize to a direct `CanvasRenderingContext2d` adapter (and `OffscreenCanvas`
in a worker) only if profiling demands it.
