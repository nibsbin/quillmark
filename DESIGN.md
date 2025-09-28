# QuillMark Architecture Design Document

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

QuillMark is a flexible, trait-based markdown rendering library designed to support multiple output backends through a clean abstraction layer. The system transforms markdown documents with YAML frontmatter into various output formats (PDF, SVG, TXT) using pluggable backend implementations.

### High-Level Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Markdown      │    │   Quill          │    │   Output        │
│   Document      │───▶│   Template       │───▶│   Artifacts     │
│   + Frontmatter │    │   + Assets       │    │   (PDF/SVG/TXT) │
└─────────────────┘    └──────────────────┘    └─────────────────┘
        │                        │                        ▲
        ▼                        ▼                        │
┌─────────────────┐    ┌──────────────────┐               │
│   Parsed        │    │   Template       │               │
│   Document      │───▶│   Engine         │───────────────┘
│   (Fields+Body) │    │   (Glue+Filters) │
└─────────────────┘    └──────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │   Backend        │
                       │   Implementation │
                       │   (Typst/etc.)   │
                       └──────────────────┘
```

The system operates through clear separation of concerns:
- **Parsing**: Decomposes markdown into structured fields and body content
- **Templating**: Merges parsed data with quill templates using filters
- **Backend Processing**: Compiles template output into final artifacts
- **Asset Management**: Handles fonts, images, and package dependencies

## Core Design Principles

### 1. **Explicit Backend Selection**
Backends are provided directly in `RenderConfig` rather than through global registration, making backend selection explicit and avoiding global mutable state.

### 2. **Trait-Based Extensibility** 
The `Backend` trait provides a clean interface for implementing new output formats while maintaining type safety and consistent behavior.

### 3. **Template-Driven Approach**
Quill templates define the structure and styling of documents, with markdown content injected through a powerful filter system.

### 4. **Zero-Copy Where Possible**
String processing and template rendering minimize allocations through careful use of references and `Cow<'static, str>` types.

### 5. **Error Transparency**
Error handling preserves context and provides actionable diagnostic information, especially for template and compilation errors.

### 6. **Dynamic Resource Loading**
Package and asset management supports dynamic discovery and loading without hardcoded dependencies.

## Crate Structure and Responsibilities

### `quillmark-core`
**Core abstractions and shared functionality**

**Responsibilities:**
- Defines fundamental types (`Backend`, `Artifact`, `RenderConfig`, `OutputFormat`)
- Provides markdown parsing and frontmatter extraction (`decompose`, `ParsedDocument`)
- Implements template engine abstraction (`Glue`, filter system)
- Quill template management (`Quill` structure)
- Error type definitions (`RenderError`, `TemplateError`)
- Test utilities and context helpers

**Key Design Decision**: No external backend dependencies - maintains clean separation allowing backend crates to depend only on core without circular dependencies.

### `quillmark`
**Main API and orchestration layer**

**Responsibilities:**
- Re-exports all core types for backward compatibility
- Provides primary `render()` function that orchestrates the entire pipeline
- Handles quill template loading from filesystem paths
- Coordinates between parsing, templating, and backend compilation
- Manages configuration validation and error propagation

**Key Design Decision**: Thin orchestration layer that delegates specialized work to appropriate components while providing a unified API.

### `quillmark-typst`
**Typst backend implementation**

**Responsibilities:**
- Implements `Backend` trait for Typst output (PDF, SVG)
- Provides markdown-to-Typst conversion (`mark_to_typst`)
- Implements Typst-specific filters (String, Lines, Date, Dict, Body)
- Manages Typst compilation environment (`QuillWorld`)
- Dynamic package loading with `typst.toml` support
- Font and asset management for Typst documents
- Error formatting with source location information

**Key Design Decision**: Self-contained implementation with sophisticated package management system that replaces previous hardcoded approaches.

## Core Interfaces and Structures

