# Architecture Overview

This document provides an overview of Quillmark's architecture and design principles.

## System Overview

Quillmark is a flexible, **template-first** Markdown rendering system that converts Markdown with YAML frontmatter into output artifacts (PDF, SVG, TXT, etc.). The architecture is built around an orchestration API for high-level use, backed by trait-based extensibility for backends.

### High-Level Data Flow

```
Markdown + YAML → Parse → Template → Compile → Artifacts
                    ↓        ↓          ↓
              ParsedDocument  Glue   PDF/SVG/TXT
```

The workflow follows three main stages:

1. **Parse** - Extract YAML frontmatter and body from markdown
2. **Template** - Compose backend-specific glue via MiniJinja with registered filters
3. **Compile** - Backend processes glue to generate output artifacts

## Core Design Principles

### 1. Sealed Orchestration, Explicit Backend

A single entry point (`Quillmark` engine) encapsulates orchestration. Backend choice is **explicit** at engine construction via feature flags.

### 2. Trait-Based Extensibility

New output formats implement the `Backend` trait (thread-safe, zero global state). This allows:

- Independent backend development
- Plugin-like architecture
- No tight coupling between backends

### 3. Template-First Philosophy

Quill templates fully control structure and styling; Markdown provides content via filters. This separation means:

- Content authors focus on writing
- Template designers control layout
- Clear separation of concerns

### 4. YAML-First Frontmatter

YAML is the single supported frontmatter format presented to templates. Backends may inject/convert to their native preferences (e.g., TOML for Typst) via filters.

### 5. Zero-Copy Where Practical

Minimize allocations; prefer references and `Cow<'static, str>` in hot paths for performance.

### 6. Error Transparency

Preserve context (sources, spans) and provide actionable diagnostics with:

- Source location tracking (file, line, column)
- Helpful hints for fixing errors
- Error chaining for context preservation

### 7. Dynamic Resource Loading

Discover templates, assets, fonts, and packages at runtime; no hardcoded dependencies.

### 8. Zero-Config Defaults

Standard directory conventions and default Quills let basic projects work without configuration.

## Project Structure

Quillmark is organized as a workspace with multiple crates:

### Core Crates

#### `quillmark-core`

Foundation layer providing:

- **Types**: `Backend`, `Artifact`, `OutputFormat`
- **Parsing**: `ParsedDocument` with `from_markdown()` constructor
- **Templating**: `Glue` + stable `filter_api`
- **Template model**: `Quill` (+ `Quill.toml`)
- **Errors & Diagnostics**: `RenderError`, `TemplateError`, `Diagnostic`, `Severity`, `Location`
- **Utilities**: TOML⇄YAML conversion helpers

**Design Note:** No external backend deps; backends depend on core → no cycles.

#### `quillmark`

Orchestration layer providing:

- **High-level API**: `Quillmark` for managing backends and quills
- **Sealed rendering API**: `Workflow`
- **Orchestration**: parse → compose → compile
- **Validation**: Structured error propagation
- **Backend auto-registration**: Based on enabled features
- **Default Quill registration**: During backend setup

### Backend Crates

#### `backends/quillmark-typst`

Typst backend for PDF/SVG output:

- Implements `Backend` trait
- Markdown→Typst conversion
- Template filters: String, Lines, Date, Dict, Content, Asset
- Compilation environment with font & asset resolution
- Structured diagnostics with source locations

#### `backends/quillmark-acroform`

AcroForm backend for PDF form filling:

- Implements `Backend` trait
- Templates field values using MiniJinja
- Supports tooltip-based and value-based templating
- Returns filled PDF as single artifact
- TXT format support for debugging

### Language Bindings

#### `bindings/quillmark-python`

PyO3-based Python bindings:

- Mirrors Rust API with Pythonic conventions
- Published to PyPI as `quillmark` package
- Full feature parity with Rust API
- Error delegation to core types

#### `bindings/quillmark-wasm`

wasm-bindgen based WebAssembly bindings:

- JSON data exchange for JavaScript interop
- Published to npm as `@quillmark-test/wasm`
- Supports bundler, Node.js, and web targets
- Error delegation to core types

### Supporting Crates

#### `quillmark-fixtures`

Centralized test resources and utilities:

- Test resources under `resources/`
- Helper functions for test setup and output
- Shared fixtures across test suites

#### `quillmark-fuzz`

Fuzz testing suite for:

- Parsing robustness
- Template rendering
- Backend compilation

## Component Architecture

### Main Components

**Quillmark Engine** - High-level orchestration managing backends and quills

**Workflow** - Rendering pipeline orchestration (parse → template → compile)

**Backend Trait** - Interface for implementing output formats (PDF, SVG, etc.)

**Quill** - Template bundle (glue template + assets/packages)

**ParsedDocument** - Frontmatter + body from markdown

**Diagnostic** - Structured error information

