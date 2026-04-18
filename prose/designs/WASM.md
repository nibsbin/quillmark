# Quillmark WASM API

> **Status**: Implemented  
> **Implementation**: `crates/bindings/wasm/src/`  
> **NPM**: `@quillmark-test/wasm`

## API (current)

```typescript
class Quill {
  static fromJson(source: string | object): Quill;
  static fromTree(tree: Map<string, Uint8Array> | Record<string, Uint8Array>): Quill;
}

class Quillmark {
  constructor();
  static parseMarkdown(markdown: string): ParsedDocument;
  registerQuill(quill: Quill): QuillInfo;
  getQuillInfo(name: string): QuillInfo;
  getQuillSchema(name: string): string; // YAML
  compileData(markdown: string): object;
  dryRun(markdown: string): void;
  render(parsed: ParsedDocument, options?: RenderOptions): RenderResult;
  compile(parsed: ParsedDocument, options?: CompileOptions): CompiledDocument;
  listQuills(): string[];
  unregisterQuill(name: string): void;
}
```

## Implementation notes

- `Quill.fromTree` accepts `Map<string, Uint8Array>` or a plain `Record<string, Uint8Array>`. Directory hierarchy is inferred from `/` path separators in keys (e.g. `"assets/fonts/Inter.ttf"` inserts into `assets/fonts/`). Values must be `Uint8Array`; passing a string throws.
- The WASM `Quill` struct holds `Arc<quillmark_core::Quill>`. The JS handle is not consumed on registration, and `registerQuill` may be called on multiple engines with the same handle. Each registration clones the underlying `Quill` once at storage time (the core engine stores its own copy), so the JS-level `Arc` prevents handle invalidation but does not eliminate the per-engine copy.

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
