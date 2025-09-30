# Quillmark Architecture

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
* Parsing: `decompose`, `ParsedDocument`
* Templating: `Glue` + stable `filter_api`
* Template model: `Quill` (+ `quill.toml`)
* **Errors & Diagnostics:** `RenderError`, `TemplateError`, `Diagnostic`, `Severity`, `Location`
* Utilities: TOML⇄YAML conversion helpers (for backend filters)

**Design Note:** No external backend deps; backends depend on core → no cycles.

### `quillmark` (sealed engine)

* High-level API: `Quillmark` engine for managing backends and quills
* Sealed rendering API: `Workflow`
* Orchestration (parse → compose → compile)
* Validation and **structured error propagation**
* QuillRef for ergonomic quill references
* *Compatibility shim:* legacy `render(markdown, RenderConfig)` calls through the engine (see [Migration](#migration--compatibility)).

### `quillmark-typst` (Typst backend)

* Implements `Backend` for PDF/SVG
* Markdown→Typst conversion (`mark_to_typst`)
* Filters: `String`, `Lines`, `Date`, `Dict`, `Body`, and YAML→TOML injector
* Compilation environment (`QuillWorld`)
* Dynamic package loading (`typst.toml`), font & asset resolution
* **Structured diagnostics** with source locations (maps Typst diagnostics → `Diagnostic`)

### `quillmark-fixtures` (dev/test utilities)

* Centralized resources under `resources/`
* `resource_path()`, `example_output_dir()`, `write_example_output()`
* Workspace discovery and standardized example outputs

---

## Core Interfaces and Structures

### Quillmark (high-level engine API)

```rust
pub struct Quillmark {
    backends: HashMap<String, Box<dyn Backend>>,
    quills: HashMap<String, Quill>,
}

impl Quillmark {
    pub fn new() -> Self;
    pub fn register_quill(&mut self, quill: Quill);
    pub fn load<'a>(&self, quill_ref: impl Into<QuillRef<'a>>) -> Result<Workflow, RenderError>;
    pub fn registered_backends(&self) -> Vec<&str>;
    pub fn registered_quills(&self) -> Vec<&str>;
    
    #[deprecated(since = "0.1.0", note = "Use `load()` instead")]
    pub fn get_workflow(&self, quill_name: &str) -> Result<Workflow, RenderError>;
}
```

**Usage pattern:**

```rust
// Create engine with auto-registered backends
let mut engine = Quillmark::new();

// Register quills
let quill = Quill::from_path("path/to/quill")?;
engine.register_quill(quill);

// Load workflow by name or object
let workflow = engine.load("my-quill")?;
let workflow = engine.load(&quill)?;  // Also accepts Quill reference
```

### Workflow (render execution API)

```rust
pub struct Workflow {
    backend: Box<dyn Backend>,
    quill: Quill,
}

impl Workflow {
    pub fn new(backend: Box<dyn Backend>, quill: Quill) -> Result<Self, RenderError>;
    pub fn render(&self, markdown: &str, format: Option<OutputFormat>) -> Result<RenderResult, RenderError>;
    pub fn render_content(&self, content: &str, format: Option<OutputFormat>) -> Result<RenderResult, RenderError>;
    pub fn process_glue(&self, markdown: &str) -> Result<String, RenderError>;
    pub fn backend_id(&self) -> &str;
    pub fn supported_formats(&self) -> &'static [OutputFormat];
    pub fn quill_name(&self) -> &str;
}
```

### QuillRef (ergonomic quill references)

```rust
pub enum QuillRef<'a> {
    Name(&'a str),
    Object(&'a Quill),
}
```

Implements `From` for `&str`, `&String`, `&Quill`, and `&Cow<str>` for ergonomic API usage.

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
    pub glue_template: String,
    pub metadata: HashMap<String, serde_yaml::Value>,
    pub base_path: PathBuf,
    pub name: String,
    pub glue_file: String,
    pub files: HashMap<PathBuf, FileEntry>,
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

### Artifact & Output Format

```rust
pub struct Artifact { pub bytes: Vec<u8>, pub output_format: OutputFormat }
```

```rust
pub struct RenderResult {
    pub artifacts: Vec<Artifact>,
    pub warnings: Vec<Diagnostic>,
}
```

---

## End-to-End Orchestration Workflow

**Public usage (high-level Quillmark API):**

```rust
// Create engine with auto-registered backends (typst by default)
let mut engine = Quillmark::new();

// Register quills
let quill = Quill::from_path("path/to/quill")?;
engine.register_quill(quill);

// Load workflow and render
let workflow = engine.load("my-quill")?;
let result = workflow.render("# Hello", Some(OutputFormat::Pdf))?;
for a in result.artifacts { /* write bytes */ }
```

**Public usage (direct Workflow API):**

```rust
let backend = Box::new(TypstBackend::default());
let quill = Quill::from_path("path/to/quill")?;
let workflow = Workflow::new(backend, quill)?;
let result = workflow.render(markdown, Some(OutputFormat::Pdf))?;
```

**Internal steps (encapsulated in Workflow::render):**

1. **Parse Markdown**: `decompose(markdown)` → YAML frontmatter + body.
2. **Setup Glue**: `Glue::new(quill.glue_template)`; backend `register_filters(&mut glue)`.
3. **Compose**: `glue.compose(parsed.fields().clone())` → backend-specific glue source.
4. **Compile**: `backend.compile(&glue_src, &quill, &opts)` → `Vec<Artifact>` (PDF/SVG/TXT…).

### Implementation Hints

#### Quillmark Engine Construction

* **Auto-registration**: Backends are registered based on enabled crate features (e.g., `#[cfg(feature = "typst")]`)
* **Backend storage**: Use `HashMap<String, Box<dyn Backend>>` keyed by backend ID
* **Quill storage**: Use `HashMap<String, Quill>` keyed by quill name
* **Clone workaround**: Trait objects can't clone directly - implement `clone_backend()` helper that matches on backend ID

#### Workflow Construction (`Workflow::new`)

* **Input change**: Now takes `Quill` object directly instead of `PathBuf` - quill loading moved to caller
* **Backend validation**: Check `backend.id()` matches expected backend type
* **No additional validation**: `Quill::from_path()` already validates, so no need to validate again

#### Load Method Implementation

* **QuillRef pattern**: Accept `impl Into<QuillRef<'a>>` for ergonomic API
* **Name lookup**: Check `self.quills` HashMap when given a name
* **Object reference**: Use provided Quill directly when given an object
* **Backend resolution**: Extract backend ID from `quill.metadata.get("backend")`
* **Cloning**: Clone both backend and quill for the new Workflow instance

#### Orchestration Error Handling

* **Step isolation**: Each step should handle its own errors and provide context
* **Rollback strategy**: No partial state - either complete success or clean failure
* **Context preservation**: Chain errors with source information through each step

#### Performance Considerations

* **Asset preloading**: Load fonts/assets once per engine instance
* **Memory management**: Use references where possible, avoid unnecessary clones

#### Workflow State Management

```rust
// Typical orchestration pattern (encapsulated in Workflow::render):
let parsed = decompose(markdown)?;                      // Step 1  
let mut glue = Glue::new(&quill.glue_template)?;        // Step 2a
backend.register_filters(&mut glue);                    // Step 2b
let glue_source = glue.compose(parsed.fields().clone())?; // Step 3
let artifacts = backend.compile(&glue_source, &quill, &opts)?; // Step 4
```

#### Render Method Variants

* **render()**: Full pipeline from markdown to artifacts with optional format
* **render_content()**: Skip parsing, compile pre-processed glue content
* **process_glue()**: Extract just the glue composition step (markdown → glue source)

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

  * Empty frontmatter between `---` markers → return just body
  * Missing closing `---` → treat entire content as body
  * YAML parsing failure → graceful degradation, log warning, return entire content as body

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
* **Loading pattern**: 
  * **Asset fonts**: Read font files to `Vec<u8>` and store eagerly in `FontBook` (unchanged behavior)
  * **System fonts**: Use `typst-kit::fonts::FontSearcher` for lazy loading via `FontSlot` - fonts are only loaded into memory when actually accessed through the `font()` method
  * **Memory efficiency**: System fonts are discovered but not pre-loaded, reducing memory footprint

##### Error Formatting (backend-internal)

* Convert Typst `SourceDiagnostic` → core `Diagnostic` (see [Error Handling Patterns](#error-handling-patterns)).

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

  * Required fields: `[package] name, version, entrypoint`
  * Optional: `namespace` (defaults to `"local"`)
  * Use `toml::from_str()` with proper error handling
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

  * Assets: `FileId::new(None, virtual_path)`
  * Packages: `FileId::new(Some(package_spec), virtual_path)`

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
    pub related: Vec<Location>,
    pub hint: Option<String>,
}
```

These types are used **everywhere** (templating, parsing, compilation). `RenderResult.warnings` uses the same `Diagnostic` type.

### Error enums (no premature stringification)

```rust
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("Engine creation failed")] 
    EngineCreation { diag: Diagnostic, #[source] source: Option<anyhow::Error> },

    #[error("Invalid YAML frontmatter")] 
    InvalidFrontmatter { diag: Diagnostic, #[source] source: Option<anyhow::Error> },

    #[error("Template rendering failed")] 
    TemplateFailed { #[source] source: minijinja::Error, diag: Diagnostic },

    #[error("Backend compilation failed with {0} error(s)")]
    CompilationFailed(usize, Vec<Diagnostic>),

    #[error("{format:?} not supported by {backend}")]
    FormatNotSupported { backend: String, format: OutputFormat },

    #[error("Unsupported backend: {0}")]
    UnsupportedBackend(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
```

> **Why this shape?**
>
> * Callers can **enumerate** diagnostics and render UI links.
> * We keep a human message via `Display` but never lose machine data.

### Mapping external errors → `Diagnostic`

**MiniJinja (templating):**

```rust
impl From<minijinja::Error> for RenderError {
    fn from(e: minijinja::Error) -> Self {
        let loc = e
            .name()
            .and_then(|name| e.line().zip(e.column()).map(|(l,c)| Location { file: name.to_string(), line: l as u32, col: c as u32 }))
            .or_else(|| e.line().zip(e.column()).map(|(l,c)| Location { file: "template".into(), line: l as u32, col: c as u32 }));
        let diag = Diagnostic { severity: Severity::Error, code: Some(format!("minijinja::{:?}", e.kind())), message: e.to_string(), primary: loc, related: vec![], hint: None };
        RenderError::TemplateFailed { source: e, diag }
    }
}
```

**Typst (backend):** convert each `SourceDiagnostic` into a `Diagnostic`:

```rust
fn map_typst(errors: &[SourceDiagnostic], world: &QuillWorld) -> Vec<Diagnostic> {
    errors.iter().map(|e| {
        let (file, line, col) = world.resolve_span(&e.span); // backend helper → (String,u32,u32)
        Diagnostic {
            severity: Severity::Error,
            code: e.code.clone(),
            message: e.message.clone(),
            primary: Some(Location { file, line, col }),
            related: e.trace.iter().filter_map(|s| world.resolve_span(s).ok())
                          .map(|(f,l,c)| Location{file:f,line:l,col:c}).collect(),
            hint: e.hints.get(0).cloned(),
        }
    }).collect()
}
```

Then in `compile`:

```rust
let diags = map_typst(&errors, &world);
return Err(RenderError::CompilationFailed(diags.len(), diags));
```

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
}

pub fn print_errors(err: &RenderError) {
    match err {
        RenderError::CompilationFailed(_, diags) => {
            for d in diags { eprintln!("{}", d.fmt_pretty()); }
        }
        RenderError::TemplateFailed { diag, .. } => eprintln!("{}", diag.fmt_pretty()),
        _ => eprintln!("{err}")
    }
}
```

Expose `--json` (or a library method) returning `serde_json::Value` with the full error payload.

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
* **Error context**: Return meaningful diagnostics via `RenderError`

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
* **Error mapping**: Convert backend-native errors to `RenderError` with `Diagnostic`

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