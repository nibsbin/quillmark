# Quill Resource File Structure and API

> **Status**: Final Design - Opinionated, No Backward Compatibility
> **Implementation**: `quillmark-core/src/quill.rs`

## Internal File Structure

```rust
pub enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}

pub struct Quill {
    pub metadata: HashMap<String, QuillValue>,
    pub name: String,
    pub backend: String,
    pub plate: Option<String>,
    pub example: Option<String>,
    pub schema: QuillValue,
    pub defaults: HashMap<String, QuillValue>,
    pub examples: HashMap<String, Vec<QuillValue>>,
    pub files: FileTreeNode,
}
```

## JSON Contract

Root object with a `files` key:

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

Node types:
- **File (UTF-8)**: `"file.txt": { "contents": "Hello, world!" }`
- **File (binary)**: `"image.png": { "contents": [137, 80, 78, 71, ...] }`
- **Directory**: `"assets": { "logo.png": {...}, "icon.svg": {...} }`
- **Empty directory**: `"empty_dir": {}`

Validation rules:
1. Root MUST be an object with a `files` key
2. File nodes MUST have a `contents` key (string or byte array)
3. Directory nodes are objects without a `contents` key
4. `Quill.toml` MUST exist and be valid
5. The plate file referenced in `Quill.toml` MUST exist

## Quill.toml Structure

```toml
[Quill]
name = "my-quill"
backend = "typst"
description = "A beautiful template"  # required
plate_file = "plate.typ"  # optional
example_file = "example.md"  # optional
version = "1.0.0"  # optional
author = "Template Author"  # optional

[typst]
packages = ["@preview/bubble:0.2.2"]

[fields]
author = { description = "Author of document", type = "str", default = "Anonymous" }
title = { description = "Document title", type = "str" }
```

Metadata resolution:
- `name` always read from `Quill.toml` `[Quill].name` (required)
- `metadata` HashMap includes `backend`, `description`, `author`, and custom `[Quill]` fields; Typst config fields are prefixed `typst_`

## API

Construction:
- `Quill::from_path(path)` — load from filesystem directory
- `Quill::from_tree(root)` — load from in-memory file tree
- `Quill::from_json(json_str)` — load from JSON string

File access:
- `file_exists(path)` / `get_file(path)` — check/read file
- `dir_exists(path)` / `list_files(path)` / `list_subdirectories(path)` — directory navigation

Path rules:
- Always use forward slashes (`/`)
- Directory paths must end with `/` for `list_files()` and `list_subdirectories()`
- Root: use `""` or `"/"`
- `get_file()` returns `Vec<u8>` for all files
