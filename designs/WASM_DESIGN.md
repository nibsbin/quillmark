# Quillmark WASM Design - Clean Frontend API

> **Status**: Implemented - Production Ready
>
> This document defines the complete WebAssembly API for Quillmark, providing JavaScript/TypeScript bindings for browser, Node.js, and bundler environments.

---

## Table of Contents

1. [Design Principles](#design-principles)
2. [Architecture Overview](#architecture-overview)
3. [Quill JSON Contract](#quill-json-contract)
4. [WASM API Surface](#wasm-api-surface)
5. [Type System & Serialization](#type-system--serialization)
6. [Error Handling](#error-handling)
7. [Usage Examples](#usage-examples)
8. [Build Targets & Distribution](#build-targets--distribution)
9. [Performance Characteristics](#performance-characteristics)

---

## Design Principles

1. **JSON-Only Data Exchange**: All structured data uses JSON serialization via `serde-wasm-bindgen`
2. **JavaScript Handles I/O**: The WASM layer only handles rendering; JavaScript fetches files, reads filesystems, and unzips archives
3. **Synchronous Operations**: Rendering is fast enough (typically <100ms) that async operations are unnecessary
4. **No File System Abstractions**: No `fromPath()`, `fromUrl()`, or `fromZip()` methods - JavaScript prepares all data
5. **Frontend-Friendly**: Intuitive API that maps naturally to JavaScript/TypeScript patterns
6. **Rich Error Diagnostics**: Comprehensive error information with location details and suggestions

---

## Architecture Overview

The WASM API consists of a single main class with minimal surface area:

```
JavaScript Frontend
        ↓ (JSON)
┌─────────────────────────────────┐
│        Quillmark WASM           │
│  ┌─────────────────────────────┐│
│  │     Engine Management       ││
│  │   - Register Quills         ││
│  │   - Load Workflows          ││
│  │   - Memory Management       ││
│  └─────────────────────────────┘│
│  ┌─────────────────────────────┐│
│  │     Quill Processing        ││
│  │   - JSON Parsing            ││
│  │   - Validation              ││
│  │   - Tree Structure          ││
│  └─────────────────────────────┘│
│  ┌─────────────────────────────┐│
│  │      Rendering              ││
│  │   - Template Processing     ││
│  │   - Backend Compilation     ││
│  │   - Artifact Generation     ││
│  └─────────────────────────────┘│
└─────────────────────────────────┘
        ↑ (Artifacts + Diagnostics)
   JavaScript Frontend
```

---

## Quill JSON Contract

The WASM API accepts Quills in a specific JSON format that represents the file structure and metadata. This is the **canonical format** that users pass to the WASM module.

### Standard Format

The JSON format MUST have a root object with a `files` key. The optional `metadata` key provides additional metadata that overrides defaults extracted from `Quill.toml`.

```json
{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"my-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n" },
    "glue.typ": { "contents": "= Template\n\n{{ body }}" },
    "assets": {
      "logo.png": { "contents": [137, 80, 78, 71, ...] }
    }
  }
}
```

### With Optional Metadata Override

```json
{
  "metadata": {
    "name": "my-quill-override",
    "version": "1.0.0",
    "description": "A beautiful letter template",
    "author": "John Doe",
    "license": "MIT",
    "tags": ["letter", "professional"]
  },
  "files": {
    "Quill.toml": { "contents": "..." },
    "glue.typ": { "contents": "..." }
  }
}
```

### Node Types

**File with UTF-8 string contents:**
```json
"file.txt": { "contents": "Hello, world!" }
```

**File with binary contents (byte array):**
```json
"image.png": { "contents": [137, 80, 78, 71, 13, 10, 26, 10] }
```

**Directory (nested object):**
```json
"assets": {
  "logo.png": { "contents": [...] },
  "icon.svg": { "contents": "..." }
}
```

**Empty directory:**
```json
"empty_dir": {}
```

### Validation Rules

1. Root MUST be an object with a `files` key
2. The `files` value MUST be an object
3. The `metadata` key is optional and overrides `Quill.toml` values
4. File nodes MUST have a `contents` key with either:
   - A string (UTF-8 text content)
   - An array of numbers 0-255 (binary content)
5. Directory nodes are objects without a `contents` key
6. Empty objects represent empty directories
7. After parsing, `Quill.toml` MUST exist and be valid
8. The glue file referenced in `Quill.toml` MUST exist

### Frontend Construction Example (TypeScript)

```typescript
// Build from file uploads
async function buildQuillFromUpload(files: File[]): Promise<object> {
  const fileTree: any = {};

  for (const file of files) {
    const path = file.webkitRelativePath || file.name;
    const parts = path.split('/');
    let current = fileTree;

    // Build nested directory structure
    for (let i = 0; i < parts.length - 1; i++) {
      if (!current[parts[i]]) current[parts[i]] = {};
      current = current[parts[i]];
    }

    const fileName = parts[parts.length - 1];
    const isBinary = /\.(png|jpg|jpeg|gif|pdf|woff|woff2|ttf|otf)$/i.test(fileName);

    current[fileName] = {
      contents: isBinary
        ? Array.from(new Uint8Array(await file.arrayBuffer()))
        : await file.text()
    };
  }

  return {
    metadata: {
      name: files[0]?.webkitRelativePath?.split('/')[0] || 'uploaded-quill'
    },
    files: fileTree
  };
}
```

---

## WASM API Surface

### Core Class: Quillmark

The main WASM interface provides a single class for all operations:

```typescript
class Quillmark {
  /// Create a new Quillmark engine
  static create(): Quillmark;

  /// Register a Quill template bundle
  /// Accepts either a JSON string or a JavaScript object representing the Quill file tree
  registerQuill(name: string, quillJson: string | object): void;

  /// Process markdown through template engine (debugging)
  /// Returns template source code (Typst, LaTeX, etc.)
  renderGlue(quillName: string, markdown: string): string;

  /// Render markdown to final artifacts (PDF, SVG, TXT)
  render(quillName: string, markdown: string, options?: RenderOptions): RenderResult;

  /// List registered Quill names
  listQuills(): string[];

  /// Unregister a Quill (free memory)
  unregisterQuill(name: string): void;
}
```

### Supporting Types

```typescript
interface RenderOptions {
  format?: 'pdf' | 'svg' | 'txt';
  assets?: Record<string, Uint8Array>;
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
  relatedLocations: Location[];
  hint?: string;
}

interface Location {
  file: string;
  line: number;
  column: number;
}

interface QuillmarkError {
  message: string;
  location?: Location;
  hint?: string;
  diagnostics?: Diagnostic[];
}
```

---

## Type System & Serialization

All data crossing the JavaScript ↔ WebAssembly boundary uses JSON/serde-compatible serialization via `serde-wasm-bindgen`.

### Serialization Rules

**Enums**: Exported Rust enums serialize as strings (not numeric discriminants)
```typescript
// ✅ Correct
{ format: "pdf" }
{ format: OutputFormat.PDF } // using generated enum

// ❌ Incorrect  
{ format: 0 } // numeric discriminant
```

**Binary Data**: `Vec<u8>` maps to `Uint8Array`
```typescript
// For Quill JSON construction - use numeric arrays
{
  "image.png": { 
    "contents": [137, 80, 78, 71, ...] // byte array
  }
}

// For runtime APIs - use Uint8Array directly
workflow.withAsset("logo.png", new Uint8Array([137, 80, 78, 71, ...]));
```

**Collections**: `Vec<T>` ↔ JavaScript arrays, `HashMap<String, T>` ↔ plain objects or `Map`

**Nullability**: `Option<T>` ↔ `value | null`

**Errors**: Rust `Result` errors surface as thrown exceptions containing serialized `QuillmarkError`

---

## Error Handling

All errors are thrown as JavaScript exceptions containing serialized `QuillmarkError` objects with rich diagnostic information.

### Error Structure
```typescript
try {
  engine.render('my-quill', markdown, options);
} catch (error) {
  const quillError = error as QuillmarkError;
  
  console.error('Error:', quillError.message);
  
  if (quillError.location) {
    console.error(`At ${quillError.location.file}:${quillError.location.line}:${quillError.location.column}`);
  }
  
  if (quillError.hint) {
    console.log('Hint:', quillError.hint);
  }
  
  if (quillError.diagnostics) {
    for (const diag of quillError.diagnostics) {
      console.log(`[${diag.severity}] ${diag.message}`);
    }
  }
}
```

### Common Error Scenarios

1. **Quill Registration Errors**:
   - Invalid JSON structure
   - Missing `Quill.toml`
   - Invalid TOML syntax
   - Missing glue file

2. **Rendering Errors**:
   - Template compilation failures
   - Invalid frontmatter
   - Backend-specific compilation errors
   - Missing assets

3. **Engine Errors**:
   - Quill not found
   - Invalid render options
   - Memory allocation failures

---

## Usage Examples

### Basic Usage

```typescript
import { Quillmark } from '@quillmark/wasm';

// Create engine
const engine = Quillmark.create();

// Register a simple Quill
const quillJson = {
  files: {
    "Quill.toml": { 
      contents: `[Quill]\nname = "simple-letter"\nbackend = "typst"\nglue = "glue.typ"\n` 
    },
    "glue.typ": { 
      contents: "= {{ title | String(default=\"Document\") }}\n\n{{ body | Content }}" 
    }
  }
};

engine.registerQuill('simple-letter', quillJson);

// Render markdown
const markdown = `---
title: "My Letter"
---

# Hello World

This is a simple letter.`;

const result = engine.render('simple-letter', markdown);

// Access the PDF bytes
const pdfArtifact = result.artifacts.find(a => a.format === 'pdf');
if (pdfArtifact) {
  // Create blob URL for download or display
  const blob = new Blob([pdfArtifact.bytes], { type: pdfArtifact.mimeType });
  const url = URL.createObjectURL(blob);
  window.open(url);
}
```

### With Custom Assets

```typescript
// Load custom font
const fontBytes = await fetch('/fonts/custom-font.ttf').then(r => r.arrayBuffer());

const result = engine.render('my-quill', markdown, {
  format: 'pdf',
  assets: {
    'custom-font.ttf': new Uint8Array(fontBytes)
  }
});
```

### Multiple Output Formats

```typescript
// Render to multiple formats
const pdfResult = engine.render('my-quill', markdown, { format: 'pdf' });
const svgResult = engine.render('my-quill', markdown, { format: 'svg' });
const txtResult = engine.render('my-quill', markdown, { format: 'txt' });
```

### Debugging with Template Source

```typescript
try {
  // Get the generated template source for debugging
  const glueSource = engine.renderGlue('my-quill', markdown);
  console.log('Generated template:', glueSource);
  
  // Then render normally
  const result = engine.render('my-quill', markdown);
} catch (error) {
  console.error('Template generation failed:', error);
}
```

---

## Build Targets & Distribution

The WASM module is built for three targets with separate packages:

### Build Commands

```bash
# Build all targets
bash scripts/build-wasm.sh

# Individual targets
wasm-pack build --target bundler --scope quillmark  # pkg-bundler/
wasm-pack build --target nodejs --scope quillmark   # pkg-nodejs/
wasm-pack build --target web --scope quillmark      # pkg-web/
```

### NPM Packages

- **Bundler Target** (`pkg-bundler/`): For webpack, rollup, vite, etc.
- **Node.js Target** (`pkg-nodejs/`): For server-side Node.js applications  
- **Web Target** (`pkg-web/`): For direct browser usage without bundler

### Installation

```bash
# Choose appropriate package for your environment
npm install @quillmark-test/wasm-bundler  # For bundlers
npm install @quillmark-test/wasm-nodejs   # For Node.js
npm install @quillmark-test/wasm-web      # For direct browser
```

---

## Performance Characteristics

### Rendering Performance
- **Typical render time**: 50-200ms for standard documents
- **Memory usage**: ~10-50MB depending on Quill complexity
- **Concurrent rendering**: Not supported (single-threaded WASM)

### Memory Management
- **Quill storage**: Quills remain in memory until explicitly unregistered
- **Asset caching**: Assets are processed per-render (no caching)
- **Garbage collection**: Automatic cleanup of temporary render data

### Optimization Recommendations

1. **Reuse engines**: Create one `Quillmark` instance and reuse it
2. **Batch operations**: Group multiple renders to avoid engine recreation
3. **Unregister unused Quills**: Call `unregisterQuill()` to free memory
4. **Minimize asset size**: Compress images and fonts before passing to WASM
5. **Use appropriate formats**: SVG for smaller file sizes, PDF for fidelity

### Performance Monitoring

```typescript
const start = performance.now();
const result = engine.render('my-quill', markdown);
const renderTime = performance.now() - start;

console.log(`Render took ${renderTime}ms (WASM reported: ${result.renderTimeMs}ms)`);
```

---

## Current Implementation Status

### ✅ Implemented Features
- Single `Quillmark` class for engine management
- Quill registration from JSON objects/strings
- Synchronous rendering to PDF/SVG/TXT
- Dynamic asset injection via `RenderOptions`
- Rich error diagnostics with location information
- Memory management (register/unregister Quills)
- Debug template source generation (`renderGlue`)

### ❌ Not Implemented (By Design)
- `Quill.fromZip()`, `fromUrl()`, `fromPath()` - JavaScript handles I/O
- Progress callbacks - rendering is fast enough to be synchronous
- Streaming APIs - unnecessary for quick operations
- Async operations - complexity not justified by performance gains

### 🔄 Future Considerations
- WebWorker integration for true non-blocking rendering
- Streaming artifact generation for very large documents
- WASM threads support when broadly available
- Advanced caching strategies for repeated renders

---

## References

- [quillmark-wasm/src/](../quillmark-wasm/src/) - Implementation source
- [QUILL_DESIGN.md](./QUILL_DESIGN.md) - Quill file structure specification
- [quillmark-wasm/README.md](../quillmark-wasm/README.md) - Package documentation
