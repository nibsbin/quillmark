# Quillmark WASM API Design

> **Status**: Design Phase - Opinionated High-Level Interface
>
> This document defines the WebAssembly API surface for exposing Quillmark's rendering capabilities to web applications. It focuses on the core capabilities needed for markdown typesetting apps, including dynamic Quill management, asset handling, and optimized rendering workflows.

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
2. **Stateless by Default**: Each render is independent unless explicitly cached
3. **Streaming-Ready**: Support incremental rendering and progress callbacks
4. **Web-Native**: Leverage Web APIs (Fetch, Blob, URL, etc.)
5. **Zero Configuration**: Sensible defaults for 90% of use cases

---

## Core Philosophy

### Opinionated Decisions

**1. Dynamic Quill Loading is First-Class**

Web apps need to download Quills from CDNs, package registries, or user uploads. The API treats Quill loading as a primary concern, not an afterthought.

**2. Explicit Over Implicit**

No global state. No auto-registration. Every Quill, every asset, every render must be explicitly managed. This makes the system predictable and testable.

**3. Async Everything**

All operations return Promises. Even if some operations are synchronous in Rust, the WASM boundary is async to support future optimizations (worker threads, streaming, etc.).

**4. Memory Management is Transparent**

WASM owns memory during render; JavaScript gets immutable copies. No shared buffers, no manual cleanup. Use typed arrays for binary data.

**5. Fail Fast with Rich Context**

Errors include diagnostics with file locations, line numbers, and actionable hints. Never fail silently.

---

## Primary Use Cases

### 1. Document Editor with Live Preview

```typescript
// User edits markdown; app renders PDF on-demand
const engine = await QuillmarkEngine.create();
const quill = await engine.downloadQuill('https://cdn.quillmark.io/letter.zip');
const workflow = await engine.loadWorkflow(quill);

// Re-render on every keystroke (debounced)
const result = await workflow.render(editorContent, { format: 'pdf' });
displayPreview(result.artifacts[0].bytes);
```

### 2. Batch Document Generation (Server)

```typescript
// API endpoint: generate invoices from templates
const engine = await QuillmarkEngine.create();
await engine.registerQuill(await Quill.fromPath('./quills/invoice'));

for (const order of orders) {
  const markdown = generateMarkdown(order);
  const workflow = await engine.loadWorkflow('invoice');
  const result = await workflow.render(markdown, { format: 'pdf' });
  await saveToS3(result.artifacts[0]);
}
```

### 3. User-Uploaded Custom Templates

```typescript
// User uploads a Quill ZIP; app validates and uses it
const zipFile = await uploadForm.getFile();
const quill = await Quill.fromZip(await zipFile.arrayBuffer());
await quill.validate(); // Throws if invalid

const engine = await QuillmarkEngine.create();
const workflow = await engine.loadWorkflow(quill);
```

### 4. Multi-Format Export

