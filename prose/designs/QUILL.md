# Quill Resource File Structure and API

> **Status**: Final Design - Opinionated, No Backward Compatibility
>
> This document defines the canonical Quill file structure and API for creating and managing Quill template bundles.

> **Implementation**: `quillmark-core/src/quill.rs`

---

## Design Principles

1. **Separation of Concerns**: Metadata and files are completely separate
2. **Tree Structure**: Internal representation uses tree + HashMap hybrid for optimal performance
3. **Explicit over Implicit**: No magic, no reserved keys mixed with file entries
4. **Frontend-Friendly**: JSON format is intuitive and easy to construct
5. **Type-Safe**: Clear schemas for metadata and file structures

---

## Internal File Structure

### Structure Definition

```rust
pub enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}

pub struct Quill {
    /// Quill-specific metadata
    pub metadata: HashMap<String, QuillValue>,
    /// Name of the quill
    pub name: String,
    /// Backend identifier (e.g., "typst")
    pub backend: String,
    /// Plate template content (optional)
    pub plate: Option<String>,
    /// Markdown template content (optional)
    pub example: Option<String>,
    /// Field JSON schema (single source of truth for schema and defaults)
    pub schema: QuillValue,
    /// Cached default values extracted from schema (for performance)
    pub defaults: HashMap<String, QuillValue>,
    /// Cached example values extracted from schema (for performance)
    pub examples: HashMap<String, Vec<QuillValue>>,
    /// In-memory file system (tree structure)
    pub files: FileTreeNode,
}

pub struct QuillConfig {
    /// Human-readable name
    pub name: String,
    /// Description of the quill
    pub description: String,
    /// Backend identifier (e.g., "typst")
    pub backend: String,
    /// Semantic version of the quill
    pub version: Option<String>,
    /// Author of the quill
    pub author: Option<String>,
    /// Example markdown file
    pub example_file: Option<String>,
    /// Plate file
    pub plate_file: Option<String>,
    /// Field schemas
    pub fields: HashMap<String, FieldSchema>,
    /// Additional metadata from [Quill] section (excluding standard fields)
    pub metadata: HashMap<String, QuillValue>,
    /// Typst-specific configuration from `[typst]` section
    pub typst_config: HashMap<String, QuillValue>,
}
```

### Design Rationale

**Why Tree + HashMap?**
- Directory operations are essential (`list_files()`, `dir_exists()`)
- Typical Quill depth is shallow (1-3 levels)
- Memory efficient with no redundant path storage
- Clear semantics with explicit files vs directories

**Performance:**
- Per-directory lookup: O(1) via HashMap
- Deep path access: O(depth) - negligible for typical structures
- Memory: O(total_files) with no path duplication

---

## JSON Contract

### Standard Format

The JSON format has a root object with a `files` key. The optional `metadata` key provides a default name (Quill.toml name takes precedence).

```json
{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"my-quill\"\n..." },
    "plate.typ": { "contents": "#import \"@local/quillmark-helper:0.1.0\": data, eval-markup\n= Template\n\n#eval-markup(data.BODY)" },
    "assets": {
      "logo.png": { "contents": [137, 80, 78, 71, ...] }
    }
  }
}
```

### Node Types

- **File with UTF-8 string**: `"file.txt": { "contents": "Hello, world!" }`
- **File with binary**: `"image.png": { "contents": [137, 80, 78, 71, ...] }`
- **Directory**: `"assets": { "logo.png": {...}, "icon.svg": {...} }`
- **Empty directory**: `"empty_dir": {}`

### Validation Rules

1. Root MUST be an object with a `files` key
2. File nodes MUST have a `contents` key (string or byte array)
3. Directory nodes are objects without a `contents` key
4. `Quill.toml` MUST exist and be valid
5. The plate file referenced in `Quill.toml` MUST exist

---

## Metadata Handling

### Quill.toml Structure

```toml
[Quill]
name = "my-quill"
backend = "typst"
description = "A beautiful template"  # required
plate_file = "plate.typ"  # optional - if not provided, auto plate is used
example_file = "example.md"  # optional
version = "1.0.0"  # optional
author = "Template Author"  # optional

[typst]
# Typst-specific configuration
packages = ["@preview/bubble:0.2.2"]

[fields]
# Field schemas for template variables
author = { description = "Author of document", type = "str", default = "Anonymous" }
title = { description = "Document title", type = "str" }
```

### Metadata Handling

**Name Resolution:**
- Always read from `Quill.toml` `[Quill].name` field (required)
- The `default_name` parameter in constructors is ignored (kept for API compatibility)

**Metadata Storage:**
- The `metadata` HashMap includes:
  - `backend` - Backend identifier from `[Quill].backend`
  - `description` - Template description from `[Quill].description`
  - `author` - Author name if specified in `[Quill].author`
  - Any custom fields from `[Quill]` section (excluding standard fields)
  - Typst configuration with `typst_` prefix (e.g., `typst_packages`)

**Schema Handling:**
- JSON schema is always built from `[fields]` section
- Defaults and examples are cached from schema for performance

---

## API Surface

### Core Construction APIs

- `Quill::from_path(path)` - Load from filesystem directory
- `Quill::from_tree(root)` - Load from in-memory file tree (canonical constructor)
- `Quill::from_json(json_str)` - Load from JSON string

**Usage Examples:**

```rust
// Load from filesystem path
let quill = Quill::from_path("path/to/my-quill")?;

// Load from JSON (useful for web frontends)
let json_data = r#"{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"demo\"\nbackend = \"typst\"\ndescription = \"Demo quill\"" },
    "plate.typ": { "contents": "#set document(title: \"{{ title }}\")\n\n{{ body | Content }}" }
  }
}"#;
let quill = Quill::from_json(json_data)?;
```

### File Access APIs

- `file_exists(path)` - Check if a file exists
- `get_file(path)` - Get file contents
- `dir_exists(path)` - Check if directory exists
- `list_files(path)` - List files in a directory (non-recursive)
- `list_subdirectories(path)` - List subdirectories (non-recursive)

**File Tree Navigation Examples:**

```rust
// Check if asset exists before using
if quill.file_exists("assets/logo.png") {
    let logo_bytes = quill.get_file("assets/logo.png")?;
}

// List all fonts in a directory
if quill.dir_exists("fonts/") {
    let font_files = quill.list_files("fonts/")?;
    for font in font_files {
        println!("Found font: {}", font);
    }
}

// Navigate nested directories
let packages = quill.list_subdirectories("packages/")?;
for package_dir in packages {
    let package_files = quill.list_files(&format!("packages/{}/", package_dir))?;
}
```

**Edge Cases:**

1. **Path Separators**: Always use forward slashes (`/`) regardless of platform
2. **Directory Paths**: Directory paths must end with `/` for `list_files()` and `list_subdirectories()`
3. **Root Access**: Use `""` or `"/"` to access root directory
4. **Non-existent Paths**: File access methods return `Err` for non-existent paths
5. **Binary Files**: `get_file()` returns `Vec<u8>` for all files (convert to UTF-8 as needed)
