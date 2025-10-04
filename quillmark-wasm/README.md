# Quillmark WASM

WebAssembly bindings for the Quillmark markdown rendering engine.

## Overview

This crate provides WASM bindings for Quillmark, enabling use in web browsers, Node.js, and other JavaScript/TypeScript environments.

## Publishing

This package is published to npm as `@quillmark/wasm`.

### Building for Web

```bash
wasm-pack build --target bundler --scope quillmark
```

### Building for Node.js

```bash
wasm-pack build --target nodejs --scope quillmark
```

## Usage

See the main [Quillmark repository](https://github.com/nibsbin/quillmark) for usage examples.

## API

The WASM API closely mirrors the Rust API, with these main classes:

- `QuillmarkEngine` - Main engine for managing Quills and workflows
- `Quill` - Represents a Quill template bundle
- `Workflow` - Rendering workflow for a specific Quill
- `QuillmarkError` - Error type with rich diagnostics

See `designs/WASM_API.md` in the repository for the complete API specification.

## License

Licensed under the Apache License, Version 2.0.
