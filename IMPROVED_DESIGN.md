# QuillMark Improved Architecture Design Document

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

## System Overview

QuillMark is a highly opinionated, sealed markdown rendering library designed around a single, unified workflow. The system transforms markdown documents with YAML frontmatter into various output formats (PDF, SVG, TXT) through the primary `QuillEngine` API.

The system operates through a single, opinionated workflow:
- **Unified API**: All rendering operations go through the sealed `QuillEngine` struct
- **YAML Frontmatter**: All markdown frontmatter uses YAML format exclusively
- **Template-Driven**: Single Quill template structure drives document generation
- **Backend Abstraction**: Pluggable backends handle format-specific compilation

## Core Design Principles

### 1. **Sealed API Design**
The `QuillEngine` struct provides the single, authoritative API for all rendering operations. No alternative rendering paths exist.

### 2. **Highly Opinionated Structure**
- One rendering workflow through `QuillEngine`
- One Quill package structure (standardized directory layout)
- YAML-only frontmatter parsing
- Single template loading strategy

### 3. **Template-First Architecture**
Quill templates define document structure completely, with markdown content injected through a standardized filter system.

### 4. **Backend Specialization**
While the core API is unified, backends can inject content using their preferred formats (e.g., TOML in `quillmark-typst`).

### 5. **Zero Configuration Philosophy**
The system works with minimal configuration by enforcing sensible defaults and standard directory structures.

## Crate Structure and Responsibilities

### `quillmark-core`
**Core abstractions and shared functionality**

**Responsibilities:**
- Defines fundamental types (`Backend`, `Artifact`, `OutputFormat`)
- Provides markdown parsing and YAML frontmatter extraction (`decompose`, `ParsedDocument`)
- Implements template engine abstraction (`Glue`, filter system)
- Quill template management (`Quill` structure with `quill.toml` support)
- Error type definitions (`RenderError`, `TemplateError`)

**Key Design Decision**: No external backend dependencies - maintains clean separation allowing backend crates to depend only on core without circular dependencies.

### `quillmark`
**Main API and sealed engine**

**Responsibilities:**
- Provides the sealed `QuillEngine` struct as the primary and only API
- Encapsulates `Backend` and `Quill` within the engine
- Handles all rendering orchestration through engine methods
- Manages configuration validation and error propagation

**Key Design Decision**: Sealed design - users interact exclusively through `QuillEngine`. No alternative rendering functions exist.

### `quillmark-typst`
**Typst backend implementation**

**Responsibilities:**
- Implements `Backend` trait for Typst output (PDF, SVG)
- Provides markdown-to-Typst conversion (`mark_to_typst`)
- Implements Typst-specific filters (String, Lines, Date, Dict, Body)
- Manages Typst compilation environment (`QuillWorld`)
- Dynamic package loading with `typst.toml` support
- **Special**: May inject TOML content into templates for Typst-specific features

**Key Design Decision**: While frontmatter is YAML, filters can inject TOML content for Typst's native TOML support.

### `quillmark-fixtures`
**Test fixtures and resource management (dev-only)**

**Responsibilities:**
- Centralized example and test resource management
- Standardized resource path resolution (`resource_path()`)
- Example output directory management (`example_output_dir()`)

## Core Interfaces and Structures

### QuillEngine (Sealed Primary API)
```rust
pub struct QuillEngine {
    backend: Box<dyn Backend>,
    quill: Quill,
}
```

**Key Methods:**
- `new(backend: Box<dyn Backend>, quill_path: PathBuf) -> Result<Self, RenderError>`: Create engine
- `render(&self, markdown: &str) -> RenderResult`: Primary rendering method
- `render_with_format(&self, markdown: &str, format: OutputFormat) -> RenderResult`: Render with specific format
- `backend_id(&self) -> &str`: Get backend identifier
- `supported_formats(&self) -> &[OutputFormat]`: Get supported output formats
- `quill_name(&self) -> &str`: Get loaded quill name

**Design Notes:**
- Sealed struct - all fields are private
- Single construction path through `new()`
- All rendering operations go through this struct
- No global state or alternative APIs

### Backend Trait (Unchanged Interface)
```rust
pub trait Backend: Send + Sync {
    /// Stable identifier (e.g., "typst", "latex", "mock")
    fn id(&self) -> &'static str;
    
    /// Formats this backend supports in *this* build
    fn supported_formats(&self) -> &'static [OutputFormat];
    
    /// File extension for the document type this backend processes
    fn glue_type(&self) -> &'static str;
    
    /// Register filters with the given Glue instance
    fn register_filters(&self, glue: &mut Glue);
    
    /// Compile the rendered glue content into final artifacts
    fn compile(&self, glue_content: &str, quill: &Quill, opts: &RenderOptions) 
        -> Result<Vec<Artifact>, RenderError>;
}
```

