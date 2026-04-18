# JavaScript/WASM API Reference

## Install

```bash
npm install @quillmark-test/wasm
```

## Core flow

```javascript
import { Quill, Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
const enc = new TextEncoder();
const quill = Quill.fromTree(new Map([
  ["Quill.yaml", enc.encode(quillYamlString)],
  ["plate.typ", enc.encode(plateTypString)],
]));
engine.registerQuill(quill);

const parsed = Quillmark.parseMarkdown(markdown); // requires QUILL in frontmatter
const result = engine.render(parsed, { format: "pdf" });
```

## Main APIs

### `Quillmark.parseMarkdown(markdown)`

Parses markdown + YAML frontmatter into:

```ts
type ParsedDocument = {
  fields: Record<string, any>;
  quillRef: string;
};
```

### `Quill.fromTree(tree)`

Builds a `Quill` handle from a flat `Map<string, Uint8Array>` (or plain object record) of relative paths to bytes.

### `engine.registerQuill(quill)`

Registers a pre-built `Quill` handle and returns `QuillInfo`.  
`registerQuill` is handle-only: pass a `Quill` from `Quill.fromTree(...)`.

### `engine.getQuillInfo(name)`

Returns:

```ts
type QuillInfo = {
  name: string;
  backend: string;
  metadata: Record<string, any>;
  example?: string;
  schema: string; // YAML schema text
  defaults: Record<string, any>;
  examples: Record<string, any[]>;
  supportedFormats: Array<"pdf" | "svg" | "txt" | "png">;
};
```

### `engine.getQuillSchema(name)`

Returns the public schema contract as YAML text.

### `engine.dryRun(markdown)`

Validates parse + schema/coercion without full rendering.

### `engine.render(parsed, options)`

Renders artifacts from a parsed document. Quill resolution always comes from `parsed.quillRef`.

## Notes

- `schema` is YAML text (not a JSON object).
- There is no stripped-schema API.
- `render`/`compile` do not accept quill override options; use `QUILL` in frontmatter.
- `registerQuill` does not accept raw JSON/tree payloads directly.