### Backend Trait
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
    fn compile(&self, glue_content: &str, quill: &Quill, opts: &RenderConfig) 
        -> Result<Vec<Artifact>, RenderError>;
}
```

**Design Notes:**
- Thread-safe (`Send + Sync`) for concurrent rendering
- Multiple artifact support for multi-page documents
- Backend-specific filter registration enables specialized template functions
- Glue type determines template file extension (`.typ`, `.tex`, etc.)

### Quill Template Structure
```rust
pub struct Quill {
    /// The template content 
    pub template_content: String,
    /// Quill-specific data that backends might need
    pub metadata: HashMap<String, serde_yaml::Value>,
    /// Base path for resolving relative paths
    pub base_path: PathBuf,
    /// Name of the quill (derived from directory name)
    pub name: String,
    /// Glue template file name
    pub glue_file: String,
}
```

**Key Methods:**
- `glue_path()`, `assets_path()`, `packages_path()`: Directory navigation
- `validate()`: Ensures template integrity
- `from_path()`: Factory for filesystem-based quills

**Design Notes:**
- Self-contained template representation with metadata
- Flexible asset and package directory structure
- Backend-agnostic design supports multiple template formats

### ParsedDocument
```rust
pub struct ParsedDocument {
    /// Dictionary containing frontmatter fields and the body field
    pub fields: HashMap<String, serde_yaml::Value>,
}
```

**Key Methods:**
- `body()`: Access to markdown body content (stored under `BODY_FIELD`)
- `get_field()`: Access to specific frontmatter fields
- `fields()`: Complete field map for template context

**Design Notes:**
- Unified representation of frontmatter and body content
- YAML-based frontmatter with flexible value types
- Reserved `BODY_FIELD` constant for body content access

### Glue Template Engine
```rust
pub struct Glue {
    env: Environment<'static>,
    template: String,
}
```

**Key Methods:**
- `new()`: Create from template string
- `register_filter()`: Add backend-specific filters
- `compose()`: Render template with context data

**Filter API:**
- Thread-safe filter functions: `Fn(&State, Value, Kwargs) -> Result<Value, Error>`
- Stable ABI through `filter_api` module - external crates don't need minijinja dependency
- Type-safe parameter passing with validation

### Artifact and Configuration
```rust
pub struct Artifact {
    pub bytes: Vec<u8>,
    pub output_format: OutputFormat,
}

