# Quillmark Architecture

> This document merges **“Quillmark Architecture Design Document”** and **“Quillmark Improved Architecture Design Document”** into a single, authoritative DESIGN.md. Where the two differed, this doc reconciles them and notes compatibility.

---

**See also:**
- [PARSE.md](PARSE.md) - Detailed parsing and Extended YAML Metadata Standard documentation
- [ERROR.md](ERROR.md) - Error handling system documentation and implementation guide

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
* **Templating** → MiniJinja-based "Glue" composition with backend-registered filters
* **Backend Processing** → Compile composed glue to final artifacts
* **Assets/Packages** → Fonts, images, and backend packages resolved dynamically

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
* Parsing: `ParsedDocument` with `from_markdown()` constructor; `decompose()` function for direct parsing
* Templating: `Glue` + stable `filter_api`
* Template model: `Quill` (+ `Quill.toml`)
* **Errors & Diagnostics:** `RenderError`, `TemplateError`, `Diagnostic`, `Severity`, `Location`
* Utilities: TOML⇄YAML conversion helpers (for backend filters)

**Design Note:** No external backend deps; backends depend on core → no cycles.

### `quillmark` (orchestration layer)

* High-level API: `Quillmark` for managing backends and quills
* Sealed rendering API: `Workflow`
* Orchestration (parse → compose → compile)
* Validation and **structured error propagation**
* QuillRef for ergonomic quill references

**API Documentation:** See the crate's rustdoc for comprehensive API documentation with usage examples, including module-level overview, detailed method documentation, and doc tests.

### `quillmark-typst` (Typst backend)

* Implements `Backend` for PDF/SVG
* Markdown→Typst conversion (`mark_to_typst`)
* Filters: `String`, `Lines`, `Date`, `Dict`, `Content`, `Asset` (via JSON injection)
* Compilation environment (`QuillWorld`)
* Dynamic package loading (`typst.toml`), font & asset resolution
* **Structured diagnostics** with source locations (maps Typst diagnostics → `Diagnostic`)

### `quillmark-fixtures` (dev/test utilities)

* Centralized resources under `resources/`
* `resource_path()`, `example_output_dir()`, `write_example_output()`
* Workspace discovery and standardized example outputs

---

## Core Interfaces and Structures

> **Note:** This section provides high-level architecture overview. For complete API documentation, see the rustdoc (available at docs.rs or via `cargo doc --open`).

### Quillmark (high-level engine API)

The `Quillmark` struct is the main entry point that manages:
- Backend registration (auto-registered based on features)
- Quill template registration
- Workflow creation from quills or parsed documents

**Key methods:**
- `new()` - Create engine with auto-registered backends
- `register_quill()` - Add quill templates
- `workflow_from_*()` - Create workflows for rendering

### Workflow (render execution API)

The `Workflow` struct orchestrates the rendering pipeline:
- Holds backend and quill references
- Manages dynamic assets
- Executes the parse → template → compile pipeline

**Key methods:**
- `render()` - Full pipeline from parsed document to artifacts
- `add_asset()` / `add_assets()` - Runtime asset injection
- `backend_id()`, `supported_formats()` - Introspection

### Backend Trait (stable)

Backends implement output format support:
- `id()` - Unique backend identifier
- `supported_formats()` - Available output formats
- `register_filters()` - Template filter registration
- `compile()` - Glue content → final artifacts

### Core Types

- `ParsedDocument` - Frontmatter + body from markdown
- `Quill` - Template bundle (glue template + assets/packages)
- `Diagnostic` - Structured error information with source chains
- `RenderResult` - Output artifacts + warnings
- `Artifact` - Output bytes + format type

---

## End-to-End Orchestration Workflow

The workflow follows a three-stage pipeline:

1. **Parse** - Extract YAML frontmatter + body from markdown (`ParsedDocument::from_markdown()`)
2. **Template** - Compose backend-specific glue via MiniJinja with registered filters
3. **Compile** - Backend processes glue to generate output artifacts

**High-level flow:**
- Engine manages backends and quills
- Workflow orchestrates parse → template → compile
- Backends register filters and handle compilation
- Dynamic assets injected before compilation

### Key Concepts

**Backend Auto-Registration**: Backends are automatically registered based on enabled crate features (e.g., `typst` feature enables `TypstBackend`).

