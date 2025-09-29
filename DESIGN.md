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

---

## Parsing and Document Decomposition

* **Frontmatter:** YAML delimited by `---` … `---` at the top of the document.
* **Process:**

  1. Detect frontmatter block; parse to `HashMap<String, serde_yaml::Value>`
  2. Store the remainder as body under `BODY_FIELD`
  3. Preserve original content on failures; errors are reported but non-fatal
* **Policy:** YAML-only input; no TOML frontmatter. Backends can convert via filters.

**Error posture:** graceful degradation (invalid frontmatter → treat as body; log/return warnings).

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