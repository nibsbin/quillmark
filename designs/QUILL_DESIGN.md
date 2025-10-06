# Quill Resource File Tree/Structure and API Design

> **Status**: Design Phase - Formalizing Quill API Surface and Internal Structure
>
> This document analyzes the current Quill file structure, API surface, and proposes a unified, robust design for creating and managing Quill template bundles.

---

## Table of Contents

1. [Current State Analysis](#current-state-analysis)
2. [Problem Statement](#problem-statement)
3. [Design Goals](#design-goals)
4. [Internal File Structure: HashMap vs Tree](#internal-file-structure-hashmap-vs-tree)
5. [JSON Structure: Tree-Based Only](#json-structure-tree-based-only)
6. [Proposed Unified Design](#proposed-unified-design)
7. [API Surface Enumeration](#api-surface-enumeration)
8. [Metadata Handling](#metadata-handling)
9. [Implementation Recommendations](#implementation-recommendations)
10. [Migration Path](#migration-path)

---

## Current State Analysis

### Existing APIs (Rust)

The current implementation provides three primary loading methods:

```rust
impl Quill {
    /// Load from filesystem directory
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn StdError + Send + Sync>>;
    
    /// Load from in-memory file tree structure
    pub fn from_tree(
        root: FileTreeNode,
        base_path: Option<PathBuf>,
        default_name: Option<String>,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>>;
    
    /// Load from JSON string representation
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn StdError + Send + Sync>>;
}
```

### Current Internal Structure

**FileTreeNode** (Tree-based with HashMap children):
```rust
pub enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}
```

This is a **hybrid approach**: tree structure with HashMap storage at each level for O(1) lookups.

### Current JSON Contract

The JSON format currently mixes metadata and files at the root level:

```json
{
  "name": "my-quill",           // METADATA (reserved key)
  "base_path": "/",             // METADATA (reserved key)
  "Quill.toml": { "contents": "..." },  // FILE
  "glue.typ": { "contents": "..." },    // FILE
  "assets": {                           // DIRECTORY
    "logo.png": { "contents": [...] }
  }
}
```

**Issues:**
- Reserved keys (`name`, `base_path`) mixed with file entries
- Fragile: adding new metadata requires checking for name conflicts
- Not robust for extensibility (e.g., adding `version`, `author`, etc.)

### Data Flow

Current architecture routes all loading methods through `from_tree`:

```
from_path ──┐
            ├──> from_tree ──> Quill instance
from_json ──┘
```

This is elegant but `from_json` has the metadata mixing issue.

---

## Problem Statement

The current design has several issues:

1. **Metadata Pollution**: JSON structure mixes metadata with file entries at root level
2. **API Confusion**: Three different entry points with unclear relationships
3. **Frontend Complexity**: Unclear which JSON structure (table vs tree) is better for web frontends
4. **Inconsistent Structure**: Internal representation differs from JSON representation
5. **Limited Extensibility**: Hard to add new metadata fields without breaking compatibility

**Requirements:**
1. Load from filesystem directory (existing `from_path`)
2. Load from JSON structure that emulates a Quill folder
3. Both routes converge to a common internal structure
4. Robust metadata handling (separate from files)
5. Frontend-friendly JSON format
6. Fast file lookups (HashMap or equivalent)

---

## Design Goals

1. **Unified Internal Representation**: Single canonical structure for all loading paths
2. **Robust Metadata**: Metadata completely separate from file tree
3. **Frontend-Friendly**: JSON structure easy to construct in JavaScript/TypeScript
4. **Fast Lookups**: O(1) file access for rendering performance
5. **Extensible**: Easy to add new metadata fields without breaking changes
6. **Clear API**: Obvious which method to use for each use case
7. **Backward Compatible**: Smooth migration path from current JSON format

---

## Internal File Structure: HashMap vs Tree

### Analysis

**Current Hybrid (Tree + HashMap per level):**
```rust
enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}
```

**Pros:**
- O(1) lookup at each directory level
- Natural filesystem representation
- Supports directory operations (list, exists)
- Memory efficient (no duplicate path storage)

**Cons:**
- Multi-component path requires traversal
- More complex than flat HashMap

**Alternative: Flat HashMap with path keys:**
```rust
struct FileTree {
    files: HashMap<PathBuf, Vec<u8>>,  // or HashMap<String, Vec<u8>>
}
```

**Pros:**
- Single O(1) lookup for any path
- Simpler implementation
- Faster for deep paths

**Cons:**
- No directory concept (need to synthesize)
- More memory (paths stored redundantly)
- Harder to implement "list directory" operations
- Path normalization complexity (separators, `.`, `..`)

### Recommendation: **Keep Tree + HashMap Hybrid**

**Rationale:**
1. **Directory operations needed**: Quill uses `list_files()`, `dir_exists()`, etc.
2. **Typical paths are shallow**: Most Quills have 1-2 levels max
3. **Memory efficiency**: No redundant path storage
4. **Clear semantics**: Directory vs file is explicit
5. **Matches filesystem**: Natural mental model

**Performance**: For typical Quill structures (< 100 files, < 3 levels deep), traversal cost is negligible compared to I/O and rendering.

---

## JSON Structure: Tree-Based Only

### Tree-Based Format

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
      "logo.png": { "contents": [...] }
    }
  }
}
```

**Benefits:**
- Natural nesting matches directory structure
- Easy to construct in JS: mirrors filesystem
- Clear directory boundaries
- Intuitive for developers
- Explicit structure at a glance

**Tradeoffs:**
- Deep nesting for deep paths (acceptable for typical Quill structures)
- Requires recursion for parsing (standard approach for tree structures)

### Frontend Considerations

**JavaScript/TypeScript code to build tree-based format:**

```typescript
const quill = {
  metadata: { name: "my-quill", base_path: "/" },
  files: {
    "Quill.toml": { contents: quillTomlText },
    "glue.typ": { contents: glueText },
    "assets": {
      "logo.png": { contents: Array.from(logoBytes) }
    }
  }
};
```

**For programmatic generation from file uploads:**
```typescript
// Helper to build nested structure from flat file list
function buildFileTree(files: File[]): object {
  const tree: any = {};
  
  for (const file of files) {
    const path = file.webkitRelativePath || file.name;
    const parts = path.split('/');
    let current = tree;
    
    for (let i = 0; i < parts.length - 1; i++) {
      if (!current[parts[i]]) current[parts[i]] = {};
      current = current[parts[i]];
    }
    
    current[parts[parts.length - 1]] = { 
      contents: isImage(file) 
        ? Array.from(new Uint8Array(await file.arrayBuffer()))
        : await file.text()
    };
  }
  
  return tree;
}
```

### Recommendation: **Tree-Based Only**

**Rationale:**
- Matches mental model of filesystem
- Natural for both hand-crafting and programmatic generation
- Consistent with internal `FileTreeNode` structure
- Simpler implementation without format detection
- Reduces API surface and complexity

---

## Proposed Unified Design

### Core Principles

1. **Metadata is separate**: Never mixed with file entries
2. **Common structure**: All loading paths produce identical internal state
3. **Multiple JSON formats**: Support both tree and table, auto-detect
4. **Explicit validation**: Metadata extraction and validation in one place

### Updated JSON Contracts

#### Tree-Based Format (Standard)

```json
{
  "metadata": {
    "name": "my-quill",
    "base_path": "/",
    "version": "1.0.0",
    "description": "A beautiful letter template",
    "author": "John Doe"
  },
  "files": {
    "Quill.toml": { "contents": "[Quill]\n..." },
    "glue.typ": { "contents": "= Template\n..." },
    "assets": {
      "logo.png": { "contents": [137, 80, 78, 71, ...] }
    }
  }
}
```

**Rules:**
- Root must have `files` key (object)
- `metadata` is optional (defaults extracted from `Quill.toml`)
- Files object mirrors directory structure
- File nodes have `contents` (string or byte array)
- Directory nodes are nested objects

#### Legacy Format (Deprecated, Backward Compatibility)

```json
{
  "name": "my-quill",
  "base_path": "/",
  "Quill.toml": { "contents": "..." },
  "glue.typ": { "contents": "..." }
}
```

**Rules:**
- Root level mixes metadata and files (current format)
- Reserved keys: `name`, `base_path` (treated as metadata)
- Everything else is a file/directory
- Deprecated but supported for migration

### Internal Structure (Unchanged)

```rust
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

pub enum FileTreeNode {
    File { contents: Vec<u8> },
    Directory { files: HashMap<String, FileTreeNode> },
}
```

**No changes needed** - structure already optimal.

---

## API Surface Enumeration

### Core Creation APIs

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
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn StdError + Send + Sync>>;
    
    /// Load from in-memory file tree
    /// 
    /// This is the canonical constructor - all other methods route through here.
    /// Validates Quill.toml exists and extracts metadata.
    /// 
    /// # Example
    /// ```rust
    /// let mut files = HashMap::new();
    /// files.insert("Quill.toml".to_string(), FileTreeNode::File { ... });
    /// files.insert("glue.typ".to_string(), FileTreeNode::File { ... });
    /// let root = FileTreeNode::Directory { files };
    /// 
    /// let quill = Quill::from_tree(root, None, None)?;
    /// ```
    pub fn from_tree(
        root: FileTreeNode,
        base_path: Option<PathBuf>,
        default_name: Option<String>,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>>;
    
    /// Load from JSON string
    /// 
    /// Parses tree-based JSON format with explicit metadata object.
    /// Extracts metadata and builds internal tree structure.
    /// 
    /// # Example
    /// ```rust
    /// let json = r#"{
    ///   "metadata": { "name": "letter" },
    ///   "files": {
    ///     "Quill.toml": { "contents": "..." },
    ///     "glue.typ": { "contents": "..." }
    ///   }
    /// }"#;
    /// let quill = Quill::from_json(json)?;
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn StdError + Send + Sync>>;
}
```

### Builder Pattern (Future Enhancement)

For programmatic construction without JSON overhead:

```rust
impl Quill {
    /// Create a new builder
    pub fn builder() -> QuillBuilder;
}

pub struct QuillBuilder {
    files: HashMap<String, FileTreeNode>,
    metadata: QuillMetadata,
}

impl QuillBuilder {
    /// Set metadata
    pub fn metadata(mut self, metadata: QuillMetadata) -> Self;
    
    /// Add a file
    pub fn file(mut self, path: impl AsRef<Path>, contents: Vec<u8>) -> Self;
    
    /// Add a text file
    pub fn text_file(mut self, path: impl AsRef<Path>, contents: impl AsRef<str>) -> Self;
    
    /// Add a directory
    pub fn directory(mut self, path: impl AsRef<Path>) -> Self;
    
    /// Build the Quill
    pub fn build(self) -> Result<Quill, Box<dyn StdError + Send + Sync>>;
}
```

### File Access APIs (Existing)

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

### WASM API Surface

Matching the WASM design document:

```typescript
class Quill {
    /// Create from file map (wraps from_json internally)
    static fromFiles(files: object, metadata?: QuillMetadata): Quill;
    
    /// Create from JSON string directly
    static fromJson(json: string): Quill;
    
    /// Validate structure
    validate(): void;
    
    /// Get metadata
    getMetadata(): QuillMetadata;
    
    /// List all files (recursive paths)
    listFiles(): string[];
}
```

**Note:** `fromFiles` in WASM converts JS object to JSON internally, then calls Rust `from_json`.

---

## Metadata Handling

### Problem with Current Approach

Metadata is currently mixed with file entries in JSON:
```json
{
  "name": "my-quill",        // metadata
  "base_path": "/",          // metadata
  "Quill.toml": { ... },     // file
  "glue.typ": { ... }        // file
}
```

This is fragile and limits extensibility.

### Proposed Solution: Explicit Metadata Object

```json
{
  "metadata": {
    "name": "my-quill",
    "base_path": "/",
    "version": "1.0.0",
    "description": "Letter template",
    "author": "John Doe",
    "license": "MIT",
    "tags": ["letter", "professional"],
    "custom": {
      "company": "Acme Corp"
    }
  },
  "files": { ... }
}
```

### Metadata Schema

```rust
pub struct QuillMetadata {
    /// Quill name (from metadata or Quill.toml)
    pub name: String,
    
    /// Base path for asset resolution (optional)
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

### Metadata Priority

When creating a Quill, metadata comes from multiple sources:

1. **JSON metadata object** (highest priority)
2. **Quill.toml `[Quill]` section** (medium priority)
3. **Function arguments** (`default_name`, `base_path`) (lowest priority)

### Backward Compatibility

Support legacy format by detecting reserved keys at root:

```rust
fn parse_json_metadata(json: &JsonValue) -> QuillMetadata {
    // Prefer explicit metadata object
    if let Some(metadata_obj) = json.get("metadata") {
        return QuillMetadata::from_json(metadata_obj);
    }
    
    // Fall back to legacy root-level keys
    let mut metadata = QuillMetadata::default();
    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
        metadata.name = name.to_string();
    }
    if let Some(base_path) = json.get("base_path").and_then(|v| v.as_str()) {
        metadata.base_path = Some(PathBuf::from(base_path));
    }
    metadata
}
```

---

## Implementation Recommendations

### Phase 1: Enhance JSON Parsing (Backward Compatible)

1. Support explicit `metadata` object in JSON
2. Parse tree-based `files` object format
3. Maintain legacy format support (root-level reserved keys)

### Phase 2: Deprecate Legacy Format

1. Add deprecation warnings for legacy format
2. Update documentation to use new format
3. Provide migration tool/script

### Phase 3: Remove Legacy Support (Major Version)

1. Remove legacy format parsing
2. Simplify code by removing backward compatibility

### Implementation Details

```rust
pub fn from_json(json_str: &str) -> Result<Self, Box<dyn StdError + Send + Sync>> {
    use serde_json::Value as JsonValue;
    
    let json: JsonValue = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    // Extract metadata
    let metadata = extract_metadata(&json)?;
    
    // Parse files based on format
    let root_files = if has_files_object(&json) {
        parse_tree_based_files(&json)?
    } else {
        parse_legacy_format(&json)?
    };
    
    let root = FileTreeNode::Directory { files: root_files };
    
    // Create Quill from tree
    Self::from_tree(
        root,
        metadata.base_path.clone(),
        Some(metadata.name.clone()),
    )
}

fn has_files_object(json: &JsonValue) -> bool {
    json.get("files").map(|v| v.is_object()).unwrap_or(false)
}

fn parse_tree_based_files(json: &JsonValue) -> Result<HashMap<String, FileTreeNode>, Box<dyn StdError>> {
    let files_obj = json.get("files")
        .and_then(|v| v.as_object())
        .ok_or("files must be an object")?;
    
    let mut root_files = HashMap::new();
    
    for (key, value) in files_obj {
        root_files.insert(key.clone(), FileTreeNode::from_json_value(value)?);
    }
    
    Ok(root_files)
}
```

---

## Migration Path

### For Rust Users

**Before (current):**
```rust
let json = r#"{
  "name": "letter",
  "Quill.toml": { "contents": "..." },
  "glue.typ": { "contents": "..." }
}"#;
let quill = Quill::from_json(json)?;
```

**After (recommended):**
```rust
let json = r#"{
  "metadata": { "name": "letter" },
  "files": {
    "Quill.toml": { "contents": "..." },
    "glue.typ": { "contents": "..." }
  }
}"#;
let quill = Quill::from_json(json)?;
```

**Both work** during transition period.

### For JavaScript/WASM Users

**Before:**
```typescript
const quillData = {
  name: "letter",
  "Quill.toml": { contents: quillToml },
  "glue.typ": { contents: glue }
};
const quill = Quill.fromJson(JSON.stringify(quillData));
```

**After:**
```typescript
const quillData = {
  metadata: { name: "letter" },
  files: {
    "Quill.toml": { contents: quillToml },
    "glue.typ": { contents: glue }
  }
};
const quill = Quill.fromJson(JSON.stringify(quillData));
```

### Deprecation Timeline

1. **v0.1.0**: Introduce new formats, maintain legacy support
2. **v0.2.0**: Add deprecation warnings for legacy format
3. **v1.0.0**: Remove legacy format support

---

## Summary and Recommendations

### Key Decisions

1. **Internal Structure**: Keep tree + HashMap hybrid
   - Provides O(1) per-level lookup with directory semantics
   - Optimal for typical Quill structures

2. **JSON Format**: Tree-based only
   - Natural structure that mirrors filesystem
   - Intuitive for both hand-crafting and programmatic generation
   - Consistent with internal `FileTreeNode` structure
   - Simpler implementation without format detection

3. **Metadata**: Explicit `metadata` object in JSON
   - Robust, extensible, no name conflicts
   - Backward compatible with legacy root-level keys

4. **API Surface**: Three clear entry points
   - `from_path`: Load from filesystem
   - `from_tree`: Canonical constructor (internal)
   - `from_json`: Load from JSON (tree-based format)

### Implementation Priority

**High Priority (v0.1.0):**
- [ ] Add `metadata` object support to `from_json`
- [ ] Parse tree-based `files` object format
- [ ] Update JSON_CONTRACT.md with new format

**Medium Priority (v0.2.0):**
- [ ] Add deprecation warnings for legacy format
- [ ] Create migration guide
- [ ] Add `QuillBuilder` for programmatic construction

**Low Priority (Future):**
- [ ] Remove legacy format support (v1.0.0)
- [ ] Add validation helpers for metadata schema
- [ ] Support YAML format for Quill definitions

### Frontend Guidance

**For web applications:**
- Use tree-based format for all Quill creation
- Always use explicit `metadata` object
- Build nested structure for file uploads using helper function

**Example (file upload handler):**
```typescript
async function buildQuillFromUpload(files: File[]): Promise<Quill> {
  // Build nested file tree from flat file list
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
    const isImage = /\.(png|jpg|jpeg|gif)$/i.test(fileName);
    
    current[fileName] = {
      contents: isImage
        ? Array.from(new Uint8Array(await file.arrayBuffer()))
        : await file.text()
    };
  }
  
  const quillData = {
    metadata: {
      name: files[0]?.webkitRelativePath?.split('/')[0] || 'uploaded-quill'
    },
    files: fileTree
  };
  
  return Quill.fromJson(JSON.stringify(quillData));
}
```

---

## Open Questions

1. **Path normalization**: How to handle edge cases in nested paths?
   - **Recommendation**: Validate against path traversal (no `..`, no absolute paths)
   
2. **Empty directories**: Should tree-based format support explicit empty dirs?
   - **Recommendation**: Yes, via empty object: `"empty_dir": {}`
   
3. **Symlinks**: Support in file tree?
   - **Recommendation**: No, keep simple for security

4. **Max file size**: Limit for in-memory Quills?
   - **Recommendation**: Yes, add validation (e.g., 50MB per file, 200MB total)

5. **Binary detection**: Auto-detect binary vs text in JSON?
   - **Recommendation**: No, explicit format required (string or byte array)

---

## References

- [JSON_CONTRACT.md](../quillmark-core/docs/JSON_CONTRACT.md) - Current JSON contract
- [WASM_API.md](./WASM_API.md) - WASM API design
- [DESIGN.md](./DESIGN.md) - Overall architecture
- [quillmark-core/src/quill.rs](../quillmark-core/src/quill.rs) - Implementation
