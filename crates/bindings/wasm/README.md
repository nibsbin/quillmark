# Quillmark WASM

WebAssembly bindings for Quillmark.

Maintained by [TTQ](https://tonguetoquill.com).

## Overview

Use Quillmark in browsers/Node.js with explicit in-memory trees (`Map<string, Uint8Array>` / `Record<string, Uint8Array>`).

## Build

```bash
bash scripts/build-wasm.sh
```

The script builds for `bundler` and `experimental-nodejs-module` targets with
`--weak-refs` enabled (see [Lifecycle](#lifecycle)).

## Test

```bash
bash scripts/build-wasm.sh
cd crates/bindings/wasm
npm install
npm test
```

## Usage

```ts
import { Document, Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
const quill = engine.quill(tree);

const markdown = `---
QUILL: my_quill
title: My Document
---

# Hello`;

const parsed = Document.fromMarkdown(markdown);
const result = quill.render(parsed, { format: "pdf" });
```

## API

### `new Quillmark()`
Create engine.

### `engine.quill(tree)`
Build + validate + attach backend. Returns a render-ready `Quill`.

### `Document.fromMarkdown(markdown)`
Parse markdown to a parsed document. Throws a JS `Error` (with `.diagnostics`
attached, see [Errors](#errors)) on any parse failure, including missing
`QUILL`, malformed YAML, and inputs over the 10 MB `parse::input_too_large`
limit.

### `doc.toMarkdown()`
Emit canonical Quillmark Markdown. Type-fidelity round-trip safe:
`Document.fromMarkdown(doc.toMarkdown())` returns a document equal to `doc`
under [`doc.equals`](#docequalsother). The output is **not** guaranteed
byte-equal to the original source — YAML quoting, key ordering, and
whitespace are normalised. Use `equals` (not string comparison) to test
semantic equality.

### `doc.equals(other)`
Structural equality between two `Document` handles. Compares `main` and
`cards` by value; parse-time `warnings` are intentionally excluded.

Use this to debounce upstream prop updates: keep the last parsed `Document`
and compare instead of re-parsing on every keystroke.

### `doc.cardCount`
O(1) getter for the number of composable cards (excluding the main card).
Use this to validate indices before calling card mutators (`removeCard`,
`updateCardField`, etc.) without allocating the full `cards` array.

### `quill.render(parsed, opts?)`
Render with a pre-parsed `Document`.

### `quill.open(parsed)` + `session.render(opts?)`
Open once, render all or selected pages (`opts.pages`).

The session also exposes `pageCount`, `backendId`, `warnings` (snapshot of
session-level diagnostics attached at `open` time), `pageSize(page)`, and
`paint(ctx, page, scale)` for canvas previews. See below.

### Canvas Preview (Typst only)

`session.paint(ctx, page, scale)` rasterizes a page directly into a
`CanvasRenderingContext2D`, skipping PNG/SVG byte round-trips. Pair with
`session.pageSize(page)` to size the canvas:

```ts
const dpr = window.devicePixelRatio || 1;
const userZoom = 1;                              // your zoom UI
const scale = dpr * userZoom;                    // multiplier on 72 ppi

const { widthPt, heightPt } = session.pageSize(0);
canvas.width  = Math.round(widthPt  * scale);    // device px
canvas.height = Math.round(heightPt * scale);
canvas.style.width  = `${widthPt  * userZoom}px`;
canvas.style.height = `${heightPt * userZoom}px`;

session.paint(canvas.getContext("2d"), 0, scale);
```

- `scale` is a multiplier on Typst's natural 72 ppi (1 pt → 1 device
  pixel at `scale = 1`). Always include `devicePixelRatio` for crisp
  output.
- `pageCount` and `pageSize(page)` are stable for the session's
  lifetime (immutable snapshot) — cache them.
- Setting `canvas.width` / `canvas.height` clears the backing store; if
  you reuse a canvas without resizing, call `clearRect` before `paint`.
- Currently main-thread only: `paint` accepts `CanvasRenderingContext2D`,
  not `OffscreenCanvasRenderingContext2D`. Worker support is on the
  follow-up list.
- Backend support: Typst only. Calling `paint` on a session opened by
  any other backend throws an error that includes the resolved
  `backendId`.

### Errors

Every method that can fail throws a JS `Error` with `.diagnostics` attached:

```ts
{ message: string, diagnostics: Diagnostic[] }
```

`diagnostics` is always non-empty — length 1 for most failures, length N for
backend compilation errors. `message` is derived from `diagnostics`
(`diagnostics[0].message` for single-diagnostic errors; an aggregate
`"<N> error(s): <first.message>"` summary for compilation failures).

Read `err.diagnostics[0]` for the primary diagnostic; iterate the array for
compilation failures. The same shape applies to every throw site:

- `Document.fromMarkdown` — parse errors (missing `QUILL`, YAML errors,
  `parse::input_too_large` for inputs > 10 MB).
- `Document` mutators (`setField`, `updateCardField`, etc.) — `EditError`
  variants (`ReservedName`, `InvalidFieldName`, `InvalidTagName`,
  `IndexOutOfRange`) appear in `diagnostics[0].message` with the
  `[EditError::<Variant>]` prefix.
- `quill.render` / `session.render` — backend compilation failures and
  validation errors.

### Lifecycle

The wasm bindings are built with `--weak-refs`, so dropped `Document`,
`Quill`, and `RenderSession` handles are reclaimed by `FinalizationRegistry`
without manual `.free()` discipline. `.free()` is still emitted as an eager
teardown hook for callers that want deterministic release. Requires
Node 14.6+ / current evergreen browsers (all supported targets).

## Notes

- Parsed markdown requires top-level `QUILL` in frontmatter. Empty input
  surfaces a dedicated "Empty markdown input cannot be parsed" message.
- QUILL mismatch during `quill.render(parsed)` is a warning (`quill::ref_mismatch`), not an error.
- Output schema APIs are no longer engine-level in WASM.

## License

Apache-2.0
