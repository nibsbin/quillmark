# Quillmark WASM

WebAssembly bindings for the Quillmark markdown rendering engine.

## Overview

This crate provides WASM bindings for Quillmark, enabling use in web browsers, Node.js, and other JavaScript/TypeScript environments. All data exchange uses JSON serialization, and JavaScript is responsible for all I/O operations.

## Building

### For Web (bundler)

```bash
wasm-pack build --target bundler --scope quillmark
```

### For Node.js

```bash
wasm-pack build --target nodejs --scope quillmark
```

### All targets

```bash
bash scripts/build-wasm.sh
```

## Usage

```typescript
import { Quillmark } from '@quillmark-test/wasm';

// Create engine
const engine = new Quillmark();

// Prepare a Quill (template bundle)
const quillObj = {
  'Quill.toml': { contents: '[Quill]\nname = "my-quill"\nbackend = "typst"\nglue = "glue.typ"\n' },
  'glue.typ': { contents: '= Hello\n\n{{ body }}' }
};

// Register the Quill
engine.registerQuill('my-quill', quillObj);

// Render markdown
const markdown = `---
QUILL: my-quill
---

# Hello World`;

const result = engine.render(markdown, { format: 'pdf' });

// Access the PDF bytes
const pdfArtifact = result.artifacts.find(a => a.format === 'pdf');
if (pdfArtifact) {
  const blob = new Blob([pdfArtifact.bytes], { type: pdfArtifact.mimeType });
  const url = URL.createObjectURL(blob);
  window.open(url);
}
```

## API

The `Quillmark` class provides the following methods:

- `new Quillmark()` - Create a new engine instance
- `registerQuill(name, quillJson)` - Register a Quill template bundle from JSON
- `render(markdown, options)` - Render markdown to PDF/SVG/TXT
- `renderGlue(quillName, markdown)` - Debug helper to generate template source
- `listQuills()` - List registered Quill names
- `unregisterQuill(name)` - Unregister a Quill to free memory

### Render Options

```typescript
{
  format?: 'pdf' | 'svg' | 'txt',  // Output format (default: 'pdf')
  assets?: Map<string, Uint8Array>,  // Additional assets to inject
  quillName?: string  // Override QUILL frontmatter field
}
```

## WASM Boundary Types

Data crossing the JavaScript ↔ WebAssembly boundary:

- **Enums**: Serialized as lowercase strings (`"pdf"`, `"svg"`, `"txt"`)
- **Binary data**: `Vec<u8>` maps to `Uint8Array`
- **Collections**: `Vec<T>` maps to JS arrays; `HashMap<String, T>` maps to JS objects
- **Option**: `Option<T>` maps to `T | null`
- **Errors**: Thrown as exceptions with `QuillmarkError` containing diagnostics

## Design Principles

- **JSON-Only Data Exchange**: All structured data uses `serde-wasm-bindgen`
- **JavaScript Handles I/O**: WASM layer only handles rendering
- **Synchronous Operations**: Rendering is fast enough (<100ms typically)
- **No File System Abstractions**: JavaScript prepares all data

## License

Licensed under the Apache License, Version 2.0.
