## Quill JSON Contract

Summary
- Input to `Quill::from_json` (core) and the WASM wrapper `Quill.fromJson` (JS) is a JSON string whose root value MUST be an object.
- The root object represents the Quill file tree. Two reserved top-level metadata keys are supported: `name` and `base_path`.

Node shapes
- File with UTF-8 string contents:
  "path/to/file.txt": { "contents": "...utf-8 text..." }

- File with JSON-encoded bytes (use when embedding binary files into JSON):
  "image.png": { "contents": [137,80,78,71, ...] }

- Directory using explicit `files` map:
  "dir": { "files": { "a.txt": { "contents": "..." } } }

- Directory using direct nested object (shorthand):
  "dir": { "a.txt": { "contents": "..." }, "sub": { "files": { ... } } }

Reserved keys
- name (optional): a default name used if `Quill.toml` does not provide one.
- base_path (optional): base path for resolving assets and packages.

Validation
- After parsing the file tree, the implementation validates the Quill (for example `Quill.toml` must exist and reference an existing glue file). Validation errors are returned as failures from `from_json`.

Usage notes (JS / WASM)
- The WASM wrapper `Quill.fromJson` expects a JSON string. Build a JS object matching this contract and call `JSON.stringify(quillObj)` before passing it into WASM.
- When embedding binary files into JSON, convert a `Uint8Array` to an array of numeric bytes (e.g. `Array.from(uint8arr)`).
- For runtime APIs that accept binary buffers directly (e.g. `withAsset`), pass `Uint8Array`/`Buffer` instead of JSON-encoding the bytes.

Minimal example
```json
{
  "name": "my-quill",
  "base_path": "/",
  "Quill.toml": { "contents": "[Quill]\nname = \"my-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n" },
  "glue.typ": { "contents": "= Template\n\n{{ body }}" }
}
```

Binary example (embed image)
```json
{
  "Quill.toml": { "contents": "..." },
  "glue.typ": { "contents": "..." },
  "assets": {
    "logo.png": { "contents": [137,80,78,71, ...] }
  }
}
```

Implementation note
- This contract is enforced by `quillmark-core::Quill::from_json` (parsing and merging) and by the WASM binding which exposes `Quill.fromJson` (JS name) that forwards to the same core parser.

Keep this document short — it is the canonical contract referenced by core and wasm bindings.
