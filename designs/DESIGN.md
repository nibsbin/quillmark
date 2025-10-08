# Quillmark Architecture

> This document merges **“Quillmark Architecture Design Document”** and **“Quillmark Improved Architecture Design Document”** into a single, authoritative DESIGN.md. Where the two differed, this doc reconciles them and notes compatibility.

---

**See also:**
- [quillmark-core/PARSE.md](quillmark-core/PARSE.md) - Detailed parsing and Extended YAML Metadata Standard documentation
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
* Parsing: `ParsedDocument` with `from_markdown()` constructor (internal `decompose` function)
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

> **Note:** The following sections provide high-level API signatures and design rationale. For detailed API documentation with comprehensive examples, error handling, and usage patterns, see the `quillmark` crate's rustdoc (available at docs.rs or via `cargo doc --open`).

### Quillmark (high-level engine API)

```rust
pub struct Quillmark {
    backends: HashMap<String, Box<dyn Backend>>,
    quills: HashMap<String, Quill>,
}

impl Quillmark {
    pub fn new() -> Self;
    pub fn register_quill(&mut self, quill: Quill);
    pub fn workflow_from_parsed(&self, parsed: &ParsedDocument) -> Result<Workflow, RenderError>;
    pub fn workflow_from_quill<'a>(&self, quill_ref: impl Into<QuillRef<'a>>) -> Result<Workflow, RenderError>;
    pub fn workflow_from_quill_name(&self, name: &str) -> Result<Workflow, RenderError>;
    pub fn registered_backends(&self) -> Vec<&str>;
    pub fn registered_quills(&self) -> Vec<&str>;
}
```

**Usage pattern:**

```rust
// Create engine with auto-registered backends
let mut engine = Quillmark::new();

// Register quills
let quill = Quill::from_path("path/to/quill")?;
engine.register_quill(quill);

// Parse markdown once
let markdown = "---\ntitle: Example\n---\n\n# Content";
let parsed = ParsedDocument::from_markdown(markdown)?;

// Load workflow by name or from parsed document
let workflow = engine.workflow_from_quill_name("my-quill")?;
let workflow = engine.workflow_from_parsed(&parsed)?;  // Auto-detects from !quill tag
let workflow = engine.workflow_from_quill(&quill)?;    // Also accepts Quill reference
```

### Workflow (render execution API)

```rust
pub struct Workflow {
    backend: Box<dyn Backend>,
    quill: Quill,
    dynamic_assets: HashMap<String, Vec<u8>>,
}

impl Workflow {
    pub fn new(backend: Box<dyn Backend>, quill: Quill) -> Result<Self, RenderError>;
    pub fn render(&self, parsed: &ParsedDocument, format: Option<OutputFormat>) -> Result<RenderResult, RenderError>;
    pub fn render_source(&self, content: &str, format: Option<OutputFormat>) -> Result<RenderResult, RenderError>;
    pub fn process_glue_parsed(&self, parsed: &ParsedDocument) -> Result<String, RenderError>;
    pub fn backend_id(&self) -> &str;
    pub fn supported_formats(&self) -> &'static [OutputFormat];
    pub fn quill_name(&self) -> &str;
    
    // Dynamic asset management
    pub fn with_asset(self, filename: impl Into<String>, contents: impl Into<Vec<u8>>) -> Result<Self, RenderError>;
    pub fn with_assets(self, assets: impl IntoIterator<Item = (String, Vec<u8>)>) -> Result<Self, RenderError>;
    pub fn clear_assets(self) -> Self;
}
```

**Dynamic Assets:** The `Workflow` supports adding runtime assets through a builder pattern. Dynamic assets are prefixed with `DYNAMIC_ASSET__` and stored under `assets/` in the virtual file system, accessible via the `Asset` filter in templates.

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

> **Note:** `RenderOptions` is internal to engine orchestration; public callers use the engine methods.

### Quill (template bundle)

See `designs/QUILL_DESIGN.md` for full design rationale.


### ParsedDocument

```rust
pub struct ParsedDocument {
    fields: HashMap<String, serde_yaml::Value>,  // private - access via methods
}
```

**Public API:** `new(fields)`, `from_markdown(markdown)`, `body()`, `get_field()`, `fields()`, `quill_tag()`; body is stored under reserved `BODY_FIELD` constant. The `from_markdown()` constructor parses markdown and extracts frontmatter and body in one step.

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

// Parse markdown once
let markdown = "# Hello";
let parsed = ParsedDocument::from_markdown(markdown)?;

// Load workflow and render
let workflow = engine.workflow_from_quill_name("my-quill")?;
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;
for a in result.artifacts { /* write bytes */ }
```

**Public usage (direct Workflow API):**

```rust
let backend = Box::new(TypstBackend::default());
let quill = Quill::from_path("path/to/quill")?;
let workflow = Workflow::new(backend, quill)?;

