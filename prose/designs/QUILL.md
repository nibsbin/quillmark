# Quill Resource File Structure and API

> **Status**: Final Design - Opinionated, No Backward Compatibility
> **Implementation**: `crates/core/src/quill.rs` (`QuillSource`),
> `crates/quillmark/src/orchestration/quill.rs` (`Quill`)

## Type split: `QuillSource` vs `Quill`

Two types model a loaded quill:

- **`QuillSource`** (in `quillmark-core`) is the authored input — file bundle,
  parsed config, and metadata. It does not render.
- **`Quill`** (in `quillmark`) is the renderable shape — an `Arc<QuillSource>`
  paired with a resolved backend. Constructed only by the engine.

Bindings expose `Quill` only; `QuillSource` is a Rust-internal type.

## Internal File Structure

```rust
pub enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}

pub struct QuillSource {
    pub metadata: HashMap<String, QuillValue>,
    pub name: String,
    pub backend_id: String,
    pub plate: Option<String>,
    pub example: Option<String>,
    pub config: QuillConfig,
    pub defaults: HashMap<String, QuillValue>,
    pub examples: HashMap<String, Vec<QuillValue>>,
    pub files: FileTreeNode,
}

pub struct Quill {
    source: Arc<QuillSource>,
    backend: Arc<dyn Backend>,
}
```

`metadata` is populated from `Quill.yaml` fields plus computed entries: `backend`, `description`, `version`, `author`, and any `typst_*` keys from the `[typst]` section.

## In-memory Tree Contract (`engine.quill(tree)`)

In-memory construction routes through the engine as `engine.quill(tree)`. The
core `QuillSource::from_tree` constructor is the single authoritative in-memory
entry point; filesystem walking (`engine.quill_from_path`) lives in
`quillmark` rather than in core. Input is a `FileTreeNode` directory tree
with UTF-8 and binary file contents represented as bytes.

For JS/WASM consumers this is exposed as `engine.quill(...)` accepting a
`Map<string, Uint8Array>` (path→bytes). Plain objects are not accepted; only
`Map` instances are supported.

Validation rules:
1. Root MUST be a directory node
2. `Quill.yaml` MUST exist and be valid YAML
3. The `plate_file` referenced in `Quill.yaml`, if specified, MUST exist
4. File paths use `/` separators and are resolved relative to root

## `Quill.yaml` Structure

Required top-level sections: `Quill` (bundle metadata). Optional: `main` (document fields), `cards` (card type definitions), `typst` (backend config).

```yaml
quill:
  name: my_quill          # required; snake_case
  backend: typst          # required
  version: "1.0.0"        # required; semver (MAJOR.MINOR.PATCH or MAJOR.MINOR)
  description: A beautiful format  # required; non-empty
  author: Jane Doe        # optional; defaults to "Unknown"
  plate_file: plate.typ   # optional; path to Typst template
  example_file: example.md  # optional; example document for preview

main:
  fields:
    title:
      type: string
      description: Document title
    count:
      type: integer
      description: Whole-number count

cards:
  quote:
    title: Quote block
    description: A single pull quote
    fields:
      author:
        type: string
        description: Quote author

typst:
  packages:
    - "@preview/some-package:1.0.0"
```

Field names must be `snake_case`. Capitalized keys (e.g. `BODY`, `CARDS`, `CARD`) are reserved by the engine. Standalone `object` fields are not supported; use `array` with `items: {type: object, properties: {...}}` instead.

Metadata resolution:
- `name`, `backend`, `version`, `description`, `author` are required/defaulted struct fields in `QuillConfig`
- `metadata` on `Quill` stores `backend`, `description`, `version`, `author`, any extra `Quill.*` keys, and `typst_*` keys from the `[typst]` section
- `example_file` also accepts the alias `example` in YAML

## File Ignore Rules

When loading from disk, `Quill::from_path` respects a `.quillignore` file at the bundle root. If absent, default patterns apply: `.git/`, `.gitignore`, `.quillignore`, `target/`, `node_modules/`.

## API

Construction:
- `Quillmark::quill_from_path(path)` — load render-ready quill from filesystem directory
- `Quillmark::quill(tree)` — load render-ready quill from in-memory file tree

Note: `Quill::from_json` is not part of the public API.

File access on `FileTreeNode`:
- `file_exists(path)` / `get_file(path)` — check/read file
- `dir_exists(path)` / `list_files(path)` / `list_subdirectories(path)` — directory navigation

Path rules:
- Use forward slashes (`/`); absolute paths and `..` traversal are rejected
- Root: use `""` (empty string)
- `get_file()` returns `&[u8]` for all files