**Design Notes:**
- Interface remains the same for backend implementations
- Backends can still inject format-specific content (e.g., TOML in Typst)
- Thread-safe requirements maintained

### Quill Template Structure (Standardized)
```rust
pub struct Quill {
    /// The template content 
    pub template_content: String,
    /// Quill-specific data that backends might need (YAML values)
    pub metadata: HashMap<String, serde_yaml::Value>,
    /// Base path for resolving relative paths
    pub base_path: PathBuf,
    /// Name of the quill (derived from directory name or quill.toml)
    pub name: String,
    /// Glue template file name (configurable via quill.toml)
    pub glue_file: String,
}
```

**Opinionated Quill Structure:**
```
quill-template/
├── quill.toml         # Template metadata (standardized format)
├── glue.typ           # Main template file (extension matches backend)
├── packages/          # Backend-specific packages
│   └── [package-dirs]/
└── assets/            # Static assets (fonts, images)
    ├── fonts/
    └── images/
```

**Key Methods:**
- `from_path()`: Single factory method for filesystem loading
- `validate()`: Ensures template meets requirements
- Directory navigation methods remain unchanged

**Design Notes:**
- Enforces standard directory structure
- `quill.toml` format is standardized and required for production templates
- Flexible but opinionated metadata structure

## End-to-End Orchestration Workflow

The rendering workflow is completely encapsulated within `QuillEngine`:

### 1. **Engine Creation**
```rust
let backend = Box::new(TypstBackend::default());
let engine = QuillEngine::new(backend, quill_path)?;
```

### 2. **Single Rendering Path**
```rust
let artifacts = engine.render(markdown_content)?;
// OR with specific format
let artifacts = engine.render_with_format(markdown_content, OutputFormat::Pdf)?;
```

**Internal Orchestration (Hidden from Users):**
1. **YAML Frontmatter Parsing**: Extract and parse YAML frontmatter exclusively
2. **Template Loading**: Load and validate Quill template
3. **Filter Registration**: Backend registers its specialized filters
4. **Template Rendering**: Generate intermediate content with context
5. **Backend Compilation**: Produce final artifacts

**Design Notes:**
- All complexity hidden behind `QuillEngine`
- No configuration objects or multi-step setup
- Single method call for most use cases

## Template System Design

### YAML-First Frontmatter
All markdown documents use YAML frontmatter exclusively:

```yaml
---
title: "Document Title"
author: "Author Name"
date: "2024-01-01"
tags: ["example", "demo"]
config:
  margin: "1in"
  font_size: 12
---

# Document Content
```

**Design Notes:**
- No TOML frontmatter support
- Standardized field names across all templates
- Rich data type support (strings, numbers, arrays, objects)

### Backend-Specific Template Injection
While frontmatter is YAML, backends can inject content using their preferred formats:

#### **Typst Backend Special Case**
```typst
// Backends can inject TOML for Typst's native support
#let config = toml("margin: 1in\nfont_size: 12pt")

// But frontmatter data comes as YAML
{{ title | String }}
{{ config | Dict }}
```

**Filter System (Unchanged):**
- String, Lines, Date, Dict, Body filters
- Backend-specific transformations
- Thread-safe registration

## Parsing and Document Decomposition

### Exclusive YAML Frontmatter Support

The `decompose()` function handles only YAML frontmatter:

#### **Detection and Parsing**
```rust
if markdown.starts_with("---\n") || markdown.starts_with("---\r\n") {
    // Parse YAML frontmatter only
}
```

**Processing Steps:**
1. **YAML-Only Detection**: Only `---` delimited YAML blocks accepted
2. **Strict Parsing**: Invalid YAML frontmatter causes parsing errors
3. **Type Preservation**: Rich YAML data types preserved as `serde_yaml::Value`
4. **Body Storage**: Markdown body stored under `BODY_FIELD` key

**Design Notes:**
- No fallback to TOML or other formats
- Strict YAML validation
- Consistent error handling

## Backend Architecture

### Typst Backend (Enhanced for TOML Injection)

The Typst backend can inject TOML content while maintaining YAML frontmatter:

#### **Filter Enhancements for TOML**
```rust
// New filter for TOML injection
pub fn toml_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    let yaml_data = v_to_yaml(&value);
    let toml_string = yaml_to_toml_string(yaml_data)?;
    let injector = format!("toml(\"{}\")", escape_string(&toml_string));
    Ok(Value::from(injector))
}
```

#### **Template Usage**
```typst
// Inject YAML frontmatter as TOML for Typst
#let metadata = {{ frontmatter | Toml }}
#let title = {{ title | String }}
#let body_content = {{ body | Body }}
```

