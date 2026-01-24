# Quillmark Architecture

This document outlines the architecture and design principles of Quillmark, a flexible Markdown rendering engine that outputs to fully typesetted documents or forms.

## Table of Contents

1. [System Overview](#system-overview)
2. [Core Design Principles](#core-design-principles)
3. [Crate Structure and Responsibilities](#crate-structure-and-responsibilities)
4. [Core Interfaces and Structures](#core-interfaces-and-structures)
5. [End-to-End Orchestration Workflow](#end-to-end-orchestration-workflow)
6. [Data Injection and Plate Consumption](#data-injection-and-plate-consumption)
7. [Parsing and Document Decomposition](#parsing-and-document-decomposition)
8. [Backend Architecture](#backend-architecture)
9. [Package Management and Asset Handling](#package-management-and-asset-handling)
10. [Error Handling Patterns](#error-handling-patterns)
11. [Extension Points](#extension-points)
12. [Key Design Decisions](#key-design-decisions)

---

## System Overview

Quillmark is a flexible, **template-first** Markdown rendering system that converts Markdown with YAML frontmatter into output artifacts (PDF, SVG, TXT, etc.). The architecture is built around an orchestration API for high-level use, backed by trait-based extensibility for backends.

High-level data flow:

* **Parsing & Normalization** → YAML/frontmatter extraction, CARD aggregation, bidi stripping, HTML fence normalization
* **Schema Coercion & Defaults** → Apply type coercion and defaults from Quill schema
* **Backend Field Transforms** → Backend-specific field shaping (e.g., markdown→Typst markup)
* **Backend Processing** → Compile plate content with injected JSON data
* **Assets/Packages** → Fonts, images, and packages resolved dynamically (including dynamic assets/fonts)

---

## Core Design Principles

1. **Sealed Orchestration, Explicit Backend**
   A single entry point (`Workflow`) encapsulates orchestration. Backend choice is **explicit** at engine construction.
2. **Trait-Based Extensibility**
   New output formats implement the `Backend` trait (thread-safe, zero global state).
3. **Template-First**
   Quill templates fully control structure and styling; Markdown provides content through JSON injection.
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
* Template model: `Quill` (+ `Quill.yaml`)
* Errors & Diagnostics: `RenderError`, `TemplateError`, `Diagnostic`, `Severity`, `Location`
* Utilities: YAML parsing and conversion helpers

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
* Markdown→Typst conversion via backend `transform_fields`
* `@local/quillmark-helper` package exposes injected JSON data inside Typst
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
- **Quill** - Template bundle (plate template + assets/packages)
- **ParsedDocument** - Frontmatter + body from markdown
- **Diagnostic** - Structured error information
- **RenderResult** - Output artifacts + warnings

---

## End-to-End Orchestration Workflow

The workflow follows a three-stage pipeline:

1. **Parse** - Extract YAML frontmatter + body from markdown
2. **Normalize & Shape** - Coerce types, apply defaults, normalize bidi/HTML fences, and run backend `transform_fields`
3. **Compile** - Backend processes plate with injected JSON data to generate output artifacts

### Key Concepts

- **Backend Auto-Registration**: Backends registered based on enabled features
- **Default Quill System**: Backends provide fallback templates for documents without `QUILL:` tags
- **Dynamic Assets**: Runtime assets and fonts injected into the quill file tree with `DYNAMIC_ASSET__`/`DYNAMIC_FONT__` prefixes
- **Error Handling**: Structured diagnostics with source chain preservation

---

## Data Injection and Plate Consumption

Plate content is passed to backends **without** MiniJinja composition. Backends receive:

- `plate_content`: the raw plate from `Quill.plate` (or empty for plate-less backends)
- `json_data`: JSON produced by `Workflow::compile_data()` after coercion, defaults, normalization, and backend `transform_fields`
- `quill`: the template bundle with assets, packages, and optional dynamic assets/fonts injected

### Typst Backend

- Loads plate content directly into Typst
- Injects `json_data` as a virtual package `@local/quillmark-helper:<version>` (see [GLUE_METADATA.md](GLUE_METADATA.md))
- Uses backend `transform_fields` to convert markdown schema fields to Typst markup before serialization

### AcroForm Backend

- Ignores plate content and reads `form.pdf` from the quill
- Uses MiniJinja to render field values from `json_data`

---

## Parsing and Document Decomposition

Quillmark supports advanced markdown parsing with the **Extended YAML Metadata Standard**.

### Basic Frontmatter

- YAML delimited by `---` at document start
- Converted to `HashMap<String, QuillValue>`
- Body stored under reserved `BODY_FIELD` constant
- Fail-fast error reporting for malformed YAML

### Extended YAML Metadata Standard

Supports **inline metadata sections** using `CARD` and `QUILL` keys:

- **CARD**: Creates typed card blocks aggregated into a unified `CARDS` array
- **QUILL**: Specifies which template to use
- **Horizontal rule disambiguation**: Smart detection distinguishes metadata from markdown
- **Validation**: Card names follow `[a-z_][a-z0-9_]*` pattern

**Example:**
```markdown
---
title: Product Catalog
---
Main description.

---
CARD: products
name: Widget
---
Widget description.
```

Parses to: `{ title: "...", CARDS: [{ CARD: "products", name: "...", BODY: "..." }] }`

**See `designs/PARSE.md` for complete specification.**

---

## Backend Architecture

### Typst Backend

The Typst backend implements PDF and SVG output:

**Key Features:**
- Markdown→Typst conversion via backend `transform_fields` for fields annotated with `contentMediaType = "text/markdown"`
- JSON data injection exposed to Typst through the virtual `@local/quillmark-helper` package
- Dynamic package loading from `packages/` directory
- Font and asset resolution from `assets/` directory (including dynamic assets/fonts)
- Structured error mapping with source locations

**Compilation Environment (`QuillWorld`):**
- Implements Typst `World` trait
- Virtual file system for packages and assets
- Helper package generates `lib.typ`/`typst.toml` from injected JSON data
- Line/column mapping for error diagnostics

### AcroForm Backend

The AcroForm backend implements PDF form filling:

**Key Features:**
- Reads PDF forms from `form.pdf` in quill bundle
- Templates field values using MiniJinja
- Supports tooltip-based (`description__{{template}}`) and value-based templating
- Returns filled PDF as single artifact

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
├─ Quill.yaml              # Metadata and configuration
├─ plate.<ext>              # Template file (e.g., plate.typ)
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

Implement the `Backend` trait with required methods: `id()`, `supported_formats()`, `plate_extension_types()`, `transform_fields()`, and `compile()`. Optionally provide `default_quill()` for zero-config support.

**Requirements:** Thread-safe (`Send + Sync`), structured error reporting, format validation.

See `backends/quillmark-typst` for reference implementation.

### Custom Filters

Register via `plate.register_filter(name, func)` using stable `filter_api` types. Return `Result<Value, Error>` for error handling.

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
