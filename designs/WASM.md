# Quillmark WASM API

> **Status**: Implemented - Production Ready
>
> This document defines the WebAssembly API for Quillmark, providing JavaScript/TypeScript bindings for bundler environments.

> **Implementation**: `quillmark-wasm/src/`

---

## Design Principles

1. **JSON-Only Data Exchange**: All structured data uses JSON serialization
2. **JavaScript Handles I/O**: WASM layer only handles rendering
3. **Synchronous Operations**: Rendering is fast enough (<100ms) for sync APIs
4. **Frontend-Friendly**: Intuitive API for JavaScript/TypeScript
5. **Rich Error Diagnostics**: Comprehensive error information with locations and suggestions

---

## Quill JSON Contract

JSON format with a root object containing a `files` key:

```json
{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"my-quill\"\n..." },
    "glue.typ": { "contents": "= Template\n\n{{ body }}" },
    "assets": {
      "logo.png": { "contents": [137, 80, 78, 71, ...] }
    }
  }
}
```

**Node Types:**
- File with UTF-8: `"file.txt": { "contents": "Hello" }`
- File with binary: `"image.png": { "contents": [137, 80, 78, 71, ...] }`
- Directory: `"assets": { "logo.png": {...} }`
- Empty directory: `"empty_dir": {}`

---

## WASM API Surface

### Main Class

```typescript
class Quillmark {
  constructor();
  registerQuill(name: string, quillJson: string | object): void;
  render(markdown: string, options?: RenderOptions): RenderResult;
  renderGlue(quillName: string, markdown: string): string;
  listQuills(): string[];
  unregisterQuill(name: string): void;
}
```

### Types

```typescript
interface RenderOptions {
  format?: 'pdf' | 'svg' | 'txt';
  assets?: Record<string, Uint8Array>;
  quillName?: string;  // overrides/fills in QUILL frontmatter field
}

interface RenderResult {
  artifacts: Artifact[];
  warnings: Diagnostic[];
  renderTimeMs: number;
}

interface Artifact {
  format: 'pdf' | 'svg' | 'txt';
  bytes: Uint8Array;
  mimeType: string;
}

interface Diagnostic {
  severity: 'error' | 'warning' | 'note';
  code?: string;
  message: string;
  location?: Location;
  hint?: string;
}
```

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
const result = engine.render(markdown, { quillName: 'simple-letter' });
```

---

## Build & Distribution

**Build Command:**
```bash
bash scripts/build-wasm.sh
# or directly: wasm-pack build quillmark-wasm --target bundler
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
