# QuillMark – Unified Architecture Design

> This document merges **“QuillMark Architecture Design Document”** and **“QuillMark Improved Architecture Design Document”** into a single, authoritative DESIGN.md. Where the two differed, this doc reconciles them and notes compatibility.

---

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
13. [Migration & Compatibility](#migration--compatibility)

---

## System Overview

QuillMark is a flexible, **template-first** Markdown rendering system that converts Markdown with YAML frontmatter into output artifacts (PDF, SVG, TXT, etc.). The architecture is built around a **sealed engine API** for day‑to‑day use, backed by trait-based extensibility for backends.

High-level data flow:

* **Parsing** → YAML frontmatter + body extraction
* **Templating** → MiniJinja-based "Glue" composition with backend-registered filters
* **Backend Processing** → Compile composed glue to final artifacts
* **Assets/Packages** → Fonts, images, and backend packages resolved dynamically

---

## Core Design Principles

1. **Sealed Engine, Explicit Backend**
   A single entry point (`QuillEngine`) encapsulates orchestration. Backend choice is **explicit** at engine construction.
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
* Parsing: `decompose`, `ParsedDocument`
* Templating: `Glue` + stable `filter_api`
* Template model: `Quill` (+ `quill.toml`)
* Errors: `RenderError`, `TemplateError`
* Utilities: TOML⇄YAML conversion helpers (for backend filters)

**Design Note:** No external backend deps; backends depend on core → no cycles.

### `quillmark` (sealed engine)

* Sealed primary API: `QuillEngine`
* Orchestration (parse → compose → compile)
* Validation and error propagation
* *Compatibility shim:* legacy `render(markdown, RenderConfig)` calls through the engine (see [Migration](#migration--compatibility)).

### `quillmark-typst` (Typst backend)

* Implements `Backend` for PDF/SVG
* Markdown→Typst conversion (`mark_to_typst`)
* Filters: `String`, `Lines`, `Date`, `Dict`, `Body`, and YAML→TOML injector
* Compilation environment (`QuillWorld`)
* Dynamic package loading (`typst.toml`), font & asset resolution
* Rich error formatting with source locations

### `quillmark-fixtures` (dev/test utilities)

* Centralized resources under `resources/`
* `resource_path()`, `example_output_dir()`, `write_example_output()`
* Workspace discovery and standardized example outputs

---

## Core Interfaces and Structures

### QuillEngine (primary high-level API)

```rust
pub struct QuillEngine {
    backend: Box<dyn Backend>,
    quill: Quill,
}

impl QuillEngine {
    pub fn new(backend: Box<dyn Backend>, quill_path: PathBuf) -> Result<Self, RenderError>;
    pub fn render(&self, markdown: &str) -> RenderResult;
    pub fn render_with_format(&self, markdown: &str, format: OutputFormat) -> RenderResult;
    pub fn backend_id(&self) -> &str;
    pub fn supported_formats(&self) -> &'static [OutputFormat];
    pub fn quill_name(&self) -> &str;
}
```

### Backend Trait (stable)

```rust
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;                       // e.g., "typst", "latex"
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn glue_type(&self) -> &'static str;                // file extension, e.g., ".typ"
    fn register_filters(&self, glue: &mut Glue);
    fn compile(&self, glue_content: &str, quill: &Quill, opts: &RenderOptions)
        -> Result<Vec<Artifact>, RenderError>;
}
```

> **Note:** `RenderOptions` is internal to engine orchestration; public callers use the engine methods. Legacy `RenderConfig` maps to `RenderOptions` (see [Migration](#migration--compatibility)).

### Quill (template bundle)

```rust
pub struct Quill {
    pub template_content: String,
    pub metadata: HashMap<String, serde_yaml::Value>,
    pub base_path: PathBuf,
    pub name: String,
    pub glue_file: String,
}
```

**Key methods:** `from_path()`, `validate()`, `glue_path()`, `assets_path()`, `packages_path()`, `toml_to_yaml_value()`.

### ParsedDocument

```rust
pub struct ParsedDocument {
    pub fields: HashMap<String, serde_yaml::Value>,
}
```

**Helpers:** `body()`, `get_field()`, `fields()`; body is stored under reserved `BODY_FIELD`.

### Glue (MiniJinja wrapper with stable filter API)

```rust
pub struct Glue {
    env: Environment<'static>,
    template: String,
}
```

**Methods:** `new()`, `register_filter()`, `compose(context)`.

**Filter ABI (stable surface):**

```rust
pub mod filter_api {
    pub use minijinja::{Error, ErrorKind, State};
    pub use minijinja::value::{Kwargs, Value};
    pub trait DynFilter: Send + Sync + 'static {}
    impl<T> DynFilter for T where T: Send + Sync + 'static {}
}
```

#### Implementation Hints

##### Glue Template Processing
* **MiniJinja setup**: Create `Environment` 
* **Template compilation**: Parse template string once, reuse for multiple `compose()` calls  
* **Error propagation**: Convert MiniJinja errors to `TemplateError` with source context

##### Filter Registration Best Practices
* **Type safety**: Use `filter_api` types only - don't leak MiniJinja internals
* **Error handling**: Return `filter_api::Error` with appropriate `ErrorKind`
* **Documentation**: Each filter should handle null/missing values gracefully
* **Performance**: Consider caching expensive conversions within filter implementations

### Artifact & Output Format

```rust
pub struct Artifact { pub bytes: Vec<u8>, pub output_format: OutputFormat }
```

---

## End-to-End Orchestration Workflow

**Public usage:**

```rust
let engine = QuillEngine::new(Box::new(TypstBackend::default()), quill_path)?;
let artifacts = engine.render(markdown)?;                // or render_with_format(...)
```

**Internal steps (encapsulated):**

1. **Load Quill**: `Quill::from_path(quill_path)` → validate; pick glue file by backend `glue_type()` or `quill.toml` override.
2. **Parse Markdown**: `decompose(markdown)` → YAML frontmatter + body.
3. **Setup Glue**: `Glue::new(quill.template_content)`; backend `register_filters(&mut glue)`.
4. **Compose**: `glue.compose(parsed.fields().clone())` → backend-specific glue source.
5. **Compile**: `backend.compile(&glue_src, &quill, &opts)` → `Vec<Artifact>` (PDF/SVG/TXT…).

### Implementation Hints

#### Engine Construction (`QuillEngine::new`)
* **Backend validation**: Check `backend.id()` matches expected backend type
* **Quill loading**: Use `Quill::from_path()` with proper error context 
* **Glue file selection**: Priority: `quill.toml` override → `backend.glue_type()` extension

#### Orchestration Error Handling
* **Step isolation**: Each step should handle its own errors and provide context
* **Rollback strategy**: No partial state - either complete success or clean failure
* **Context preservation**: Chain errors with source information through each step

#### Performance Considerations  
* **Template caching**: Reuse parsed Glue templates when possible
* **Asset preloading**: Load fonts/assets once per engine instance
* **Memory management**: Use references where possible, avoid unnecessary clones

#### Workflow State Management
```rust
// Typical orchestration pattern:
let quill = Quill::from_path(quill_path)?;              // Step 1
let parsed = decompose(markdown)?;                      // Step 2  
let mut glue = Glue::new(&quill.template_content)?;     // Step 3a
backend.register_filters(&mut glue);                   // Step 3b
let glue_source = glue.compose(parsed.fields().clone())?; // Step 4
let artifacts = backend.compile(&glue_source, &quill, &opts)?; // Step 5
```

---

## Template System Design

* Engine uses **MiniJinja** with a **stable filter API** so backends do not directly depend on MiniJinja.
* Backend-provided filters bridge YAML values → backend-native constructs.

**Common Filters**

* **String**: escape/quote; `default=` kwarg; special handling for `none` sentinel
* **Lines**: string array for multi-line embedding
* **Date**: strict date parsing; produces TOML-like object when needed
* **Dict**: objects → JSON string; type validation
* **Body**: Markdown body → backend markup (e.g., Typst) and inject with `eval()` as needed
* **Toml** *(Typst-only convenience)*: YAML → TOML string injection for `toml()` usage

**Template usage example (Typst glue):**

```typst
{{ title | String(default="Untitled") }}
{{ recipients | Lines }}
{{ date | Date }}
{{ frontmatter | Toml }}          // optional: for Typst-native toml(...)
{{ body | Body }}
```

### Implementation Hints

#### Filter API Stability Pattern
* **Abstraction layer**: Core exposes `filter_api` module re-exporting MiniJinja types
* **Backend isolation**: Backends import only `quillmark_core::templating::filter_api` 
* **Key types**:
  ```rust
  use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
  
  pub fn my_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error>
  ```

#### Filter Implementation Patterns

##### String Filter (`string_filter`)
* **Default handling**: Check `kwargs_default()` for `default=` parameter
* **Null coercion**: `value.is_null() || (string && empty)` → use default or `""`
* **Sentinel values**: `"none"` string → return `none_value()` (unquoted Typst literal)
* **Escaping**: Use `escape_string()` for quotes, newlines, control chars
* **Output format**: Return quoted string `"escaped_content"`

##### Lines Filter (`lines_filter`) 
* **Input validation**: Must be array, error if not
* **Element conversion**: Convert each array element to string via `json_to_string_lossy()`
* **JSON output**: Return compact JSON array of strings for template embedding

##### Dict Filter (`dict_filter`)
* **YAML→JSON**: Convert `serde_yaml::Value` to `serde_json::Value` 
* **Typst embedding**: Wrap in `json(bytes("..."))` for Typst evaluation
* **Escaping**: Use `escape_string()` on serialized JSON

##### Body Filter (`body_filter`)
* **Markdown conversion**: Apply `mark_to_typst()` to body content
* **Eval wrapping**: Return `eval("typst_markup")` for safe template injection
* **Content type**: Treats input as markdown string, outputs Typst markup

#### Value Conversion Helpers
```rust
// JSON ↔ MiniJinja Value bridge functions
fn v_to_json(v: &Value) -> Json;          // MiniJinja → serde_json
fn json_to_string_lossy(j: &Json) -> String;  // JSON → display string
fn kwargs_default(kwargs: &Kwargs) -> Result<Option<Json>, Error>; // Extract default= param
```

---

## Parsing and Document Decomposition

* **Frontmatter:** YAML delimited by `---` … `---` at the top of the document.
* **Process:**

  1. Detect frontmatter block; parse to `HashMap<String, serde_yaml::Value>`
  2. Store the remainder as body under `BODY_FIELD`
  3. Preserve original content on failures; errors are reported but non-fatal
* **Policy:** YAML-only input; no TOML frontmatter. Backends can convert via filters.

**Error posture:** graceful degradation (invalid frontmatter → treat as body; log/return warnings).

### Implementation Hints

#### Frontmatter Detection (`decompose` function)
* **Line ending gotcha**: Check both `"---\n"` and `"---\r\n"` for Windows compatibility
* **End marker search**: Skip first line (opening `---`), find closing `---` with `line.trim() == "---"`
* **Body extraction**: Join lines after closing `---` with `trim_start()` to remove leading whitespace
* **Edge cases to handle**:
  - Empty frontmatter between `---` markers → return just body
  - Missing closing `---` → treat entire content as body
  - YAML parsing failure → graceful degradation, log warning, return entire content as body

```rust
// Key implementation pattern:
if markdown.starts_with("---\n") || markdown.starts_with("---\r\n") {
    let lines: Vec<&str> = markdown.lines().collect();
    // Find closing --- (skip line 0 which is opening ---)
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" { /* Found end */ }
    }
}
```

#### ParsedDocument Structure
* **Fields storage**: Single `HashMap<String, serde_yaml::Value>` for both frontmatter and body
* **Body access**: Special field `BODY_FIELD = "body"` - use constants to avoid typos
* **YAML value types**: Support strings, numbers, arrays, objects via `serde_yaml::Value`

---

## Backend Architecture

### Typst Backend

* **Formats:** PDF, multi-page SVG (validated at runtime against build features)
* **Markdown→Typst:** `mark_to_typst()` supports emphasis, links, lists, code, breaks; escapes Typst-reserved chars (`* _ ` # [ ] $ < > @`).
* **Filters:** robust escaping, JSON/TOML embedding, date handling, markup `eval()` generation.
* **Compilation (`QuillWorld`):** implements Typst `World` for:

  * *Dynamic Packages:* load from `packages/` with `typst.toml` (namespace, version, entrypoint)
  * *Assets:* fonts/images under `assets/` with stable virtual paths
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
  - `None` for main documents and assets
  - `Some(package_spec)` for package files
* **Virtual path gotchas**:
  - Use `VirtualPath::new(path_str)` - path must be forward-slash separated
  - Assets: `assets/image.png`, packages: `src/lib.typ` (preserve directory structure)
  - Manual path construction for subdirs: `format!("{}/{}", base, name)`
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
* **Loading pattern**: Read font files to `Vec<u8>` and store in `FontBook`

##### Error Formatting (`format_compilation_errors`)  
* **Multi-error reporting**: Iterate through `Vec<SourceDiagnostic>`, format each with context
* **Span to line mapping**: Extract line info from `typst::syntax::Span` using source references
* **Include hints and traces**: `error.hints` and `error.trace` provide debugging context

---

## Package Management and Asset Handling

**Quill template layout (opinionated):**

```
quill-template/
├─ quill.toml              # metadata; can override glue file name
├─ glue.<ext>              # e.g., glue.typ
├─ packages/               # backend packages
│  └─ <pkg>/typst.toml …
└─ assets/                 # fonts/, images/, data/
```

**Package loading (algorithm):**

1. Scan `packages/` recursively
2. Parse `typst.toml` metadata
3. Build virtual paths; register namespace (`@preview`, `@local`, custom)
4. Resolve entrypoints; load all package files preserving structure

**Assets:**

* Fonts: `.ttf`, `.otf`, `.woff`, `.woff2`
* Binary assets: images/data as bytes
* Recursive discovery; prefix-preserving virtual paths

### Implementation Hints

#### Package Discovery (`load_packages_recursive`)
* **Directory scanning**: Use `std::fs::read_dir()` on `packages/` directory
* **TOML parsing gotchas**:
  - Required fields: `[package] name, version, entrypoint` 
  - Optional: `namespace` (defaults to `"local"`)
  - Use `toml::from_str()` with proper error handling
* **Version parsing**: `semver::Version::parse()` for proper semantic versioning
* **Namespace handling**: Support `@preview`, `@local`, custom namespaces

#### Virtual Path Management
* **Critical pattern**: Preserve directory structure in virtual file system
  ```rust
  // Manual path construction (avoid `join()` for virtual paths):
  let virtual_path = if base_path.as_rootless_path().as_os_str().is_empty() {
      VirtualPath::new(&filename)
  } else {
      let base_str = base_path.as_rootless_path().to_string_lossy();
      let full_path = format!("{}/{}", base_str, filename);
      VirtualPath::new(&full_path)  // Forward slashes required
  };
  ```
* **FileId creation**: 
  - Assets: `FileId::new(None, virtual_path)` 
  - Packages: `FileId::new(Some(package_spec), virtual_path)`

#### Asset Loading Strategy
* **Recursive traversal**: Use helper function to maintain virtual path hierarchy
* **File type detection**: Check extensions for fonts vs binary assets
* **Loading pattern**:
  ```rust
  let data = std::fs::read(&physical_path)?;
  let file_id = FileId::new(None, virtual_path);
  binaries.insert(file_id, Bytes::new(data));
  ```

#### Package File Loading  
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

### RenderError (engine/backends)

```rust
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("Engine creation failed: {0}")] EngineCreation(String),
    #[error("Invalid YAML frontmatter: {0}")] InvalidFrontmatter(String),
    #[error("Quill template error: {0}")] QuillError(String),
    #[error("Backend compilation failed: {0}")] CompilationError(String),
    #[error("{format:?} not supported by {backend:?}")]
    FormatNotSupported { backend: String, format: OutputFormat },
    #[error("{0:?} backend is not built in this binary")] UnsupportedBackend(String),
    #[error(transparent)] Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```

### TemplateError (templating)

```rust
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("{0}")] RenderError(#[from] minijinja::Error),
    #[error("{0}")] InvalidTemplate(String, #[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")] FilterError(String),
}
```

**Error context:**

* Source chaining with `#[source]`
* Location enrichment for template/compile errors
* Human-friendly formatting; partial outputs when safe

### Implementation Hints

#### Error Chaining Strategy
* **thiserror usage**: Use `#[from]` for automatic conversions, `#[source]` for cause chains
* **Context preservation**: Box complex errors to avoid large enum variants
* **Transparent wrapper**: `#[error(transparent)]` for pass-through errors

#### Compilation Error Formatting
* **Multi-error handling**: Typst compilation returns `Vec<SourceDiagnostic>`
* **Rich context**: Include line numbers, error severity, hints, and traces
* **Format pattern**:
  ```rust
  for (i, error) in errors.iter().enumerate() {
      formatted.push_str(&format!("\nError #{}: {}", i + 1, error.message));
      // Add span/line info, severity, hints, traces
  }
  ```

#### Graceful Degradation Patterns  
* **Parse failures**: Invalid YAML → treat as body, log warning
* **Missing resources**: Missing fonts/assets → clear error message with paths
* **Package failures**: Individual package errors → warn and continue with others
* **Template errors**: Provide line/column context from MiniJinja errors

#### Error Recovery Best Practices
* **Early validation**: Check file paths, formats, backend compatibility upfront  
* **Partial success**: Return what was successfully processed with error context
* **Debug information**: Include file paths, package names, source locations
* **User-actionable messages**: Explain what to check/fix, not just what failed

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

### Custom Filters

* Register via `glue.register_filter(name, func)` using stable `filter_api`.
* Example: YAML→backend-native structure converter.

### Template Extensions

* Backend-specific syntax and asset workflows are allowed behind filters/compile.
* Multiple artifacts per render supported (e.g., multi-page SVGs).

### Implementation Hints

#### Backend Implementation Checklist
* **Thread safety**: Ensure `Send + Sync` - no global mutable state
* **Format validation**: Check `OutputFormat` against `supported_formats()` in `compile()`
* **Resource handling**: Implement asset/package loading appropriate to your backend
* **Error context**: Return meaningful `RenderError::CompilationError` with context

#### Filter Registration Pattern
```rust
fn register_filters(&self, glue: &mut Glue) {
    glue.register_filter("my_filter", my_filter_impl);
    glue.register_filter("my_format", my_format_filter);
    // Register backend-specific filters
}
```

#### Compilation Implementation Strategy  
* **Input parsing**: Parse `glue_content` string (your backend's template format)
* **Asset resolution**: Use `quill.assets_path()` and `quill.packages_path()` 
* **Multi-format support**: Return different `Artifact` instances based on requested format
* **Error mapping**: Convert backend errors to `RenderError` with source context

#### Asset Integration Patterns
* **Font handling**: Load fonts from `quill.assets_path().join("fonts")`  
* **Image resources**: Resolve image paths relative to assets directory
* **Package dependencies**: Scan `quill.packages_path()` for backend-specific packages
* **Virtual paths**: Maintain consistent path mapping for template references

#### Testing Strategy
* **Unit tests**: Test filter functions independently with various input types
* **Integration tests**: Test full `compile()` flow with sample quill templates
* **Error cases**: Verify graceful handling of missing assets, invalid templates
* **Format coverage**: Test each supported `OutputFormat` produces correct `Artifact`

---

## Key Design Decisions

1. **Sealed Engine w/ Explicit Backend**
   Simplifies usage; avoids global registries; deterministic selection.
2. **Template-First, YAML-Only Frontmatter**
   Reduces parsing complexity; backends may inject TOML/etc. via filters.
3. **Dynamic Package Loading**
   Runtime discovery enables user-provided packages and versions.
4. **Filter-Based Data Transformation**
   Backend-optimized transforms while keeping a stable templating ABI.
5. **Unified Error Hierarchy**
   Consistent, contextual errors across engine, templating, and backends.
6. **Thread-Safe Design**
   `Send + Sync` across core traits enables concurrent rendering.
7. **Centralized Fixtures**
   Dev resources isolated; standardized example outputs.

---

## Migration & Compatibility

### For Users

* **Preferred:**

  ```rust
  let engine = QuillEngine::new(Box::new(TypstBackend::default()), quill_path)?;
  let artifacts = engine.render(markdown)?;
  ```
* **Legacy compatibility:** `render(markdown, RenderConfig)` is maintained as a thin shim that internally constructs `QuillEngine` (mapping `RenderConfig` → internal `RenderOptions`) and calls `render_with_format` as needed.
* Ensure frontmatter is **YAML**; templates should follow the opinionated directory structure.

### For Backend Authors

* The `Backend` trait is unchanged and remains thread-safe.
* Implement format-specific filters; provide YAML→native conversions when beneficial.
* Package/asset loading lives within your backend crate; depend only on `quillmark-core`.

### Benefits of the Unified Model

* Smaller public surface; clearer mental model
* Deterministic backend selection without global state
* Stronger error narratives and easier debugging
* Future-proof for internal optimizations without API breakage