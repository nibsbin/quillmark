# Quillmark WASM API Design

> **Status**: Implementation Phase - Formalized Minimal Interface
>
> This document defines the WebAssembly API surface for exposing Quillmark's rendering capabilities to web applications. The API is intentionally minimal and uses JSON serialization for all data interchange between JavaScript and Rust.

---

## Table of Contents

1. [Overview](#overview)
2. [Core Philosophy](#core-philosophy)
3. [Primary Use Cases](#primary-use-cases)
4. [API Surface](#api-surface)
5. [Quill Management](#quill-management)
6. [Rendering Pipeline](#rendering-pipeline)
7. [Asset Management](#asset-management)
8. [Advanced Capabilities](#advanced-capabilities)
9. [Performance & Optimization](#performance--optimization)
10. [Error Handling](#error-handling)
11. [Future Considerations](#future-considerations)

---

## Overview

### Purpose

The Quillmark WASM API provides a production-ready interface for web applications to render markdown documents with professional typesetting. The API is designed for:

- **Document Editor Applications**: Real-time markdown preview with PDF/SVG export
- **Content Management Systems**: Server-side and client-side document generation
- **API Services**: Stateless document rendering endpoints
- **Static Site Generators**: Build-time document compilation

### Target Environments

- **Browsers**: Modern browsers with WebAssembly support (Chrome, Firefox, Safari, Edge)
- **Node.js**: Server-side rendering, APIs, and CLI tools (v20+)
- **Edge Functions**: Cloudflare Workers, Vercel Edge, Deno Deploy
- **Electron/Tauri**: Desktop applications with document generation needs

### Design Goals

1. **Minimal Surface Area**: Expose only what's necessary; hide internal complexity
2. **JSON-Only Serialization**: All data passes through JSON for simplicity and debuggability
3. **Instant Rendering**: Rendering is fast enough that progress tracking is unnecessary
4. **Stateless by Default**: Each render is independent unless explicitly cached
5. **Zero Configuration**: Sensible defaults for 90% of use cases

---

## Core Philosophy

### Core Principles

**1. JSON for All Data Exchange**

All structured data crossing the WASM boundary uses JSON serialization via `serde-wasm-bindgen`. Quill files are passed as JSON objects mapping filenames to Uint8Array byte arrays.

**2. JavaScript Handles I/O**

The WASM layer only handles rendering. JavaScript is responsible for fetching files, reading file systems, unzipping archives, and preparing data.

**3. Explicit Management**

No global state. No auto-registration. Every Quill, asset, and render is explicitly managed for predictability.

**4. Fail Fast with Rich Context**

Errors include diagnostics with file locations, line numbers, and actionable hints.

---

## Primary Use Cases

### 1. Document Editor with Live Preview

```typescript
// User edits markdown; app renders PDF on-demand
const engine = QuillmarkEngine.create();

// JavaScript fetches and prepares the Quill
const quillZip = await fetch('https://cdn.quillmark.io/letter.zip').then(r => r.arrayBuffer());
const quillFiles = await unzipToFileMap(quillZip);
const quill = Quill.fromFiles(quillFiles, { name: 'letter', backend: 'typst' });

engine.registerQuill(quill);
const workflow = engine.loadWorkflow('letter');

// Re-render on every keystroke (debounced)
const result = workflow.render(editorContent, { format: OutputFormat.PDF });
displayPreview(result.artifacts[0].bytes);
```

### 2. Batch Document Generation (Server)

```typescript
// API endpoint: generate invoices from templates
const engine = QuillmarkEngine.create();

// Node.js reads Quill from filesystem
const fs = require('fs');
const quillFiles = {
  'Quill.toml': fs.readFileSync('./quills/invoice/Quill.toml'),
  'glue.typ': fs.readFileSync('./quills/invoice/glue.typ')
};
const quill = Quill.fromFiles(quillFiles, { name: 'invoice', backend: 'typst' });
engine.registerQuill(quill);

const workflow = engine.loadWorkflow('invoice');
for (const order of orders) {
  const markdown = generateMarkdown(order);
  const result = workflow.render(markdown, { format: OutputFormat.PDF });
  await saveToS3(result.artifacts[0]);
}
```

### 3. User-Uploaded Custom Templates

```typescript
// User uploads a Quill ZIP; app validates and uses it
const zipFile = await uploadForm.getFile();
const quillFiles = await unzipToFileMap(await zipFile.arrayBuffer());
const quill = Quill.fromFiles(quillFiles, { name: 'custom', backend: 'typst' });
quill.validate(); // Throws if invalid

const engine = QuillmarkEngine.create();
engine.registerQuill(quill);
const workflow = engine.loadWorkflow('custom');
```

### 4. Multi-Format Export

```typescript
// Generate PDF and SVG from same markdown
const workflow = engine.loadWorkflow('report');
const markdown = buildReport(data);

const pdfResult = workflow.render(markdown, { format: OutputFormat.PDF });
const svgResult = workflow.render(markdown, { format: OutputFormat.SVG });
```

---

## API Surface

### Core Types

```typescript
/**
 * Output formats supported by backends
 */
enum OutputFormat {
  PDF = 'pdf',
  SVG = 'svg',
  TXT = 'txt'
}

/**
 * Severity levels for diagnostics
 */
enum Severity {
  ERROR = 'error',
  WARNING = 'warning',
  NOTE = 'note'
}

/**
 * Source location for errors and warnings
 */
interface Location {
  readonly file: string;
  readonly line: number;
  readonly column: number;
}

/**
 * Diagnostic message (error, warning, or note)
 */
interface Diagnostic {
  readonly severity: Severity;
  readonly code?: string;
  readonly message: string;
  readonly location?: Location;
  readonly relatedLocations: Location[];
  readonly hint?: string;
}

/**
 * Rendered artifact (PDF, SVG, etc.)
 */
interface Artifact {
  readonly format: OutputFormat;
  readonly bytes: Uint8Array;
  readonly mimeType: string; // e.g., 'application/pdf'
}

/**
 * Result of a render operation
 */
interface RenderResult {
  readonly artifacts: Artifact[];
  readonly warnings: Diagnostic[];
  readonly metadata: {
    renderTimeMs: number;
    backend: string;
    quillName: string;
  };
}

/**
 * Options for rendering
 */
interface RenderOptions {
  format?: OutputFormat;
}

/**
 * Quill metadata
 */
interface QuillMetadata {
  readonly name: string;
  readonly version?: string;
  readonly backend: string;
  readonly description?: string;
  readonly author?: string;
}
```

### Main Classes

```typescript
/**
 * Represents a Quill template bundle
 */
class Quill {
  /**
   * Create Quill from in-memory file map
   * Files are passed as a JSON object: { [filename: string]: Uint8Array }
   */
  static fromFiles(files: object, metadata: QuillMetadata): Quill;

  /**
   * Validate Quill structure (throws on error)
   */
  validate(): void;

  /**
   * Get Quill metadata
   */
  getMetadata(): QuillMetadata;

  /**
   * List files in the Quill
   */
  listFiles(): string[];
}

/**
 * Rendering workflow for a specific Quill
 */
class Workflow {
  /**
   * Render markdown to artifacts
   */
  render(markdown: string, options?: RenderOptions): RenderResult;

  /**
   * Render pre-processed glue content (advanced)
   */
  renderSource(content: string, options?: RenderOptions): RenderResult;

  /**
   * Process markdown to glue without compilation (for debugging)
   */
  processGlue(markdown: string): string;

  /**
   * Add dynamic assets (builder pattern)
   * Assets are passed as a JSON object: { [filename: string]: Uint8Array }
   */
  withAsset(filename: string, bytes: Uint8Array): Workflow;

  /**
   * Add multiple dynamic assets
   */
  withAssets(assets: object): Workflow;

  /**
   * Clear all dynamic assets
   */
  clearAssets(): Workflow;

  /**
   * Get workflow metadata
   */
  readonly backendId: string;
  readonly supportedFormats: OutputFormat[];
  readonly quillName: string;
}

/**
 * Main engine for managing Quills and rendering
 */
class QuillmarkEngine {
  /**
   * Create a new engine instance
   */
  static create(options?: EngineOptions): QuillmarkEngine;

  /**
   * Register a Quill by name
   */
  registerQuill(quill: Quill): void;

  /**
   * Unregister a Quill
   */
  unregisterQuill(name: string): void;

  /**
   * List registered Quill names
   */
  listQuills(): string[];

  /**
   * Get details about a registered Quill
   */
  getQuill(name: string): Quill | undefined;

  /**
   * Load a workflow for rendering
   */
  loadWorkflow(quillOrName: Quill | string): Workflow;

  /**
   * List available backends
   */
  listBackends(): string[];

  /**
   * Get supported formats for a backend
   */
  getSupportedFormats(backend: string): OutputFormat[];
}

/**
 * Engine creation options
 */
interface EngineOptions {
  /**
   * Enable caching of compiled Quills (default: false)
   */
  enableCache?: boolean;

  /**
   * Maximum cache size in bytes (default: 100MB)
   */
  maxCacheSize?: number;
}

/**
 * Error thrown by Quillmark operations
 */
class QuillmarkError extends Error {
  readonly diagnostics: Diagnostic[];
  readonly kind: 'render' | 'validation' | 'network' | 'system';
}
```

---

## Quill Management

### Loading Quills

JavaScript handles all I/O. The WASM layer only receives prepared file data:

```typescript
// Browser: fetch and unzip
const response = await fetch('https://cdn.quillmark.io/quills/letter.zip');
const zipBuffer = await response.arrayBuffer();
const files = await unzipToObject(zipBuffer); // { [filename]: Uint8Array }
const quill = Quill.fromFiles(files, { name: 'letter', backend: 'typst' });

// Node.js: read from filesystem
const fs = require('fs');
const files = {
  'Quill.toml': fs.readFileSync('./quills/letter/Quill.toml'),
  'glue.typ': fs.readFileSync('./quills/letter/glue.typ')
};
const quill = Quill.fromFiles(files, { name: 'letter', backend: 'typst' });
```

### Registration and Lifecycle

Explicit registration prevents naming collisions and makes dependencies clear:

```typescript
// Validate and register
const quill = Quill.fromFiles(files, metadata);
quill.validate(); // Always validate before use
engine.registerQuill(quill);

// Use registered Quill by name
const workflow = engine.loadWorkflow('letter');

// Unregister when done (free memory)
engine.unregisterQuill('letter');
```

### Validation

Validation should be explicit and comprehensive. Fail early before first render:

```typescript
const quill = Quill.fromFiles(userUploadedFiles, metadata);

try {
  quill.validate();
  // Checks:
  // - Quill.toml exists and is valid
  // - glue.typ exists
  // - backend is supported
  // - no malicious file paths
  // - size limits
} catch (e) {
  showUserFriendlyError(e.diagnostics);
}
```

### Enumeration and Discovery

Apps can list and inspect Quills for UI purposes:

```typescript
// List all registered Quills
const quillNames = engine.listQuills();

// Get details for UI
const quills = quillNames.map(name => {
  const quill = engine.getQuill(name)!;
  const metadata = quill.getMetadata();
  return {
    name: metadata.name,
    description: metadata.description,
    backend: metadata.backend
  };
});

// Show in dropdown
populateQuillSelector(quills);
```

---

## Rendering Pipeline

### Basic Rendering

Rendering is synchronous (fast enough to not need async):

```typescript
const workflow = engine.loadWorkflow('report');
const markdown = '# Report\n\nContent here...';

const result = workflow.render(markdown, {
  format: OutputFormat.PDF
});

// Access artifacts
const pdf = result.artifacts[0];
console.log(`Generated ${pdf.bytes.length} bytes of ${pdf.mimeType}`);

// Check warnings
if (result.warnings.length > 0) {
  console.warn('Rendering warnings:', result.warnings);
}
```

### Multi-Format Rendering

Same workflow can produce multiple formats:

```typescript
const workflow = engine.loadWorkflow('article');

// Render to different formats
const pdfResult = workflow.render(markdown, { format: OutputFormat.PDF });
const svgResult = workflow.render(markdown, { format: OutputFormat.SVG });

// Download both
downloadFile(pdfResult.artifacts[0].bytes, 'article.pdf');
downloadFile(svgResult.artifacts[0].bytes, 'article.svg');
```

### Debugging Output

Expose intermediate glue for debugging template issues:

```typescript
const workflow = engine.loadWorkflow('custom-template');

// Generate glue without compiling
const glue = workflow.processGlue(markdown);
console.log('Generated Typst:', glue);

// Copy to editor for tweaking
navigator.clipboard.writeText(glue);
```

---

## Asset Management

### Dynamic Assets

Dynamic assets are essential for user-generated content:

```typescript
const workflow = engine.loadWorkflow('invoice');

// Add company logo
const logo = await fetch('/api/company/logo').then(r => r.arrayBuffer());
const withLogo = workflow.withAsset('logo.png', new Uint8Array(logo));

// Add multiple assets (passed as JSON object)
const withAllAssets = workflow.withAssets({
  'logo.png': logoBytes,
  'signature.png': signatureBytes,
  'chart.svg': chartBytes
});

// Render with assets
const result = withAllAssets.render(markdown);
```

### Asset Naming and Security

Asset names are validated to prevent path traversal:

```typescript
// Safe: just filename
workflow.withAsset('logo.png', bytes); // ✓

// Unsafe: path separators rejected
workflow.withAsset('../../../etc/passwd', bytes); // ✗ Throws error
workflow.withAsset('assets/logo.png', bytes); // ✗ Throws error
```

### Asset in Templates

Templates reference dynamic assets in frontmatter:

```markdown
<!-- In markdown document -->
---
logo: "company-logo.png"
---

# Invoice
```

```typst
// In glue.typ template
#let logo = Asset("company-logo.png")
#image(logo)
```

### Clearing Assets

Asset builder is immutable; clearing returns new workflow:

```typescript
const workflow1 = workflow.withAsset('a.png', bytesA);
const workflow2 = workflow1.withAsset('b.png', bytesB);
const workflow3 = workflow2.clearAssets();

// workflow1 has a.png
// workflow2 has a.png and b.png
// workflow3 has no assets
```

---

## Advanced Capabilities

### Caching and Reuse

Caching is opt-in for apps that need it:

```typescript
const engine = QuillmarkEngine.create({
  enableCache: true,
  maxCacheSize: 200 * 1024 * 1024 // 200MB
});

// Subsequent renders may benefit from cached compilation
const result = workflow.render(markdown);
```

### Batch Processing

Process multiple documents with the same Quill:

```typescript
const workflow = engine.loadWorkflow('report');

// Process each document
const results = documents.map(doc => workflow.render(doc.content));

// Save all PDFs
results.forEach((result, i) => {
  saveFile(`report-${i}.pdf`, result.artifacts[0].bytes);
});
```

### Format Detection

Auto-detect best format based on backend capabilities:

```typescript
const workflow = engine.loadWorkflow('article');

// No format specified: backend chooses best default
const result = workflow.render(markdown);
console.log(`Rendered as ${result.artifacts[0].format}`);

// List what's available
console.log(`Supported: ${workflow.supportedFormats.join(', ')}`);
```

---

## Performance & Optimization

### WASM Binary Size

Target < 5MB gzipped for browser use through aggressive optimization (`wasm-opt -Oz`).

### Initialization Time

Engine creation is fast (< 100ms) with minimal setup:

```typescript
// Fast: no I/O, minimal setup
const engine = QuillmarkEngine.create();

// First render may compile templates
const workflow = engine.loadWorkflow('letter');
const result = workflow.render(markdown);
```

### Memory Management

WASM manages memory internally; JavaScript gets copies via JSON:

```typescript
// Good: reuse workflow for multiple renders
const workflow = engine.loadWorkflow('letter');
for (const data of dataset) {
  const result = workflow.render(generateMarkdown(data));
  processResult(result);
}

// Bad: creating new workflow each time
for (const data of dataset) {
  const workflow = engine.loadWorkflow('letter'); // ✗ Wasteful
  const result = workflow.render(generateMarkdown(data));
}
```

### Parallelization with Web Workers

For CPU-intensive rendering in browsers:

```typescript
// Main thread
const worker = new Worker('quillmark-worker.js');
worker.postMessage({ markdown, quillName: 'report', quillFiles });

worker.onmessage = (e) => {
  displayPDF(e.data.artifacts[0].bytes);
};

// Worker thread (quillmark-worker.js)
const engine = QuillmarkEngine.create();

self.onmessage = (e) => {
  const quill = Quill.fromFiles(e.data.quillFiles, { name: e.data.quillName, backend: 'typst' });
  engine.registerQuill(quill);
  const workflow = engine.loadWorkflow(e.data.quillName);
  const result = workflow.render(e.data.markdown);
  self.postMessage(result);
};
```

---

## Error Handling

### Error Categories

**Opinion**: Categorize errors for better handling and UX.

```typescript
class QuillmarkError extends Error {
  readonly kind: 'render' | 'validation' | 'network' | 'system';
  readonly diagnostics: Diagnostic[];
}

// Usage
try {
  const result = await workflow.render(markdown);
} catch (e) {
  if (e instanceof QuillmarkError) {
    switch (e.kind) {
      case 'render':
        showRenderErrors(e.diagnostics);
        break;
      case 'validation':
        showQuillValidationErrors(e.diagnostics);
        break;
      case 'network':
        showRetryDialog();
        break;
      case 'system':
        reportToSentry(e);
        break;
    }
  }
}
```

### Diagnostic Display

**Opinion**: Rich diagnostics should be easy to display in UI.

```typescript
function renderDiagnostic(diag: Diagnostic): string {
  const icon = {
    error: '❌',
    warning: '⚠️',
    note: 'ℹ️'
  }[diag.severity];
  
  let output = `${icon} ${diag.message}`;
  
  if (diag.location) {
    output += `\n  at ${diag.location.file}:${diag.location.line}:${diag.location.column}`;
  }
  
  if (diag.hint) {
    output += `\n  💡 ${diag.hint}`;
  }
  
  return output;
}
```

### Graceful Degradation

**Opinion**: Apps should handle unsupported features gracefully.

```typescript
const workflow = await engine.loadWorkflow('article');

try {
  const result = await workflow.render(markdown, {
    format: OutputFormat.PDF
  });
} catch (e) {
  if (e.message.includes('not supported')) {
    // Fallback to SVG
    const result = await workflow.render(markdown, {
      format: OutputFormat.SVG
    });
  }
}
```

### Validation Before Render

**Opinion**: Validate inputs before expensive rendering.

```typescript
// Quick markdown validation
function validateMarkdown(markdown: string): Diagnostic[] {
  const diagnostics: Diagnostic[] = [];
  
  // Check for required frontmatter
  if (!markdown.startsWith('---')) {
    diagnostics.push({
      severity: Severity.ERROR,
      message: 'Missing YAML frontmatter',
      hint: 'Add --- at the start of the document'
    });
  }
  
  return diagnostics;
}

const issues = validateMarkdown(userInput);
if (issues.some(d => d.severity === Severity.ERROR)) {
  showErrors(issues);
} else {
  const result = await workflow.render(userInput);
}
```

---

## Future Considerations

### Web Components

**Opinion**: Consider a `<quillmark-renderer>` web component for easy integration.

```html
<quillmark-renderer
  quill="letter"
  format="pdf"
  src="/api/document.md"
  on-render="handleRender(event)"
></quillmark-renderer>
```

### Plugin System

**Opinion**: Future backends could be loaded as plugins.

```typescript
// Load custom backend
await engine.loadBackend('https://cdn.example.com/backends/latex.wasm');

// Use it
const quill = await Quill.fromUrl('https://cdn.example.com/quills/latex-article.zip');
const workflow = await engine.loadWorkflow(quill);
```

### Incremental Rendering

**Opinion**: For large documents, consider rendering pages independently.

```typescript
// Render specific pages
const result = await workflow.render(markdown, {
  format: OutputFormat.PDF,
  pages: [1, 2, 3] // Only render first 3 pages
});
```

### Collaborative Editing

**Opinion**: Support for OT/CRDT-based collaborative editing.

```typescript
// Apply incremental changes
const delta = computeDelta(oldMarkdown, newMarkdown);
const result = await workflow.renderDelta(delta);
```

### Offline Support

**Opinion**: Full offline mode with service worker integration.

```typescript
// Service worker caches Quills and WASM
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open('quillmark-v1').then((cache) => {
      return cache.addAll([
        '/quillmark.wasm',
        '/quills/letter.zip',
        '/quills/report.zip'
      ]);
    })
  );
});
```

---

## Summary

This WASM API design prioritizes:

1. **Explicit Quill Management**: Download, validate, register, and enumerate Quills
2. **Flexible Rendering**: Multiple formats, progress tracking, dynamic assets
3. **Production-Ready Error Handling**: Rich diagnostics with actionable messages
4. **Performance**: Caching, lazy loading, Web Worker support
5. **Web-Native**: Promises, typed arrays, Fetch API, AbortSignal

The API surface is intentionally minimal but complete, covering the essential capabilities for building professional markdown typesetting applications on the web.
