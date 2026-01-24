# WASM Migration: Quill.toml to Quill.yaml

> **Status**: Breaking Change
> **Component**: `registerQuill`

This guide details breaking changes for WASM consumers related to the standardization of configuration files to YAML.

## `Quill.toml` Replaced by `Quill.yaml`

The Quillmark engine now strictly requires a `Quill.yaml` configuration file. Support for `Quill.toml` has been removed.

### Breakdown of Changes

| Feature | Old Behavior | New Behavior |
|---------|--------------|--------------|
| **Config File** | `Quill.toml` | `Quill.yaml` |
| **Format** | TOML | YAML |
| **Error Handling** | Validated `Quill.toml` | Returns error if `Quill.yaml` is missing |

### Impact on `registerQuill`

When calling `engine.registerQuill(quillTree)`, the input object (or JSON string) representing the file tree must now include a `Quill.yaml` file node.

#### Before (Invalid)

```javascript
const quillBucket = {
  files: {
    // ❌ TOML file is no longer recognized
    "Quill.toml": {
      contents: `
[Quill]
name = "resume"
version = "1.0"
backend = "typst"
description = "A resume template"
`
    },
    "main.typ": { contents: "..." }
  }
};

engine.registerQuill(quillBucket);
```

#### After (Valid)

```javascript
const quillBucket = {
  files: {
    // ✅ Must use Quill.yaml with valid YAML content
    "Quill.yaml": {
      contents: `
Quill:
  name: resume
  version: "1.0"
  backend: typst
  description: A resume template
`
    },
    "main.typ": { contents: "..." }
  }
};

engine.registerQuill(quillBucket);
```

### Migration Steps

1. **Rename** the configuration file in your file tree generator from `Quill.toml` to `Quill.yaml`.
2. **Convert** the content string from TOML syntax to YAML syntax.
   - Headers: `[Quill]` -> `Quill:`
   - Key-Value: `key = "value"` -> `key: value`
   - Lists: `list = ["a", "b"]` -> `list:\n  - a\n  - b`
   - Nested tables: `[section]` -> `section:`
3. **Verify** that the `Quill.yaml` key exists in the `files` object passed to `registerQuill`.

### Error Reference

If you attempt to register a Quill with the old format, you will receive an error similar to:

```text
Quill.yaml not found in file tree
```
