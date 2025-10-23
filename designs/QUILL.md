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

**Requires Update**

## Internal File Structure

### Structure Definition

```rust
pub enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}

pub struct Quill {
    pub glue_template: String,
    pub metadata: HashMap<String, QuillValue>,
    pub name: String,
    pub glue_file: String,
    pub template_file: Option<String>,
    pub template: Option<String>,
    pub field_schemas: HashMap<String, FieldSchema>,
    pub files: FileTreeNode,
}
```

**Requires Update**

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
    "glue.typ": { "contents": "= Template\n\n{{ body }}" },
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
5. The glue file referenced in `Quill.toml` MUST exist

---

## Metadata Handling

### Quill.toml Structure

```toml
[Quill]
name = "my-quill"
backend = "typst"
glue_file = "glue.typ"
description = "A beautiful template"  # required
example_file = "template.md"  # optional
version = "1.0.0"  # optional

[fields]
# Field schemas for template variables
author = { description = "Author of document", required = true }
title = { description = "Document title", required = true }
```

### Metadata Priority

1. **Quill.toml `[Quill]` section** - Always takes precedence
2. **Function arguments** - `default_name` passed to constructors (fallback)
3. **JSON `metadata` object** - Provides default_name for `from_json`
4. **Defaults** - Fallback value "unnamed"

---

## API Surface

### Core Construction APIs

- `Quill::from_path(path)` - Load from filesystem directory
- `Quill::from_tree(root, default_name)` - Load from in-memory file tree (canonical constructor)
- `Quill::from_json(json_str)` - Load from JSON string

### File Access APIs

- `file_exists(path)` - Check if a file exists
- `get_file(path)` - Get file contents
- `dir_exists(path)` - Check if directory exists
- `list_files(path)` - List files in a directory (non-recursive)
- `list_subdirectories(path)` - List subdirectories (non-recursive)
