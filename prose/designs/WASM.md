# Quillmark WASM API

> **Status**: Implemented  
> **Implementation**: `crates/bindings/wasm/src/`  
> **NPM**: `@quillmark-test/wasm`

## API (current)

```typescript
class Quillmark {
  constructor();
  static parseMarkdown(markdown: string): ParsedDocument;
  registerQuill(quillJson: string | object): QuillInfo;
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
