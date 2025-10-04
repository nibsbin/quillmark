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

### Building all targets

```bash
bash scripts/build-wasm.sh
```

This will create `pkg-bundler/`, `pkg-nodejs/`, and `pkg-web/` directories with the compiled WASM modules for each target.

## Installation

```bash
npm install @quillmark/wasm
```

## Usage

### Basic Example

```typescript
import { QuillmarkEngine, Quill, OutputFormat } from '@quillmark/wasm';

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

## Current Limitations

- `Quill.fromZip()` - Not yet implemented (use `fromFiles` instead)
- `Quill.toZip()` - Not yet implemented
- Package downloading is disabled in WASM builds (embed packages in your Quills instead)

## License

Licensed under the Apache License, Version 2.0.