let markdown = "# Hello";
let parsed = ParsedDocument::from_markdown(markdown)?;
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;
```

**Internal steps (encapsulated in Workflow::render):**

1. **Parse Markdown**: Already done by caller via `ParsedDocument::from_markdown(markdown)` → YAML frontmatter + body.
2. **Setup Glue**: `Glue::new(quill.glue_template)`; backend `register_filters(&mut glue)`.
3. **Compose**: `glue.compose(parsed.fields().clone())` → backend-specific glue source.
4. **Compile**: `backend.compile(&glue_src, &quill, &opts)` → `Vec<Artifact>` (PDF/SVG/TXT…).

### Implementation Hints

#### Quillmark Construction

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
// Parsing is done externally by caller via ParsedDocument::from_markdown()
let mut glue = Glue::new(&quill.glue_template)?;        // Step 1a
backend.register_filters(&mut glue);                    // Step 1b
let glue_source = glue.compose(parsed.fields().clone())?; // Step 2
let prepared_quill = self.prepare_quill_with_assets();  // Step 2.5: inject dynamic assets
let artifacts = backend.compile(&glue_source, &prepared_quill, &opts)?; // Step 3
```

**Dynamic Asset Workflow**: Dynamic assets added via `with_asset()` are stored in the `Workflow` and injected into a cloned `Quill` during rendering. The `prepare_quill_with_assets()` method prefixes each filename with `DYNAMIC_ASSET__` and adds it to `assets/` in the quill's virtual file system, ensuring no collisions with static assets.

#### Render Method Variants

* **render()**: Full pipeline from parsed document to artifacts with optional format
* **render_source()**: Skip parsing, compile pre-processed glue content
* **process_glue_parsed()**: Extract just the glue composition step (ParsedDocument → glue source)

---

## Template System Design

* Engine uses **MiniJinja** with a **stable filter API** so backends do not directly depend on MiniJinja.
* Backend-provided filters bridge YAML values → backend-native constructs.

**Common Filters**

* **String**: escape/quote; `default=` kwarg; special handling for `none` sentinel
* **Lines**: string array for multi-line embedding
* **Date**: strict date parsing; produces datetime constructor for Typst
* **Dict**: objects → JSON string; type validation
* **Content**: Markdown body → backend markup (e.g., Typst) and inject with `eval()` as needed
* **Asset**: transform dynamic asset filename to virtual path (e.g., `"chart.png"` → `"assets/DYNAMIC_ASSET__chart.png"`)

**Template usage example (Typst glue):**

```typst
{{ title | String(default="Untitled") }}
{{ recipients | Lines }}
{{ date | Date }}
{{ metadata | Dict }}
{{ body | Content }}
#image({{ "chart.png" | Asset }}) // dynamic asset
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

##### Content Filter (`content_filter`)

* **Markdown conversion**: Apply `mark_to_typst()` to body content
* **Eval wrapping**: Return `eval("typst_markup", mode: "markup")` for safe template injection
* **Content type**: Treats input as markdown string, outputs Typst markup

##### Asset Filter (`asset_filter`)

* **Dynamic asset path**: Transform filename to prefixed path `assets/DYNAMIC_ASSET__{filename}`
* **Security validation**: Reject filenames containing path separators (`/` or `\`)
* **Output format**: Return quoted Typst string literal `"assets/DYNAMIC_ASSET__filename"`
* **Usage**: Enables runtime asset injection via `Workflow.with_asset()` builder pattern

#### Value Conversion Helpers

```rust
// JSON ↔ MiniJinja Value bridge functions
fn v_to_json(v: &Value) -> Json;          // MiniJinja → serde_json
fn json_to_string_lossy(j: &Json) -> String;  // JSON → display string
fn kwargs_default(kwargs: &Kwargs) -> Result<Option<Json>, Error>; // Extract default= param
```

---

## Parsing and Document Decomposition

Quillmark supports advanced markdown parsing with both traditional frontmatter and the **Extended YAML Metadata Standard** for structured content organization.

### Basic Frontmatter Parsing

* **Frontmatter:** YAML delimited by `---` … `---` at the top of the document.
* **Process:**

  1. Detect frontmatter block; parse to `HashMap<String, serde_yaml::Value>`
  2. Store the remainder as body under `BODY_FIELD`
  3. Validate YAML syntax with fail-fast error reporting
  4. Preserve all body whitespace (including leading/trailing)
* **Policy:** YAML-only input; no TOML frontmatter. Backends can convert via filters.

### Extended YAML Metadata Standard (Implemented)

The parser now supports **inline metadata sections** throughout documents using tag directives:

* **Tag Directive Syntax**: Use `!attribute_name` after opening `---` to create collections
* **Collection Aggregation**: Multiple blocks with same tag → array of objects
* **Horizontal Rule Disambiguation**: Smart detection distinguishes metadata blocks from markdown `---` horizontal rules
* **Contiguity Validation**: Metadata blocks must be contiguous (no blank lines within YAML content)
* **Validation**: Tag names match `[a-z_][a-z0-9_]*` pattern; reserved name protection; name collision detection

**Grammar:**
```
metadata_block ::= "---" NEWLINE tag_directive? yaml_content "---" NEWLINE body_content
tag_directive ::= "!" attribute_name NEWLINE
attribute_name ::= [a-z_][a-z0-9_]*
yaml_content ::= (yaml_line NEWLINE)+  // No blank lines allowed
```

**Key Rules:**
* Tag directive MUST appear on first line after opening `---` (if present)
* If no tag directive, block is treated as global frontmatter
* Metadata blocks MUST be contiguous - `---` followed by blank line is treated as horizontal rule
* YAML parsing uses same rigor for both frontmatter and tagged blocks
* Body content preserves all whitespace (including leading/trailing)

**Example:**

```markdown
---
title: Product Catalog
---
Main description.

