# Quillmark Core API Documentation

This document provides comprehensive API documentation for the `quillmark-core` crate, which provides foundational types and functionality for the Quillmark template-first Markdown rendering system.

**See also:**
- [DESIGN.md](../DESIGN.md) - High-level architecture and design principles
- [PARSE.md](PARSE.md) - Detailed parsing and Extended YAML Metadata Standard documentation

## Table of Contents

1. [Overview](#overview)
2. [Core Types](#core-types)
   - [OutputFormat](#outputformat)
   - [Artifact](#artifact)
   - [RenderOptions](#renderoptions)
   - [FileEntry](#fileentry)
3. [Parsing](#parsing)
   - [ParsedDocument](#parseddocument)
   - [decompose Function](#decompose-function)
   - [BODY_FIELD Constant](#body_field-constant)
4. [Template System](#template-system)
   - [Quill](#quill)
   - [QuillIgnore](#quillignore)
   - [Glue](#glue)
   - [TemplateError](#templateerror)
   - [Filter API](#filter-api)
5. [Backend Trait](#backend-trait)
6. [Error Handling](#error-handling)
   - [RenderError](#rendererror)
   - [RenderResult](#renderresult)
   - [Diagnostic](#diagnostic)
   - [Location](#location)
   - [Severity](#severity)
7. [Usage Examples](#usage-examples)

---

## Overview

`quillmark-core` is the foundation layer of Quillmark that provides:

- **Parsing**: YAML frontmatter extraction and Extended YAML Metadata Standard support
- **Templating**: MiniJinja-based template composition with stable filter API
- **Template model**: `Quill` type for managing template bundles with in-memory file system
- **Backend trait**: Extensible interface for implementing output format backends
- **Error handling**: Structured diagnostics with source location tracking
- **Utilities**: TOML⇄YAML conversion helpers

**Key design principles:**
- No external backend dependencies (backends depend on core, not vice versa)
- Stable public API for filter registration
- Structured error propagation with actionable diagnostics
- Zero-copy operations where practical

---

## Core Types

### OutputFormat

Enumeration of supported output formats.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum OutputFormat {
    Txt,
    Svg,
    Pdf,
}
```

**Usage:**
```rust
use quillmark_core::OutputFormat;

let format = OutputFormat::Pdf;
match format {
    OutputFormat::Pdf => println!("PDF output"),
    OutputFormat::Svg => println!("SVG output"),
    OutputFormat::Txt => println!("Text output"),
}
```

### Artifact

Represents a rendered output artifact with its binary content and format.

```rust
#[derive(Debug)]
pub struct Artifact {
    pub bytes: Vec<u8>,
    pub output_format: OutputFormat,
}
```

**Fields:**
- `bytes` - The binary content of the artifact
- `output_format` - The format of the output (PDF, SVG, or TXT)

**Usage:**
```rust
use quillmark_core::{Artifact, OutputFormat};

let artifact = Artifact {
    bytes: vec![0x50, 0x44, 0x46], // PDF header
    output_format: OutputFormat::Pdf,
};

// Save to file
std::fs::write("output.pdf", &artifact.bytes)?;
```

### RenderOptions

Internal rendering options used by engine orchestration.

```rust
#[derive(Debug)]
pub struct RenderOptions {
    pub output_format: Option<OutputFormat>,
}
```

**Fields:**
- `output_format` - Optional output format specification

**Note:** This type is primarily used internally by the engine. Most users interact with higher-level APIs in the `quillmark` crate.

### FileEntry

Represents a file in the in-memory file system maintained by a `Quill`.

```rust
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub contents: Vec<u8>,
    pub path: PathBuf,
    pub is_dir: bool,
}
```

**Fields:**
- `contents` - The file contents as bytes (empty for directories)
- `path` - File path relative to the quill root
- `is_dir` - Whether this entry represents a directory

---

## Parsing

### ParsedDocument

Container for parsed markdown document with frontmatter fields and body content.

```rust
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    // fields is private - access via methods
}
```

#### Methods

##### `new`
```rust
pub fn new(fields: HashMap<String, serde_yaml::Value>) -> Self
```

Create a new ParsedDocument with the given fields.

**Parameters:**
- `fields` - HashMap containing frontmatter fields and body (under `BODY_FIELD`)

**Example:**
```rust
use quillmark_core::{ParsedDocument, BODY_FIELD};
use std::collections::HashMap;

let mut fields = HashMap::new();
fields.insert("title".to_string(), serde_yaml::Value::String("My Document".into()));
fields.insert(BODY_FIELD.to_string(), serde_yaml::Value::String("# Content".into()));

let doc = ParsedDocument::new(fields);
```

##### `body`
```rust
pub fn body(&self) -> Option<&str>
```

Get the document body content.

**Returns:** `Option<&str>` - The body content if present

**Example:**
```rust
let markdown = r#"---
title: Test
---

# Hello World
"#;

let doc = decompose(markdown)?;
let body = doc.body().unwrap();
assert!(body.contains("# Hello World"));
```

##### `get_field`
```rust
pub fn get_field(&self, name: &str) -> Option<&serde_yaml::Value>
```

Get a specific frontmatter field by name.

**Parameters:**
- `name` - Field name to retrieve

**Returns:** `Option<&serde_yaml::Value>` - The field value if present

**Example:**
```rust
let doc = decompose(markdown)?;
if let Some(title) = doc.get_field("title") {
    println!("Title: {}", title.as_str().unwrap_or(""));
}
```

##### `fields`
```rust
pub fn fields(&self) -> &HashMap<String, serde_yaml::Value>
```

Get reference to all fields (including body).

**Returns:** `&HashMap<String, serde_yaml::Value>` - All fields

**Example:**
```rust
let doc = decompose(markdown)?;
for (key, value) in doc.fields() {
    println!("{}: {:?}", key, value);
}
```

### decompose Function

Parse markdown with YAML frontmatter into a structured document.

```rust
pub fn decompose(
    markdown: &str,
) -> Result<ParsedDocument, Box<dyn std::error::Error + Send + Sync>>
```

**Parameters:**
- `markdown` - Markdown content with optional YAML frontmatter

**Returns:** `Result<ParsedDocument, Box<dyn Error>>` - Parsed document or error

**Behavior:**
- Supports standard YAML frontmatter (delimited by `---`)
- Supports Extended YAML Metadata Standard with tag directives (e.g., `!items`)
- Handles both Unix (`\n`) and Windows (`\r\n`) line endings
- Distinguishes metadata blocks from horizontal rules via contiguity rules
- Fail-fast on malformed YAML to prevent silent data corruption

**Example:**
```rust
use quillmark_core::{decompose, BODY_FIELD};

// Simple frontmatter
let markdown = r#"---
title: My Document
author: John Doe
date: 2024-01-01
---

# Introduction

Document content here.
"#;

let doc = decompose(markdown)?;
assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "My Document");
assert_eq!(doc.get_field("author").unwrap().as_str().unwrap(), "John Doe");
assert!(doc.body().unwrap().contains("# Introduction"));

// Extended metadata with tagged blocks
let markdown_extended = r#"---
title: Product Catalog
---

Catalog description.

---
!products
name: Widget
price: 19.99
---

Widget description.

---
!products
name: Gadget
price: 29.99
---

Gadget description.
"#;

let doc = decompose(markdown_extended)?;
if let Some(products) = doc.get_field("products").and_then(|v| v.as_sequence()) {
    assert_eq!(products.len(), 2);
    for product in products {
        let name = product.get("name").unwrap().as_str().unwrap();
        let price = product.get("price").unwrap().as_f64().unwrap();
        let body = product.get("body").unwrap().as_str().unwrap();
        println!("{}: ${} - {}", name, price, body);
    }
}
```

**Error cases:**
- Invalid YAML syntax → `InvalidFrontmatter`
- Unclosed frontmatter (missing closing `---`) → Error
- Multiple global frontmatter blocks → Error
- Invalid tag name syntax → Error
- Reserved field name used as tag → Error
- Name collision between global field and tagged attribute → Error

**See [PARSE.md](PARSE.md) for comprehensive documentation of the Extended YAML Metadata Standard.**

### BODY_FIELD Constant

The field name used to store document body content.

```rust
pub const BODY_FIELD: &str = "body";
```

**Usage:**
```rust
use quillmark_core::BODY_FIELD;

let mut fields = HashMap::new();
fields.insert(BODY_FIELD.to_string(), serde_yaml::Value::String("Content".into()));
```

---

## Template System

### Quill

A template bundle containing template content, metadata, and an in-memory file system.

```rust
#[derive(Debug, Clone)]
pub struct Quill {
    pub glue_template: String,
    pub metadata: HashMap<String, serde_yaml::Value>,
    pub base_path: PathBuf,
    pub name: String,
    pub glue_file: String,
    pub files: HashMap<PathBuf, FileEntry>,
}
```

**Fields:**
- `glue_template` - The template content (glue file)
- `metadata` - Quill-specific metadata from Quill.toml
- `base_path` - Base directory path for resolving relative paths
- `name` - Quill name (from Quill.toml or directory name)
- `glue_file` - Template file name (e.g., "glue.typ")
- `files` - In-memory file system with all quill files

#### Methods

##### `from_path`
```rust
pub fn from_path<P: AsRef<Path>>(
    path: P,
) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
```

Create a Quill from a directory path.

**Parameters:**
- `path` - Path to quill directory containing Quill.toml

**Returns:** `Result<Self, Box<dyn Error>>` - Loaded quill or error

**Behavior:**
- Reads and parses `Quill.toml` (capitalized)
- Loads glue template file
- Recursively loads all files into memory (respects `.quillignore`)
- Converts TOML metadata to YAML values
- Validates quill structure

**Example:**
```rust
use quillmark_core::Quill;

let quill = Quill::from_path("path/to/quill")?;
println!("Loaded quill: {}", quill.name);
println!("Template length: {}", quill.glue_template.len());
```

**Directory structure:**
```
my-quill/
├── Quill.toml
├── glue.typ
├── assets/
│   ├── logo.png
│   └── style.css
└── packages/
    └── custom.typ
```

**Quill.toml format:**
```toml
[Quill]
name = "my-quill"
backend = "typst"
glue = "glue.typ"

[typst]
packages = ["@preview/cetz:0.2.0"]
```

##### `validate`
```rust
pub fn validate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Validate the quill structure.

**Returns:** `Result<(), Box<dyn Error>>` - Ok if valid

**Checks:**
- Glue file exists in memory

**Example:**
```rust
quill.validate()?;
```

##### `toml_to_yaml_value`
```rust
pub fn toml_to_yaml_value(
    toml_val: &toml::Value,
) -> Result<serde_yaml::Value, Box<dyn std::error::Error + Send + Sync>>
```

Convert TOML value to YAML value (via JSON).

**Parameters:**
- `toml_val` - TOML value to convert

**Returns:** `Result<serde_yaml::Value, Box<dyn Error>>` - Converted YAML value

**Example:**
```rust
use quillmark_core::Quill;

let toml_val = toml::Value::String("example".to_string());
let yaml_val = Quill::toml_to_yaml_value(&toml_val)?;
```

##### `assets_path`
```rust
pub fn assets_path(&self) -> PathBuf
```

Get the path to the assets directory.

**Returns:** `PathBuf` - Path to assets directory

**Example:**
```rust
let assets = quill.assets_path();
// Returns: base_path/assets
```

##### `packages_path`
```rust
pub fn packages_path(&self) -> PathBuf
```

Get the path to the packages directory.

**Returns:** `PathBuf` - Path to packages directory

##### `glue_path`
```rust
pub fn glue_path(&self) -> PathBuf
```

Get the path to the glue file.

**Returns:** `PathBuf` - Path to glue file

##### `typst_packages`
```rust
pub fn typst_packages(&self) -> Vec<String>
```

Get list of Typst packages to download from Quill.toml.

**Returns:** `Vec<String>` - Package specifiers (e.g., "@preview/cetz:0.2.0")

**Example:**
```rust
for package in quill.typst_packages() {
    println!("Package: {}", package);
}
```

##### `get_file`
```rust
pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&[u8]>
```

Get file contents by path (relative to quill root).

**Parameters:**
- `path` - File path relative to quill root

**Returns:** `Option<&[u8]>` - File contents if exists

**Example:**
```rust
if let Some(logo) = quill.get_file("assets/logo.png") {
    std::fs::write("output/logo.png", logo)?;
}
```

##### `get_file_entry`
```rust
pub fn get_file_entry<P: AsRef<Path>>(&self, path: P) -> Option<&FileEntry>
```

Get file entry by path (includes metadata).

**Parameters:**
- `path` - File path relative to quill root

**Returns:** `Option<&FileEntry>` - File entry if exists

##### `file_exists`
```rust
pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool
```

Check if a file exists in memory.

**Parameters:**
- `path` - File path to check

**Returns:** `bool` - True if file exists

##### `list_directory`
```rust
pub fn list_directory<P: AsRef<Path>>(&self, dir_path: P) -> Vec<PathBuf>
```

List all files in a directory (non-recursive).

**Parameters:**
- `dir_path` - Directory path relative to quill root

**Returns:** `Vec<PathBuf>` - Sorted list of file paths

**Example:**
```rust
let assets = quill.list_directory("assets");
for file in assets {
    println!("Asset: {}", file.display());
}
```

##### `list_subdirectories`
```rust
pub fn list_subdirectories<P: AsRef<Path>>(&self, dir_path: P) -> Vec<PathBuf>
```

List all subdirectories in a directory (non-recursive).

**Parameters:**
- `dir_path` - Directory path relative to quill root

**Returns:** `Vec<PathBuf>` - Sorted list of directory paths

##### `find_files`
```rust
pub fn find_files<P: AsRef<Path>>(&self, pattern: P) -> Vec<PathBuf>
```

Get all files matching a pattern (supports simple wildcards).

**Parameters:**
- `pattern` - Pattern with wildcards (`*` and `directory/*`)

**Returns:** `Vec<PathBuf>` - Sorted list of matching file paths

**Supported patterns:**
- `*` - All files
- `*.typ` - Files ending with .typ
- `assets/*` - All files in assets directory
- Exact matches without wildcards

**Example:**
```rust
// Find all .typ files
let typ_files = quill.find_files("*.typ");

// Find all assets
let assets = quill.find_files("assets/*");

// Find specific file
let readme = quill.find_files("README.md");
```

### QuillIgnore

Pattern matcher for .quillignore files (gitignore-style).

```rust
#[derive(Debug, Clone)]
pub struct QuillIgnore {
    // patterns is private
}
```

#### Methods

##### `new`
```rust
pub fn new(patterns: Vec<String>) -> Self
```

Create a new QuillIgnore from pattern strings.

**Parameters:**
- `patterns` - List of ignore patterns

##### `from_content`
```rust
pub fn from_content(content: &str) -> Self
```

Parse .quillignore content into patterns.

**Parameters:**
- `content` - .quillignore file content

**Returns:** `Self` - QuillIgnore instance

**Behavior:**
- Ignores blank lines
- Ignores comment lines (starting with `#`)
- Trims whitespace from patterns

**Example:**
```rust
use quillmark_core::QuillIgnore;

let content = r#"
# Ignore build artifacts
*.tmp
target/
node_modules/
"#;

let ignore = QuillIgnore::from_content(content);
```

##### `is_ignored`
```rust
pub fn is_ignored<P: AsRef<Path>>(&self, path: P) -> bool
```

Check if a path should be ignored.

**Parameters:**
- `path` - Path to check

**Returns:** `bool` - True if path matches any ignore pattern

**Pattern matching:**
- `*.ext` - Files with extension
- `dirname/` - Directory and all contents
- Exact matches

**Example:**
```rust
let ignore = QuillIgnore::new(vec![
    "*.tmp".to_string(),
    "target/".to_string(),
]);

assert!(ignore.is_ignored("test.tmp"));
assert!(ignore.is_ignored("target/debug"));
assert!(!ignore.is_ignored("src/main.rs"));
```

### Glue

Template rendering interface for backends.

```rust
pub struct Glue {
    // fields are private
}
```

#### Methods

##### `new`
```rust
pub fn new(template: String) -> Self
```

Create a new Glue instance with a template string.

**Parameters:**
- `template` - Template content (e.g., Typst markup with MiniJinja variables)

**Example:**
```rust
use quillmark_core::Glue;

let template = r#"
#set document(title: {{ title | String }})

{{ body | Content }}
"#;

let glue = Glue::new(template.to_string());
```

##### `register_filter`
```rust
pub fn register_filter(&mut self, name: &str, func: FilterFn)
```

Register a filter with the template environment.

**Parameters:**
- `name` - Filter name to use in templates
- `func` - Filter function (see [Filter API](#filter-api))

**Example:**
```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error};

fn uppercase_filter(
    _state: &State,
    value: Value,
    _kwargs: Kwargs,
) -> Result<Value, Error> {
    let s = value.as_str().unwrap_or("");
    Ok(Value::from(s.to_uppercase()))
}

let mut glue = Glue::new(template);
glue.register_filter("uppercase", uppercase_filter);
```

##### `compose`
```rust
pub fn compose(
    &mut self,
    context: HashMap<String, serde_yaml::Value>,
) -> Result<String, TemplateError>
```

Compose template with context from markdown decomposition.

**Parameters:**
- `context` - Field map from ParsedDocument

**Returns:** `Result<String, TemplateError>` - Rendered template or error

**Example:**
```rust
use std::collections::HashMap;

let mut context = HashMap::new();
context.insert("title".to_string(), serde_yaml::Value::String("My Doc".into()));
context.insert("body".to_string(), serde_yaml::Value::String("Content".into()));

let output = glue.compose(context)?;
```

### TemplateError

Error types for template rendering.

```rust
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("{0}")]
    RenderError(#[from] minijinja::Error),
    
    #[error("{0}")]
    InvalidTemplate(String, #[source] Box<dyn std::error::Error + Send + Sync>),
    
    #[error("{0}")]
    FilterError(String),
}
```

**Variants:**
- `RenderError` - Template rendering failed (syntax error, undefined variable, etc.)
- `InvalidTemplate` - Template compilation failed
- `FilterError` - Filter execution error

### Filter API

Stable ABI for backend filter registration (no direct minijinja dependency required).

```rust
pub mod filter_api {
    pub use minijinja::value::{Kwargs, Value};
    pub use minijinja::{Error, ErrorKind, State};

    pub trait DynFilter: Send + Sync + 'static {}
    impl<T> DynFilter for T where T: Send + Sync + 'static {}
}
```

**Filter function signature:**
```rust
type FilterFn = fn(
    &filter_api::State,
    filter_api::Value,
    filter_api::Kwargs,
) -> Result<filter_api::Value, minijinja::Error>;
```

**Example filter:**
```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};

fn lines_filter(
    _state: &State,
    value: Value,
    _kwargs: Kwargs,
) -> Result<Value, Error> {
    let text = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "Expected string")
    })?;
    
    let lines: Vec<&str> = text.lines().collect();
    Ok(Value::from(lines))
}
```

**Template usage:**
```
{% for line in body | lines %}
  Process line: {{ line }}
{% endfor %}
```

---

## Backend Trait

Trait for implementing output format backends.

```rust
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn glue_type(&self) -> &'static str;
    fn register_filters(&self, glue: &mut Glue);
    fn compile(
        &self,
        glue_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<Vec<Artifact>, RenderError>;
}
```

### Methods

#### `id`
```rust
fn id(&self) -> &'static str
```

Get the backend identifier (e.g., "typst", "latex").

**Returns:** `&'static str` - Backend identifier

#### `supported_formats`
```rust
fn supported_formats(&self) -> &'static [OutputFormat]
```

Get supported output formats.

**Returns:** `&'static [OutputFormat]` - Slice of supported formats

#### `glue_type`
```rust
fn glue_type(&self) -> &'static str
```

Get the glue file extension (e.g., ".typ", ".tex").

**Returns:** `&'static str` - File extension

#### `register_filters`
```rust
fn register_filters(&self, glue: &mut Glue)
```

Register backend-specific filters with the glue environment.

**Parameters:**
- `glue` - Glue instance to register filters with

**Example implementation:**
```rust
fn register_filters(&self, glue: &mut Glue) {
    glue.register_filter("String", string_filter);
    glue.register_filter("Content", content_filter);
    glue.register_filter("Lines", lines_filter);
}
```

#### `compile`
```rust
fn compile(
    &self,
    glue_content: &str,
    quill: &Quill,
    opts: &RenderOptions,
) -> Result<Vec<Artifact>, RenderError>
```

Compile the glue content into final artifacts.

**Parameters:**
- `glue_content` - Rendered template content (e.g., Typst markup)
- `quill` - Quill bundle with assets and metadata
- `opts` - Rendering options

**Returns:** `Result<Vec<Artifact>, RenderError>` - Generated artifacts or error

**Example implementation structure:**
```rust
fn compile(
    &self,
    glue_content: &str,
    quill: &Quill,
    opts: &RenderOptions,
) -> Result<Vec<Artifact>, RenderError> {
    // 1. Create compilation environment
    // 2. Load assets from quill
    // 3. Compile glue content
    // 4. Handle errors and map to Diagnostics
    // 5. Return artifacts
    
    Ok(vec![Artifact {
        bytes: compiled_pdf,
        output_format: OutputFormat::Pdf,
    }])
}
```

---

## Error Handling

### RenderError

Main error type for rendering operations.

```rust
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("Engine creation failed")]
    EngineCreation {
        diag: Diagnostic,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("Invalid YAML frontmatter")]
    InvalidFrontmatter {
        diag: Diagnostic,
        #[source]
        source: Option<anyhow::Error>,
    },

    #[error("Template rendering failed")]
    TemplateFailed {
        #[source]
        source: minijinja::Error,
        diag: Diagnostic,
    },

    #[error("Backend compilation failed with {0} error(s)")]
    CompilationFailed(usize, Vec<Diagnostic>),

    #[error("{format:?} not supported by {backend}")]
    FormatNotSupported {
        backend: String,
        format: OutputFormat,
    },

    #[error("Unsupported backend: {0}")]
    UnsupportedBackend(String),

    #[error("Dynamic asset collision: {filename}")]
    DynamicAssetCollision {
        filename: String,
        message: String,
    },

    #[error(transparent)]
    Internal(#[from] anyhow::Error),

    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Template error: {0}")]
    Template(#[from] TemplateError),
}
```

**Variants:**
- `EngineCreation` - Failed to create rendering engine
- `InvalidFrontmatter` - Malformed YAML frontmatter
- `TemplateFailed` - Template rendering error
- `CompilationFailed` - Backend compilation errors (contains diagnostics)
- `FormatNotSupported` - Requested format not supported by backend
- `UnsupportedBackend` - Backend not registered
- `DynamicAssetCollision` - Asset filename collision
- `Internal` - Internal error
- `Other` - Other errors
- `Template` - Template error

**Example error handling:**
```rust
use quillmark_core::{RenderError, error::print_errors};

match workflow.render(markdown, None) {
    Ok(result) => {
        // Success - process artifacts
        for artifact in result.artifacts {
            std::fs::write(
                format!("output.{:?}", artifact.output_format),
                &artifact.bytes
            )?;
        }
    }
    Err(e) => {
        // Print structured diagnostics
        print_errors(&e);
        
        // Match specific error types
        match e {
            RenderError::CompilationFailed(count, diags) => {
                eprintln!("Compilation failed with {} errors:", count);
                for diag in diags {
                    eprintln!("{}", diag.fmt_pretty());
                }
            }
            RenderError::InvalidFrontmatter { diag, .. } => {
                eprintln!("Frontmatter error: {}", diag.message);
            }
            _ => eprintln!("Error: {}", e),
        }
    }
}
```

### RenderResult

Result type containing artifacts and warnings.

```rust
#[derive(Debug)]
pub struct RenderResult {
    pub artifacts: Vec<Artifact>,
    pub warnings: Vec<Diagnostic>,
}
```

**Fields:**
- `artifacts` - Generated output artifacts
- `warnings` - Non-fatal diagnostic messages

#### Methods

##### `new`
```rust
pub fn new(artifacts: Vec<Artifact>) -> Self
```

Create a new result with artifacts.

##### `with_warning`
```rust
pub fn with_warning(mut self, warning: Diagnostic) -> Self
```

Add a warning to the result (builder pattern).

**Example:**
```rust
let result = RenderResult::new(artifacts)
    .with_warning(Diagnostic::new(
        Severity::Warning,
        "Deprecated field used".to_string(),
    ));
```

### Diagnostic

Structured diagnostic information with source location.

```rust
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

**Fields:**
- `severity` - Error severity level
- `code` - Optional error code (e.g., "E001", "typst::syntax")
- `message` - Human-readable error message
- `primary` - Primary source location
- `related` - Related source locations (for context)
- `hint` - Optional hint for fixing the error

#### Methods

##### `new`
```rust
pub fn new(severity: Severity, message: String) -> Self
```

Create a new diagnostic.

##### `with_code`
```rust
pub fn with_code(mut self, code: String) -> Self
```

Set the error code (builder pattern).

##### `with_location`
```rust
pub fn with_location(mut self, location: Location) -> Self
```

Set the primary location (builder pattern).

##### `with_related`
```rust
pub fn with_related(mut self, location: Location) -> Self
```

Add a related location (builder pattern).

##### `with_hint`
```rust
pub fn with_hint(mut self, hint: String) -> Self
```

Set a hint (builder pattern).

##### `fmt_pretty`
```rust
pub fn fmt_pretty(&self) -> String
```

Format diagnostic for pretty printing.

**Returns:** `String` - Formatted diagnostic message

**Example:**
```rust
use quillmark_core::{Diagnostic, Location, Severity};

let diag = Diagnostic::new(Severity::Error, "Undefined variable".to_string())
    .with_code("E001".to_string())
    .with_location(Location {
        file: "template.typ".to_string(),
        line: 10,
        col: 5,
    })
    .with_hint("Check variable spelling".to_string());

println!("{}", diag.fmt_pretty());
// Output:
// [ERROR] Undefined variable (E001) at template.typ:10:5
//   hint: Check variable spelling
```

### Location

Location information for diagnostics.

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub col: u32,
}
```

**Fields:**
- `file` - Source file name (e.g., "glue.typ", "input.md")
- `line` - Line number (1-indexed)
- `col` - Column number (1-indexed)

### Severity

Error severity levels.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Severity {
    Error,
    Warning,
    Note,
}
```

**Variants:**
- `Error` - Fatal error that prevents completion
- `Warning` - Non-fatal issue that may need attention
- `Note` - Informational message

---

## Usage Examples

### Complete Rendering Pipeline

```rust
use quillmark_core::{
    decompose, Quill, Glue, Backend, RenderOptions, OutputFormat,
    Artifact, RenderError, BODY_FIELD,
};

// 1. Load the quill template
let quill = Quill::from_path("templates/my-quill")?;

// 2. Parse markdown document
let markdown = r#"---
title: My Document
author: Jane Doe
---

# Introduction

This is the document content.
"#;

let parsed = decompose(markdown)?;

// 3. Setup template with backend filters
let mut glue = Glue::new(quill.glue_template.clone());
backend.register_filters(&mut glue);

// 4. Compose glue source
let glue_source = glue.compose(parsed.fields().clone())?;

// 5. Compile to artifacts
let opts = RenderOptions {
    output_format: Some(OutputFormat::Pdf),
};
let artifacts = backend.compile(&glue_source, &quill, &opts)?;

// 6. Save output
for artifact in artifacts {
    let ext = match artifact.output_format {
        OutputFormat::Pdf => "pdf",
        OutputFormat::Svg => "svg",
        OutputFormat::Txt => "txt",
    };
    std::fs::write(format!("output.{}", ext), &artifact.bytes)?;
}
```

### Working with Extended Metadata

```rust
use quillmark_core::decompose;

let markdown = r#"---
catalog_title: Product Catalog
---

# Products

---
!products
name: Widget
sku: WID-001
price: 19.99
---

A versatile widget for all occasions.

---
!products
name: Gadget
sku: GAD-002
price: 29.99
---

An advanced gadget with extra features.
"#;

let doc = decompose(markdown)?;

// Access global fields
let title = doc.get_field("catalog_title")
    .and_then(|v| v.as_str())
    .unwrap();
println!("Catalog: {}", title);

// Access tagged collections
if let Some(products) = doc.get_field("products")
    .and_then(|v| v.as_sequence())
{
    for product in products {
        let name = product.get("name").and_then(|v| v.as_str()).unwrap();
        let sku = product.get("sku").and_then(|v| v.as_str()).unwrap();
        let price = product.get("price").and_then(|v| v.as_f64()).unwrap();
        let desc = product.get("body").and_then(|v| v.as_str()).unwrap();
        
        println!("{} ({}): ${}", name, sku, price);
        println!("  {}", desc.trim());
    }
}
```

### Custom Filter Implementation

```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};

// Filter to convert markdown to uppercase
fn shout_filter(
    _state: &State,
    value: Value,
    _kwargs: Kwargs,
) -> Result<Value, Error> {
    let text = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "Expected string value")
    })?;
    
    Ok(Value::from(text.to_uppercase()))
}

// Register with glue
let mut glue = Glue::new(template);
glue.register_filter("shout", shout_filter);

// Use in template:
// {{ title | shout }}
```

### File System Operations

```rust
use quillmark_core::Quill;

let quill = Quill::from_path("my-quill")?;

// List assets
let assets = quill.list_directory("assets");
for asset in assets {
    println!("Asset: {}", asset.display());
}

// Find specific files
let images = quill.find_files("assets/*.png");
for image in images {
    if let Some(data) = quill.get_file(&image) {
        std::fs::write(format!("output/{}", image.display()), data)?;
    }
}

// Check file existence
if quill.file_exists("README.md") {
    let content = quill.get_file("README.md").unwrap();
    println!("README: {}", String::from_utf8_lossy(content));
}
```

### Error Handling Best Practices

```rust
use quillmark_core::{RenderError, Diagnostic, Severity, error::print_errors};

fn render_document(markdown: &str) -> Result<(), RenderError> {
    // Parsing errors
    let doc = decompose(markdown).map_err(|e| {
        let diag = Diagnostic::new(Severity::Error, e.to_string())
            .with_code("PARSE_ERROR".to_string());
        RenderError::InvalidFrontmatter {
            diag,
            source: Some(e.into()),
        }
    })?;
    
    // Template errors are automatically converted
    let output = glue.compose(doc.fields().clone())?;
    
    // Compilation errors with diagnostics
    let artifacts = backend.compile(&output, &quill, &opts)?;
    
    Ok(())
}

// Usage
match render_document(markdown) {
    Ok(()) => println!("Success!"),
    Err(e) => {
        print_errors(&e);
        std::process::exit(1);
    }
}
```

---

## Thread Safety

All public types in `quillmark-core` are designed for concurrent use:

- `Backend` trait requires `Send + Sync`
- `Quill` is `Clone` and can be shared across threads
- `ParsedDocument` is `Clone` and thread-safe
- Filter functions must be `Send + Sync + 'static`

**Example:**
```rust
use std::sync::Arc;
use std::thread;

let quill = Arc::new(Quill::from_path("my-quill")?);

let handles: Vec<_> = (0..4)
    .map(|i| {
        let quill = Arc::clone(&quill);
        thread::spawn(move || {
            // Each thread can use the quill independently
            let files = quill.find_files("*.typ");
            println!("Thread {}: found {} files", i, files.len());
        })
    })
    .collect();

for handle in handles {
    handle.join().unwrap();
}
```

---

## Version Compatibility

This documentation is for `quillmark-core` version 0.x. The API is still evolving and may have breaking changes between minor versions until 1.0.

**See also:**
- [Changelog](../CHANGELOG.md) for version history
- [DESIGN.md](../DESIGN.md) for architecture rationale
- [PARSE.md](PARSE.md) for detailed parsing documentation