```typescript
// Generate PDF and SVG from same markdown
const workflow = await engine.loadWorkflow('report');
const markdown = buildReport(data);

const [pdf, svg] = await Promise.all([
  workflow.render(markdown, { format: 'pdf' }),
  workflow.render(markdown, { format: 'svg' })
]);
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
  progressCallback?: (percent: number) => void;
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
   * Load Quill from a directory (Node.js only)
   */
  static fromPath(path: string): Promise<Quill>;
  
  /**
   * Load Quill from a ZIP archive
   */
  static fromZip(buffer: ArrayBuffer): Promise<Quill>;
  
  /**
   * Load Quill from a URL (downloads and parses)
   */
  static fromUrl(url: string, options?: FetchOptions): Promise<Quill>;
  
  /**
   * Create Quill from in-memory file map (browser-friendly)
   */
  static fromFiles(files: Map<string, Uint8Array>, metadata: QuillMetadata): Promise<Quill>;
  
  /**
   * Validate Quill structure (throws on error)
   */
  validate(): Promise<void>;
  
  /**
   * Get Quill metadata
   */
  getMetadata(): QuillMetadata;
  
  /**
   * List files in the Quill
   */
  listFiles(): string[];
  
  /**
   * Export Quill as ZIP
   */
  toZip(): Promise<Uint8Array>;
}

/**
 * Rendering workflow for a specific Quill
 */
class Workflow {
  /**
   * Render markdown to artifacts
   */
  render(markdown: string, options?: RenderOptions): Promise<RenderResult>;
  
  /**
   * Render pre-processed glue content (advanced)
   */
  renderContent(content: string, options?: RenderOptions): Promise<RenderResult>;
  
  /**
   * Process markdown to glue without compilation (for debugging)
   */
  processGlue(markdown: string): Promise<string>;
  
  /**
   * Add dynamic assets (builder pattern)
   */
  withAsset(filename: string, bytes: Uint8Array): Workflow;
  
  /**
   * Add multiple dynamic assets
   */
  withAssets(assets: Map<string, Uint8Array>): Workflow;
  
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
  static create(options?: EngineOptions): Promise<QuillmarkEngine>;
  
  /**
   * Register a Quill by name
   */
  registerQuill(quill: Quill): Promise<void>;
  
  /**
   * Unregister a Quill
   */
  unregisterQuill(name: string): void;
  
  /**
   * Download and register a Quill from URL
   */
  downloadQuill(url: string, options?: FetchOptions): Promise<Quill>;
  
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
  loadWorkflow(quillOrName: Quill | string): Promise<Workflow>;
  
  /**
   * List available backends
   */
  listBackends(): string[];
  
  /**
   * Get supported formats for a backend
   */
  getSupportedFormats(backend: string): OutputFormat[];
  
  /**
   * Dispose of the engine and free resources
   */
  dispose(): void;
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
  
  /**
   * Custom backend configurations
   */
  backends?: Record<string, unknown>;
}

/**
 * Fetch options for downloading Quills
 */
interface FetchOptions {
  headers?: Record<string, string>;
  credentials?: 'include' | 'omit' | 'same-origin';
  signal?: AbortSignal;
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

### Downloading and Caching

**Opinion**: Web apps should download Quills from URLs, not bundle them. This enables:
- Centralized template repositories
- Version updates without app redeployment
- User-contributed templates

```typescript
// Download from CDN
const quill = await engine.downloadQuill('https://cdn.quillmark.io/quills/letter-v2.zip');

// Download with auth
const quill = await engine.downloadQuill('https://api.internal.com/quills/invoice', {
  headers: { 'Authorization': 'Bearer ...' }
});

// Handle failures gracefully
try {
  const quill = await engine.downloadQuill(url);
} catch (e) {
  if (e instanceof QuillmarkError && e.kind === 'network') {
    showOfflineError();
  }
}
```

### Registration and Lifecycle

**Opinion**: Explicit registration prevents naming collisions and makes dependencies clear.

```typescript
// Register after download
const quill = await Quill.fromUrl(url);
await quill.validate(); // Always validate before use
await engine.registerQuill(quill);

// Use registered Quill by name
const workflow = await engine.loadWorkflow('letter-v2');

// Unregister when done (free memory)
engine.unregisterQuill('letter-v2');
```

### Validation

**Opinion**: Validation should be explicit and comprehensive. Fail early before first render.

```typescript
const quill = await Quill.fromZip(userUpload);

try {
  await quill.validate();
  // Checks:
  // - quill.toml exists and is valid
  // - template.typ exists
  // - backend is supported
  // - no malicious file paths
  // - size limits
} catch (e) {
  showUserFriendlyError(e.diagnostics);
}
```

### Enumeration and Discovery

**Opinion**: Apps should be able to list and inspect Quills for UI purposes (dropdowns, galleries, etc.).

```typescript
// List all registered Quills
const quillNames = engine.listQuills();

// Get details for UI
const quills = quillNames.map(name => {
  const quill = engine.getQuill(name)!;
  return {
    name: quill.getMetadata().name,
    description: quill.getMetadata().description,
    backend: quill.getMetadata().backend
  };
});