pub struct RenderConfig {
    pub backend: Box<dyn Backend>,
    pub output_format: Option<OutputFormat>,
    pub quill_path: PathBuf,
}
```

**Design Notes:**
- Binary artifact representation supports any output format
- Configuration bundles backend with options for atomic operation
- Optional format specification allows backend to choose default

## End-to-End Orchestration Workflow

The `render()` function orchestrates the complete transformation pipeline:

### 1. **Configuration Validation**
```rust
pub fn render(markdown: &str, config: &RenderConfig) -> RenderResult {
    let backend = &config.backend;
    // Backend is provided directly in RenderConfig
```

### 2. **Quill Template Loading**
```rust
    let quill_data = load_quill_data(config, backend.glue_type())?;
    // Loads template based on backend's glue_type (.typ, .tex, etc.)
```

**Template Discovery Process:**
- Locate `glue{backend.glue_type()}` file in quill directory
- Load template content and extract metadata
- Validate template structure and required files

### 3. **Markdown Parsing and Decomposition**
```rust
    let parsed_doc = quillmark_core::decompose(markdown)
        .map_err(|e| RenderError::Other(format!("Failed to parse markdown: {}", e).into()))?;
```

**Parsing Process:**
- Detect and extract YAML frontmatter (delimited by `---`)
- Parse YAML into structured values (`serde_yaml::Value`)
- Store body content under reserved `BODY_FIELD` key
- Handle parsing errors gracefully (fallback to treating entire content as body)

### 4. **Template Engine Setup**
```rust
    let mut glue = Glue::new(quill_data.template_content.clone());
    backend.register_filters(&mut glue);
```

**Filter Registration:**
- Each backend registers its specialized filters
- Filters handle type conversion and format-specific transformations
- Thread-safe registration through stable API

### 5. **Template Rendering**
```rust
    let glue_content = glue.compose(parsed_doc.fields().clone())
        .map_err(|e| RenderError::Other(Box::new(e)))?;
```

**Composition Process:**
- Merge frontmatter fields with template variables
- Apply filters to transform data for target format
- Generate intermediate template content (e.g., Typst source)

### 6. **Backend Compilation**
```rust
    backend.compile(&glue_content, &quill_data, config)
```

**Compilation Process:**
- Backend-specific processing of template output
- Asset and package resolution
- Final artifact generation (PDF, SVG, etc.)

## Template System Design

### Template Engine Architecture

The template system is built on MiniJinja with a stable filter API that allows backend crates to avoid direct MiniJinja dependencies:

```rust
pub mod filter_api {
    pub use minijinja::{Error, ErrorKind, State};
    pub use minijinja::value::{Kwargs, Value};
    
    pub trait DynFilter: Send + Sync + 'static {}
    impl<T> DynFilter for T where T: Send + Sync + 'static {}
}
```

### Filter System Design

Filters provide the bridge between parsed markdown data and backend-specific template formats:

#### **String Filter**
- Handles string escaping for safe template injection
- Supports default values through `kwargs["default"]`
- Special handling for `"none"` sentinel value (outputs unquoted `none`)

#### **Lines Filter** 
- Converts arrays to JSON strings for multi-line data
- Escaped for safe embedding in template source
- Used for lists like recipients, references, etc.

#### **Date Filter**
- Strict TOML datetime parsing for consistent date handling
- Generates TOML-wrapped date objects for template use
- Error reporting for invalid date formats

#### **Dict Filter**
- JSON serialization of object data
- Type validation ensures input is object/dictionary
- Escaped JSON string output for template embedding

#### **Body Filter**
- Converts markdown body to target format (e.g., Typst markup)
- Uses format-specific conversion (`mark_to_typst`)
- Generates `eval()` statements for dynamic content injection

### Template Variable Injection

Template variables use double-brace syntax with filter application:
```typst
// Basic string injection
{{ title | String(default="Untitled Document") }}

// Array/list injection  
{{ recipients | Lines }}

// Date injection
{{ date | Date }}

// Markdown body injection
{{ body | Body }}
```

## Parsing and Document Decomposition

### YAML Frontmatter Processing

The `decompose()` function handles markdown documents with optional YAML frontmatter:

#### **Detection Logic**
```rust
if markdown.starts_with("---\n") || markdown.starts_with("---\r\n") {
    // Process frontmatter
}
```

#### **Extraction Process**
1. **Delimiter Scanning**: Find closing `---` marker
2. **Content Separation**: Split frontmatter from body content  
3. **YAML Parsing**: Parse frontmatter into `HashMap<String, serde_yaml::Value>`
4. **Body Storage**: Store body content under `BODY_FIELD` key
5. **Error Handling**: Fall back to treating entire content as body on parse errors

#### **ParsedDocument Structure**
- Unified field map containing both frontmatter and body
- Type-safe access methods for common operations
- Flexible value types supporting strings, numbers, arrays, objects

### Error Handling in Parsing

- **Graceful Degradation**: Invalid YAML frontmatter doesn't prevent processing
- **Warning Output**: Parse errors logged but don't halt pipeline  
- **Content Preservation**: Original content always accessible even with parse failures

## Backend Architecture

### Typst Backend Implementation

The Typst backend demonstrates the full power of the backend architecture:

#### **Format Support**
- **PDF Generation**: Using `typst-pdf` crate with configurable options
- **SVG Generation**: Multi-page SVG output using `typst-svg`
- **Format Validation**: Runtime checks for supported format requests

#### **Markdown to Typst Conversion**
The `mark_to_typst()` function converts markdown to Typst markup:

**Supported Elements:**
- **Text Formatting**: Bold (`*bold*`), italic (`_italic_`), strikethrough (`#strike[text]`)
- **Links**: `#link("url")[text]` format
- **Lists**: Bullet (`+`) and numbered (`1.`) with proper nesting
- **Code**: Inline code with backtick preservation
- **Line Breaks**: Soft breaks (space) and hard breaks (newline)

**Character Escaping:**
- Typst-specific characters escaped: `* _ ` # [ ] $ < > @`
- String content properly quoted and escaped for safe injection

#### **Filter Implementation**
Typst-specific filters handle data transformation:

- **String Escaping**: Safe injection into Typst source with quote wrapping
- **JSON Embedding**: Arrays and objects serialized and escaped for `json()` function
- **Date Handling**: TOML datetime format for Typst's `toml()` function  
- **Markup Generation**: Markdown-to-Typst conversion with `eval()` mode

### QuillWorld: Typst Compilation Environment

The `QuillWorld` struct implements Typst's `World` trait for resource management:

#### **Dynamic Package Loading**
- **Automatic Discovery**: Scans `packages/` directory for package folders
- **typst.toml Support**: Reads package metadata, namespace, version, entrypoint
- **Virtual Path Management**: Preserves directory structure in Typst's virtual filesystem
- **Namespace Handling**: Supports `@preview`, `@local`, and custom namespaces

#### **Asset Management** 
- **Font Loading**: Automatic font discovery in `assets/` directory
- **Binary Assets**: Images, data files loaded with proper virtual paths
- **Recursive Directory Support**: Maintains folder structure for organized assets

#### **Error Handling Improvements**
- **Source Location**: Line and column information for compilation errors
- **Context Preservation**: Original source code shown with error messages
- **Multi-Error Reporting**: All compilation errors presented with clear formatting

## Package Management and Asset Handling

### Quill Directory Structure
```
quill-template/
├── glue.typ           # Main template file  
├── packages/          # Typst packages
│   ├── package1/
│   │   ├── typst.toml # Package metadata
│   │   ├── src/
│   │   │   └── lib.typ # Package entry point
│   │   └── ...
│   └── package2/
└── assets/            # Static assets
    ├── fonts/         # Font files
    ├── images/        # Images  
    └── ...
```

### Package Loading Algorithm

1. **Directory Scanning**: Recursively scan `packages/` directory
2. **Metadata Parsing**: Read `typst.toml` for package configuration
3. **Virtual Path Construction**: Map filesystem paths to Typst virtual paths
4. **Entrypoint Resolution**: Locate main package file based on metadata
5. **Namespace Registration**: Register package with appropriate namespace
6. **File Loading**: Load all package files while preserving directory structure

### Asset Loading Strategy

- **Font Discovery**: Support `.ttf`, `.otf`, `.woff`, `.woff2` formats
- **Binary Assets**: Load images and data files as `Bytes`
- **Virtual Path Mapping**: Maintain `assets/` prefix in virtual filesystem
- **Recursive Loading**: Support nested directory structures
- **Format Detection**: Automatic file type detection based on extensions

### Error Handling in Resource Loading

- **Missing Package Errors**: Clear messaging for package not found
- **Parse Errors**: Helpful messages for malformed `typst.toml` files  
- **Font Loading Failures**: Graceful handling of corrupt font files
- **Asset Loading**: Continue processing even if some assets fail to load

## Error Handling Patterns

### Error Type Hierarchy

```rust
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("{0:?} backend is not built in this binary")]
    UnsupportedBackend(String),
    
    #[error("{format:?} not supported by {backend:?}")]
    FormatNotSupported { backend: String, format: OutputFormat },
    
    #[error("multiple backends can produce {0:?}; specify one explicitly")]
    AmbiguousBackend(OutputFormat),
    
    #[error(transparent)]
    Other(#[from] Box<dyn Error + Send + Sync>),
}
```

### Template Error Handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("{0}")]
    RenderError(#[from] minijinja::Error),
    
    #[error("{0}")]
    InvalidTemplate(String, #[source] Box<dyn StdError + Send + Sync>),
    
    #[error("{0}")]
    FilterError(String),
}
```

### Error Context Preservation

- **Source Chaining**: `#[source]` attributes preserve error causality
- **Location Information**: Line/column data for template and compilation errors
- **Diagnostic Formatting**: Human-readable error messages with context
- **Graceful Degradation**: Partial functionality when some components fail

### Backend Error Handling

- **Compilation Errors**: Detailed Typst error messages with source location
- **Resource Errors**: Clear messages for missing fonts, packages, assets
- **Format Errors**: Validation of backend format support before processing
- **Recovery Strategies**: Fallback behaviors when possible

## Extension Points

### Adding New Backends

To implement a new backend:

1. **Implement Backend Trait**:
```rust
pub struct MyBackend;

impl Backend for MyBackend {
    fn id(&self) -> &'static str { "my-backend" }
    fn supported_formats(&self) -> &'static [OutputFormat] { &[OutputFormat::Pdf] }
    fn glue_type(&self) -> &'static str { ".my" }
    fn register_filters(&self, glue: &mut Glue) { /* Register filters */ }
    fn compile(&self, content: &str, quill: &Quill, opts: &RenderConfig) -> Result<Vec<Artifact>, RenderError> { /* Implementation */ }
}
```

2. **Implement Format-Specific Filters**: Create filters that transform data for your backend's template format

3. **Handle Compilation**: Process template output into final artifacts

4. **Resource Management**: Implement any backend-specific asset or package loading

### Custom Filters

Backends can register custom filters:

```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error};

fn my_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    // Transform value for backend-specific needs
    Ok(Value::from(transformed_content))
}

impl Backend for MyBackend {
    fn register_filters(&self, glue: &mut Glue) {
        glue.register_filter("my_filter", my_filter);
    }
}
```

### Template Extensions

- **Custom Template Syntax**: Backends can define their own template languages
- **Specialized Filters**: Domain-specific data transformations
- **Asset Processing**: Backend-specific asset handling and optimization
- **Output Customization**: Multiple artifact generation from single template

## Key Design Decisions

### 1. **Explicit Backend Provisioning vs. Global Registry**

**Decision**: Backends provided directly in `RenderConfig`

**Rationale**:
- Eliminates global mutable state
- Makes backend selection explicit and deterministic  
- Improves testability and concurrent safety
- Avoids hidden dependencies and initialization order issues

**Trade-offs**:
- Slightly more verbose API
- Backend instances must be managed by calling code

### 2. **Template-First Architecture**

**Decision**: Quill templates define document structure, markdown provides content

**Rationale**:
- Separates content from presentation 
- Enables sophisticated document formatting and layout
- Supports backend-specific template features
- Allows template reuse across different content

**Trade-offs**:
- Requires template creation for complex documents
- Learning curve for template syntax

### 3. **Dynamic Package Loading**

**Decision**: Runtime package discovery vs. compile-time linking

**Rationale**:
- Flexibility for user-provided packages
- No need to rebuild for new packages
- Supports multiple package versions
- Cleaner separation of core from packages

**Trade-offs**:
- Runtime overhead for package discovery
- More complex error handling
- Potential for version conflicts

### 4. **Filter-Based Data Transformation**

**Decision**: Backend-specific filters vs. universal data format

**Rationale**:
- Allows format-specific optimizations
- Enables rich data type support
- Maintains type safety through template system
- Supports domain-specific transformations

**Trade-offs**:
- Backends must implement their own filters
- Potential for inconsistent data handling

### 5. **Unified Error Hierarchy**

**Decision**: Single error type with transparent wrapping vs. error enums per component

**Rationale**:
- Simplified error handling for users
- Context preservation through error chaining
- Flexibility for new error types
- Consistent error reporting across backends

**Trade-offs**:
- Less specific error matching capability
- Potential for large error type variants

### 6. **Thread-Safe Design**

**Decision**: `Send + Sync` requirements throughout

**Rationale**:
- Enables concurrent document processing
- Future-proofs for parallel rendering
- Supports multi-threaded server environments
- Clean architecture with immutable data where possible

**Trade-offs**:
- More restrictive trait bounds
- Additional complexity for shared resources

---

This design document captures the current architecture of QuillMark as a flexible, extensible markdown rendering system. The separation of concerns between parsing, templating, and backend processing provides clean abstraction boundaries while supporting sophisticated document generation workflows.