**Dynamic Assets**: Runtime assets are prefixed with `DYNAMIC_ASSET__` and injected into the quill's virtual file system before compilation, accessible via the `Asset` filter.

**Error Handling**: All error paths provide structured `Diagnostic` information with source chains preserved through the `source` field.

---

## Template System Design

The template system uses **MiniJinja** with a **stable filter API** to keep backends decoupled from the template engine.

### Filter Architecture

Backends register custom filters via the stable `filter_api` module:
- **String** - Escape/quote values with `default=` support
- **Lines** - Array to multi-line string conversion
- **Date** - Strict date parsing for backend datetime types
- **Dict** - Objects to backend-native structures (e.g., JSON)
- **Content** - Markdown body to backend markup
- **Asset** - Transform dynamic asset filenames to virtual paths

Filters bridge YAML values to backend-specific constructs while maintaining a stable ABI.

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
* **Tag directive parsing**: Extracts `!tag_name` from first line after opening `---`
* **Collection aggregation**: Groups tagged blocks into arrays under tag name
* **Edge cases handled**:

  * Empty frontmatter between `---` markers → valid, empty fields
  * Missing closing `---` at document start → error (fail-fast)
  * YAML parsing failure → error with descriptive message
  * Name collisions → error (prevents conflicts)
  * Reserved field names → error (protects `body` field)
  * Invalid tag syntax → error (validates `[a-z_][a-z0-9_]*` pattern)
  * End-of-file delimiters → supported (no trailing newline required)

**Key implementation pattern:**

```rust
// Internal function - use ParsedDocument::from_markdown() instead
fn decompose(markdown: &str) -> Result<ParsedDocument, Error> {
    let blocks = find_metadata_blocks(markdown)?;
    // Separate global frontmatter from tagged blocks
    // Validate no name collisions or reserved names
    // Aggregate tagged blocks into arrays
    // Extract global body content
}
```

#### ParsedDocument Structure

* **Fields storage**: Single `HashMap<String, QuillValue>` for both frontmatter and body
* **Body access**: Special field `BODY_FIELD = "body"` - use constants to avoid typos
* **Value types**: Support strings, numbers, arrays, objects via `QuillValue` (backed by `serde_json::Value`)

---

## Backend Architecture

### Typst Backend