// Show in dropdown
populateQuillSelector(quills);
```

---

## Rendering Pipeline

### Basic Rendering

**Opinion**: Render is always async, even if fast. Enables progressive enhancement.

```typescript
const workflow = await engine.loadWorkflow('report');
const markdown = '# Report\n\nContent here...';

const result = await workflow.render(markdown, {
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

**Opinion**: Same workflow can produce multiple formats. Render independently to avoid state issues.

```typescript
const workflow = await engine.loadWorkflow('article');

// Parallel rendering
const [pdfResult, svgResult] = await Promise.all([
  workflow.render(markdown, { format: OutputFormat.PDF }),
  workflow.render(markdown, { format: OutputFormat.SVG })
]);

// Download both
downloadFile(pdfResult.artifacts[0].bytes, 'article.pdf');
downloadFile(svgResult.artifacts[0].bytes, 'article.svg');
```

### Progress Tracking

**Opinion**: Long renders should report progress for better UX.

```typescript
const workflow = await engine.loadWorkflow('book');

const result = await workflow.render(largeMarkdown, {
  format: OutputFormat.PDF,
  progressCallback: (percent) => {
    updateProgressBar(percent);
  }
});
```

### Debugging Output

**Opinion**: Expose intermediate glue for debugging template issues.

```typescript
const workflow = await engine.loadWorkflow('custom-template');

// Generate glue without compiling
const glue = await workflow.processGlue(markdown);
console.log('Generated Typst:', glue);

// Copy to editor for tweaking
navigator.clipboard.writeText(glue);
```

---

## Asset Management

### Dynamic Assets

**Opinion**: Dynamic assets are essential for user-generated content (logos, signatures, charts).

```typescript
const workflow = await engine.loadWorkflow('invoice');

// Add company logo
const logo = await fetch('/api/company/logo').then(r => r.arrayBuffer());
const withLogo = workflow.withAsset('logo.png', new Uint8Array(logo));

// Add multiple assets
const withAllAssets = workflow.withAssets(new Map([
  ['logo.png', logoBytes],
  ['signature.png', signatureBytes],
  ['chart.svg', chartBytes]
]));

// Render with assets
const result = await withAllAssets.render(markdown);
```

### Asset Naming and Security

**Opinion**: Asset names must be validated to prevent path traversal attacks.

```typescript
// Safe: just filename
workflow.withAsset('logo.png', bytes); // ✓

// Unsafe: path separators rejected
workflow.withAsset('../../../etc/passwd', bytes); // ✗ Throws error
workflow.withAsset('assets/logo.png', bytes); // ✗ Throws error
```

### Asset in Templates

**Opinion**: Templates reference dynamic assets via the `Asset` filter.

```markdown
<!-- In markdown document -->
---
logo: "company-logo.png"
---

# Invoice
```

```typst
// In template.typ
#let logo = Asset("company-logo.png")
#image(logo)
```

### Clearing Assets

**Opinion**: Asset builder is immutable; clearing returns new workflow.

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

**Opinion**: Caching is opt-in. Most apps don't need it; those that do can enable it.

```typescript
const engine = await QuillmarkEngine.create({
  enableCache: true,
  maxCacheSize: 200 * 1024 * 1024 // 200MB
});

// First render: compiles and caches
const result1 = await workflow.render(markdown1);

// Second render: uses cached compilation
const result2 = await workflow.render(markdown2);
```

### Batch Processing

**Opinion**: Provide helpers for common batch patterns.

```typescript
// Process multiple documents with same Quill
const workflow = await engine.loadWorkflow('report');

const results = await Promise.all(
  documents.map(doc => workflow.render(doc.content))
);

// Save all PDFs
results.forEach((result, i) => {
  saveFile(`report-${i}.pdf`, result.artifacts[0].bytes);
});
```

### Abort and Timeout

**Opinion**: Long-running renders should be abortable.

```typescript
const controller = new AbortController();

// Render with timeout
const timeoutId = setTimeout(() => controller.abort(), 30000);

try {
  const result = await workflow.render(markdown, {
    signal: controller.signal
  });
} catch (e) {
  if (e.name === 'AbortError') {
    console.log('Render timed out');
  }
} finally {
  clearTimeout(timeoutId);
}
```

### Streaming Artifacts

**Opinion**: Large artifacts (multi-page PDFs) should support streaming.

```typescript
// Future enhancement
const stream = await workflow.renderStream(markdown, {
  format: OutputFormat.PDF
});

// Stream to file or network
const response = new Response(stream);
await downloadStream(response.body, 'output.pdf');
```

### Metadata Extraction

**Opinion**: Apps often need to extract metadata without full rendering.

```typescript
// Quick metadata extraction
const metadata = await workflow.extractMetadata(markdown);
console.log(metadata.title, metadata.author, metadata.date);

// Useful for search indexing, previews, etc.
```

### Format Detection

**Opinion**: Auto-detect best format based on backend capabilities.

```typescript
const workflow = await engine.loadWorkflow('article');

// No format specified: backend chooses best default
const result = await workflow.render(markdown);
console.log(`Rendered as ${result.artifacts[0].format}`);

// List what's available
console.log(`Supported: ${workflow.supportedFormats.join(', ')}`);
```

---

## Performance & Optimization

### WASM Binary Size

**Opinion**: Target < 5MB gzipped for browser use. This is achievable with:
- Aggressive size optimization (`wasm-opt -Oz`)
- Lazy loading of backends
- Shared dependencies

### Initialization Time

**Opinion**: Engine creation should be fast (< 100ms). Defer heavy work to first render.

```typescript
// Fast: no I/O, minimal setup
const engine = await QuillmarkEngine.create();

// Heavy work happens here
const workflow = await engine.loadWorkflow(quill);
const result = await workflow.render(markdown);
```

### Memory Management

**Opinion**: WASM manages memory internally; JavaScript gets copies. This prevents memory leaks but requires discipline.

```typescript
// Good: reuse workflow for multiple renders
const workflow = await engine.loadWorkflow('letter');
for (const data of dataset) {
  const result = await workflow.render(generateMarkdown(data));
  processResult(result);
}

// Bad: creating new workflow each time
for (const data of dataset) {
  const workflow = await engine.loadWorkflow('letter'); // ✗ Wasteful
  const result = await workflow.render(generateMarkdown(data));
}
```

### Parallelization

**Opinion**: Use Web Workers for CPU-intensive rendering in browsers.

```typescript
// Main thread
const worker = new Worker('quillmark-worker.js');
worker.postMessage({ markdown, quillName: 'report' });

worker.onmessage = (e) => {
  const result = e.data;
  displayPDF(result.artifacts[0].bytes);
};

// Worker thread (quillmark-worker.js)
const engine = await QuillmarkEngine.create();
await engine.registerQuill(await Quill.fromUrl(quillUrl));

self.onmessage = async (e) => {
  const workflow = await engine.loadWorkflow(e.data.quillName);
  const result = await workflow.render(e.data.markdown);
  self.postMessage(result);
};
```

### Caching Strategy

**Opinion**: Three-level cache for optimal performance:
1. **Engine cache**: Compiled Quills (in-memory)
2. **Browser cache**: Downloaded Quills (IndexedDB)
3. **CDN cache**: Quill distribution (HTTP cache headers)

```typescript
// Level 1: Engine cache
const engine = await QuillmarkEngine.create({ enableCache: true });

// Level 2: Browser cache (app implements)
async function getCachedQuill(url: string): Promise<Quill> {
  const db = await openDB('quillmark-cache');
  const cached = await db.get('quills', url);
  if (cached) return Quill.fromZip(cached);
  
  const quill = await Quill.fromUrl(url);
  await db.put('quills', await quill.toZip(), url);
  return quill;
}

// Level 3: CDN cache (server configures)
// Cache-Control: public, max-age=31536000, immutable
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
