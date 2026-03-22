# Quillmark WASM API

Status: **Implemented** (2026-03-22)  
Package: `@quillmark-test/wasm`  
Source: `crates/bindings/wasm/src/`

## Quill JSON Contract
Root object with `files`; each entry is either `{ contents: "utf8" }`, `{ contents: [byte,…] }`, or a nested directory object. `files["Quill.yaml"]` is required; other files (plate, assets, packages) mirror the Rust bundle layout.

## API Surface (TypeScript)
```ts
class Quillmark {
  constructor();
  static parseMarkdown(markdown: string): ParsedDocument;
  registerQuill(quillJson: string | object): QuillInfo;
  getQuillInfo(nameOrRef: string): QuillInfo;
  getStrippedSchema(nameOrRef: string): object;
  compileData(markdown: string): object;
  dryRun(markdown: string): void;          // coercion + schema validation
  render(parsed: ParsedDocument, opts?: RenderOptions): RenderResult;
  listQuills(): string[];
  unregisterQuill(name: string): void;
}

type RenderOptions = {
  format?: 'pdf' | 'svg' | 'png' | 'txt';
  assets?: Record<string, number[]>;  // runtime assets
  quillName?: string;                 // override QUILL/frontmatter
  ppi?: number;                       // PNG PPI (default 144)
};
```

`QuillInfo` exposes `name`, `backend`, `metadata`, `schema`, `defaults`, `examples`, `example`, `supportedFormats`, plus `getStrippedSchema()` to drop `x-ui`.

`ParsedDocument` carries `fields` and `quillTag` (string).

`RenderResult` returns `artifacts` (`bytes`, `outputFormat`, `mime_type`) and `warnings` (SerializableDiagnostic).

## Behavior
- Registry is delegated to the core engine (no duplicated state).
- Errors throw `JsValue` containing `SerializableDiagnostic` or `RenderError` payloads.
- Formats match backend support: Typst → pdf/svg/png/txt; AcroForm → pdf.

Related: [QUILL.md](QUILL.md) (bundle contract), [GLUE_METADATA.md](GLUE_METADATA.md) (JSON injection).

**Benefits of Delegation:**
- Single source of truth for error structure
- Automatic propagation of new error fields
- Consistency with Python bindings approach
- No duplication of error handling logic

---

## Quill Selection

Two ways to specify which Quill to use:

1. **Inferred from Markdown**: Use `QUILL` frontmatter field in markdown
2. **Explicit via Options**: Pass `quillName` in `RenderOptions`

```markdown
---
QUILL: simple-letter
title: "My Document"
---
Content here
```

Or:

```typescript
const parsed = Quillmark.parseMarkdown(markdown);
const result = engine.render(parsed, { quillName: 'simple-letter' });
```

---

## Build & Distribution

**Build Command:**
```bash
bash scripts/build-wasm.sh
# or directly: wasm-pack build bindings/quillmark-wasm --target bundler
```

**NPM Package:** `@quillmark-test/wasm` for bundlers (webpack, rollup, vite)

**Installation:**
```bash
npm install @quillmark-test/wasm
```

---

## Performance

- **Typical render time**: 50-200ms for standard documents
- **Memory usage**: ~10-50MB depending on Quill complexity
- **Recommendations**: Reuse engines, batch operations, unregister unused Quills, minimize asset sizes
