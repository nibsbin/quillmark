# Quill Resource File Tree/Structure and API Design

> **Status**: Final Design - Opinionated, No Backward Compatibility
>
> This document defines the canonical Quill file structure, API surface, and internal design for creating and managing Quill template bundles.

---

## Table of Contents

1. [Design Principles](#design-principles)
2. [Internal File Structure](#internal-file-structure)
3. [JSON Contract](#json-contract)
4. [Metadata Handling](#metadata-handling)
5. [API Surface](#api-surface)
6. [File Access APIs](#file-access-apis)
7. [Implementation Guidelines](#implementation-guidelines)
8. [Open Questions](#open-questions)

---

## Design Principles

1. **Separation of Concerns**: Metadata and files are completely separate
2. **Tree Structure**: Internal representation uses tree + HashMap hybrid for optimal performance
3. **Explicit over Implicit**: No magic, no reserved keys mixed with file entries
4. **Frontend-Friendly**: JSON format is intuitive and easy to construct
5. **Extensible**: Adding new metadata fields requires no code changes
6. **Type-Safe**: Clear schemas for metadata and file structures
7. **No Backward Compatibility**: Clean slate, opinionated design

---

## Internal File Structure

### Structure Definition

```rust
pub enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}

pub struct Quill {
    /// Glue template content
    pub glue_template: String,

    /// Quill-specific metadata from Quill.toml
    pub metadata: HashMap<String, serde_yaml::Value>,

    /// Base path for resolving relative paths
    pub base_path: PathBuf,

    /// Name of the quill
    pub name: String,

    /// Glue template file name
    pub glue_file: String,

    /// Markdown template file name (optional)
    pub template_file: Option<String>,

    /// Markdown template content (optional)
    pub template: Option<String>,

    /// In-memory file system (tree structure)
    pub files: FileTreeNode,
}
```

### Design Rationale

**Why Tree + HashMap (not flat HashMap)?**

1. **Directory operations are essential**: `list_files()`, `dir_exists()`, `list_subdirectories()`
2. **Typical Quill depth is shallow**: Most Quills have 1-3 levels, so O(depth) traversal is fast
3. **Memory efficient**: No redundant path storage (e.g., `assets/logo.png` stored once)
4. **Clear semantics**: Explicit distinction between files and directories
5. **Natural mental model**: Matches filesystem structure developers understand

**Performance characteristics:**
- Per-directory lookup: O(1) via HashMap
- Deep path access: O(depth) - negligible for typical structures (< 3 levels)
- Directory listing: O(n) where n = entries in directory
- Memory: O(total_files) with no path duplication

---

## JSON Contract

### Standard Format

The JSON format MUST have a root object with a `files` key. The optional `metadata` key provides additional metadata that overrides defaults extracted from `Quill.toml`.

```json
{
  "files": {
    "Quill.toml": { "contents": "[Quill]\nname = \"my-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n" },
    "glue.typ": { "contents": "= Template\n\n{{ body }}" },
    "assets": {
      "logo.png": { "contents": [137, 80, 78, 71, ...] }
    }
  }
}
```

### With Optional Metadata

```json
{
  "metadata": {
    "name": "my-quill",
    "base_path": "/custom/path",
    "version": "1.0.0",
    "description": "A beautiful letter template",
    "author": "John Doe",
    "license": "MIT",
    "tags": ["letter", "professional"]
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

### Node Types

**File with UTF-8 string contents:**
```json
"file.txt": { "contents": "Hello, world!" }
```

**File with binary contents (byte array):**
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

### Validation Rules

1. Root MUST be an object with a `files` key
2. The `files` value MUST be an object
3. The `metadata` key is optional
4. All file nodes MUST have a `contents` key with either:
   - A string (UTF-8 text content)
   - An array of numbers 0-255 (binary content)
5. Directory nodes are objects without a `contents` key
6. Empty objects represent empty directories
7. After parsing, `Quill.toml` MUST exist and be valid
8. The glue file referenced in `Quill.toml` MUST exist

### Frontend Example (TypeScript)

```typescript
// Minimal example
const quill = {
  files: {
    "Quill.toml": { contents: "[Quill]\nname = \"my-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n" },
    "glue.typ": { contents: "= Template\n\n{{ body }}" }
  }
};

// With metadata override
const quillWithMetadata = {
  metadata: {
    name: "my-custom-name",
    base_path: "/custom"
  },
  files: {
    "Quill.toml": { contents: quillToml },
    "glue.typ": { contents: glue },
    "assets": {
      "logo.png": { contents: Array.from(logoBytes) }
    }
  }
};

// Build from file uploads
async function buildQuillFromUpload(files: File[]): Promise<object> {
  const fileTree: any = {};

  for (const file of files) {
    const path = file.webkitRelativePath || file.name;
    const parts = path.split('/');
    let current = fileTree;

    for (let i = 0; i < parts.length - 1; i++) {
      if (!current[parts[i]]) current[parts[i]] = {};
      current = current[parts[i]];
    }

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

## Metadata Handling

### Metadata Schema

```rust
pub struct QuillMetadata {
    /// Quill name (required)
    pub name: String,

    /// Base path for asset resolution (optional, defaults to "/")
    pub base_path: Option<PathBuf>,

    /// Semantic version (optional)
    pub version: Option<String>,

    /// Human-readable description (optional)
    pub description: Option<String>,

    /// Author name(s) (optional)
    pub author: Option<String>,

    /// License identifier (optional)
    pub license: Option<String>,

    /// Tags for categorization (optional)
    pub tags: Vec<String>,

    /// Custom metadata (extensibility)
    pub custom: HashMap<String, serde_json::Value>,
}
```

### Metadata Priority (Highest to Lowest)

1. **JSON `metadata` object** - Explicit overrides in JSON
2. **Quill.toml `[Quill]` section** - Metadata from Quill.toml
3. **Function arguments** - `default_name`, `base_path` passed to constructors
4. **Defaults** - Sensible defaults (e.g., `base_path = "/"`)

### Example Priority Resolution

```json
{
  "metadata": {
    "name": "override-name"  // Highest priority
  },
  "files": {
    "Quill.toml": {
      "contents": "[Quill]\nname = \"toml-name\"\n..."  // Medium priority
    }
  }
}
```

Result: `name = "override-name"`

---

## API Surface

### Core Construction APIs

```rust
impl Quill {
    /// Load from filesystem directory
    ///
    /// Recursively reads all files, respecting .quillignore patterns.
    /// Extracts metadata from Quill.toml.
    ///
    /// # Example
    /// ```rust
    /// let quill = Quill::from_path("./templates/letter")?;
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, QuillError>;

    /// Load from in-memory file tree
    ///
    /// This is the canonical constructor - all other methods route through here.
    /// Validates Quill.toml exists and extracts metadata.
    ///
    /// # Example
    /// ```rust
    /// let mut files = HashMap::new();
    /// files.insert("Quill.toml".to_string(), FileTreeNode::File { contents: b"...".to_vec() });
    /// files.insert("glue.typ".to_string(), FileTreeNode::File { contents: b"...".to_vec() });
    /// let root = FileTreeNode::Directory { files };
    ///
    /// let quill = Quill::from_tree(root, None, None)?;
    /// ```
    pub fn from_tree(
        root: FileTreeNode,
        base_path: Option<PathBuf>,
        default_name: Option<String>,
    ) -> Result<Self, QuillError>;

    /// Load from JSON string
    ///
    /// Parses tree-based JSON format with explicit metadata object.
    /// Extracts metadata and builds internal tree structure.
    ///
    /// # Example
    /// ```rust
    /// let json = r#"{
    ///   "files": {
    ///     "Quill.toml": { "contents": "..." },
    ///     "glue.typ": { "contents": "..." }
    ///   }
    /// }"#;
    /// let quill = Quill::from_json(json)?;
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, QuillError>;
}
```

### Data Flow

All loading methods converge to `from_tree`:

```
from_path ──┐
            ├──> from_tree ──> Quill instance
from_json ──┘
```

---

## File Access APIs

```rust
impl Quill {
    /// Check if a file exists
    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool;

    /// Get file contents
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&[u8]>;

    /// Check if directory exists
    pub fn dir_exists<P: AsRef<Path>>(&self, path: P) -> bool;

    /// List files in a directory (non-recursive)
    pub fn list_files<P: AsRef<Path>>(&self, path: P) -> Vec<String>;

    /// List subdirectories in a directory (non-recursive)
    pub fn list_subdirectories<P: AsRef<Path>>(&self, path: P) -> Vec<String>;
}
```

---

## Implementation Guidelines

### Parsing JSON

```rust
pub fn from_json(json_str: &str) -> Result<Self, QuillError> {
    use serde_json::Value as JsonValue;

    let json: JsonValue = serde_json::from_str(json_str)
        .map_err(|e| QuillError::InvalidJson(format!("Failed to parse JSON: {}", e)))?;

    // Root must be an object
    let obj = json.as_object()
        .ok_or_else(|| QuillError::InvalidJson("Root must be an object".to_string()))?;

    // Extract metadata (optional)
    let metadata = obj.get("metadata")
        .map(QuillMetadata::from_json)
        .transpose()?;

    // Extract files (required)
    let files_obj = obj.get("files")
        .and_then(|v| v.as_object())
        .ok_or_else(|| QuillError::InvalidJson("Missing or invalid 'files' key".to_string()))?;

    // Parse file tree
    let mut root_files = HashMap::new();
    for (key, value) in files_obj {
        root_files.insert(key.clone(), FileTreeNode::from_json_value(value)?);
    }

    let root = FileTreeNode::Directory { files: root_files };

    // Create Quill from tree
    Self::from_tree(
        root,
        metadata.as_ref().and_then(|m| m.base_path.clone()),
        metadata.as_ref().map(|m| m.name.clone()),
    )
}
```

### Parsing FileTreeNode

```rust
impl FileTreeNode {
    fn from_json_value(value: &JsonValue) -> Result<Self, QuillError> {
        let obj = value.as_object()
            .ok_or_else(|| QuillError::InvalidJson("Node must be an object".to_string()))?;

        // Check if it's a file (has "contents" key)
        if let Some(contents) = obj.get("contents") {
            let bytes = match contents {
                JsonValue::String(s) => s.as_bytes().to_vec(),
                JsonValue::Array(arr) => {
                    arr.iter()
                        .map(|v| {
                            v.as_u64()
                                .and_then(|n| u8::try_from(n).ok())
                                .ok_or_else(|| QuillError::InvalidJson("Byte array must contain 0-255".to_string()))
                        })
                        .collect::<Result<Vec<u8>, QuillError>>()?
                }
                _ => return Err(QuillError::InvalidJson("contents must be string or byte array".to_string())),
            };
            return Ok(FileTreeNode::File { contents: bytes });
        }

        // Otherwise, it's a directory
        let mut files = HashMap::new();
        for (key, value) in obj {
            files.insert(key.clone(), Self::from_json_value(value)?);
        }
        Ok(FileTreeNode::Directory { files })
    }
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum QuillError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    #[error("Quill.toml not found")]
    QuillTomlNotFound,

    #[error("Invalid Quill.toml: {0}")]
    InvalidQuillToml(String),

    #[error("Glue file not found: {0}")]
    GlueFileNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
}
```

---

## Open Questions

### 1. Path Normalization

**Question**: How to handle edge cases in nested paths (`.`, `..`, absolute paths)?

**Answer**:
- **Reject** absolute paths (starting with `/` or `C:\`)
- **Reject** path traversal (`..` components)
- **Normalize** `.` components (current directory)
- **Validate** no empty path components (e.g., `foo//bar`)

### 2. Empty Directories

**Question**: Should empty directories be explicitly supported?

**Answer**: **Yes**, via empty objects: `"empty_dir": {}`

### 3. Symlinks

**Question**: Support symlinks in file tree?

**Answer**: **No**. Security concerns and complexity. Keep simple.

### 4. File Size Limits

**Question**: Should we enforce max file/total size limits?

**Answer**: **Yes**, add validation:
- Max file size: 50 MB
- Max total Quill size: 200 MB
- Make configurable via feature flags for embedded systems

### 5. Binary Detection

**Question**: Auto-detect binary vs text in JSON?

**Answer**: **No**. Require explicit format:
- String → UTF-8 text
- Byte array → Binary

No magic. No heuristics.

---

## WASM API Surface

```typescript
class Quill {
    /// Create from files object
    static fromFiles(files: object, metadata?: QuillMetadata): Quill;

    /// Create from JSON string
    static fromJson(json: string): Quill;

    /// Validate structure
    validate(): void;

    /// Get metadata
    getMetadata(): QuillMetadata;

    /// List all files (recursive paths)
    listFiles(): string[];

    /// Check if file exists
    fileExists(path: string): boolean;

    /// Get file contents as Uint8Array
    getFile(path: string): Uint8Array | null;

    /// Get file contents as string (UTF-8)
    getFileAsString(path: string): string | null;
}

interface QuillMetadata {
    name: string;
    base_path?: string;
    version?: string;
    description?: string;
    author?: string;
    license?: string;
    tags?: string[];
    custom?: Record<string, any>;
}
```

**Note:** `fromFiles` in WASM converts JS object to JSON internally, then calls Rust `from_json`.

---

## References

- [JSON_CONTRACT.md](../quillmark-core/docs/JSON_CONTRACT.md) - JSON contract specification
- [quillmark-core/src/quill.rs](../quillmark-core/src/quill.rs) - Implementation
