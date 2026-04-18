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

## In-memory Tree Contract (`Quill::from_tree`)

`Quill::from_tree` is the only in-memory constructor. Input is a `FileTreeNode`
directory tree with UTF-8 and binary file contents represented as bytes.

For JS/WASM consumers this is exposed as `Quill.fromTree(...)` with a flat
`Map<string, Uint8Array>` (or plain object) path→bytes shape.

Validation rules:
1. Root MUST be a directory node
2. `Quill.yaml` MUST exist and be valid
3. The `plate_file` referenced in `Quill.yaml` MUST exist
4. File paths use `/` separators and are resolved relative to root

## `Quill.yaml` Structure

```yaml
Quill:
  name: my_quill
  backend: typst
  version: "1.0.0"
  description: A beautiful format
  plate_file: plate.typ
  example_file: example.md

main:
  fields:
    author:
      type: string
      description: Author of document
    title:
      type: string
      description: Document title
```

Metadata resolution:
- `name` always read from `Quill.yaml` `Quill.name` (required)
- `metadata` includes `backend`, `description`, `version`, and other Quill-level keys

## API

Construction:
- `Quill::from_path(path)` — load from filesystem directory
- `Quill::from_tree(root)` — load from in-memory file tree

Note: `Quill::from_json` is removed from the public API.

File access:
- `file_exists(path)` / `get_file(path)` — check/read file
- `dir_exists(path)` / `list_files(path)` / `list_subdirectories(path)` — directory navigation

Path rules:
- Always use forward slashes (`/`)
- Directory paths must end with `/` for `list_files()` and `list_subdirectories()`
- Root: use `""` or `"/"`
- `get_file()` returns `Vec<u8>` for all files
