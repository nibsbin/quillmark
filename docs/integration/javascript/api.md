# JavaScript/WASM API Reference

## Install

```bash
npm install @quillmark-test/wasm
```

## Core flow

```javascript
import { Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
engine.registerQuill(quillBundle);

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

### `engine.registerQuill(quillJson)`

Registers a quill bundle and returns `QuillInfo`.

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
