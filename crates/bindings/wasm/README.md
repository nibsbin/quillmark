# Quillmark WASM

WebAssembly bindings for the Quillmark markdown rendering engine.

Maintained by [TTQ](https://tonguetoquill.com).

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
- `resolve.test.js` - Quill version resolution against the WASM engine

## Usage

```typescript
import { Quill, Quillmark } from '@quillmark-test/wasm';

// Step 1: Parse markdown
const markdown = `---
title: My Document
author: Alice
QUILL: my_quill
---

# Hello World

This is my document.
`;

const parsed = Quillmark.parseMarkdown(markdown);

// Step 2: Create engine and build/register Quill
const engine = new Quillmark();

const enc = new TextEncoder();
const quill = Quill.fromTree(new Map([
  ['Quill.yaml', enc.encode('Quill:\n  name: my_quill\n  version: "1.0"\n  backend: typst\n  plate_file: plate.typ\n  description: My template\n')],
  ['plate.typ', enc.encode('= {{ title }}\n\n{{ body | Content }}')],
]));
engine.registerQuill(quill);

// Step 3: Get Quill info (optional)
const info = engine.getQuillInfo('my-quill');
console.log('Supported formats:', info.supportedFormats);
console.log('Schema YAML:', info.schema);

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
- `Quill.fromTree(tree)` - Build a Quill from `Map<string, Uint8Array>` or plain object tree (Step 2)
- `registerQuill(quill)` - Register a pre-built Quill handle (Step 3)
- `render(parsedDoc, options)` - Render a ParsedDocument to final artifacts using the required `QUILL` reference parsed from the document (Step 4)

### Utility Methods

Additional methods for managing the engine and debugging:

- `new Quillmark()` - Create a new engine instance
- `getQuillInfo(name)` - Get metadata, schema, and supported formats for a registered Quill
- `getQuillSchema(name)` - Get the public YAML schema for a registered Quill
- `resolveQuill(ref)` - Return `QuillInfo` if the ref is already registered, or `null`
- `listQuills()` - List all registered Quills as `"name@version"` strings
- `unregisterQuill(name)` - Unregister a Quill by name or `"name@version"`
- `dryRun(markdown)` - Validate without backend compilation (fast feedback)
- `compile(parsed, opts?)` - Compile to an opaque `CompiledDocument` handle
- `compileData(markdown)` - Return the intermediate JSON data structure for debugging

### Quill handle lifetime

`Quill.fromTree` returns a handle backed by an `Arc`. The same handle can be registered with multiple engines. Once all `registerQuill` calls are done you may call `quill.free()` to release the WASM-side reference; do not use the handle again after calling `free()`.

### Factory types

```typescript
// fromTree — flat path → bytes map; paths must be relative with no .. or .
Quill.fromTree(tree: Map<string, Uint8Array> | Record<string, Uint8Array>): Quill
```

The factory throws a `WasmError` with `code: "quill::invalid_bundle"` on invalid input.

### Render Options

```typescript
type RenderOptions = {
  format?: 'pdf' | 'svg' | 'txt'
  // Assets are plain arrays of byte values, not Uint8Array
  assets?: Record<string, number[]>
}
```

### ParsedDocument

Returned by `parseMarkdown()`:

```typescript
{
  fields: object,  // YAML frontmatter fields
  quillRef: string  // Quill reference from required QUILL field
}
```

### QuillInfo

Returned by `getQuillInfo()`:

```typescript
{
  name: string,
  backend: string,  // e.g., "typst"
  metadata: object,  // Quill metadata from Quill.yaml
  example?: string,  // Example markdown (if available)
  schema: string,  // Public schema YAML text
  supportedFormats: Array<'pdf' | 'svg' | 'txt'>  // Formats this backend supports
}
```

## WASM Boundary Types

Data crossing the JavaScript ↔ WebAssembly boundary:

- **Enums**: Serialized as lowercase strings (`"pdf"`, `"svg"`, `"txt"`)
- **Binary data**: `Vec<u8>` maps to `Uint8Array`
- **Collections**: `Vec<T>` maps to JS arrays; object types use plain JS objects `{}`
- **Option**: `Option<T>` maps to `T | null`
- **Errors**: Thrown as `WasmError` objects with `type`, `severity`, `message`, and (where available) `code` fields. Factory errors use `code: "quill::invalid_bundle"`; registration and render errors carry codes from core diagnostics

## Design Principles

- **JSON-Only Data Exchange**: All structured data uses `serde-wasm-bindgen`
- **JavaScript Handles I/O**: WASM layer only handles rendering
- **Synchronous Operations**: Rendering is fast enough (<100ms typically)
- **No File System Abstractions**: JavaScript prepares all data
- **Error Delegation**: Error handling delegated to core types (`SerializableDiagnostic`) for consistency with Python bindings

## License

Licensed under the Apache License, Version 2.0.
