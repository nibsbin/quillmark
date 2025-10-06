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
5. [JSON Structure: Table vs Tree-Based](#json-structure-table-vs-tree-based)
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

## JSON Structure: Table vs Tree-Based

### Option 1: Tree-Based (Current)

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

**Pros:**
- Natural nesting matches directory structure
- Easy to construct in JS: mirrors filesystem
- Clear directory boundaries
- Intuitive for developers

**Cons:**
- Deep nesting for deep paths
- Harder to programmatically generate (nested object creation)
- Parsing requires recursion

### Option 2: Table-Based (Flat)

```json
{
  "metadata": {
    "name": "my-quill",
    "base_path": "/",
    "version": "1.0.0"
  },
  "files": [
    { "path": "Quill.toml", "contents": "..." },
    { "path": "glue.typ", "contents": "..." },
    { "path": "assets/logo.png", "contents": [...] }
  ]
}
```

**Pros:**
- Easy to generate programmatically (map over array)
- No recursion needed for parsing
- Simpler for table-driven UIs
- Easy to filter/search

**Cons:**
- Less intuitive (doesn't mirror filesystem)
- Path separator issues (Unix `/` vs Windows `\`)
- No explicit directory entries
- Harder to see structure at a glance

### Frontend Considerations

**JavaScript/TypeScript code to build each:**

**Tree-based:**
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

**Table-based:**
```typescript
const quill = {
  metadata: { name: "my-quill", base_path: "/" },
  files: [
    { path: "Quill.toml", contents: quillTomlText },
    { path: "glue.typ", contents: glueText },
    { path: "assets/logo.png", contents: Array.from(logoBytes) }
  ]
};
```

**Verdict:** Tree-based is slightly more verbose but more intuitive. However, for programmatic generation (e.g., from file uploads), table-based is easier.

### Recommendation: **Support Both, Default to Tree**

**Primary (Tree-based):**
- Default for documentation and examples
- Natural for hand-crafting
- Better developer experience

**Secondary (Table-based):**
- Optional format for programmatic generation
- Easier for tools that scan filesystems
- Better for large Quills with many files

**Implementation:** Detect format automatically:
```rust
if json.contains_key("files") && json["files"].is_array() {
    // Table-based
} else {
    // Tree-based (current behavior)
}
```

---

## Proposed Unified Design

### Core Principles

1. **Metadata is separate**: Never mixed with file entries
2. **Common structure**: All loading paths produce identical internal state
3. **Multiple JSON formats**: Support both tree and table, auto-detect
4. **Explicit validation**: Metadata extraction and validation in one place

### Updated JSON Contracts

#### Tree-Based Format (Primary)

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

#### Table-Based Format (Alternative)

```json
{
  "metadata": {
    "name": "my-quill",
    "base_path": "/",
    "version": "1.0.0"
  },
  "files": [
    { "path": "Quill.toml", "contents": "[Quill]\n..." },
    { "path": "glue.typ", "contents": "= Template\n..." },
    { "path": "assets/logo.png", "contents": [137, 80, 78, 71, ...] }
  ]
}
```

**Rules:**
- Root must have `files` key (array)
- Each entry has `path` and `contents`
- Paths use `/` separator (Unix-style)
- Directories inferred from paths (no explicit entries needed)

#### Legacy Format (Backward Compatibility)

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
    /// Auto-detects format (tree-based, table-based, or legacy).
    /// Extracts metadata and builds internal tree structure.
    /// 
    /// # Example (tree-based)
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
    /// 
    /// # Example (table-based)
    /// ```rust
    /// let json = r#"{
    ///   "metadata": { "name": "letter" },
    ///   "files": [
    ///     { "path": "Quill.toml", "contents": "..." },
    ///     { "path": "glue.typ", "contents": "..." }
    ///   ]
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
2. Support table-based `files` array format
3. Auto-detect format and parse accordingly
4. Maintain legacy format support (root-level reserved keys)

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
    let root_files = if has_files_array(&json) {
        parse_table_based_files(&json)?
    } else if has_files_object(&json) {
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

fn has_files_array(json: &JsonValue) -> bool {
    json.get("files").map(|v| v.is_array()).unwrap_or(false)
}

fn has_files_object(json: &JsonValue) -> bool {
    json.get("files").map(|v| v.is_object()).unwrap_or(false)
}

fn parse_table_based_files(json: &JsonValue) -> Result<HashMap<String, FileTreeNode>, Box<dyn StdError>> {
    let files_array = json.get("files")
        .and_then(|v| v.as_array())
        .ok_or("files must be an array")?;
    
    let mut root_files = HashMap::new();
    
    for file_entry in files_array {
        let path = file_entry.get("path")
            .and_then(|v| v.as_str())
            .ok_or("file entry must have 'path' field")?;
        
        let contents_value = file_entry.get("contents")
            .ok_or("file entry must have 'contents' field")?;
        
        let contents = parse_contents(contents_value)?;
        
        // Insert into tree at correct path
        insert_at_path(&mut root_files, path, contents)?;
    }
    
    Ok(root_files)
}

fn insert_at_path(
    root: &mut HashMap<String, FileTreeNode>,
    path: &str,
    contents: Vec<u8>,
) -> Result<(), Box<dyn StdError>> {
    let parts: Vec<&str> = path.split('/').collect();
    
    if parts.is_empty() {
        return Err("empty path".into());
    }
    
    if parts.len() == 1 {
        // File at root
        root.insert(parts[0].to_string(), FileTreeNode::File { contents });
        return Ok(());
    }
    
    // Navigate/create directories
    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last component - insert file
            current.insert(part.to_string(), FileTreeNode::File { contents });
        } else {
            // Intermediate directory
            current = match current.entry(part.to_string()) {
                Entry::Occupied(e) => match e.into_mut() {
                    FileTreeNode::Directory { files } => files,
                    FileTreeNode::File { .. } => {
                        return Err(format!("path conflict: {} is a file", part).into())
                    }
                },
                Entry::Vacant(e) => {
                    let new_dir = FileTreeNode::Directory {
                        files: HashMap::new(),
                    };
                    match e.insert(new_dir) {
                        FileTreeNode::Directory { files } => files,
                        _ => unreachable!(),
                    }
                }
            };
        }
    }
    
    Ok(())
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

**After (tree-based):**
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

**After (table-based, for programmatic generation):**
```typescript
const quillData = {
  metadata: { name: "letter" },
  files: [
    { path: "Quill.toml", contents: quillToml },
    { path: "glue.typ", contents: glue },
    { path: "assets/logo.png", contents: Array.from(logoBytes) }
  ]
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

2. **JSON Format**: Support both tree-based and table-based
   - **Tree-based (primary)**: Natural, mirrors filesystem, better DX
   - **Table-based (secondary)**: Easier for programmatic generation
   - Auto-detect format in `from_json`

3. **Metadata**: Explicit `metadata` object in JSON
   - Robust, extensible, no name conflicts
   - Backward compatible with legacy root-level keys

4. **API Surface**: Three clear entry points
   - `from_path`: Load from filesystem
   - `from_tree`: Canonical constructor (internal)
   - `from_json`: Load from JSON (auto-detects format)

### Implementation Priority

**High Priority (v0.1.0):**
- [ ] Add `metadata` object support to `from_json`
- [ ] Implement table-based format parsing
- [ ] Auto-detect format in `from_json`
- [ ] Update JSON_CONTRACT.md with new formats

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
- Use **tree-based** format for hand-crafted templates
- Use **table-based** format when building from file uploads
- Always use explicit `metadata` object

**Example (file upload handler):**
```typescript
async function buildQuillFromUpload(files: File[]): Promise<Quill> {
  const fileEntries = await Promise.all(
    files.map(async (file) => ({
      path: file.webkitRelativePath || file.name,
      contents: file.name.endsWith('.png') || file.name.endsWith('.jpg')
        ? Array.from(new Uint8Array(await file.arrayBuffer()))
        : await file.text()
    }))
  );
  
  const quillData = {
    metadata: {
      name: files[0]?.webkitRelativePath?.split('/')[0] || 'uploaded-quill'
    },
    files: fileEntries
  };
  
  return Quill.fromJson(JSON.stringify(quillData));
}
```

---

## Open Questions

1. **Path normalization**: Should table-based format allow Windows paths (`\`)?
   - **Recommendation**: No, enforce Unix-style `/` for portability
   
2. **Empty directories**: Should table-based format support explicit empty dirs?
   - **Recommendation**: No, directories inferred from file paths
   
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
