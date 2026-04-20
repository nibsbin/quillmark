# Quillmark WASM API

> **Status**: Implemented  
> **Implementation**: `crates/bindings/wasm/src/`  
> **NPM**: `@quillmark-test/wasm`

## API (current)

```typescript
class Quillmark {
  constructor();
  quill(tree: Map<string, Uint8Array>): Quill;
}

class Quill {
  render(parsed: ParsedDocument, opts?: RenderOptions): RenderResult;
  open(parsed: ParsedDocument): RenderSession;
}

class RenderSession {
  readonly pageCount: number;
  render(opts?: RenderOptions): RenderResult;
}

class ParsedDocument {
  static fromMarkdown(markdown: string): ParsedDocument;
  fields: Record<string, any>;
  quillRef: string;
}

interface RenderOptions {
  format?: "pdf" | "svg" | "txt" | "png";
  assets?: Record<string, Uint8Array | number[]>;
  ppi?: number;
  pages?: number[];
}

interface RenderResult {
  artifacts: Artifact[];
  warnings: Diagnostic[];
  outputFormat: "pdf" | "svg" | "txt" | "png";
  renderTimeMs: number;
}

interface Artifact {
  format: "pdf" | "svg" | "txt" | "png";
  bytes: Uint8Array;
  mimeType: string;
}

interface Diagnostic {
  severity: "error" | "warning" | "note";
  code?: string;
  message: string;
  location?: { file: string; line: number; column: number };
  hint?: string;
  sourceChain?: string[];
}
```

## Implementation notes

- `engine.quill(tree)` requires a `Map<string, Uint8Array>`; plain objects are rejected with an error. Directory hierarchy is inferred from `/` separators in keys (e.g. `"assets/fonts/Inter.ttf"` inserts into `assets/fonts/`). Values must be `Uint8Array`; anything else throws.
- `RenderOptions.ppi` defaults to 144.0 (2× at 72 pt/inch) and is only applied to PNG output.
- `RenderOptions.pages` is intended for page-addressable outputs (SVG, PNG), not PDF.

## Key contracts

- `ParsedDocument.quillRef` is required and sourced from `QUILL` frontmatter; parsing fails without it.
- `quill.render()` emits a warning (not an error) if `parsed.quillRef` does not match the quill name.
- No schema projection API is exposed.
- Render options do not include quill override fields.
