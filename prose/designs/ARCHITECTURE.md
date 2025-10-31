# Quillmark Architecture

This document outlines the architecture and design principles of Quillmark, a flexible Markdown rendering engine that outputs to fully typesetted documents or forms.

## Table of Contents

1. [System Overview](#system-overview)
2. [Core Design Principles](#core-design-principles)
3. [Crate Structure and Responsibilities](#crate-structure-and-responsibilities)
4. [Core Interfaces and Structures](#core-interfaces-and-structures)
5. [End-to-End Orchestration Workflow](#end-to-end-orchestration-workflow)
6. [Template System Design](#template-system-design)
7. [Parsing and Document Decomposition](#parsing-and-document-decomposition)
8. [Backend Architecture](#backend-architecture)
9. [Package Management and Asset Handling](#package-management-and-asset-handling)
10. [Error Handling Patterns](#error-handling-patterns)
11. [Extension Points](#extension-points)
12. [Key Design Decisions](#key-design-decisions)

---

## System Overview

Quillmark is a flexible, **template-first** Markdown rendering system that converts Markdown with YAML frontmatter into output artifacts (PDF, SVG, TXT, etc.). The architecture is built around a an orchestration API for high-level use, backed by trait-based extensibility for backends.

High-level data flow:

* **Parsing** → YAML frontmatter + body extraction
* **Templating** → MiniJinja-based composition with backend-registered filters
* **Backend Processing** → Compile to final artifacts
* **Assets/Packages** → Fonts, images, and packages resolved dynamically

---

## Core Design Principles

1. **Sealed Orchestration, Explicit Backend**
   A single entry point (`Workflow`) encapsulates orchestration. Backend choice is **explicit** at engine construction.
2. **Trait-Based Extensibility**
   New output formats implement the `Backend` trait (thread-safe, zero global state).
3. **Template-First**
   Quill templates fully control structure and styling; Markdown provides content via filters.
4. **YAML-First Frontmatter**
   YAML is the single supported frontmatter format presented to templates. Backends may inject/convert to their native preferences (e.g., TOML for Typst) via filters.
5. **Zero-Copy Where Practical**
   Minimize allocations; prefer references and `Cow<'static, str>` in hot paths.
6. **Error Transparency**
   Preserve context (sources, spans) and provide actionable diagnostics.
7. **Dynamic Resource Loading**
   Discover templates, assets, fonts, and packages at runtime; no hardcoded deps.
8. **Zero-Config Defaults**
   Standard directory conventions let basic projects work without configuration.

---

## Crate Structure and Responsibilities

### `quillmark-core` (foundations)

* Types: `Backend`, `Artifact`, `OutputFormat`
* Parsing: `ParsedDocument` with `from_markdown()` constructor
* Templating: `Glue` + stable `filter_api`
* Template model: `Quill` (+ `Quill.toml`)
* Errors & Diagnostics: `RenderError`, `TemplateError`, `Diagnostic`, `Severity`, `Location`
* Utilities: TOML⇄YAML conversion helpers

**Design Note:** No external backend deps; backends depend on core → no cycles.

### `quillmark` (orchestration layer)

* High-level API: `Quillmark` for managing backends and quills
* Sealed rendering API: `Workflow`
* Orchestration (parse → compose → compile)
* Validation and structured error propagation
* Backend auto-registration on engine creation
* Default Quill registration during backend setup

**See crate rustdoc for complete API documentation.**

### `backends/quillmark-typst` (Typst backend)

* Implements `Backend` for PDF/SVG
* Markdown→Typst conversion
* Template filters: String, Lines, Date, Dict, Content, Asset
* Compilation environment with font & asset resolution
* Structured diagnostics with source locations

### `quillmark-fixtures` (dev/test utilities)

* Centralized test resources under `resources/`
* Helper functions for test setup and output

### `backends/quillmark-acroform` (AcroForm backend)

* Implements `Backend` for PDF form filling
* Templates field values using MiniJinja
* Supports tooltip-based and value-based templating

### `bindings/quillmark-python` (Python bindings)

* PyO3-based bindings mirroring Rust API with Pythonic conventions
* Published to PyPI as `quillmark` package

### `bindings/quillmark-wasm` (WebAssembly bindings)

* wasm-bindgen based bindings with JSON data exchange
* Published to npm as `@quillmark-test/wasm` package
* Supports bundler, Node.js, and web targets

### `quillmark-fuzz` (fuzzing tests)

* Fuzz testing suite for parsing, templating, and rendering

---

## Core Interfaces and Structures

> **Note:** For complete API documentation, see the rustdoc (available at docs.rs or via `cargo doc --open`).

### Main Components

- **Quillmark** - High-level engine managing backends and quills
- **Workflow** - Rendering pipeline orchestration (parse → template → compile)
- **Backend** - Trait for implementing output formats (PDF, SVG, etc.)
- **Quill** - Template bundle (glue template + assets/packages)
- **ParsedDocument** - Frontmatter + body from markdown
- **Diagnostic** - Structured error information
- **RenderResult** - Output artifacts + warnings

---

## End-to-End Orchestration Workflow

The workflow follows a three-stage pipeline:

1. **Parse** - Extract YAML frontmatter + body from markdown
2. **Template** - Compose backend-specific glue via MiniJinja with registered filters
3. **Compile** - Backend processes glue to generate output artifacts

### Key Concepts

- **Backend Auto-Registration**: Backends registered based on enabled features
- **Default Quill System**: Backends provide fallback templates for documents without `QUILL:` tags
- **Dynamic Assets**: Runtime assets accessible via `Asset` filter
- **Error Handling**: Structured diagnostics with source chain preservation

---

## Template System Design

The template system uses **MiniJinja** with a **stable filter API** to keep backends decoupled from the template engine.

### Filter Architecture

### Filter Architecture

Backends register custom filters via the stable `filter_api` module:
- **String** - Escape/quote values for backend syntax
- **Lines** - Array to multi-line string conversion
- **Date** - Date parsing for backend datetime types  
- **Dict** - Objects to backend-native structures
- **Content** - Markdown body to backend markup conversion
- **Asset** - Dynamic asset filename transformation

The `filter_api` provides a stable ABI, preventing version conflicts and enabling independent backend development.

### Template Context

Templates receive parsed document fields via the MiniJinja context:
- **Top-level fields**: All frontmatter fields and `body` accessible directly
- **`__metadata__` field**: System-generated field containing all frontmatter fields except `body`

See [GLUE_METADATA.md](GLUE_METADATA.md) for metadata access patterns.

---

## Parsing and Document Decomposition

Quillmark supports advanced markdown parsing with the **Extended YAML Metadata Standard**.

### Basic Frontmatter

- YAML delimited by `---` at document start
- Converted to `HashMap<String, QuillValue>`
- Body stored under reserved `BODY_FIELD` constant
- Fail-fast error reporting for malformed YAML

### Extended YAML Metadata Standard

Supports **inline metadata sections** using `SCOPE` and `QUILL` keys:

- **SCOPE**: Creates named collections (aggregates blocks into arrays)
- **QUILL**: Specifies which template to use
- **Horizontal rule disambiguation**: Smart detection distinguishes metadata from markdown
- **Validation**: Scope names follow `[a-z_][a-z0-9_]*` pattern

**Example:**
```markdown
---
title: Product Catalog
---
Main description.

---
SCOPE: products
name: Widget
---
Widget description.
```

Parses to: `{ title: "...", products: [{ name: "...", body: "..." }] }`

**See `designs/PARSE.md` for complete specification.**

---

## Backend Architecture

### Typst Backend

The Typst backend implements PDF and SVG output:

**Key Features:**
- Markdown→Typst conversion with proper escaping
- Dynamic package loading from `packages/` directory
- Font and asset resolution from `assets/` directory
- Runtime asset injection with `DYNAMIC_ASSET__` prefix
- Structured error mapping with source locations

**Filter Support:**
- String, Lines, Date, Dict filters for data transformation
- Content filter for Markdown→Typst conversion
- Asset filter for dynamic asset path mapping

**Compilation Environment (`QuillWorld`):**
- Implements Typst `World` trait
- Virtual file system for packages and assets
- Line/column mapping for error diagnostics

### AcroForm Backend

The AcroForm backend implements PDF form filling:

**Key Features:**
- Reads PDF forms from `form.pdf` in quill bundle
- Templates field values using MiniJinja
- Supports tooltip-based (`description__{{template}}`) and value-based templating
- Returns filled PDF as single artifact
- TXT format support for debugging (returns field values as JSON)

**Compilation Process:**
1. Load PDF form from quill's `form.pdf` file
2. Extract field names and current values
3. Render templated values using MiniJinja with JSON context
4. Write rendered values back to PDF form
5. Return filled PDF as byte vector

---

## Package Management and Asset Handling

### Quill Structure

```
quill-template/
├─ Quill.toml              # Metadata and configuration
├─ glue.<ext>              # Template file (e.g., glue.typ)
├─ packages/               # Backend packages
└─ assets/                 # Fonts, images, data
```

### Package Loading

- External packages specified in `[backend].packages` are downloaded
- Embedded packages in `packages/` directory are discovered
- Virtual file system maintains directory structure
- External packages override embedded ones on name collision

### Asset Management

- **Static assets**: Fonts and images in `assets/`
- **Dynamic assets**: Runtime-injected via `Workflow.add_asset()`
  - Prefixed with `DYNAMIC_ASSET__` to avoid collisions
  - Accessible via `Asset` filter in templates

---

## Error Handling Patterns

See [ERROR.md](ERROR.md) for complete documentation.

### Core Error Types

- **Diagnostic** - Structured error with severity, code, message, location, hints, and source chain
- **RenderError** - Main error enum containing diagnostics
- **SerializableDiagnostic** - For FFI boundaries (Python, WASM)

External errors are converted to structured diagnostics preserving location and context.

---

## Extension Points

### New Backends

Implement the `Backend` trait with required methods: `id()`, `supported_formats()`, `glue_extension_types()`, `allow_auto_glue()`, `register_filters()`, and `compile()`. Optionally provide `default_quill()` for zero-config support.

**Requirements:** Thread-safe (`Send + Sync`), structured error reporting, format validation.

See `backends/quillmark-typst` for reference implementation.

### Custom Filters

Register via `glue.register_filter(name, func)` using stable `filter_api` types. Return `Result<Value, Error>` for error handling.

---

## Key Design Decisions

1. **Sealed Engine w/ Explicit Backend** - Simplifies usage; deterministic backend selection at engine creation
2. **Template-First, YAML-Only Frontmatter** - Reduces parsing complexity; backends convert via filters
3. **Default Quill System** - Backends provide embedded default templates for zero-config usage
4. **Dynamic Package Loading** - Runtime discovery of packages and versions
5. **Filter-Based Data Transformation** - Stable templating ABI across backends via `filter_api` module
6. **Unified Error Hierarchy** - Consistent diagnostics with source chains and location tracking
7. **Thread-Safe Design** - `Send + Sync` traits enable concurrent rendering
8. **Backend Auto-Registration** - Feature-based backend registration for simplified setup