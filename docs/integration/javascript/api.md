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

### `quill.render(input, options?)`

Renders artifacts. `input` may be:

- `string` markdown
- `ParsedDocument`

`options`:

```ts
type RenderOptions = {
  format?: "pdf" | "svg" | "txt" | "png";
  ppi?: number;
};
```

### `quill.compile(input)`

Compiles into an opaque `CompiledDocument` handle for page-selective rendering.

### `compiled.renderPages(pages?, opts)`

Renders selected pages from a compiled document.

## Notes

- `QUILL` in frontmatter is required when parsing markdown.
- `quill.render(parsed)` emits a warning (not error) if `parsed.quillRef` does not match the quill name.