**Design Notes:**
- Frontmatter remains YAML in all documents
- Filters can convert YAML to TOML for Typst templates
- Maintains single frontmatter format while supporting backend preferences

## Error Handling Patterns

### Simplified Error Hierarchy

```rust
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("Engine creation failed: {0}")]
    EngineCreation(String),
    
    #[error("Invalid YAML frontmatter: {0}")]
    InvalidFrontmatter(String),
    
    #[error("Quill template error: {0}")]
    QuillError(String),
    
    #[error("Backend compilation failed: {0}")]
    CompilationError(String),
    
    #[error(transparent)]
    Other(#[from] Box<dyn Error + Send + Sync>),
}
```

**Design Notes:**
- Simplified error types for opinionated API
- Clear error categories for common failures
- Preserved error context through chaining

## Extension Points

### Adding New Backends

Backends implement the same `Backend` trait but integrate with `QuillEngine`:

```rust
pub struct MyBackend;

impl Backend for MyBackend {
    fn id(&self) -> &'static str { "my-backend" }
    fn supported_formats(&self) -> &'static [OutputFormat] { &[OutputFormat::Pdf] }
    fn glue_type(&self) -> &'static str { ".my" }
    fn register_filters(&self, glue: &mut Glue) { /* Register filters */ }
    fn compile(&self, content: &str, quill: &Quill, opts: &RenderOptions) -> Result<Vec<Artifact>, RenderError> { /* Implementation */ }
}

// Usage through sealed API
let engine = QuillEngine::new(Box::new(MyBackend), quill_path)?;
```

### Custom Filters

Backends register filters that can handle format conversion:

```rust
fn yaml_to_my_format_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    let yaml_data = v_to_yaml(&value);
    let my_format_string = convert_yaml_to_my_format(yaml_data)?;
    Ok(Value::from(my_format_string))
}
```

## Key Design Decisions

### 1. **Sealed Engine Architecture**

**Decision**: Single `QuillEngine` struct encapsulates all functionality

**Rationale**:
- Eliminates API confusion and alternative paths
- Enforces consistent usage patterns  
- Simplifies documentation and learning curve
- Enables future optimizations within sealed boundary

**Trade-offs**:
- Less flexibility for advanced users
- Cannot mix-and-match components easily

### 2. **YAML-Only Frontmatter**

**Decision**: Exclusive YAML frontmatter support

**Rationale**:
- Reduces parsing complexity and edge cases
- Provides rich data type support
- Industry standard for many markdown tools
- Clear, readable syntax

**Trade-offs**:
- No TOML frontmatter for users who prefer it
- Must convert formats for backend-specific needs

### 3. **Backend Format Injection**

**Decision**: Allow backends to inject content in their preferred formats while maintaining YAML frontmatter

**Rationale**:
- Supports backend-specific optimizations (e.g., Typst's native TOML support)
- Maintains consistent user-facing format (YAML)
- Enables rich backend features without forcing users to learn multiple formats

**Trade-offs**:
- Additional complexity in filter implementations
- Potential for format conversion errors

### 4. **Opinionated Quill Structure**

**Decision**: Standardized directory structure and `quill.toml` format

**Rationale**:
- Reduces setup complexity and documentation burden
- Enables tooling and automation around standard structure
- Clear expectations for template organization
- Facilitates template sharing and reuse

**Trade-offs**:
- Less flexibility for custom organizations
- Migration required for existing non-standard templates

### 5. **Engine-Encapsulated Workflow**

**Decision**: All orchestration logic hidden within `QuillEngine`

**Rationale**:
- Simplifies API surface and reduces misuse
- Enables internal optimizations without breaking changes
- Consistent error handling and logging
- Clear ownership and responsibility boundaries

**Trade-offs**:
- Reduced control for advanced use cases
- Harder to customize individual workflow steps

---

This improved design document captures QuillMark as a highly opinionated, sealed markdown rendering system centered around the `QuillEngine` API. The design prioritizes simplicity, consistency, and ease of use while maintaining powerful backend extensibility through the standardized `Backend` trait interface.

## Migration from Current Design

### For Users
- Replace direct `render()` function calls with `QuillEngine::new()` followed by `engine.render()`
- Ensure all frontmatter uses YAML format
- Organize quill templates using standard directory structure

### For Backend Implementers
- `Backend` trait interface remains unchanged
- Can implement format conversion filters (e.g., YAML to TOML)
- Integration through `QuillEngine` instead of direct rendering functions

### Benefits of Migration
- Simplified API surface reduces learning curve
- Consistent error handling and debugging experience
- Future-proof architecture enables performance optimizations
- Standardized template structure improves template sharing