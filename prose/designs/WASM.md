# Quillmark WASM API

> **Status**: Implemented - Production Ready
>
> This document defines the WebAssembly API for Quillmark, providing JavaScript/TypeScript bindings for bundler environments.

> **Implementation**: `bindings/quillmark-wasm/src/`

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
    "Quill.yaml": { "contents": "Quill:\n  name: my-quill\n  ..." },
    "plate.typ": { "contents": "#import \"@local/quillmark-helper:0.1.0\": data, eval-markup\n= Template\n\n#eval-markup(data.BODY)" },
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
  static parseMarkdown(markdown: string): ParsedDocument;
  registerQuill(quillJson: string | object): QuillInfo;
  getQuillInfo(name: string): QuillInfo;
  getStrippedSchema(name: string): object;
  compileData(markdown: string): object;
  dryRun(markdown: string): void; // throws on validation errors
  render(parsed: ParsedDocument, options?: RenderOptions): RenderResult;
  listQuills(): string[];
  unregisterQuill(name: string): void;
}
```

### Types

```typescript
interface ParsedDocument {
  fields: object;  // YAML frontmatter fields
  quillTag: string;  // Value of QUILL field or "__default__"
}

interface QuillInfo {
  name: string;
  backend: string;  // e.g., "typst"
  metadata: object;  // Quill metadata from Quill.yaml
  example?: string;  // Example markdown (if available)
  schema: object;  // JSON schema for fields (always includes full schema with UI metadata)
  defaults: object;  // Default values extracted from schema
  examples: object;  // Example values extracted from schema
  supportedFormats: Array<'pdf' | 'svg' | 'txt'>;  // Formats this backend supports
  getStrippedSchema(): object;  // Returns schema without UI metadata ("x-ui" fields)
}

interface RenderOptions {
  format?: 'pdf' | 'svg' | 'txt';
  assets?: Record<string, number[]>;  // Asset name to byte array mapping
  quillName?: string;  // overrides/fills in QUILL frontmatter field
}

interface RenderResult {
  artifacts: Artifact[];
  warnings: Diagnostic[];
  outputFormat: 'pdf' | 'svg' | 'txt';
  renderTimeMs: number;
}

interface Artifact {
  outputFormat: 'pdf' | 'svg' | 'txt';
  bytes: number[];  // Byte array
  mime_type: string;
}

interface Diagnostic {
  severity: 'error' | 'warning' | 'note';
  code?: string;
  message: string;
  location?: Location;
  hint?: string;
  source_chain?: string[];  // Source chain flattened
}

interface Location {
  file: string;
  line: number;
  column: number;
}
```

**API Design Notes:**

The `getQuillInfo` method always returns the full schema including UI metadata. For cases where you need the schema without UI-specific fields, use the `getStrippedSchema()` method on the returned `QuillInfo` object.

**Example usage:**
```typescript
// Get full quill info (always includes complete schema)
const info = engine.getQuillInfo("my-quill");

// Get schema without UI metadata using helper method
const strippedSchema = info.getStrippedSchema();
```

---

## Registry Architecture

**Design**: WASM bindings delegate to core engine's quill registry (no duplicate storage)

The WASM `Quillmark` struct wraps the core `quillmark::Quillmark` engine and delegates all quill registry operations to it. This ensures:

- **Single source of truth**: Quills are stored only in the core engine
- **Memory efficiency**: No duplicate HashMap in WASM layer
- **Consistency**: Impossible for registries to drift
- **Simplified code**: No synchronization overhead

**Implementation**:

```rust
pub struct Quillmark {
    inner: quillmark::Quillmark,  // Core engine with registry
}
```

**Registry Operations**:

- `registerQuill()` → `inner.register_quill()`
- `getQuillInfo()` → `inner.get_quill()`
- `listQuills()` → `inner.registered_quills()`
- `unregisterQuill()` → `inner.unregister_quill()`

**Benefits**:
- Reduced WASM binary size (no duplicate data structures)
- Lower memory footprint in browser environments
- Guaranteed consistency between core and WASM layers
- Automatic propagation of core engine improvements

---

## Error Handling

**Delegation to Core Types:** WASM bindings use `SerializableDiagnostic` from `quillmark-core` directly, not custom error wrappers.

**Error Structure:**
- All errors are serialized `SerializableDiagnostic` or `RenderError` objects from core
- Thrown as `JsValue` containing the serialized diagnostic
- Contains complete error information: severity, code, message, location, hint, and source chain

**Error Mapping:**
```typescript
// JavaScript catches errors with full diagnostic info
try {
  const result = engine.render(parsed, options);
} catch (error) {
  // error is a WasmError containing diagnostic information
  console.error(`Error: ${error.message}`);
  if (error.location) {
    console.error(`  at ${error.location.file}:${error.location.line}:${error.location.column}`);
  }
  if (error.hint) {
    console.error(`  hint: ${error.hint}`);
  }
  if (error.diagnostics) {
    // CompilationFailed case with multiple diagnostics
    for (const diag of error.diagnostics) {
      console.error(`  - ${diag.severity}: ${diag.message}`);
    }
  }
}
```

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
