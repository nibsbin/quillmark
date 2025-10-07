## Quill JSON Contract

### Summary

Input to `Quill::from_json` (core) and the WASM wrapper `Quill.fromJson` (JS) is a JSON string whose root value MUST be an object with a `files` key. An optional `metadata` key provides metadata overrides.

---

### Structure

**Root object:**
```json
{
  "files": { ... },      // Required: file tree
  "metadata": { ... }    // Optional: metadata overrides
}
```

**Minimal example:**
```json
{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"my-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n" },
    "glue.typ": { "contents": "= Template\n\n{{ body }}" }
  }
}
```

**With metadata:**
```json
{
  "metadata": {
    "name": "my-quill",
    "base_path": "/",
    "version": "1.0.0"
  },
  "files": {
    "Quill.toml": { "contents": "..." },
    "glue.typ": { "contents": "..." },
    "assets": {
      "logo.png": { "contents": [137, 80, 78, 71, ...] }
    }
  }
}
```

---

### Node Types

**File with UTF-8 string contents:**
```json
"file.txt": { "contents": "Hello, world!" }
```

**File with binary contents (byte array 0-255):**
```json
"image.png": { "contents": [137, 80, 78, 71, 13, 10, 26, 10, ...] }
```

**Directory (nested object):**
```json
"assets": {
  "logo.png": { "contents": [...] },
  "icon.svg": { "contents": "..." }
}
```

**Empty directory:**
```json
"empty_dir": {}
```

---

### Metadata Object (Optional)

```json
{
  "name": "my-quill",              // Quill name (overrides Quill.toml)
  "base_path": "/",                // Base path for asset resolution
  "version": "1.0.0",              // Semantic version
  "description": "...",            // Human-readable description
  "author": "John Doe",            // Author name
  "license": "MIT",                // License identifier
  "tags": ["letter", "formal"]     // Tags for categorization
}
```

All metadata fields are optional. If not provided, values are extracted from `Quill.toml` or use defaults.

**Metadata priority (highest to lowest):**
1. JSON `metadata` object
2. `Quill.toml` `[Quill]` section
3. Function arguments (`default_name`, `base_path`)
4. Defaults (e.g., `base_path = "/"`)

---

### Validation Rules

1. Root MUST be an object with a `files` key
2. `files` value MUST be an object
3. `metadata` key is optional
4. File nodes MUST have a `contents` key with:
   - A string (UTF-8 text), OR
   - An array of numbers 0-255 (binary)
5. Directory nodes are objects without a `contents` key
6. Empty objects represent empty directories
7. After parsing, `Quill.toml` MUST exist and be valid
8. Glue file referenced in `Quill.toml` MUST exist

---

### Usage Notes (JS / WASM)

**Creating Quill from JS object:**
```typescript
const quillData = {
  files: {
    "Quill.toml": { contents: "[Quill]\nname = \"my-quill\"\n..." },
    "glue.typ": { contents: "= Template\n\n{{ body }}" }
  }
};

const quill = Quill.fromJson(JSON.stringify(quillData));
```

**Embedding binary files:**
```typescript
const imageBytes = new Uint8Array(await file.arrayBuffer());

const quillData = {
  files: {
    "Quill.toml": { contents: "..." },
    "glue.typ": { contents: "..." },
    "assets": {
      "logo.png": { contents: Array.from(imageBytes) }
    }
  }
};
```

**Building from file uploads:**
```typescript
async function buildQuillFromUpload(files: File[]): Promise<object> {
  const fileTree: any = {};

  for (const file of files) {
    const path = file.webkitRelativePath || file.name;
    const parts = path.split('/');
    let current = fileTree;

    // Build nested structure
    for (let i = 0; i < parts.length - 1; i++) {
      if (!current[parts[i]]) current[parts[i]] = {};
      current = current[parts[i]];
    }

    // Add file
    const fileName = parts[parts.length - 1];
    const isBinary = /\.(png|jpg|jpeg|gif|pdf)$/i.test(fileName);

    current[fileName] = {
      contents: isBinary
        ? Array.from(new Uint8Array(await file.arrayBuffer()))
        : await file.text()
    };
  }

  return {
    metadata: {
      name: files[0]?.webkitRelativePath?.split('/')[0] || 'uploaded-quill'
    },
    files: fileTree
  };
}
```

---

### Implementation Note

This contract is enforced by:
- `quillmark-core::Quill::from_json` (Rust core)
- WASM binding `Quill.fromJson` (forwards to core parser)

See [QUILL_DESIGN.md](../../designs/QUILL_DESIGN.md) for full design rationale.
