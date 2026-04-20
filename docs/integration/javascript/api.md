# JavaScript/WASM API Reference

## Install

```bash
npm install @quillmark-test/wasm
```

## Core Flow

```javascript
import { ParsedDocument, Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
const quill = engine.quill(tree);

const parsed = ParsedDocument.fromMarkdown(markdown); // requires QUILL in frontmatter
const result = quill.render(parsed, { format: "pdf" });
```

## Main APIs

### `new Quillmark()`

Creates an engine with built-in backend registrations.

### `engine.quill(tree)`

Builds and validates a quill from a `Map<string, Uint8Array>`, then attaches the declared backend.

This is the canonical WASM path for a render-ready quill.

### `ParsedDocument.fromMarkdown(markdown)`

Parses markdown + YAML frontmatter into:

```ts
type ParsedDocument = {
  fields: Record<string, any>;
  quillRef: string;
};
```

### `quill.render(parsed, options?)`

Renders artifacts from a `ParsedDocument`.

`options`:

```ts
type RenderOptions = {
  format?: "pdf" | "svg" | "txt" | "png";
  assets?: Record<string, Uint8Array | number[]>;
  ppi?: number;    // PNG only; defaults to 144 (2× at 72 pt/inch)
  pages?: number[];
};
```

Returns:

```ts
type RenderResult = {
  artifacts: Array<{ format: string; bytes: Uint8Array; mimeType: string }>;
  warnings: Diagnostic[];
  outputFormat: "pdf" | "svg" | "txt" | "png";
  renderTimeMs: number;
};
```

### `quill.open(parsed)`

Opens a reusable render session for page-selective rendering.

### `session.pageCount`

Returns the number of pages in the opened session.

### `session.render(options?)`

Renders all or selected pages from the session. Accepts the same `RenderOptions` as `quill.render()`.

## Notes

- `QUILL` in frontmatter is required when parsing markdown.
- `quill.render(parsed)` emits a warning (not error) if `parsed.quillRef` does not match the quill name.
- `pages` selection is intended for page-addressable outputs (SVG/PNG), not PDF.
