# Quillmark-Typst API Documentation

This document describes the public API for the `quillmark-typst` crate, which provides the Typst backend for Quillmark. The API enables conversion of Markdown to Typst markup and compilation of Typst documents to PDF and SVG formats.

## Table of Contents

1. [Overview](#overview)
2. [Backend Implementation](#backend-implementation)
3. [Conversion API](#conversion-api)
4. [Compilation API](#compilation-api)
5. [Filter API](#filter-api)
6. [World API](#world-api)
7. [Usage Examples](#usage-examples)
8. [Error Handling](#error-handling)
9. [Related Documentation](#related-documentation)

---

## Overview

The `quillmark-typst` crate provides a complete Typst backend implementation for the Quillmark document rendering system. 

**Primary Usage:** The main public API is the `TypstBackend` struct, which implements the `Backend` trait. Users typically interact with it through the high-level `Workflow` API from the `quillmark` crate rather than using it directly.

**Internal Modules:** The following modules contain implementation details:

- **compile**: Typst to PDF/SVG compilation functions (private)
- **convert**: Markdown to Typst markup conversion utilities (private)
- **filters**: Template filters for data transformation (private)
- **world**: Typst compilation environment with asset and package management (private)

While these modules are currently private, they contain well-documented functionality that is described in this document for:
1. Understanding how the backend works internally
2. Reference for contributors and maintainers
3. Potential future exposure if advanced use cases require direct access

**Key Features:**

- Converts CommonMark Markdown to Typst markup
- Compiles Typst documents to PDF and SVG formats
- Provides template filters for YAML data transformation
- Manages fonts, assets, and packages dynamically
- Thread-safe and suitable for concurrent rendering

---

## Backend Implementation

### `TypstBackend`

The primary entry point for using the Typst backend with Quillmark's engine.

```rust
pub struct TypstBackend;
```

**Implementation of `Backend` trait:**

#### Methods

##### `id() -> &'static str`

Returns the backend identifier.

**Returns:** `"typst"`

**Example:**
```rust
use quillmark_typst::TypstBackend;
use quillmark_core::Backend;

let backend = TypstBackend;
assert_eq!(backend.id(), "typst");
```

---

##### `supported_formats() -> &'static [OutputFormat]`

Returns the output formats supported by this backend.

**Returns:** `&[OutputFormat::Pdf, OutputFormat::Svg]`

**Example:**
```rust
use quillmark_typst::TypstBackend;
use quillmark_core::{Backend, OutputFormat};

let backend = TypstBackend;
assert!(backend.supported_formats().contains(&OutputFormat::Pdf));
assert!(backend.supported_formats().contains(&OutputFormat::Svg));
```

---

##### `glue_type() -> &'static str`

Returns the file extension for glue templates used with this backend.

**Returns:** `".typ"` (Typst template file extension)

**Example:**
```rust
use quillmark_typst::TypstBackend;
use quillmark_core::Backend;

let backend = TypstBackend;
assert_eq!(backend.glue_type(), ".typ");
```

---

##### `register_filters(&self, glue: &mut Glue)`

Registers backend-specific template filters with the glue environment.

**Parameters:**
- `glue` - The glue environment to register filters with

**Registered Filters:**
- `String` - Converts values to Typst string literals
- `Lines` - Converts arrays to Typst arrays
- `Date` - Converts date strings to Typst datetime objects
- `Dict` - Converts YAML/JSON objects to Typst dictionaries
- `Content` - Converts Markdown to Typst content
- `Asset` - Resolves asset paths for Typst

**Example:**
```rust
use quillmark_typst::TypstBackend;
use quillmark_core::{Backend, Glue};

let backend = TypstBackend;
let mut glue = Glue::new("{{ title | String }}")?;
backend.register_filters(&mut glue);
// Filters are now registered and available in templates
```

---

##### `compile(&self, glued_content: &str, quill: &Quill, opts: &RenderOptions) -> Result<Vec<Artifact>, RenderError>`

Compiles glued Typst content into final output artifacts.

**Parameters:**
- `glued_content` - The composed Typst template content
- `quill` - The quill template configuration
- `opts` - Rendering options (includes output format)

**Returns:** 
- `Ok(Vec<Artifact>)` - Vector of compiled artifacts
  - For PDF: Single artifact with PDF bytes
  - For SVG: Multiple artifacts, one per page
- `Err(RenderError)` - Compilation or format errors

**Errors:**
- `RenderError::FormatNotSupported` - Requested format not supported by backend
- `RenderError::Other` - Compilation failures from Typst

**Example:**
```rust
use quillmark_typst::TypstBackend;
use quillmark_core::{Backend, Quill, RenderOptions, OutputFormat};

let backend = TypstBackend;
let quill = Quill::from_path("path/to/quill")?;
let opts = RenderOptions {
    output_format: Some(OutputFormat::Pdf),
    ..Default::default()
};

let artifacts = backend.compile(typst_content, &quill, &opts)?;
// artifacts[0].bytes contains the PDF data
```

---

## Conversion API

> **Note:** The conversion API is currently private. This documentation is provided for reference and for potential future exposure. Users of the Typst backend access this functionality indirectly through the `Content` filter in templates.

The conversion API provides functions for transforming Markdown into Typst markup. These functions are defined in the `convert` module.

### `mark_to_typst(markdown: &str) -> String`

Converts CommonMark Markdown to Typst markup.

**Parameters:**
- `markdown` - Input markdown string

**Returns:** Typst markup string ready for compilation

**Features:**
- Supports CommonMark specification
- Enables strikethrough extension (`~~text~~`)
- Handles text formatting (bold, italic, strikethrough)
- Converts lists (bullet and ordered, including nested)
- Processes links and inline code
- Escapes Typst special characters

**Example:**
```rust
use quillmark_typst::convert::mark_to_typst;

let markdown = "This is **bold** and _italic_ text.";
let typst = mark_to_typst(markdown);
// Output: "This is *bold* and _italic_ text.\n\n"
```

**Conversion Reference:**

| Markdown | Typst Output |
|----------|--------------|
| `**bold**` | `*bold*` |
| `_italic_` | `_italic_` |
| `~~strike~~` | `#strike[strike]` |
| `` `code` `` | `` `code` `` |
| `[text](url)` | `#link("url")[text]` |
| `- item` | `+ item` |
| `1. item` | `1. item` |

**See also:** [CONVERT.md](CONVERT.md) for detailed conversion documentation.

---

### `escape_markup(s: &str) -> String`

Escapes text for safe use in Typst markup context.

**Parameters:**
- `s` - Text to escape

**Returns:** Escaped string safe for Typst markup

**Escaped Characters:**
- `\` → `\\` (backslash)
- `*` → `\*` (bold marker)
- `_` → `\_` (emphasis marker)
- `` ` `` → ``\` `` (code marker)
- `#` → `\#` (function marker)
- `[`, `]` → `\[`, `\]` (link markers)
- `$` → `\$` (math mode)
- `<`, `>` → `\<`, `\>` (angle brackets)
- `@` → `\@` (references)

**Example:**
```rust
use quillmark_typst::convert::escape_markup;

let text = "Use * for bold and # for functions";
let escaped = escape_markup(text);
// Output: "Use \\* for bold and \\# for functions"
```

**Note:** This function is primarily used internally by `mark_to_typst()`, but is exposed for advanced use cases.

---

### `escape_string(s: &str) -> String`

Escapes text for embedding in Typst string literals (within quotes).

**Parameters:**
- `s` - Text to escape

**Returns:** Escaped string safe for Typst string literals

**Escaped Characters:**
- `\` → `\\`
- `"` → `\"`
- `\n` → `\n` (literal backslash-n)
- `\r` → `\r` (literal backslash-r)
- `\t` → `\t` (literal backslash-t)
- Control characters → `\u{...}` (Unicode escapes)

**Example:**
```rust
use quillmark_typst::convert::escape_string;

let text = "Hello \"world\"\nNew line";
let escaped = escape_string(text);
// Output: "Hello \\\"world\\\"\\nNew line"
```

**Use Case:** When wrapping Typst markup in `eval()` calls or embedding in JSON structures for filter outputs.

---

## Compilation API

> **Note:** The compilation API is currently private. This documentation is provided for reference and for potential future exposure. Users of the Typst backend access this functionality indirectly through the `Backend::compile()` method.

The compilation API provides functions for compiling Typst documents to output formats. These functions are defined in the `compile` module.

### `compile_to_pdf(quill: &Quill, glued_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>>`

Compiles a Typst document to PDF format.

**Parameters:**
- `quill` - The quill template providing assets, packages, and fonts
- `glued_content` - The complete Typst source code to compile

**Returns:** 
- `Ok(Vec<u8>)` - PDF file bytes
- `Err(Box<dyn std::error::Error>)` - Compilation errors

**Example:**
```rust
use quillmark_typst::compile::compile_to_pdf;
use quillmark_core::Quill;

let quill = Quill::from_path("path/to/quill")?;
let typst_content = r#"
    #set document(title: "My Document")
    = Hello World
    This is a test document.
"#;

let pdf_bytes = compile_to_pdf(&quill, typst_content)?;
// Write to file or return via HTTP, etc.
std::fs::write("output.pdf", pdf_bytes)?;
```

**Compilation Process:**
1. Creates a `QuillWorld` with the quill's assets and packages
2. Compiles the Typst document using the Typst compiler
3. Converts the compiled document to PDF format
4. Returns the PDF bytes

---

### `compile_to_svg(quill: &Quill, glued_content: &str) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>>`

Compiles a Typst document to SVG format (one SVG per page).

**Parameters:**
- `quill` - The quill template providing assets, packages, and fonts
- `glued_content` - The complete Typst source code to compile

**Returns:**
- `Ok(Vec<Vec<u8>>)` - Vector of SVG page bytes (one per page)
- `Err(Box<dyn std::error::Error>)` - Compilation errors

**Example:**
```rust
use quillmark_typst::compile::compile_to_svg;
use quillmark_core::Quill;

let quill = Quill::from_path("path/to/quill")?;
let typst_content = r#"
    = Page 1
    Content on first page.
    
    #pagebreak()
    
    = Page 2
    Content on second page.
"#;

let svg_pages = compile_to_svg(&quill, typst_content)?;
// svg_pages[0] contains first page SVG
// svg_pages[1] contains second page SVG

for (i, svg_bytes) in svg_pages.iter().enumerate() {
    std::fs::write(format!("page_{}.svg", i + 1), svg_bytes)?;
}
```

**Note:** Each page is rendered as a separate SVG document for maximum compatibility.

---

## Filter API

> **Note:** The filter implementations are currently private. This documentation is provided for reference. Users access these filters through Typst templates using the filter syntax (e.g., `{{ value | String }}`).

Template filters transform data from YAML frontmatter into Typst-compatible representations. All filters use the stable `filter_api` from `quillmark_core::templating::filter_api`.

### Filter Signature

All filters follow this signature:

```rust
pub fn filter_name(
    _state: &State,
    value: Value,
    kwargs: Kwargs
) -> Result<Value, Error>
```

**Parameters:**
- `_state` - Filter execution state (typically unused)
- `value` - Input value from template
- `kwargs` - Keyword arguments (e.g., `default`)

**Returns:**
- `Ok(Value)` - Transformed value as safe string
- `Err(Error)` - Validation or conversion errors

---

### `string_filter`

Converts a value to a Typst string literal via JSON injection.

**Template Usage:**
```typst
{{ title | String }}
{{ author | String(default="Anonymous") }}
```

**Behavior:**
1. Applies default value if input is undefined
2. Converts value to string
3. Serializes to JSON string
4. Wraps in `json(bytes("..."))` for Typst

**Input Types:** Any value convertible to string

**Output:** Typst expression: `json(bytes("\"value\""))`

**Example:**
```rust
// In template: {{ title | String }}
// With frontmatter: title: "My Document"
// Output: json(bytes("\"My Document\""))
```

---

### `lines_filter`

Converts an array of strings to a Typst array via JSON injection.

**Template Usage:**
```typst
{{ tags | Lines }}
{{ authors | Lines(default=["Unknown"]) }}
```

**Behavior:**
1. Applies default value if input is undefined
2. Validates input is an array
3. Validates each element is a string
4. Serializes to JSON array
5. Wraps in `json(bytes("..."))` for Typst

**Input Types:** Array of strings

**Output:** Typst expression: `json(bytes("[\"item1\",\"item2\"]"))`

**Example:**
```rust
// In template: {{ tags | Lines }}
// With frontmatter: tags: [security, performance]
// Output: json(bytes("[\"security\",\"performance\"]"))
```

**Errors:**
- `InvalidOperation` - If value is not an array or contains non-string elements

---

### `date_filter`

Converts an ISO 8601 date string to a Typst datetime object.

**Template Usage:**
```typst
{{ date | Date }}
{{ published | Date(default="2024-01-01") }}
```

**Behavior:**
1. Applies default value if input is undefined
2. Uses current UTC date if still undefined
3. Validates strict ISO 8601 format (YYYY-MM-DD)
4. Converts to Typst datetime constructor

**Input Types:** ISO 8601 date string (YYYY-MM-DD)

**Output:** Typst expression: `datetime(year: YYYY, month: MM, day: DD)`

**Example:**
```rust
// In template: {{ date | Date }}
// With frontmatter: date: "2024-03-15"
// Output: datetime(year: 2024, month: 3, day: 15)
```

**Errors:**
- `InvalidOperation` - If date string is not valid ISO 8601 format

---

### `dict_filter`

Converts a YAML/JSON object to a Typst dictionary via JSON injection.

**Template Usage:**
```typst
{{ metadata | Dict }}
{{ config | Dict(default={version: "1.0"}) }}
```

**Behavior:**
1. Applies default value if input is undefined
2. Converts to JSON object
3. Wraps in `json(bytes("..."))` for Typst

**Input Types:** Objects/mappings (YAML maps, JSON objects)

**Output:** Typst expression: `json(bytes("{\"key\":\"value\"}"))`

**Example:**
```rust
// In template: {{ metadata | Dict }}
// With frontmatter:
//   metadata:
//     version: 1.0
//     author: "John"
// Output: json(bytes("{\"version\":1.0,\"author\":\"John\"}"))
```

---

### `content_filter`

Converts Markdown body content to Typst markup wrapped in an eval expression.

**Template Usage:**
```typst
{{ body | Content }}
```

**Behavior:**
1. Extracts string content from value
2. Converts Markdown to Typst using `mark_to_typst()`
3. Escapes for string literal using `escape_string()`
4. Wraps in `eval("...", mode: "markup")`

**Input Types:** Markdown string (typically the document body)

**Output:** Typst expression: `eval("typst_markup", mode: "markup")`

**Example:**
```rust
// In template: {{ body | Content }}
// With Markdown: "This is **bold** text."
// Converted: "This is *bold* text.\n\n"
// Output: eval("This is *bold* text.\\n\\n", mode: "markup")
```

**Note:** This filter performs two-stage escaping:
1. Markdown → Typst markup conversion
2. Typst markup → string literal escaping

---

### `asset_filter`

Resolves asset filenames to proper virtual paths for Typst.

**Template Usage:**
```typst
{{ logo | Asset }}
```

**Behavior:**
1. Extracts filename from value
2. Validates no path separators (security check)
3. Prepends `assets/DYNAMIC_ASSET__` prefix
4. Returns Typst string literal

**Input Types:** Filename string (no path separators)

**Output:** Typst string literal: `"assets/DYNAMIC_ASSET__filename.ext"`

**Example:**
```rust
// In template: #image({{ logo | Asset }})
// With frontmatter: logo: "company-logo.png"
// Output: #image("assets/DYNAMIC_ASSET__company-logo.png")
```

**Note:** The `DYNAMIC_ASSET__` prefix is used internally by the QuillWorld to resolve assets from the quill's in-memory file system.

**Security:**
- Rejects filenames containing `/` or `\` (path traversal protection)
- All assets must reside in the `assets/` directory
- The `DYNAMIC_ASSET__` prefix ensures proper resolution in QuillWorld

**Errors:**
- `InvalidOperation` - If filename contains path separators

---

## World API

> **Note:** The `QuillWorld` struct is currently private. This documentation is provided for reference. Users benefit from this functionality automatically when using the Typst backend through `Workflow` or `Backend::compile()`.

The `QuillWorld` struct provides the Typst compilation environment with dynamic asset and package management.

### `QuillWorld`

```rust
pub struct QuillWorld { /* private fields */ }
```

**Purpose:** Implements Typst's `World` trait to provide:
- Font loading from quill assets and system fonts
- Asset file access (images, data files)
- Package discovery and loading
- Source file management

**Key Features:**

- **Dynamic Package Discovery**: Automatically discovers packages in `{quill}/packages/`
- **Proper Virtual Path Handling**: Maintains directory structure in virtual file system
- **Entrypoint Support**: Reads `typst.toml` files for package configuration
- **Namespace Handling**: Supports `@preview` and custom namespaces
- **Asset Management**: Loads assets from `{quill}/assets/`
- **Font Management**: Loads fonts from assets first, then system fonts

---

### `QuillWorld::new(quill: &Quill, main: &str) -> Result<Self, Box<dyn std::error::Error>>`

Creates a new QuillWorld from a quill template and Typst content.

**Parameters:**
- `quill` - The quill template providing resources
- `main` - The main Typst source code to compile

**Returns:**
- `Ok(QuillWorld)` - Initialized compilation environment
- `Err(Box<dyn std::error::Error>)` - Resource loading errors

**Initialization Steps:**
1. Loads fonts from quill assets (eager loading)
2. Initializes system font searcher (lazy loading)
3. Loads binary assets from `assets/` directory
4. Loads embedded packages from `packages/` directory
5. Downloads and loads external packages from Quill.toml
6. Creates main source file

**Example:**
```rust
use quillmark_typst::world::QuillWorld;
use quillmark_core::Quill;

let quill = Quill::from_path("path/to/quill")?;
let typst_code = "#set document(title: \"Test\")\n= Hello";

let world = QuillWorld::new(&quill, typst_code)?;
// World is ready for compilation
```

**Resource Loading Order:**

1. **Fonts**:
   - Quill asset fonts loaded first (highest priority)
   - System fonts loaded second (fallback)

2. **Packages**:
   - Embedded packages in `{quill}/packages/` loaded first
   - External packages from Quill.toml loaded second (can override)

**Directory Structure:**

```
quill/
├── quill.toml           # Quill configuration
├── template.typ         # Glue template
├── assets/              # Binary assets
│   ├── logo.png
│   ├── font.ttf
│   └── data.json
└── packages/            # Embedded Typst packages
    └── my-package/
        ├── typst.toml   # Package metadata
        └── src/
            └── lib.typ  # Package code
```

**Errors:**
- No fonts available (neither assets nor system)
- Package loading failures
- Invalid `typst.toml` format

---

## Usage Examples

### Recommended Usage (via Workflow)

The recommended way to use the Typst backend is through the high-level `Workflow` API:

```rust
use quillmark::{OutputFormat, Quill, Workflow};
use quillmark_typst::TypstBackend;

// 1. Load quill template
let quill = Quill::from_path("path/to/quill")?;

// 2. Create workflow with Typst backend
let backend = Box::new(TypstBackend::default());
let workflow = Workflow::new(backend, quill)?;

// 3. Render markdown document
let markdown = r#"---
title: "My Document"
author: "John Doe"
date: "2024-03-15"
---

# Introduction

This is **bold** text with a [link](https://example.com).
"#;

let result = workflow.render(markdown, Some(OutputFormat::Pdf))?;

// 4. Save output
std::fs::write("output.pdf", &result.artifacts[0].bytes)?;
```

This approach handles all the orchestration automatically: parsing, template composition, filter registration, and compilation.

---

### Advanced Usage (Direct Backend)

For advanced use cases where you need more control over the rendering pipeline, you can use the `Backend` trait implementation directly:

```rust
use quillmark_typst::TypstBackend;
use quillmark_core::{
    Backend, Quill, Glue, RenderOptions, OutputFormat, decompose
};

// 1. Parse markdown document
let markdown = r#"---
title: "My Document"
author: "John Doe"
date: "2024-03-15"
---

# Introduction

This is **bold** text with a [link](https://example.com).
"#;

let parsed = decompose(markdown)?;

// 2. Load quill template
let quill = Quill::from_path("path/to/quill")?;

// 3. Setup glue with backend filters
let backend = TypstBackend;
let mut glue = Glue::new(&quill.glue_template)?;
backend.register_filters(&mut glue);

// 4. Compose glue template
let glued_content = glue.compose(parsed.fields().clone())?;

// 5. Compile to PDF
let opts = RenderOptions {
    output_format: Some(OutputFormat::Pdf),
    ..Default::default()
};

let artifacts = backend.compile(&glued_content, &quill, &opts)?;

// 6. Save output
std::fs::write("output.pdf", &artifacts[0].bytes)?;
```

This approach gives you access to intermediate steps and allows custom processing between stages.

---

### Internal Usage Examples

The following examples show how the conversion functions work internally. These are not directly accessible to end users but are provided for understanding the implementation.

#### Markdown to Typst Conversion

```rust
// Internal usage within filters
use quillmark_typst::convert::mark_to_typst;

// Convert various markdown elements
let examples = vec![
    ("**bold**", "*bold*\n\n"),
    ("_italic_", "_italic_\n\n"),
    ("~~strike~~", "#strike[strike]\n\n"),
    ("`code`", "`code`\n\n"),
    ("[link](url)", "#link(\"url\")[link]\n\n"),
];

for (markdown, expected) in examples {
    let typst = mark_to_typst(markdown);
    assert_eq!(typst, expected);
}
```

---

### Template Filter Usage

Users interact with filters through Typst templates:

```typst
#set document(
  title: {{ title | String }},
  author: {{ author | String(default="Anonymous") }},
  date: {{ date | Date }},
)

#set text(font: "Linux Libertine")

= {{ title | String }}

#text(size: 10pt)[
  Author: {{ author | String }}
  #h(1fr)
  Date: {{ date | Date }}
]

#line(length: 100%)

{{ body | Content }}

#if {{ tags | Lines }} != none {
  #heading(level: 2)[Tags]
  #for tag in {{ tags | Lines }} {
    #box(fill: gray, inset: 5pt, text(fill: white)[#tag])
  }
}
```

---

## Error Handling

### Error Types

The `quillmark-typst` crate uses structured error handling following the patterns described in [DESIGN.md](../../DESIGN.md).

**Compilation Errors:**

```rust
use quillmark_typst::compile::compile_to_pdf;
use quillmark_core::Quill;

let quill = Quill::from_path("path/to/quill")?;
let invalid_typst = "#invalid-function()";

match compile_to_pdf(&quill, invalid_typst) {
    Ok(pdf) => { /* success */ },
    Err(e) => {
        // Error message includes:
        // - Source file and line number
        // - Error description
        // - Surrounding context
        eprintln!("Compilation failed: {}", e);
    }
}
```

**Filter Errors:**

```rust
use quillmark_typst::filters::date_filter;
use quillmark_core::templating::filter_api::{Value, Kwargs, State};

let state = State::default();
let invalid_date = Value::from("not-a-date");
let kwargs = Kwargs::default();

match date_filter(&state, invalid_date, kwargs) {
    Ok(v) => { /* success */ },
    Err(e) => {
        // Error includes kind and message
        // ErrorKind::InvalidOperation: "Not ISO date (YYYY-MM-DD): not-a-date"
        eprintln!("Filter error: {}", e);
    }
}
```

**Backend Errors:**

```rust
use quillmark_typst::TypstBackend;
use quillmark_core::{Backend, Quill, RenderOptions, OutputFormat, RenderError};

let backend = TypstBackend;
let quill = Quill::from_path("path/to/quill")?;
let opts = RenderOptions {
    output_format: Some(OutputFormat::Txt), // Not supported
    ..Default::default()
};

match backend.compile(content, &quill, &opts) {
    Ok(_) => { /* success */ },
    Err(RenderError::FormatNotSupported { backend, format }) => {
        eprintln!("Format {} not supported by {}", format, backend);
    },
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

---

## Related Documentation

### Internal Documentation

- **[CONVERT.md](CONVERT.md)** - Detailed Markdown to Typst conversion design and implementation
  - Event-based conversion flow
  - Character escaping strategies
  - CommonMark feature coverage
  - Implementation notes and gotchas

- **[DESIGN.md](../../DESIGN.md)** - Overall Quillmark architecture
  - Core design principles
  - Error handling patterns
  - Template system design
  - Backend architecture
  - Extension points

### Module Documentation

- **[quillmark-core](../../quillmark-core/)** - Core types and traits
  - `Backend` trait specification
  - `Glue` template system
  - `ParsedDocument` and parsing utilities
  - Error types (`RenderError`, `Diagnostic`)

### External Resources

- **[Typst Documentation](https://typst.app/docs/)** - Official Typst language documentation
- **[CommonMark Specification](https://spec.commonmark.org/)** - Markdown specification
- **[MiniJinja](https://docs.rs/minijinja/)** - Template engine used by Glue

---

## API Stability

**Stability Guarantees:**

- ✅ **Stable (Public)**: `TypstBackend` struct and its `Backend` trait implementation
- 🔒 **Internal (Private)**: All other modules (`compile`, `convert`, `filters`, `world`)
  - These are implementation details that may change between versions
  - Users should rely on the `Backend` trait interface and `Workflow` API
  - Documentation is provided for reference, not as a public contract

**Breaking Changes:**

Any breaking changes to the public API (`TypstBackend`) will be clearly documented in release notes. The API follows semantic versioning:
- Major version: Breaking changes to public API (e.g., `Backend` trait methods)
- Minor version: New features, backward compatible enhancements
- Patch version: Bug fixes, no API changes

Internal module changes (compile, convert, filters, world) are not considered breaking changes as they are not part of the public API.

---

## Contributing

When extending the `quillmark-typst` API:

1. **Follow DESIGN.md patterns** - Maintain consistency with core architecture
2. **Document public APIs** - Update this file for any new public functions
3. **Add usage examples** - Include practical examples in documentation
4. **Write tests** - Cover new functionality with unit and integration tests
5. **Update CONVERT.md** - For changes to Markdown conversion logic

---

## License

`quillmark-typst` is licensed under the Apache-2.0 license. See [LICENSE](../../LICENSE) for details.