---
!products
name: Widget
price: 19.99
---
Widget description.
```

Parses to structured data with a `products` array containing objects with metadata fields and body content.

**Collection Semantics:**
* All tagged blocks with the same attribute name → aggregated into array
* Array preserves document order
* Each entry is an object containing metadata fields + `body` field
* Global frontmatter fields stored at top level
* Only one global frontmatter block allowed (subsequent untagged blocks error)

**Error posture:** Fail-fast for malformed YAML to prevent silent data corruption; clear error messages for invalid tag syntax or name collisions.

**See `quillmark-core/PARSE.md` for comprehensive documentation of the Extended YAML Metadata Standard.**

### Implementation Hints

#### Frontmatter Detection (internal implementation)

The internal implementation (accessed via `ParsedDocument::from_markdown()`) uses pattern matching for delimiter detection with full cross-platform support:

* **Line ending support**: Checks both `"---\n"` and `"---\r\n"` for Windows/Unix compatibility
* **Delimiter search**: Finds opening and closing delimiters using pattern matching
* **Horizontal rule disambiguation**: 
  - Opening `---` followed by blank line → treated as horizontal rule
  - Opening `---` followed by content → treated as metadata block
  - Metadata blocks must be contiguous (no blank lines within)
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

* **Fields storage**: Single `HashMap<String, serde_yaml::Value>` for both frontmatter and body
* **Body access**: Special field `BODY_FIELD = "body"` - use constants to avoid typos
* **YAML value types**: Support strings, numbers, arrays, objects via `serde_yaml::Value`

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
├─ Quill.toml              # metadata; can override glue file name
├─ glue.<ext>              # e.g., glue.typ
├─ packages/               # backend packages (embedded in quill)
│  └─ <pkg>/typst.toml …
└─ assets/                 # fonts/, images/, data/
```

**Quill.toml structure:**

```toml
[Quill]
name = "my-quill"
backend = "typst"
glue = "glue.typ"

[typst]
packages = [
    "@preview/bubble:0.2.2",
    "@preview/other-package:1.0.0"
]
```

The `[typst]` section is optional and allows specifying external packages to download:
* **packages**: Array of package specifications in format `@namespace/name:version`
* External packages are downloaded using typst-kit from the Typst package registry
* Downloaded packages **dominate** (override) embedded packages if there's a name collision
* Packages are cached locally for reuse

**Package loading (algorithm):**

1. **Download external packages** specified in `[typst].packages` from Quill.toml
2. Load downloaded packages into the virtual file system
3. Scan `packages/` recursively for embedded packages
4. Parse `typst.toml` metadata for each package
5. Build virtual paths; register namespace (`@preview`, `@local`, custom)
6. Resolve entrypoints; load all package files preserving structure

**Assets:**

* Fonts: `.ttf`, `.otf`, `.woff`, `.woff2`
* Binary assets: images/data as bytes
* Recursive discovery; prefix-preserving virtual paths
* **Dynamic assets**: Runtime-injected via `Workflow.with_asset()`, prefixed with `DYNAMIC_ASSET__` and accessible via `Asset` filter

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
  // Manual path construction for virtual paths (paths are relative):
  let virtual_path = VirtualPath::new(&filename); // Forward slashes required
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
* **Dynamic assets**: Added to quill at runtime via `Workflow.prepare_quill_with_assets()`, prefixed with `DYNAMIC_ASSET__` for collision avoidance with static assets

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

    #[error("Dynamic asset collision: {filename}")]
    DynamicAssetCollision { filename: String, message: String },

    #[error(transparent)]
    Internal(#[from] anyhow::Error),

    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Template error: {0}")]
    Template(#[from] crate::templating::TemplateError),
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
        let loc = e.line().map(|line| Location {
            file: e.name().unwrap_or("template").to_string(),
            line: line as u32,
            col: 0, // MiniJinja doesn't provide column info
        });

        let diag = Diagnostic {
            severity: Severity::Error,
            code: Some(format!("minijinja::{:?}", e.kind())),
            message: e.to_string(),
            primary: loc,
            related: vec![],
            hint: None,
        };

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
        RenderError::InvalidFrontmatter { diag, .. } => eprintln!("{}", diag.fmt_pretty()),
        RenderError::EngineCreation { diag, .. } => eprintln!("{}", diag.fmt_pretty()),
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