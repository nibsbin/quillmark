# Quill Resource Structure

Status: **Implemented** (2026-03-22)  
Source: `crates/core/src/quill.rs`

Canonical specification for Quill bundles (template + schema + assets).

## Filesystem Shape
```
my-quill/
├─ Quill.yaml        # required
├─ plate.typ         # optional (depends on backend)
├─ example.md        # optional (auto-used if present)
├─ packages/…        # optional backend packages
└─ assets/…          # optional fonts/images/other assets
```
- `.quillignore` supports basic ignore patterns; defaults ignore `.git/`, `target/`, `node_modules/`, etc.
- Dynamic assets/fonts injected at runtime use `DYNAMIC_ASSET__*` / `DYNAMIC_FONT__*` and live in-memory only.

## Quill.yaml (authoritative)
```yaml
Quill:                # required table
  name: resume        # required
  backend: typst      # required
  description: "..."  # required, non-empty
  version: "0.43.0"   # required, MAJOR.MINOR[.PATCH]
  author: "Author"    # default: "Unknown"
  plate_file: plate.typ        # optional
  example_file: example.md     # optional
  ui: { order: 1, group: "Docs" }   # optional container metadata

typst:                # optional backend config, stored as metadata typst_* keys
  packages: ["@preview/foo:0.1.0"]

fields:               # optional document fields
  title:
    title: "Title"
    type: "string" | "number" | "boolean" | "array" | "object" | "date" | "datetime" | "markdown"
    description: "..."
    default: "Anonymous"
    examples: ["John Doe"]
    enum: ["a", "b"]
    items: {...}              # for arrays
    properties: {...}         # for objects
    ui: { group: "...", order: 0, visible_when: { field: ["value"] }, compact: true }

cards:                # optional typed card schemas
  product:
    title: "Product card"
    description: "..."
    ui: { hide_body: true }
    fields:
      name: { type: "string", required: true }
      body: { type: "markdown" }
```

## In-Memory Model
- `Quill` holds `name`, `backend`, optional `plate` and `example`, JSON `schema`, and caches of `defaults` and `examples` derived from the schema.
- `metadata` map contains custom `Quill` fields plus injected `backend`, `description`, `author`, `version`, and `typst_*` entries.
- File tree stored as `FileTreeNode` (directory/file).

## JSON Contract (WASM/FFI)
Root object with a `files` map; each entry is either:
- `{ "contents": "utf8 string" }`
- `{ "contents": [byte, ...] }`
- Directory: nested object without `contents`.

`Quill::from_json` parses this structure, requires `files["Quill.yaml"]`, and validates referenced plate/example files.

## APIs
- `Quill::from_path`, `from_tree`, `from_json`.
- File helpers: `file_exists`, `get_file`, `dir_exists`, `list_files`, `list_subdirectories`.
- Defaults/examples: extracted once from the generated JSON Schema.

## Validation
- `Quill.yaml` must exist and be UTF-8.
- Required fields: `Quill.name`, `Quill.backend`, `Quill.description`, `Quill.version` (semver, 2–3 segments).
- `plate_file` and `example_file` must exist if specified; `example.md` is picked up automatically if present.
- Card/field names are parsed in order; UI `order` defaults to declaration order.

Related: [SCHEMAS.md](SCHEMAS.md) for JSON Schema generation, [CARDS.md](CARDS.md) for card semantics, [GLUE_METADATA.md](GLUE_METADATA.md) for data injection.
