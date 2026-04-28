# Canvas Preview

**Status:** Proposal
**Scope:** WASM bindings + Typst backend + auto-generated helper template
**Affected crates:** `quillmark-typst`, `quillmark-wasm`

## Intent

Give web consumers a Typst.app-style live preview: render pages onto a JS canvas
and let users click rendered text to focus the corresponding markdown region
(body or frontmatter field, including inside cards), and inversely highlight
preview regions as the editor cursor moves.

UX is region-flash, not character-precise. Clicking a rendered date flashes the
`date` field in the editor; placing the cursor in `cards[2].title` highlights
the rendered title region on the page.

## Non-goals

- Character-precise jumps. Landing on the right field or body is sufficient.
- Native (CLI / Python) exposure. Capability is WASM-only.
- DOM-level selection, accessibility shadow tree, or text copy from the canvas.
- A generic capability/extension registry across backends.
- Driving rendering decisions for scalar values. Typst's defaults apply; the
  plate author owns formatting.

## Approach

Reuse Typst's existing layout + IDE machinery; tag origins inside the
auto-generated `quillmark-helper` rather than threading them through the
MD→Typst conversion as a separate map.

| Concern                         | Owner                                    |
|---------------------------------|------------------------------------------|
| Layout, glyph spans             | `typst` (already in pipeline)            |
| Canvas painting                 | `typst-render` → `ImageData`             |
| Click → source offset           | `typst-ide::jump_from_click`             |
| Cursor → page coordinates       | `typst-ide::jump_from_cursor`            |
| Source offset ↔ Origin          | byte-range table over the helper source  |

### Origin

```rust
enum Origin {
    Field { card_index: Option<usize>, name: String }, // name="BODY" covers body
    Plate,
}
```

Two variants. Top-level body is `Field { card_index: None, name: "BODY" }`.
Card body is `Field { card_index: Some(i), name: "BODY" }`. Card frontmatter
fields are `Field { card_index: Some(i), name: "<field>" }`.

### Helper-instrumented `data` dict (the mechanism)

The auto-generated `quillmark-helper` (today: `lib.typ.template`) is extended
to expose every field through a wrapper that carries its `Origin`. The
wrapper is *pass-through*: it interpolates the value back into content mode so
Typst's default rendering applies, and we never pick a format.

```typst
// auto-generated, conceptually:

#let data = (
  title: [#metadata(("quill", "field", none, "title")) #eval(<converted-typst>, mode: "markup")],
  year:  [#metadata(("quill", "field", none, "year"))  #2024],
  CARDS: (
    (
      title: [#metadata(("quill", "field", 0, "title")) #eval(<converted-typst>, mode: "markup")],
      BODY:  [#metadata(("quill", "field", 0, "BODY"))  #eval(<converted-typst>, mode: "markup")],
      // ...
    ),
    // ...
  ),
  raw: ( title: "...", year: 2024, CARDS: ( ( title: "...", year: 2024, ... ), ... ) ),
)
```

Plate authors write `#data.X` for rendering and `#data.raw.X` for computation
or custom formatting:

```typst
#data.title                       // markdown, clickable, eval'd by the helper
#data.year                        // scalar, clickable, Typst's default rendering
#data.raw.year + 1                // raw int, for math
#str(data.raw.year, base: 16)     // raw, for formatting
#if data.raw.draft { ... }        // raw, for control flow

#for (i, card) in data.CARDS.enumerate() [
  = #card.title                   // clickable, card_index baked in
  #card.BODY                      // clickable
  #card.raw.year                  // raw value
]
```

Scalars covered by the wrapper: `str`, `int`, `float`, `bool`, `date`,
`duration`. Arrays and dicts pass through unwrapped (`data.tags` is a raw
array; iterate yourself). Markdown fields: the helper eval's inside the
wrapper, so plates no longer write `#eval(data.title, mode: "markup")`.

### Click resolution

Each wrapper occupies a known byte range in the auto-generated helper source.
At helper-generation time we record `(byte_range → Origin)` into a session
table. At click time:

1. `typst-ide::jump_from_click` returns `(Source, offset)`.
2. If the source is the helper, look up the range → `Origin`. Return
   `Jump::Origin`.
3. If the source is the user's plate file, return `Jump::Origin(Plate)`.
4. If Typst returned a URL jump, return `Jump::Url`.
5. Otherwise `Jump::Synthetic` (no-op).

No AST walks, no element-tree introspection, no `eval()`-source matching. The
helper source is one we own and lay out deterministically.

### Cursor resolution

`locate_cursor(origin)` looks up the origin's byte range in the helper source
and calls `typst-ide::jump_from_cursor(helper_source, range.start)` to get
page positions for highlight.

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

type Origin =
  | { kind: "field"; card_index: number | null; name: string }
  | { kind: "plate" };

type Jump =
  | { kind: "origin"; origin: Origin }
  | { kind: "url";    url: string }
  | { kind: "synthetic" };

