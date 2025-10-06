# Quillmark WASM

WebAssembly bindings for the Quillmark markdown rendering engine.

## Overview

This crate provides minimal WASM bindings for Quillmark, enabling use in web browsers, Node.js, and other JavaScript/TypeScript environments. All data exchange uses JSON serialization, and JavaScript is responsible for all I/O operations (fetching, reading files, unzipping archives).

## Publishing

This package is published to npm as `@quillmark-test/wasm`.

### Building for Web

```bash
wasm-pack build --target bundler --scope quillmark
```

### Building for Node.js

```bash
wasm-pack build --target nodejs --scope quillmark
```

### Building all targets

```bash
bash scripts/build-wasm.sh
```

This will create `pkg-bundler/`, `pkg-nodejs/`, and `pkg-web/` directories with the compiled WASM modules for each target.

## Installation

```bash
npm install @quillmark-test/wasm
```

## Usage

### Basic Example

```typescript
import { QuillmarkEngine, Quill, OutputFormat } from '@quillmark-test/wasm';

// Create the engine
const engine = QuillmarkEngine.create();

// Load a quill from files
const quillFiles = new Map([
  ['Quill.toml', tomlBytes],
  ['glue.typ', glueBytes]
]);
const metadata = {
  name: 'my-quill',
  backend: 'typst'
};
const quill = Quill.fromFiles(quillFiles, metadata);

// Register the quill
engine.registerQuill(quill);

// Load a workflow
const workflow = engine.loadWorkflow('my-quill');

// Render markdown to PDF
const markdown = '# Hello, World!\\n\\nThis is a test document.';
const result = workflow.render(markdown, { format: OutputFormat.PDF });

// Access the PDF bytes
const pdfBytes = result.artifacts[0].bytes;
```

### With Dynamic Assets

```typescript
const workflow = engine.loadWorkflow('my-quill');

// Add dynamic assets (e.g., images, logos)
const withAssets = workflow
  .withAsset('logo.png', logoBytes)
  .withAsset('signature.png', signatureBytes);

// Render with assets
const result = withAssets.render(markdown, { format: OutputFormat.PDF });
```

### Error Handling

```typescript
try {
  const result = workflow.render(markdown, { format: OutputFormat.PDF });
  
  // Check for warnings
  if (result.warnings.length > 0) {
    console.warn('Rendering warnings:', result.warnings);
  }
  
  // Use the artifacts
  const pdf = result.artifacts[0];
  downloadFile(pdf.bytes, 'output.pdf', pdf.mimeType);
} catch (error) {
  // Error is a serialized QuillmarkError object
  console.error('Rendering failed:', error);
  
  if (error.diagnostics) {
    error.diagnostics.forEach(diag => {
      console.error(`${diag.severity}: ${diag.message}`);
      if (diag.location) {
        console.error(`  at ${diag.location.file}:${diag.location.line}:${diag.location.column}`);
      }
    });
  }
}
```

## API

The WASM API closely mirrors the Rust API, with these main classes:

- `QuillmarkEngine` - Main engine for managing Quills and workflows
- `Quill` - Represents a Quill template bundle
- `Workflow` - Rendering workflow for a specific Quill
- `QuillmarkError` - Error type with rich diagnostics

See `designs/WASM_API.md` in the repository for the complete API specification.

## Design Principles

- **JSON-Only Data Exchange**: All structured data uses JSON serialization via `serde-wasm-bindgen`
- **JavaScript Handles I/O**: The WASM layer only handles rendering; JavaScript fetches files, reads filesystems, and unzips archives
- **Synchronous Operations**: Rendering is fast enough (typically <100ms) that async operations are unnecessary
- **No File System Abstractions**: No `fromPath()`, `fromUrl()`, or `fromZip()` methods - JavaScript prepares all data

## Type passing / WASM boundary

All data crossing the JavaScript <-> WebAssembly boundary uses JSON/serde-compatible serialization via `serde-wasm-bindgen`.
This means a few concrete rules you should follow when calling into the WASM module from JS/TS:

- Enums: exported Rust enums are serialized as strings (not numeric discriminants). This was a compatibility fix in the recent WASM changes — pass enum values as their string names (for example `"PDF"`) or use the generated JS enum helpers (e.g. `OutputFormat.PDF`). Avoid using raw numeric indices for enums.
- Bytes / binary data: `Vec<u8>` and similar binary buffers map to `Uint8Array`. When creating Quills or assets, pass `Uint8Array` (or `Buffer` in Node) for file contents.
- Collections: `Vec<T>` <-> JS arrays, and `HashMap<String, T>` / `BTreeMap<String, T>` map to plain JS objects or `Map` where appropriate. You can pass a `Map<string, Uint8Array>` for file maps, or a plain object whose values are `Uint8Array`.
- Option and nullability: `Option<T>` is represented as either the value or `null` in JS. Use `null` to indicate `None`.
- Errors / Result: Rust `Result` errors are surfaced to JS as thrown exceptions containing a serialized `QuillmarkError` object (see "Error Handling" above). Inspect `error.diagnostics` for rich diagnostic information.

## Current Status

Core API is implemented:
- ✅ `QuillmarkEngine` - Engine management
- ✅ `Quill.fromFiles()` - Create Quills from JSON file maps
- ✅ `Workflow.render()` - Synchronous rendering to PDF/SVG
- ✅ `Workflow.withAsset()` - Dynamic asset injection
- ✅ Rich error diagnostics

Not implemented (by design):
- ❌ `Quill.fromZip()`, `fromUrl()`, `fromPath()` - JavaScript handles I/O
- ❌ Progress callbacks - rendering is instant
- ❌ Streaming APIs - unnecessary for fast operations

## License

Licensed under the Apache License, Version 2.0.
