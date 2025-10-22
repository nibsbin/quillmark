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

## Testing

Minimal smoke tests validate the core WASM functionality:

```bash
# Build WASM module first
bash scripts/build-wasm.sh

# Run tests
cd quillmark-wasm
npm install
npm test
```

The test suite includes:
- `basic.test.js` - Core WASM API functionality tests
- `tests/usaf_memo.test.js` - Smoke test that verifies WASM PDF output is functionally identical to native cargo output for the `usaf_memo` example (Typst backend)
- `tests/usaf_form_8.test.js` - Smoke test that verifies WASM PDF output is byte-for-byte identical to native cargo output for the `usaf_form_8` example (Acroform backend)

## Usage

```typescript
import { Quillmark } from '@quillmark-test/wasm';

// Step 1: Parse markdown
const markdown = `---
title: My Document
author: Alice
QUILL: my-quill
---

# Hello World

This is my document.
`;

const parsed = Quillmark.parseMarkdown(markdown);

// Step 2: Create engine and register Quill
const engine = new Quillmark();

const quillJson = {
  files: {
    'Quill.toml': { 
      contents: '[Quill]\nname = "my-quill"\nbackend = "typst"\nglue_file = "glue.typ"\ndescription = "My template"\n' 
    },
    'glue.typ': { 
      contents: '= {{ title }}\n\n{{ body | Content }}' 
    }
  }
};

engine.registerQuill('my-quill', quillJson);

// Step 3: Get Quill info (optional)
const info = engine.getQuillInfo('my-quill');
console.log('Supported formats:', info.supportedFormats);
console.log('Field schemas:', info.fieldSchemas);

// Step 4: Render
const result = engine.render(parsed, { format: 'pdf' });

// Access the PDF bytes
const pdfArtifact = result.artifacts[0];
const blob = new Blob([pdfArtifact.bytes], { type: pdfArtifact.mimeType });
const url = URL.createObjectURL(blob);
window.open(url);
```

## API

The `Quillmark` class provides the following methods:

### Workflow Methods

The main workflow for rendering documents:

- `static parseMarkdown(markdown)` - Parse markdown into a ParsedDocument (Step 1)
- `registerQuill(name, quillJson)` - Register a Quill template bundle from JSON (Step 2)
- `getQuillInfo(name)` - Get shallow Quill metadata and configuration options (Step 3)
- `render(parsedDoc, options)` - Render a ParsedDocument to final artifacts (Step 4)

### Utility Methods

Additional methods for managing the engine and debugging:

- `new Quillmark()` - Create a new engine instance
- `renderGlue(quillName, markdown)` - Debug helper that processes markdown through the template engine and returns the intermediate template source code (e.g., Typst, LaTeX) without compiling to final artifacts. Useful for inspecting template output during development.
- `listQuills()` - List all registered Quill names
- `unregisterQuill(name)` - Unregister a Quill to free memory

### Render Options

```typescript
{
  format?: 'pdf' | 'svg' | 'txt',  // Output format (default: 'pdf')
  assets?: Record<string, Uint8Array>,  // Additional assets to inject as plain object (not Map)
  quillName?: string  // Override quill_tag from ParsedDocument
}
```

### ParsedDocument

Returned by `parseMarkdown()`:

```typescript
{
  fields: object,  // YAML frontmatter fields
  quillTag?: string  // Value of QUILL field (if present)
}
```

### QuillInfo

Returned by `getQuillInfo()`:

```typescript
{
  name: string,
  backend: string,  // e.g., "typst"
  metadata: object,  // Quill metadata from Quill.toml
  example?: string,  // Example markdown (if available)
  fieldSchemas: object,  // Field schema definitions
  supportedFormats: Array<'pdf' | 'svg' | 'txt'>  // Formats this backend supports
}
```

## WASM Boundary Types

Data crossing the JavaScript ↔ WebAssembly boundary:

- **Enums**: Serialized as lowercase strings (`"pdf"`, `"svg"`, `"txt"`)
- **Binary data**: `Vec<u8>` maps to `Uint8Array`
- **Collections**: `Vec<T>` maps to JS arrays; object types use plain JS objects `{}`
- **Option**: `Option<T>` maps to `T | null`
- **Errors**: Thrown as exceptions with `QuillmarkError` containing diagnostics

## Design Principles

- **JSON-Only Data Exchange**: All structured data uses `serde-wasm-bindgen`
- **JavaScript Handles I/O**: WASM layer only handles rendering
- **Synchronous Operations**: Rendering is fast enough (<100ms typically)
- **No File System Abstractions**: JavaScript prepares all data

## License

Licensed under the Apache License, Version 2.0.