type DocPosition = { page: number; x: number; y: number };
```

The three preview methods always travel together (they all need the cached
`PagedDocument` and `IdeWorld`), so they are bundled rather than scattered onto
`RenderSession` with per-method capability checks.

## Project organization

```
crates/
├── core/                       UNCHANGED
├── quillmark/                  UNCHANGED
├── backends/typst/
│   ├── lib.typ.template        extended  — emit wrapped `data` dict + raw mirror
│   ├── convert.rs              UNCHANGED — converted-Typst strings still produced as today
│   ├── compile.rs / world.rs   extended  — session retains PagedDocument + IdeWorld + origin table
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
  if (j?.kind === "origin") editor.flash(j.origin);
  if (j?.kind === "url")    window.open(j.url);
});

editor.onCursor(origin => {
  overlay.draw(preview?.locate_cursor(origin) ?? []);
});
```

`Preview` borrows from the session; it cannot outlive it. Same `Quill.open`
lifecycle powers bytes (`render`), pixels (`paint`), and jumps.

## Decisions and rationale

- **Helper-instrumented `data`, not a parallel OriginMap.** The helper is
  already the data binding seam. Tagging there gives one mechanism for
  markdown, scalar, and card fields, with card index baked in. No source map
  threaded through `convert.rs`; no AST walking over user plate code.
- **Pass-through wrappers, not formatted ones.** The wrapper interpolates the
  raw value into content mode (`[#metadata(...) #value]`), so Typst's defaults
  apply. Quillmark never picks a representation for dates, numbers, etc.
- **Two access paths: `data.X` (rendered, clickable) and `data.raw.X` (raw,
  for computation).** One rule for plate authors, no per-field-type branches.
  `eval()` boilerplate disappears: `#data.title` replaces
  `#eval(data.title, mode: "markup")`.
- **Two-variant Origin (`Field` + `Plate`).** Body and card body both collapse
  into `Field { name: "BODY" }`. One discriminator (`card_index`) covers the
  scope axis. Click handlers don't branch on body-vs-field separately.
- **Sub-handle (`session.preview()`), not flat methods on `RenderSession`.**
  The three methods are a unit and share a precondition. Bundling them keeps
  `RenderSession` focused on its universal verb (bytes out) and gives one-call
  capability detection. Mirrors `canvas.getContext("2d")`.
- **Not an `OutputFormat`.** Canvas is a side-effecting paint into a JS
  object, not a serializable byte stream. Forcing it into the enum either
  leaks `wasm_bindgen` types into `core` or makes `Artifact` dishonest.
- **Coalesce at the session, not at the format.** One compile feeds all three
  consumption verbs.
- **WASM-only.** Native consumers gain nothing from canvas, so `core` and
  orchestration stay unchanged and CLI/Python builds are byte-identical.
- **No generic extension registry.** One backend, one capability set; YAGNI.
- **Pre-release migration is acceptable.** Existing plates that use
  `#eval(data.foo, mode: "markup")` or `#data.draft` in conditions need
  one-line updates (`#data.foo`, `#data.raw.draft`). Scope is small enough to
  bundle with this proposal.

## Open questions

These are the design choices left to settle before planning the
implementation. Each is small in isolation but each affects code shape.

1. **`IdeWorld` integration.** `typst-ide` is a new dependency, pinned to
   `0.14.2` to match the rest of the typst stack. Decide whether the session
   caches an `IdeWorld` separately from `QuillWorld`, or wraps the existing
   world in a thin adapter. Affects `compile.rs` and `world.rs` structure.
2. **Coordinate-space contract.** `paint(ctx, page, scale)` takes a scale
   factor; `click(page, x, y)` and `DocPosition { x, y }` do not. Lock the
   units (Typst points vs. canvas pixels), and decide who handles
   `devicePixelRatio`. Without this the JS side cannot reliably round-trip
   coordinates.
3. **Edit-cadence lifecycle.** Today every edit means a fresh `quill.open`
   plus full recompile. Is that the v1 contract (drop and recreate `Preview`
   per keystroke), or do we want incremental compile on the session? Affects
   the WASM ownership story (`Preview` borrowing from `RenderSession`) and
   sets the perf budget.
4. **Wrapper-source layout determinism.** The byte-range table assumes the
   helper-generated wrappers live at predictable offsets. Confirm the helper
   generator produces stable, indexable layout (or commit to recording ranges
   as we generate, rather than computing them post hoc).

## Implementation sketch (subject to open questions)

1. Extend `lib.typ.template` generation in `quillmark-typst` to emit the
   wrapped `data` dict plus its `raw` mirror. Record `(byte_range → Origin)`
   for every wrapper.
2. Cache `PagedDocument`, `IdeWorld`, and the origin table on the Typst
   session.
3. Add `quillmark_typst::preview::TypstPreview` wrapping the cache; expose
   `jump_from_click` / `jump_from_cursor` translated through the table.
4. Add `crates/bindings/wasm/src/canvas.rs` painting via `typst-render` →
   `ImageData` → `ctx.put_image_data` (v1; direct context adapter later).
5. Expose `Preview` class and `RenderSession.preview()` in `engine.rs`.
6. Migrate fixtures and existing plates from `#eval(data.X, mode: "markup")`
   to `#data.X`, and from `data.X` in conditions/math to `data.raw.X`.

V1 painting is one allocation per repaint — fine for typical edit cadence.
Optimize to a direct `CanvasRenderingContext2d` adapter (and `OffscreenCanvas`
in a worker) only if profiling demands it.
