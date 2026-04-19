# Quillmark WASM API

> **Status**: Implemented  
> **Implementation**: `crates/bindings/wasm/src/`  
> **NPM**: `@quillmark-test/wasm`

## API (current)

```typescript
class Quill {
}

class Quillmark {
  constructor();
  static parseMarkdown(markdown: string): ParsedDocument;
  quill(tree: Map<string, Uint8Array> | Record<string, Uint8Array>): Quill;
}
```

## Implementation notes

- `engine.quill(...)` accepts `Map<string, Uint8Array>` or a plain `Record<string, Uint8Array>`. Directory hierarchy is inferred from `/` path separators in keys (e.g. `"assets/fonts/Inter.ttf"` inserts into `assets/fonts/`). Values must be `Uint8Array`; passing a string throws.

## Key contracts

- `ParsedDocument.quillRef` is required and is sourced from `QUILL` frontmatter.
- `QuillInfo.schema` is YAML text.
- No schema projection API is exposed.
- Render/compile options do not include quill override fields.

```typescript
interface ParsedDocument {
  fields: Record<string, any>;
  quillRef: string;
}

interface QuillInfo {
  name: string;
  backend: string;
  metadata: Record<string, any>;
  example?: string;
  schema: string; // YAML
  defaults: Record<string, any>;
  examples: Record<string, any[]>;
  supportedFormats: Array<"pdf" | "svg" | "png" | "txt">;
}
```
