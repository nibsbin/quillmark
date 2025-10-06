# Quillmark TypeScript/WASM Wrapper Library Design

> **Status**: Not Planned - Reference Design Only
>
> This document was an initial design for a high-level TypeScript wrapper around `@quillmark-test/wasm`. However, we've decided to keep the API minimal and publish only `@quillmark-test/wasm` directly. Users can add their own TypeScript wrappers as needed. This document remains as a reference for what a higher-level wrapper could look like.

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Goals and Requirements](#goals-and-requirements)
3. [Architecture Design](#architecture-design)
4. [WASM Compilation Strategy](#wasm-compilation-strategy)
5. [TypeScript API Design](#typescript-api-design)
6. [Package Structure](#package-structure)
7. [Build Configuration](#build-configuration)
8. [CI/CD Pipeline](#cicd-pipeline)
9. [Testing Strategy](#testing-strategy)
10. [Documentation and Examples](#documentation-and-examples)
11. [Deployment and Publishing](#deployment-and-publishing)
12. [Performance Considerations](#performance-considerations)
13. [Security Considerations](#security-considerations)
14. [Future Enhancements](#future-enhancements)

---

## Project Overview

### Purpose

The `@quillmark/web` library will provide a WebAssembly-based TypeScript wrapper around the `quillmark` Rust crate, enabling JavaScript/TypeScript applications to:

- Render Markdown documents with YAML frontmatter to PDF, SVG, and other formats
- Use Quill templates in web applications (Node.js, browsers, Deno, Bun)
- Leverage the full power of the Typst backend for professional document generation
- Integrate document rendering into web-based workflows, APIs, and applications

### Target Environments

- **Node.js** (v20+): Server-side rendering, CLI tools, build processes
- **Browsers**: Client-side document generation (with appropriate feature detection)
- **Deno** (v1.30+): Alternative runtime support
- **Bun** (v1.0+): Modern JavaScript runtime
- **Edge Functions**: Cloudflare Workers, Vercel Edge, etc. (with size considerations)

### Package Name

`@quillmark/web` - published to npm registry

---

## Goals and Requirements

### Functional Requirements

1. **API Parity**: Expose equivalent functionality to the Rust `Quillmark` high-level API
2. **Type Safety**: Full TypeScript type definitions with comprehensive intellisense
3. **Ergonomic API**: JavaScript-idiomatic patterns (Promises, async/await, builders)
4. **Format Support**: PDF and SVG output formats (matching Typst backend capabilities)
5. **Asset Management**: Support both static and dynamic asset injection
6. **Error Handling**: Rich error messages with diagnostic information
7. **Performance**: Efficient WASM execution with minimal overhead

### Non-Functional Requirements

1. **Bundle Size**: Optimized WASM binary (target < 5MB gzipped for browser use)
2. **Browser Compatibility**: Modern browsers (ES2020+, WebAssembly support)
3. **Developer Experience**: Easy installation, clear documentation, good error messages
4. **Maintainability**: Automated build/test/release pipeline
5. **Stability**: Semantic versioning with stable API contracts

---

## Architecture Design

### High-Level Architecture


```
┌─────────────────────────────────────────────────────────┐
│                   TypeScript Layer                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Public TypeScript API (@quillmark/web)         │    │
│  │  - Quillmark class                              │    │
│  │  - Workflow class                               │    │
│  │  - Type definitions                             │    │
│  │  - Error wrappers                               │    │
│  └──────────────────┬──────────────────────────────┘    │
│                     │                                    │
│  ┌──────────────────▼──────────────────────────────┐    │
│  │  wasm-bindgen Generated Bindings                │    │
│  │  - Type conversion (JS ↔ Rust)                  │    │
│  │  - Memory management                            │    │
│  │  - Error propagation                            │    │
│  └──────────────────┬──────────────────────────────┘    │
└────────────────────┼─────────────────────────────────────┘
                     │
┌────────────────────▼─────────────────────────────────────┐
│                   WASM Module                            │
│  ┌─────────────────────────────────────────────────┐    │
│  │  quillmark-web (Rust crate)                     │    │
│  │  - WASM-compatible wrapper functions            │    │
│  │  - Serialization/deserialization                │    │
│  │  - Browser-safe file system abstraction         │    │
│  └──────────────────┬──────────────────────────────┘    │
│                     │                                    │
│  ┌──────────────────▼──────────────────────────────┐    │
│  │  quillmark (Rust crate)                         │    │
│  │  - Quillmark engine                             │    │
│  │  - Workflow orchestration                       │    │
│  │  - Backend integration                          │    │
│  └──────────────────┬──────────────────────────────┘    │
│                     │                                    │
│  ┌──────────────────▼──────────────────────────────┐    │
│  │  quillmark-core + quillmark-typst               │    │
│  │  - Core types and traits                        │    │
│  │  - Typst backend implementation                 │    │
│  │  - Parsing, templating, compilation             │    │
│  └─────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

### Component Responsibilities

#### TypeScript Layer (`pkg/` directory)
- **Public API**: Ergonomic TypeScript classes and interfaces
- **Type Definitions**: Comprehensive `.d.ts` files
- **Documentation**: JSDoc comments for IDE support
- **Convenience Methods**: JavaScript-idiomatic helpers (e.g., async/await wrappers)

#### WASM Bindings Layer (`wasm-bindgen`)
- **Type Marshalling**: Convert between JavaScript and Rust types
- **Memory Management**: Handle allocations/deallocations safely
- **Error Handling**: Translate Rust errors to JavaScript exceptions

#### Rust WASM Crate (`quillmark-web/`)
- **WASM Entry Points**: Exported functions accessible from JavaScript
- **Virtual File System**: Browser-compatible file abstraction
- **Serialization**: JSON-based data exchange for complex types
- **Feature Flags**: Conditional compilation for WASM target

---

## WASM Compilation Strategy

### Toolchain

**Primary Tool**: `wasm-bindgen` (https://rustwasm.github.io/wasm-bindgen/)
- Industry standard for Rust ↔ JavaScript interop
- Automatic TypeScript definition generation
- Excellent error handling and debugging support

**Build Tool**: `wasm-pack` (https://rustwasm.github.io/wasm-pack/)
- One-command build process
- Multiple target formats (bundler, nodejs, web, deno)
- Optimized production builds

**Optimization**: `wasm-opt` (from Binaryen)
- Post-processing optimization
- Size reduction (crucial for browser deployment)
- Integrated into wasm-pack

### Cargo Configuration

Create `quillmark-web/Cargo.toml`:

```toml
[package]
name = "quillmark-web"
version = "0.1.0"
edition = "2021"
description = "WebAssembly bindings for quillmark"
license = "Apache-2.0"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
quillmark = { path = "../quillmark", default-features = true }
quillmark-core = { path = "../quillmark-core" }
wasm-bindgen = "0.2"
serde = { workspace = true }
serde_json = { workspace = true }
serde-wasm-bindgen = "0.6"
console_error_panic_hook = "0.1"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
panic = "abort"      # Smaller binary
strip = true         # Remove debug symbols

[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz", "--enable-mutable-globals"]
```

### Build Targets

**Multiple build targets** for different environments:

1. **Bundler** (webpack, rollup, vite): `wasm-pack build --target bundler`
2. **Node.js**: `wasm-pack build --target nodejs`
3. **Web** (no bundler): `wasm-pack build --target web`
4. **Deno**: `wasm-pack build --target deno`

### WASM Feature Flags

Enable/disable features based on target:

```toml
[features]
default = ["typst"]
typst = ["quillmark/typst"]
# Future: other backends
```

---

## TypeScript API Design

### Core API

The TypeScript API mirrors the Rust high-level API while following JavaScript conventions:

```typescript
// Core types
export enum OutputFormat {
  Pdf = "pdf",
  Svg = "svg",
  Txt = "txt"
}

export interface Artifact {
  readonly bytes: Uint8Array;
  readonly outputFormat: OutputFormat;
}

export interface RenderResult {
  readonly artifacts: Artifact[];
  readonly warnings: Diagnostic[];
}

export interface Diagnostic {
  readonly severity: Severity;
  readonly code?: string;
  readonly message: string;
  readonly primary?: Location;
  readonly related: Location[];
  readonly hint?: string;
}

export enum Severity {
  Error = "error",
  Warning = "warning",
  Note = "note"
}

export interface Location {
  readonly file: string;
  readonly line: number;
  readonly col: number;
}

// Quill configuration
export interface QuillConfig {
  readonly name: string;
  readonly backend: string;
  readonly glue: string;
  readonly metadata: Record<string, any>;
}

/**
 * Represents a Quill template bundle
 */
export class Quill {
  /**
   * Load a Quill from a directory structure
   * @param basePath - Path to quill directory (Node.js only)
   */
  static fromPath(basePath: string): Promise<Quill>;
  
  /**
   * Create a Quill from a files object (browser-compatible)
   * @param files - Map of file paths to contents
   * @param config - Quill configuration
   */
  static fromFiles(
    files: Map<string, Uint8Array>,
    config: QuillConfig
  ): Quill;

  readonly name: string;
  readonly backend: string;
  
  /**
   * Validate the quill structure
   */
  validate(): void;
}

/**
 * High-level workflow for rendering documents
 */
export class Workflow {
  /**
   * Create a new Workflow
   * @param quill - The quill template to use
   * @param backend - Optional backend override
   */
  constructor(quill: Quill, backend?: string);

  /**
   * Render markdown to artifacts
   * @param markdown - Markdown content with YAML frontmatter
   * @param format - Desired output format (optional)
   * @returns Rendering result with artifacts
   */
  async render(
    markdown: string,
    format?: OutputFormat
  ): Promise<RenderResult>;

  /**
   * Render pre-processed glue content
   * @param content - Glue content (e.g., Typst markup)
   * @param format - Desired output format
   */
  async renderSource(
    content: string,
    format?: OutputFormat
  ): Promise<RenderResult>;

  /**
   * Process markdown to glue content without compilation
   * @param markdown - Markdown with frontmatter
   * @returns Glue source code
   */
  async processGlue(markdown: string): Promise<string>;

  /**
   * Add a dynamic asset (builder pattern)
   * @param filename - Asset filename
   * @param contents - Asset bytes
   * @returns This workflow for chaining
   */
  withAsset(filename: string, contents: Uint8Array): Workflow;

  /**
   * Add multiple dynamic assets
   * @param assets - Map of filename to contents
   */
  withAssets(assets: Map<string, Uint8Array>): Workflow;

  /**
   * Clear all dynamic assets
   */
  clearAssets(): Workflow;

  readonly backendId: string;
  readonly supportedFormats: OutputFormat[];
  readonly quillName: string;
}

/**
 * High-level engine for managing quills and backends
 */
export class Quillmark {
  /**
   * Create a new Quillmark engine
   */
  constructor();

  /**
   * Register a quill by name
   * @param quill - Quill to register
   */
  registerQuill(quill: Quill): void;

  /**
   * Load a workflow for a registered quill
   * @param quillName - Name of registered quill
   * @returns Workflow instance
   */
  load(quillName: string): Workflow;

  /**
   * Load a workflow from a quill object
   * @param quill - Quill instance
   */
  loadQuill(quill: Quill): Workflow;

  /**
   * Get list of registered backend IDs
   */
  get registeredBackends(): string[];

  /**
   * Get list of registered quill names
   */
  get registeredQuills(): string[];
}

/**
 * Error classes
 */
export class QuillmarkError extends Error {
  readonly diagnostics?: Diagnostic[];
  constructor(message: string, diagnostics?: Diagnostic[]);
}

export class RenderError extends QuillmarkError {}
export class InvalidFrontmatterError extends QuillmarkError {}
export class CompilationError extends QuillmarkError {}
export class UnsupportedFormatError extends QuillmarkError {}
```


### Usage Examples

#### Example 1: Node.js Basic Usage

```typescript
import { Quillmark, Quill, OutputFormat } from '@quillmark/web';
import { writeFileSync } from 'fs';

async function main() {
  // Create engine
  const engine = new Quillmark();

  // Load and register quill
  const quill = await Quill.fromPath('./my-quill');
  engine.registerQuill(quill);

  // Create workflow
  const workflow = engine.load('my-quill');

  // Render markdown
  const markdown = `---
title: "My Document"
date: "2024-01-15"
---

# Hello World

This is my document content.
`;

  const result = await workflow.render(markdown, OutputFormat.Pdf);
  
  // Write PDF to file
  writeFileSync('output.pdf', result.artifacts[0].bytes);
  
  console.log('PDF generated successfully!');
}

main().catch(console.error);
```

#### Example 2: Browser Usage with Dynamic Assets

```typescript
import { Quill, Workflow, OutputFormat } from '@quillmark/web';

async function renderDocument(markdown: string, chartData: Uint8Array) {
  // Create quill from pre-loaded files
  const files = new Map([
    ['glue.typ', textEncoder.encode(glueTemplate)],
    ['Quill.toml', textEncoder.encode(config)],
    // ... other files
  ]);

  const quill = Quill.fromFiles(files, {
    name: 'my-quill',
    backend: 'typst',
    glue: 'glue.typ',
    metadata: {}
  });

  // Create workflow with dynamic asset
  const workflow = new Workflow(quill)
    .withAsset('chart.png', chartData);

  // Render to PDF
  const result = await workflow.render(markdown, OutputFormat.Pdf);
  
  // Download in browser
  const blob = new Blob([result.artifacts[0].bytes], { 
    type: 'application/pdf' 
  });
  const url = URL.createObjectURL(blob);
  
  const a = document.createElement('a');
  a.href = url;
  a.download = 'document.pdf';
  a.click();
}
```

#### Example 3: Error Handling

```typescript
import { Workflow, QuillmarkError, CompilationError } from '@quillmark/web';

async function renderWithErrorHandling(workflow: Workflow, markdown: string) {
  try {
    const result = await workflow.render(markdown);
    
    // Check for warnings
    if (result.warnings.length > 0) {
      console.warn('Rendering warnings:');
      result.warnings.forEach(w => {
        console.warn(`  ${w.message} at ${w.primary?.file}:${w.primary?.line}`);
      });
    }
    
    return result.artifacts[0].bytes;
  } catch (error) {
    if (error instanceof CompilationError) {
      console.error('Compilation failed:');
      error.diagnostics?.forEach(d => {
        console.error(`  [${d.severity}] ${d.message}`);
        if (d.primary) {
          console.error(`    at ${d.primary.file}:${d.primary.line}:${d.primary.col}`);
        }
        if (d.hint) {
          console.error(`    Hint: ${d.hint}`);
        }
      });
    } else if (error instanceof QuillmarkError) {
      console.error(`Quillmark error: ${error.message}`);
    } else {
      console.error('Unexpected error:', error);
    }
    throw error;
  }
}
```

---

## Package Structure

### Directory Layout

```
quillmark/
├── quillmark-web/              # New Rust WASM crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs             # WASM entry points
│   │   ├── quill.rs           # Quill wrapper
│   │   ├── workflow.rs        # Workflow wrapper
│   │   ├── engine.rs          # Quillmark engine wrapper
│   │   ├── types.rs           # Type conversions
│   │   ├── error.rs           # Error handling
│   │   └── utils.rs           # Utilities (panic hook, etc.)
│   └── tests/
│       └── web.rs             # Integration tests
│
├── pkg/                        # Generated by wasm-pack (gitignored)
│   ├── quillmark_web_bg.wasm # WASM binary
│   ├── quillmark_web.js      # JS bindings
│   ├── quillmark_web.d.ts    # TypeScript definitions
│   └── package.json          # Generated package.json
│
├── typescript/                 # TypeScript wrapper layer
│   ├── src/
│   │   ├── index.ts           # Main export
│   │   ├── Quillmark.ts       # Engine class
│   │   ├── Workflow.ts        # Workflow class
│   │   ├── Quill.ts           # Quill class
│   │   ├── types.ts           # Type definitions
│   │   └── errors.ts          # Error classes
│   ├── tests/
│   │   ├── basic.test.ts
│   │   ├── assets.test.ts
│   │   └── errors.test.ts
│   ├── package.json
│   ├── tsconfig.json
│   └── README.md
│
├── examples/                   # Example applications
│   ├── node-basic/
│   ├── browser-demo/
│   └── cloudflare-worker/
│
├── scripts/                    # Build and release scripts
│   ├── build-wasm.sh          # Build WASM module
│   ├── build-ts.sh            # Build TypeScript
│   ├── build-all.sh           # Full build
│   └── test-all.sh            # Run all tests
│
└── .github/
    └── workflows/
        ├── build-wasm.yml     # WASM build CI
        └── publish-npm.yml    # NPM publish workflow
```

### Package.json Structure

```json
{
  "name": "@quillmark/web",
  "version": "0.1.0",
  "description": "WebAssembly-based TypeScript wrapper for quillmark document rendering",
  "keywords": [
    "markdown",
    "pdf",
    "typst",
    "document-generation",
    "wasm",
    "webassembly"
  ],
  "author": "Quillmark Contributors",
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/nibsbin/quillmark.git",
    "directory": "typescript"
  },
  "main": "dist/index.js",
  "module": "dist/index.mjs",
  "types": "dist/index.d.ts",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.mjs",
      "require": "./dist/index.js"
    },
    "./wasm": {
      "types": "./pkg/quillmark_web.d.ts",
      "import": "./pkg/quillmark_web.js"
    }
  },
  "files": [
    "dist/",
    "pkg/",
    "README.md",
    "LICENSE"
  ],
  "scripts": {
    "build:wasm": "../scripts/build-wasm.sh",
    "build:ts": "tsup src/index.ts --format cjs,esm --dts",
    "build": "npm run build:wasm && npm run build:ts",
    "test": "vitest",
    "test:ci": "vitest run",
    "prepublishOnly": "npm run build && npm test"
  },
  "dependencies": {},
  "peerDependencies": {},
  "devDependencies": {
    "@types/node": "^20.0.0",
    "tsup": "^8.0.0",
    "typescript": "^5.3.0",
    "vitest": "^1.0.0"
  },
  "engines": {
    "node": ">=20.0.0"
  }
}
```

---

## Build Configuration

### WASM Build Script (`scripts/build-wasm.sh`)

```bash
#!/bin/bash
set -e

echo "Building WASM module..."

# Navigate to workspace root
cd "$(dirname "$0")/.."

# Build for multiple targets
targets=("bundler" "nodejs" "web")

for target in "${targets[@]}"; do
  echo "Building for target: $target"
  
  wasm-pack build quillmark-web \
    --target "$target" \
    --out-dir "../pkg/$target" \
    --release \
    --scope quillmark
done

echo "WASM build complete!"
```

### TypeScript Build Script (`scripts/build-ts.sh`)

```bash
#!/bin/bash
set -e

echo "Building TypeScript wrapper..."

cd typescript

# Install dependencies
npm install

# Build TypeScript
npm run build:ts

echo "TypeScript build complete!"
```

### Unified Build Script (`scripts/build-all.sh`)

```bash
#!/bin/bash
set -e

echo "=== Quillmark Web Library Build ==="
echo

./scripts/build-wasm.sh
echo

./scripts/build-ts.sh
echo

echo "=== Build Complete ==="
echo "WASM modules: pkg/bundler/, pkg/nodejs/, pkg/web/"
echo "TypeScript: typescript/dist/"
```

### TypeScript Configuration (`typescript/tsconfig.json`)

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "lib": ["ES2020", "DOM"],
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "allowSyntheticDefaultImports": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests"]
}
```

---

## CI/CD Pipeline

### GitHub Actions Workflow for WASM Build

`.github/workflows/build-wasm.yml`:

```yaml
name: Build and Test WASM

on:
  push:
    branches: [ main, develop ]
    paths:
      - 'quillmark-web/**'
      - 'quillmark/**'
      - 'quillmark-core/**'
      - 'quillmark-typst/**'
      - 'typescript/**'
      - '.github/workflows/build-wasm.yml'
  pull_request:
    branches: [ main, develop ]
    paths:
      - 'quillmark-web/**'
      - 'quillmark/**'
      - 'quillmark-core/**'
      - 'quillmark-typst/**'
      - 'typescript/**'

jobs:
  build-wasm:
    name: Build WASM Module
    runs-on: ubuntu-latest
    
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
          override: true
          components: rustfmt, clippy

      - name: Cache Rust dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-wasm-${{ hashFiles('**/Cargo.lock') }}

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Build WASM (bundler target)
        run: |
          wasm-pack build quillmark-web \
            --target bundler \
            --out-dir ../pkg/bundler \
            --release \
            --scope quillmark

      - name: Build WASM (nodejs target)
        run: |
          wasm-pack build quillmark-web \
            --target nodejs \
            --out-dir ../pkg/nodejs \
            --release \
            --scope quillmark

      - name: Build WASM (web target)
        run: |
          wasm-pack build quillmark-web \
            --target web \
            --out-dir ../pkg/web \
            --release \
            --scope quillmark

      - name: Check WASM size
        run: |
          echo "WASM Binary Sizes:"
          ls -lh pkg/bundler/*.wasm
          ls -lh pkg/nodejs/*.wasm
          ls -lh pkg/web/*.wasm

      - name: Run WASM tests
        run: wasm-pack test --node quillmark-web

      - name: Upload WASM artifacts
        uses: actions/upload-artifact@v3
        with:
          name: wasm-modules
          path: |
            pkg/

  build-typescript:
    name: Build TypeScript Wrapper
    runs-on: ubuntu-latest
    needs: build-wasm
    
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: typescript/package-lock.json

      - name: Download WASM artifacts
        uses: actions/download-artifact@v3
        with:
          name: wasm-modules

      - name: Install dependencies
        working-directory: typescript
        run: npm ci

      - name: Build TypeScript
        working-directory: typescript
        run: npm run build:ts

      - name: Run TypeScript tests
        working-directory: typescript
        run: npm run test:ci

      - name: Upload TypeScript build
        uses: actions/upload-artifact@v3
        with:
          name: typescript-dist
          path: typescript/dist/

  test-integration:
    name: Integration Tests
    runs-on: ubuntu-latest
    needs: [build-wasm, build-typescript]
    
    strategy:
      matrix:
        node-version: [20, 21]
    
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Setup Node.js ${{ matrix.node-version }}
        uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node-version }}

      - name: Download artifacts
        uses: actions/download-artifact@v3

      - name: Run integration tests
        run: npm run test:integration

  size-check:
    name: Bundle Size Check
    runs-on: ubuntu-latest
    needs: build-wasm
    
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Download WASM artifacts
        uses: actions/download-artifact@v3
        with:
          name: wasm-modules

      - name: Check gzipped size
        run: |
          gzip -c pkg/*.wasm > wasm.gz
          SIZE=$(stat -c%s wasm.gz)
          echo "Gzipped WASM size: $(($SIZE / 1024 / 1024))MB"
          
          # Fail if > 5MB gzipped
          if [ $SIZE -gt 5242880 ]; then
            echo "Error: WASM bundle exceeds 5MB gzipped"
            exit 1
          fi
```


### NPM Publishing Workflow

`.github/workflows/publish-npm.yml`:

```yaml
name: Publish to NPM

on:
  release:
    types: [published]
  workflow_dispatch:
    inputs:
      tag:
        description: 'NPM tag (latest, beta, etc.)'
        required: true
        default: 'latest'

jobs:
  publish:
    name: Publish @quillmark/web
    runs-on: ubuntu-latest
    
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'

      - name: Build all targets
        run: ./scripts/build-all.sh

      - name: Run tests
        working-directory: typescript
        run: npm test

      - name: Publish to NPM
        working-directory: typescript
        run: |
          TAG="${{ github.event.inputs.tag || 'latest' }}"
          npm publish --access public --tag $TAG
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

### Version Management Strategy

1. **Semantic Versioning**: Follow semver strictly (MAJOR.MINOR.PATCH)
2. **Version Sync**: Keep Rust crate and NPM package versions aligned
3. **Release Process**:
   - Update version in `quillmark-web/Cargo.toml`
   - Update version in `typescript/package.json`
   - Create Git tag: `git tag v0.1.0`
   - Push tag: `git push origin v0.1.0`
   - Create GitHub release (triggers publish workflow)

---

## Testing Strategy

### Test Layers

1. **Rust WASM Tests** (`quillmark-web/tests/`)
   - Unit tests for WASM bindings
   - Type conversion tests
   - Error handling tests
   - Run with: `wasm-pack test --node`

2. **TypeScript Unit Tests** (`typescript/tests/`)
   - API surface tests
   - Type checking
   - Error wrapper tests
   - Run with: `npm test`

3. **Integration Tests**
   - End-to-end rendering tests
   - Asset handling tests
   - Multi-format output tests
   - Node.js and browser environment tests

4. **Example Tests**
   - Verify all examples compile and run
   - Regression testing

### Test Configuration

**Vitest Configuration** (`typescript/vitest.config.ts`):

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'tests/',
        'dist/',
        '**/*.d.ts',
      ],
    },
  },
});
```

### Test Examples

**Basic Rendering Test** (`typescript/tests/basic.test.ts`):

```typescript
import { describe, it, expect, beforeAll } from 'vitest';
import { Quillmark, Quill, OutputFormat } from '../src';

describe('Basic Rendering', () => {
  let engine: Quillmark;

  beforeAll(async () => {
    engine = new Quillmark();
    const quill = await Quill.fromPath('./test-fixtures/simple-quill');
    engine.registerQuill(quill);
  });

  it('should render markdown to PDF', async () => {
    const workflow = engine.load('simple-quill');
    const markdown = '---\ntitle: Test\n---\n\n# Hello';
    
    const result = await workflow.render(markdown, OutputFormat.Pdf);
    
    expect(result.artifacts).toHaveLength(1);
    expect(result.artifacts[0].outputFormat).toBe(OutputFormat.Pdf);
    expect(result.artifacts[0].bytes).toBeInstanceOf(Uint8Array);
    expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
  });

  it('should handle errors gracefully', async () => {
    const workflow = engine.load('simple-quill');
    const invalidMarkdown = '---\ninvalid: yaml: content\n---';
    
    await expect(
      workflow.render(invalidMarkdown)
    ).rejects.toThrow();
  });
});
```

---

## Documentation and Examples

### Documentation Structure

1. **README.md**: Quick start, installation, basic usage
2. **API.md**: Comprehensive API reference
3. **EXAMPLES.md**: Cookbook-style examples
4. **MIGRATION.md**: Migration guide (for future versions)
5. **TROUBLESHOOTING.md**: Common issues and solutions

### Example Applications

#### 1. Node.js CLI Tool

```typescript
// examples/node-basic/index.ts
import { Quillmark, Quill, OutputFormat } from '@quillmark/web';
import { readFileSync, writeFileSync } from 'fs';
import { join } from 'path';

async function renderDocument(
  quillPath: string,
  markdownPath: string,
  outputPath: string
) {
  const engine = new Quillmark();
  const quill = await Quill.fromPath(quillPath);
  engine.registerQuill(quill);
  
  const workflow = engine.load(quill.name);
  const markdown = readFileSync(markdownPath, 'utf-8');
  
  const result = await workflow.render(markdown, OutputFormat.Pdf);
  writeFileSync(outputPath, result.artifacts[0].bytes);
  
  console.log(`✓ Rendered ${outputPath}`);
}

// Usage: node index.js <quill-dir> <markdown-file> <output-file>
const [quillPath, markdownPath, outputPath] = process.argv.slice(2);
renderDocument(quillPath, markdownPath, outputPath);
```

#### 2. Browser Demo

```html
<!-- examples/browser-demo/index.html -->
<!DOCTYPE html>
<html>
<head>
  <title>Quillmark Web Demo</title>
</head>
<body>
  <h1>Quillmark Document Generator</h1>
  
  <textarea id="markdown" rows="20" cols="80">---
title: "My Document"
date: "2024-01-15"
---

# Hello World

This is my document.
</textarea>
  
  <button id="render">Render PDF</button>
  
  <script type="module">
    import init, { Quillmark, Quill } from './pkg/quillmark_web.js';
    
    await init();
    
    document.getElementById('render').addEventListener('click', async () => {
      const markdown = document.getElementById('markdown').value;
      
      // Load pre-bundled quill files
      const quillFiles = await loadQuillFiles();
      const quill = Quill.fromFiles(quillFiles, {
        name: 'demo-quill',
        backend: 'typst',
        glue: 'glue.typ',
        metadata: {}
      });
      
      const workflow = new Workflow(quill);
      const result = await workflow.render(markdown);
      
      // Download PDF
      const blob = new Blob([result.artifacts[0].bytes], { 
        type: 'application/pdf' 
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'document.pdf';
      a.click();
    });
  </script>
</body>
</html>
```

#### 3. Cloudflare Worker

```typescript
// examples/cloudflare-worker/index.ts
import { Quillmark, Quill, OutputFormat } from '@quillmark/web';

// Pre-loaded quill data
const QUILL_DATA = {
  /* ... bundled quill files ... */
};

export default {
  async fetch(request: Request): Promise<Response> {
    if (request.method !== 'POST') {
      return new Response('Method not allowed', { status: 405 });
    }
    
    try {
      const { markdown } = await request.json();
      
      // Create quill from embedded data
      const quill = Quill.fromFiles(QUILL_DATA.files, QUILL_DATA.config);
      const workflow = new Workflow(quill);
      
      // Render to PDF
      const result = await workflow.render(markdown, OutputFormat.Pdf);
      
      return new Response(result.artifacts[0].bytes, {
        headers: {
          'Content-Type': 'application/pdf',
          'Content-Disposition': 'attachment; filename="document.pdf"'
        }
      });
    } catch (error) {
      return new Response(
        JSON.stringify({ error: error.message }),
        { status: 500, headers: { 'Content-Type': 'application/json' } }
      );
    }
  }
};
```

---

## Deployment and Publishing

### Pre-Publish Checklist

- [ ] All tests passing (Rust, TypeScript, integration)
- [ ] WASM bundle size < 5MB gzipped
- [ ] Version numbers updated in all files
- [ ] CHANGELOG.md updated
- [ ] README.md examples tested
- [ ] API documentation up to date
- [ ] License files included
- [ ] Git tag created

### Publishing Process

1. **Prepare Release**
   ```bash
   # Update versions
   vim quillmark-web/Cargo.toml    # Update version
   vim typescript/package.json     # Update version
   vim CHANGELOG.md                # Document changes
   
   # Build everything
   ./scripts/build-all.sh
   
   # Run all tests
   ./scripts/test-all.sh
   ```

2. **Create Git Tag**
   ```bash
   git add -A
   git commit -m "Release v0.1.0"
   git tag v0.1.0
   git push origin main
   git push origin v0.1.0
   ```

3. **Publish to NPM**
   - GitHub Actions workflow automatically publishes on tag push
   - Or manual: `cd typescript && npm publish --access public`

4. **Post-Release**
   - Create GitHub release with release notes
   - Update documentation site (if applicable)
   - Announce on social media/forums

### NPM Registry Configuration

```
Package Name: @quillmark/web
Scope: @quillmark
Access: Public
Registry: https://registry.npmjs.org/
```

---

## Performance Considerations

### WASM Binary Optimization

1. **Compilation Flags**
   - `opt-level = "z"`: Optimize for size
   - `lto = true`: Link-time optimization
   - `codegen-units = 1`: Better optimization
   - `strip = true`: Remove debug symbols

2. **wasm-opt Post-Processing**
   - `-Oz`: Aggressive size optimization
   - `--enable-mutable-globals`: Better code generation

3. **Feature Flags**
   - Disable unused Rust features
   - Conditional compilation for WASM target

### Runtime Performance

1. **Memory Management**
   - Reuse buffers where possible
   - Minimize allocations in hot paths
   - Use `wasm-bindgen` efficiently

2. **Asynchronous Operations**
   - All rendering operations are async
   - Non-blocking for UI applications

3. **Caching Strategies**
   - Cache loaded quills in `Quillmark` instance
   - Reuse `Workflow` instances when possible

### Size Budget

- **Target**: < 5MB gzipped WASM binary
- **Monitoring**: CI checks on every build
- **Tracking**: Document size changes in release notes

---

## Security Considerations

### Input Validation

1. **Markdown Content**
   - No arbitrary code execution (Typst is sandboxed)
   - Validate frontmatter YAML structure
   - Limit document size to prevent DoS

2. **File Paths**
   - Sanitize all file paths in browser context
   - Prevent directory traversal
   - Validate asset filenames

3. **Dynamic Assets**
   - Validate file sizes
   - Check file types
   - Prevent path injection

### WASM Security

1. **Memory Safety**
   - Rust's memory safety guarantees
   - No unsafe code in public API
   - Bounds checking enabled

2. **Sandboxing**
   - WASM runs in sandboxed environment
   - No direct file system access in browser
   - Limited system calls

### Dependency Security

1. **Regular Audits**
   - `npm audit` in CI pipeline
   - `cargo audit` for Rust dependencies
   - Automated security updates

2. **Minimal Dependencies**
   - Keep dependency tree small
   - Review all dependencies
   - Prefer well-maintained packages

---

## Future Enhancements

### Potential Features

1. **Additional Backends**
   - LaTeX backend support
   - HTML backend for web output
   - Plain text backend

2. **Streaming API**
   - Stream large documents
   - Progressive rendering
   - Chunked output

3. **Worker Pool**
   - Parallel rendering in Node.js
   - Web Worker support in browsers
   - Better multi-core utilization

4. **Advanced Caching**
   - Template compilation caching
   - Font caching
   - Package caching

5. **Plugin System**
   - Custom filters from JavaScript
   - Custom backends
   - Middleware hooks

6. **CLI Tool**
   - Standalone npm CLI package
   - Watch mode for development
   - Batch processing

### Performance Improvements

1. **WASM SIMD**
   - Leverage SIMD instructions
   - Faster PDF generation

2. **Multi-Threading**
   - Use Web Workers
   - Parallel compilation

3. **Lazy Loading**
   - Split WASM module
   - Load backends on demand

---

## Implementation Roadmap

### Phase 1: Foundation (v0.1.0)
- [ ] Create `quillmark-web` Rust crate
- [ ] Implement basic WASM bindings
- [ ] Build TypeScript wrapper
- [ ] Basic CI/CD pipeline
- [ ] Core documentation

**Deliverables**: Basic rendering works, published to NPM

### Phase 2: Robustness (v0.2.0)
- [ ] Comprehensive error handling
- [ ] Asset management
- [ ] Multiple environment support
- [ ] Example applications
- [ ] Integration tests

**Deliverables**: Production-ready API

### Phase 3: Optimization (v0.3.0)
- [ ] WASM size optimization
- [ ] Performance benchmarks
- [ ] Caching strategies
- [ ] Advanced TypeScript types

**Deliverables**: Optimized for production use

### Phase 4: Ecosystem (v0.4.0+)
- [ ] Additional backends
- [ ] Plugin system
- [ ] CLI tool
- [ ] Community examples

**Deliverables**: Rich ecosystem

---

## Success Metrics

### Technical Metrics
- WASM binary < 5MB gzipped
- Bundle time < 30 seconds
- Render performance within 2x of native Rust
- Test coverage > 80%

### Adoption Metrics
- NPM downloads
- GitHub stars
- Community contributions
- Issue resolution time

### Quality Metrics
- Zero critical security vulnerabilities
- Semantic versioning compliance
- Documentation completeness
- API stability

---

## Conclusion

This design provides a comprehensive blueprint for creating a production-ready TypeScript/WASM wrapper for the quillmark crate. The approach prioritizes:

1. **Developer Experience**: Ergonomic API, excellent documentation
2. **Performance**: Optimized WASM, efficient runtime
3. **Reliability**: Comprehensive testing, error handling
4. **Maintainability**: Clear structure, automated CI/CD
5. **Accessibility**: Multiple environments, easy deployment

The phased implementation roadmap allows for incremental development while maintaining quality and stability at each stage.

---

## References

- [wasm-bindgen Book](https://rustwasm.github.io/wasm-bindgen/)
- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [Rust WASM Working Group](https://rustwasm.github.io/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/handbook/)
- [NPM Publishing Guide](https://docs.npmjs.com/cli/v9/commands/npm-publish)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [WebAssembly Specification](https://webassembly.github.io/spec/)
- [Typst Documentation](https://typst.app/docs/)