**RenderResult** - Output artifacts + warnings

### Template System

The template system uses **MiniJinja** with a **stable filter API** to keep backends decoupled:

#### Filter Architecture

Backends register custom filters via the stable `filter_api` module:

- **String** - Escape/quote values for backend syntax
- **Lines** - Array to multi-line string conversion
- **Date** - Date parsing for backend datetime types
- **Dict** - Objects to backend-native structures
- **Content** - Markdown body to backend markup conversion
- **Asset** - Dynamic asset filename transformation

The `filter_api` provides a stable ABI, preventing version conflicts and enabling independent backend development.

#### Template Context

Templates receive parsed document fields via MiniJinja context:

- **Top-level fields**: All frontmatter fields and `body` accessible directly
- **`__metadata__` field**: System-generated field containing all frontmatter fields except `body`

This dual access pattern provides both convenience (top-level) and clarity (metadata object).

## Parsing Architecture

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

Parses to: `{ title: "...", body: "...", products: [{ name: "...", body: "..." }] }`

## Backend Architecture

### Typst Backend

Key features:

- Markdown→Typst conversion with proper escaping
- Dynamic package loading from `packages/` directory
- Font and asset resolution from `assets/` directory
- Runtime asset injection with `DYNAMIC_ASSET__` prefix
- Structured error mapping with source locations

Filter support:

- String, Lines, Date, Dict filters for data transformation
- Content filter for Markdown→Typst conversion
- Asset filter for dynamic asset path mapping

Compilation environment (`QuillWorld`):

- Implements Typst `World` trait
- Virtual file system for packages and assets
- Line/column mapping for error diagnostics

### AcroForm Backend

Key features:

- Reads PDF forms from `form.pdf` in quill bundle
- Templates field values using MiniJinja
- Supports tooltip-based (`description__{{template}}`) and value-based templating
- Returns filled PDF as single artifact
- TXT format support for debugging (returns field values as JSON)

Compilation process:

1. Load PDF form from quill's `form.pdf` file
2. Extract field names and current values
3. Render templated values using MiniJinja with JSON context
4. Write rendered values back to PDF form
5. Return filled PDF as byte vector

## Package and Asset Management

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

## Error Handling

### Core Error Types

**Diagnostic** - Structured error with:

- Severity level (Error, Warning, Note)
- Optional error code
- Human-readable message
- Primary source location
- Optional hint for fixing
- Source error chain

**RenderError** - Main error enum for rendering operations

**SerializableDiagnostic** - For FFI boundaries (Python, WASM)

### Error Delegation

Language bindings delegate error handling to core types:

- Python bindings use `PyDiagnostic` wrapping `SerializableDiagnostic`
- WASM bindings use `SerializableDiagnostic` directly
- Single source of truth for error structure
- Automatic propagation of new error fields

## Extension Points

### New Backends

Implement the `Backend` trait with required methods:

- `id()` - Backend identifier
- `supported_formats()` - Output formats
- `glue_extension_types()` - Template file extensions
- `allow_auto_glue()` - Whether auto-glue is supported
- `register_filters()` - Register template filters
- `compile()` - Compile glue to artifacts

Optionally provide:

- `default_quill()` - Zero-config default template

Requirements:

- Thread-safe (`Send + Sync`)
- Structured error reporting
- Format validation

### Custom Filters

Register via `glue.register_filter(name, func)` using stable `filter_api` types. Return `Result<Value, Error>` for error handling.

## Key Design Decisions

1. **Sealed Engine w/ Explicit Backend** - Simplifies usage; deterministic backend selection
2. **Template-First, YAML-Only Frontmatter** - Reduces parsing complexity
3. **Default Quill System** - Backends provide embedded default templates for zero-config usage
4. **Dynamic Package Loading** - Runtime discovery of packages and versions
5. **Filter-Based Data Transformation** - Stable templating ABI across backends
6. **Unified Error Hierarchy** - Consistent diagnostics with source chains
7. **Thread-Safe Design** - `Send + Sync` traits enable concurrent rendering
8. **Backend Auto-Registration** - Feature-based backend registration for simplified setup

## Performance Considerations

- Zero-copy parsing where possible
- Efficient memory management with `Cow<'static, str>`
- Thread-safe design enables parallel rendering
- Virtual file systems for efficient asset access
- Cached schema validation results

## Security Considerations

- No arbitrary code execution in templates
- Sandboxed backend compilation
- Path traversal prevention in asset loading
- Input validation at all boundaries
- Structured error messages without leaking internals

## References

For complete implementation details, see:

- Architecture design: `prose/designs/ARCHITECTURE.md` in the repository
- Parsing specification: `prose/designs/PARSE.md` in the repository
- Error handling: `prose/designs/ERROR.md` in the repository
- Quill structure: `prose/designs/QUILL.md` in the repository
- [Rust API Documentation](https://docs.rs/quillmark/latest/quillmark/)
