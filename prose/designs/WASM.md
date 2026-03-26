# Quillmark WASM API

> **Status**: Implemented
> **Implementation**: `bindings/quillmark-wasm/src/`
> **NPM**: `@quillmark-test/wasm`

## Quill JSON Contract

Same format as [QUILL.md](QUILL.md) JSON contract. Root object with a `files` key.

## API

```typescript
class Quillmark {
  constructor();
  static parseMarkdown(markdown: string): ParsedDocument;
  registerQuill(quillJson: string | object): QuillInfo;
  getQuillInfo(name: string): QuillInfo;
  getStrippedSchema(name: string): object;
  compileData(markdown: string): object;
  dryRun(markdown: string): void;
  render(parsed: ParsedDocument, options?: RenderOptions): RenderResult;
  compile(parsed: ParsedDocument, options?: CompileOptions): CompiledDocument;
  listQuills(): string[];
  unregisterQuill(name: string): void;
}

class CompiledDocument {
  readonly pageCount: number;
  renderPages(pages?: number[] | null, options?: RenderPagesOptions): RenderResult;
}
```

## Types

```typescript
interface ParsedDocument {
  fields: object;
  quillTag: string;
}

interface QuillInfo {
  name: string;
  backend: string;
  metadata: object;
  example?: string;
  schema: object;
  defaults: object;
  examples: object;
  supportedFormats: Array<'pdf' | 'svg' | 'png' | 'txt'>;
  getStrippedSchema(): object;
}

interface RenderOptions {
  format?: 'pdf' | 'svg' | 'png' | 'txt';
  assets?: Record<string, number[]>;
  quillName?: string;
  ppi?: number;  // PNG pixels per inch (default: 144.0)
}

interface CompileOptions {
  assets?: Record<string, number[]>;
  quillName?: string;
}

interface RenderPagesOptions {
  format?: 'pdf' | 'svg' | 'png';
  ppi?: number;
}

interface RenderResult {
  artifacts: Artifact[];
  warnings: Diagnostic[];
  outputFormat: 'pdf' | 'svg' | 'png' | 'txt';
  renderTimeMs: number;
}

interface Artifact {
  outputFormat: 'pdf' | 'svg' | 'png' | 'txt';
  bytes: number[];
  mime_type: string;
}

interface Diagnostic {
  severity: 'error' | 'warning' | 'note';
  code?: string;
  message: string;
  location?: { file: string; line: number; column: number };
  hint?: string;
  source_chain?: string[];
}
```

`getQuillInfo` always returns the full schema including `x-ui` fields. Use `getStrippedSchema()` on the returned object to get schema without UI metadata.

## Quill Selection

Via QUILL frontmatter field, or via `quillName` in `RenderOptions`.

## Selective Page Rendering

- `compile()` performs layout once and returns a `CompiledDocument`.
- `CompiledDocument.renderPages()` renders selected pages without recompiling.
- Page indices are 0-based; returned artifacts preserve requested order and allow duplicates.
- `pages = null | undefined` renders all pages in document order.
- Out-of-bounds page indices are skipped and surfaced as warnings.
- PDF does not support explicit page selection; passing `pages` with `format: 'pdf'` returns an error.

## Error Handling

Errors are thrown as `JsValue` containing serialized `SerializableDiagnostic` from `quillmark-core`. Single-error cases have top-level `message`, `location`, `hint`. Multi-diagnostic cases (compilation failures) have a `diagnostics` array.