* **Formats:** PDF, multi-page SVG (validated at runtime against build features)
* **Markdown→Typst:** `mark_to_typst()` supports emphasis, links, lists, code, breaks; escapes Typst-reserved chars (`* _ ` # [ ] $ < > @`).
* **Filters:** robust escaping, JSON/TOML embedding, date handling, markup `eval()` generation, dynamic asset path transformation.
* **Compilation (`QuillWorld`):** implements Typst `World` for:

  * *Dynamic Packages:* load from `packages/` with `typst.toml` (namespace, version, entrypoint)
  * *Assets:* fonts/images under `assets/` with stable virtual paths
  * *Dynamic Assets:* runtime-injected assets prefixed with `DYNAMIC_ASSET__` under `assets/`
  * *Errors:* line/column mapping, multi-error reporting with source context

#### Implementation Hints

##### Markdown to Typst Conversion (`mark_to_typst`)

* **Event-based parsing**: Use `pulldown_cmark::Parser` with `Options::ENABLE_STRIKETHROUGH`
* **Character escaping critical**: Typst has many reserved chars that must be escaped in text content:

  ```rust
  // Essential escapes for `escape_markup()`:
  s.replace('\\', "\\\\")  // Backslash first!
   .replace('*', "\\*")   // Bold/italic markers
   .replace('_', "\\_")   // Emphasis markers  
   .replace('#', "\\#")   // Headings
   .replace('$', "\\$")   // Math mode
   .replace('@', "\\@")   // References
  // Plus: ` [ ] < >
  ```
* **List handling gotcha**: Typst uses `+` for unordered lists, `-` for bullet points in text → convert markdown `-` to Typst `+`
* **Event processing pattern**:

  ```rust
  match event {
      Event::Start(Tag::Strong) => output.push_str("*"),
      Event::End(TagEnd::Strong) => output.push_str("*"),
      Event::Start(Tag::Emphasis) => output.push_str("_"),
      Event::End(TagEnd::Emphasis) => output.push_str("_"),
      Event::Text(text) => output.push_str(&escape_markup(&text)),
      // Handle list indentation with stack
  }
  ```

##### QuillWorld Implementation (Typst World trait)

* **FileId construction**: Use `FileId::new(Option<PackageSpec>, VirtualPath)`

  * `None` for main documents and assets
  * `Some(package_spec)` for package files
* **Virtual path gotchas**:

  * Use `VirtualPath::new(path_str)` - path must be forward-slash separated
  * Assets: `assets/image.png`, packages: `src/lib.typ` (preserve directory structure)
  * Manual path construction for subdirs: `format!("{}/{}", base, name)`
* **Package discovery flow**:

  1. Scan `packages/` directory recursively
  2. Find `typst.toml` files → parse for `namespace`, `name`, `version`, `entrypoint`
  3. Create `PackageSpec` with parsed metadata
  4. Load all files recursively maintaining directory structure
  5. Register entrypoint file for package resolution

```rust
// Package loading pattern:
let spec = PackageSpec {
    namespace: package_info.namespace.into(),
    name: package_info.name.into(), 
    version: package_info.version.parse()?,
};
// Load with preserved directory structure
let virtual_path = VirtualPath::new("src/lib.typ"); // Example
let file_id = FileId::new(Some(spec.clone()), virtual_path);
```

##### Font Loading Strategy

* **Search order**: `assets/fonts/` → `assets/` → system fonts
* **Supported formats**: `.ttf`, `.otf`, `.woff`, `.woff2`
* **Error handling**: If no fonts found, provide clear error message - Typst needs fonts for compilation
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

- **Static assets**: Fonts (ttf, otf, woff) and images in `assets/`
- **Dynamic assets**: Runtime-injected via `Workflow.add_asset()`
  - Prefixed with `DYNAMIC_ASSET__` to avoid collisions
  - Accessible via `Asset` filter in templates

* **Entrypoint verification**: After loading all files, verify entrypoint exists
* **Source vs Binary**: `.typ` files → `sources` HashMap, others → `binaries`
* **Error recovery**: Log warnings for individual package failures, continue loading others
* **Debug output**: Print loaded package names and file counts for troubleshooting

#### TOML Metadata Structure (`PackageInfo`)

```rust
#[derive(Debug, Clone)]
struct PackageInfo {
    namespace: String,      // @preview, @local, etc.
    name: String,          // Package identifier
    version: String,       // Semantic version
    entrypoint: String,    // Entry file (e.g., "src/lib.typ")
}
```

---

## Error Handling Patterns

> **📋 Implementation Guide:** See [ERROR.md](ERROR.md) for comprehensive documentation of the error handling system, including usage examples, migration notes, and future improvement plans.

This project uses a **simple, production‑usable, structured error strategy** that:

* preserves **line/column** and **source file** where available,
* keeps diagnostics **machine‑readable** and **pretty‑printable**, and
* avoids stringly‑typed errors.

### Core types (stable surface)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Severity { Error, Warning, Note }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Location {
    pub file: String,   // e.g., "glue.typ", "template.typ", "input.md"
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub primary: Option<Location>,
    pub hint: Option<String>,
}
```

---

## Error Handling Patterns

> **📋 Complete Documentation:** See [ERROR.md](ERROR.md) for comprehensive error handling documentation.

### Core Error Types

**Diagnostic** - Structured error information:
- Severity level (Error, Warning, Note)
- Error code for machine processing
- Human-readable message
- Source location (file, line, column)
- Helpful hints
- **Source chain** - Preserves full error context via `source` field

**RenderError** - Main error enum:
- Every variant contains `Diagnostic` payload(s)
- `CompilationFailed` may contain multiple diagnostics
- Extractable via `diagnostics()` method
- Display impl uses diagnostic messages

**SerializableDiagnostic** - For FFI boundaries:
- Python and WASM compatible
- Flattened source chain (list of strings)
- Derived from `Diagnostic` via `From` trait

### Error Conversion Pattern

External errors (MiniJinja, Typst, etc.) are converted to structured diagnostics:

1. Extract location information (file, line, column)
2. Create `Diagnostic` with appropriate severity and code
3. **Preserve original error** via `with_source()`
4. Generate context-aware hints
5. Wrap in appropriate `RenderError` variant

This ensures:
- No information loss during error propagation
- Full error chains for debugging
- Machine-readable diagnostic data
- Human-friendly error messages

### Source mapping policy (v1: comment anchors)

To relate Typst diagnostics in the generated glue back to the **template** or **Markdown body**, the composer injects lightweight anchors:

```typst
// @origin:template:glue.typ:123
// @origin:markdown:input.md:45
```

A small mapper walks upward from the error line to the last `@origin:` comment and rewrites `Diagnostic.primary` accordingly. This is cheap, deterministic, and works without shipping a separate source map.

> Optional v2: emit a `glue.typ.map.json` sidecar for exact mappings.

### Pretty printing & JSON output

Provide a standard formatter for CLI/logging and a JSON mode for tooling:

```rust
impl Diagnostic {
    pub fn fmt_pretty(&self) -> String { /* render code frame if available */ }
    pub fn fmt_pretty_with_source(&self) -> String { /* includes source chain */ }
    pub fn source_chain(&self) -> Vec<String> { /* extract error chain */ }
}

pub fn print_errors(err: &RenderError) {
    match err {
        RenderError::CompilationFailed { diags } => {
            for d in diags { eprintln!("{}", d.fmt_pretty()); }
        }
        RenderError::TemplateFailed { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::InvalidFrontmatter { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::EngineCreation { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::FormatNotSupported { diag } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::UnsupportedBackend { diag } => eprintln!("{}", diag.fmt_pretty()),
        // ... all other variants
    }
}
```

Expose `--json` (or a library method) returning `serde_json::Value` with the full error payload using `SerializableDiagnostic`.

### Engine behavior

* The engine **never** collapses errors to strings.
* `RenderResult.warnings` carries non-fatal diagnostics (e.g., deprecated fields, missing optional assets).
* For fatal errors, the engine returns `RenderError` with structured diagnostics.

### Operational logging

* Include a `context_id` in `RenderOptions` and attach it in each `Diagnostic` via `code` (e.g., `ctx:abcd1234`).
* Redact absolute paths in pretty output by default; keep full paths in JSON when `allow_paths=true`.
* Normalize path separators and line endings across platforms.

### Minimal test matrix

1. Invalid YAML frontmatter (both missing closer and parse error) → `InvalidFrontmatter` with location at `---` line.
2. MiniJinja syntax error → `TemplateFailed` with template filename + line/col.
3. Typst markup error in body → `CompilationFailed` with mapped location back to Markdown line via anchors.
4. Missing font/image/package → `CompilationFailed` with clear `code` and hint.
5. Concurrent renders → diagnostics are deterministic and contain correct file names.

---

## Extension Points

### New Backends

1. Implement `Backend` (formats, `glue_type`, filters, `compile`).
2. Handle assets/packages as needed; provide conversions/filters.
3. Ship as separate crate (depends on `quillmark-core`).

**Skeleton:**

```rust
pub struct MyBackend;
impl Backend for MyBackend {
    fn id(&self) -> &'static str { "my-backend" }
    fn supported_formats(&self) -> &'static [OutputFormat] { &[OutputFormat::Pdf] }
    fn glue_type(&self) -> &'static str { ".my" }
    fn register_filters(&self, glue: &mut Glue) { /* ... */ }
    fn compile(&self, content: &str, quill: &Quill, opts: &RenderOptions)
        -> Result<Vec<Artifact>, RenderError> { /* ... */ }
}
```

---

## Extension Points

### New Backends

To implement a new backend:

1. Implement the `Backend` trait
2. Define `id()`, `supported_formats()`, `glue_type()`
3. Register filters via `register_filters()`
4. Implement `compile()` for artifact generation
5. Ship as separate crate depending on `quillmark-core`

**Requirements:**
- Thread-safe (`Send + Sync`)
- Format validation in `compile()`
- Structured error reporting via `Diagnostic`
- Asset and package handling as needed

### Custom Filters

- Register via `glue.register_filter(name, func)`
- Use stable `filter_api` types only
- Return `Result<Value, Error>` for proper error handling
- Document filter behavior and type requirements

---

## Key Design Decisions

1. **Sealed Engine w/ Explicit Backend** - Simplifies usage; deterministic selection
2. **Template-First, YAML-Only Frontmatter** - Reduces parsing complexity
3. **Dynamic Package Loading** - Runtime discovery of packages and versions
4. **Filter-Based Data Transformation** - Stable templating ABI across backends
5. **Unified Error Hierarchy** - Consistent diagnostics with source chains
6. **Thread-Safe Design** - `Send + Sync` enables concurrent rendering