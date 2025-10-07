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

This WASM package follows the canonical Quill JSON contract defined in
`quillmark-core/docs/JSON_CONTRACT.md`. In short: build a JS object shaped as
the file tree, then pass a JSON string to `Quill.fromJson(...)`.

Minimal example:

```typescript
import { Quillmark, Quill, OutputFormat } from '@quillmark-test/wasm';

const engine = Quillmark.create();
const quillObj = {
  name: 'my-quill',
  'Quill.toml': { contents: '[Quill]\nname = "my-quill"\nbackend = "typst"\nglue = "glue.typ"\n' },
  'glue.typ': { contents: '= Hello\n\n{{ body }}' }
};
const quill = Quill.fromJson(JSON.stringify(quillObj));
engine.registerQuill(quill);
const workflow = engine.loadWorkflow('my-quill');
const result = workflow.render('# Hi', { format: OutputFormat.PDF });
```

See `quillmark-core/docs/JSON_CONTRACT.md` for the full contract and examples.

## API

The WASM API closely mirrors the Rust API, with these main classes:

- `Quillmark` - Main engine for managing Quills and workflows
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
- Bytes / binary data: `Vec<u8>` and similar binary buffers map to `Uint8Array` across the WASM boundary.

  - When serializing a Quill into a JSON string for `Quill.fromJson()` you must represent binary file contents as an array of numeric byte values (e.g. `[137,80,78,71,...]`). The `quillmark-core` JSON parser accepts either a UTF-8 string or a numeric array in the `contents` field.
  - For runtime APIs that accept binary buffers directly (for example `Workflow.withAsset()`), pass a `Uint8Array` in the JS call (or `Buffer` in Node) — you do NOT JSON.stringify these runtime binary arguments.
- Collections: `Vec<T>` <-> JS arrays, and `HashMap<String, T>` / `BTreeMap<String, T>` map to plain JS objects or `Map` where appropriate. You can pass a `Map<string, Uint8Array>` for file maps, or a plain object whose values are `Uint8Array`.
- Option and nullability: `Option<T>` is represented as either the value or `null` in JS. Use `null` to indicate `None`.
- Errors / Result: Rust `Result` errors are surfaced to JS as thrown exceptions containing a serialized `QuillmarkError` object (see "Error Handling" above). Inspect `error.diagnostics` for rich diagnostic information.

## Current Status

Core API is implemented:
- ✅ `Quillmark` - Engine management
- ✅ `Quill.fromJson()` - Create Quills from JSON-serialized folder structure
- ✅ `Workflow.render()` - Synchronous rendering to PDF/SVG
- ✅ `Workflow.withAsset()` - Dynamic asset injection
- ✅ Rich error diagnostics

Not implemented (by design):
- ❌ `Quill.fromZip()`, `fromUrl()`, `fromPath()` - JavaScript handles I/O and folder serialization
- ❌ Progress callbacks - rendering is instant
- ❌ Streaming APIs - unnecessary for fast operations

## License

Licensed under the Apache License, Version 2.0.
