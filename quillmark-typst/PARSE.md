# Markdown Parsing and Decomposition

This document details the design and implementation of markdown document decomposition in Quillmark, implemented in `quillmark-core/src/parse.rs`.

## Overview

Quillmark uses a **frontmatter-aware markdown parser** that separates YAML metadata from document content. This enables template-driven document generation where frontmatter fields (like title, author, date) can be processed independently from the markdown body.

**Key capabilities:**
- Parse YAML frontmatter delimited by `---` markers
- Extract frontmatter fields into a structured `HashMap`
- Preserve markdown body content separately
- Graceful error handling for malformed documents

## Design Principles

### 1. Separation of Concerns

The parser decomposes markdown documents into two distinct components:

- **Frontmatter fields**: YAML key-value pairs accessible via `HashMap<String, serde_yaml::Value>`
- **Body content**: Raw markdown text stored under the reserved `BODY_FIELD` constant

This separation allows:
- Template engines to access metadata directly
- Backend converters to process only the markdown body
- Independent validation of structure vs. content

### 2. Error Handling Strategy

The implementation follows a **strict error reporting** approach:

- **Invalid YAML**: Returns an error with descriptive message
- **Unclosed frontmatter**: Returns an error if `---` opening marker exists but closing marker is missing
- **No frontmatter**: Gracefully treats entire content as body (not an error)

This differs from the "graceful degradation" approach mentioned in `DESIGN.md` - the current implementation prefers **fail-fast** for malformed YAML to prevent silent data corruption.

### 3. YAML-Only Policy

Only YAML frontmatter is supported - no TOML or JSON alternatives. This constraint:
- Simplifies implementation
- Provides consistency across documents
- Allows backends to convert to their native formats via filters

## Core Data Structures

### ParsedDocument

```rust
pub struct ParsedDocument {
    fields: HashMap<String, serde_yaml::Value>,
}
```

**Purpose**: Container for both frontmatter fields and document body.

**Design rationale**: 
- Single `HashMap` stores all document data uniformly
- Body is stored under special `BODY_FIELD = "body"` constant
- Private fields enforce access through validated methods

**Public API:**
- `new(fields)` - Constructor accepting pre-populated field map
- `body()` - Returns `Option<&str>` for document body
- `get_field(name)` - Returns `Option<&serde_yaml::Value>` for any field
- `fields()` - Returns reference to entire field map

### BODY_FIELD Constant

```rust
pub const BODY_FIELD: &str = "body";
```

Using a constant prevents typos and makes the special "body" field discoverable through documentation.

## Implementation Details

### The `decompose` Function

**Signature:**
```rust
pub fn decompose(
    markdown: &str,
) -> Result<ParsedDocument, Box<dyn std::error::Error + Send + Sync>>
```

**Algorithm:**

1. **Frontmatter detection**
   - Check if document starts with `"---\n"` 
   - Note: Current implementation only checks Unix line endings

2. **Frontmatter extraction** (if detected)
   - Skip opening `"---\n"` (first 4 characters)
   - Search for closing `"\n---\n"` delimiter
   - Extract text between delimiters as YAML

3. **YAML parsing**
   - Parse frontmatter string with `serde_yaml::from_str`
   - Return descriptive error if YAML is malformed
   - Validate that result is a flat key-value map

4. **Body extraction**
   - Take remaining content after closing `---\n`
   - Preserve all whitespace (including leading newline)
   - Store under `BODY_FIELD` in the same HashMap

5. **Fallback for no frontmatter**
   - If no opening `---` found, treat entire input as body
   - Still wrap in `ParsedDocument` with single `BODY_FIELD` entry

### String Slice Management

The implementation uses careful string slicing to avoid copying:

```rust
let rest = &markdown[4..];                    // Skip "---\n"
let frontmatter = &rest[..end_pos];           // YAML content
let body = &rest[end_pos + 5..];              // Skip "\n---\n"
```

**Offset calculations:**
- Opening delimiter `"---\n"` = 4 bytes
- Closing delimiter `"\n---\n"` = 5 bytes
- Body starts at `end_pos + 5` relative to `rest`

### Error Messages

The implementation provides specific error messages:

- `"Invalid YAML frontmatter: {error}"` - YAML parser rejected the frontmatter
- `"Frontmatter started but not closed with ---"` - Missing closing delimiter

## Edge Cases and Behavior

### 1. Empty Frontmatter

```markdown
---
---

Body content here
```

**Behavior**: Returns empty frontmatter map with body starting at first blank line.

### 2. No Frontmatter

```markdown
# Just a heading

No metadata here
```

**Behavior**: Entire content becomes body under `BODY_FIELD`.

### 3. Unclosed Frontmatter

```markdown
---
title: Test

More content
```

**Behavior**: Returns error - explicit failure prevents ambiguous interpretation.

### 4. Nested YAML Structures

```markdown
---
title: Complex
metadata:
  version: 1.0
  nested:
    field: value
tags:
  - one
  - two
---

Body
```

**Behavior**: Full YAML support including:
- Nested maps (accessed via `serde_yaml::Value` API)
- Sequences/arrays
- All YAML scalar types (strings, numbers, booleans)

### 5. Line Ending Considerations

**Current limitation**: Only Unix line endings (`\n`) are checked.

**Potential issue**: Windows-style frontmatter (`---\r\n`) won't be recognized.

**DESIGN.md recommendation**: Check both `"---\n"` and `"---\r\n"` for cross-platform compatibility (not yet implemented).

## Usage Examples

### Basic Usage

```rust
use quillmark_core::{decompose, BODY_FIELD};

let markdown = r#"---
title: My Document
author: John Doe
---

# Introduction

Document content here.
"#;

let doc = decompose(markdown)?;

// Access frontmatter
let title = doc.get_field("title")
    .and_then(|v| v.as_str())
    .unwrap_or("Untitled");

// Access body
let body = doc.body().unwrap_or("");

// Access all fields
for (key, value) in doc.fields() {
    println!("{}: {:?}", key, value);
}
```

### Error Handling

```rust
match decompose(markdown) {
    Ok(doc) => {
        // Process successfully parsed document
        println!("Body: {}", doc.body().unwrap_or(""));
    }
    Err(e) => {
        // Handle parse error
        eprintln!("Parse error: {}", e);
    }
}
```

### Integration with Workflow

Within the Quillmark rendering pipeline:

```rust
// Step 1: Parse markdown into structured document
let parsed = decompose(markdown)?;

// Step 2: Setup template engine with backend-specific filters
let mut glue = Glue::new(&quill.glue_template)?;
backend.register_filters(&mut glue);

// Step 3: Compose glue source using parsed fields
let glue_source = glue.compose(parsed.fields().clone())?;

// Step 4: Compile to final output format
let artifacts = backend.compile(&glue_source, &quill, &opts)?;
```

## Testing Strategy

The test suite in `parse.rs` covers:

### 1. Normal Cases
- `test_no_frontmatter` - Document without any YAML metadata
- `test_with_frontmatter` - Standard frontmatter with title and author
- `test_complex_yaml_frontmatter` - Nested structures and arrays

### 2. Error Cases
- `test_invalid_yaml` - Malformed YAML syntax
- `test_unclosed_frontmatter` - Missing closing delimiter

### 3. Validation Approach

Tests verify:
- **Structural integrity**: Field count matches expected values
- **Field access**: Individual fields can be retrieved correctly
- **Body separation**: Body content excludes frontmatter
- **Error messages**: Errors contain expected keywords
- **Type preservation**: YAML types (strings, sequences, maps) are accessible

### Example Test Pattern

```rust
#[test]
fn test_with_frontmatter() {
    let markdown = r#"---
title: Test Document
author: Test Author
---

# Hello World

This is the body."#;

    let doc = decompose(markdown).unwrap();

    // Validate body extraction
    assert_eq!(doc.body(), Some("\n# Hello World\n\nThis is the body."));
    
    // Validate frontmatter fields
    assert_eq!(
        doc.get_field("title").unwrap().as_str().unwrap(),
        "Test Document"
    );
    
    // Validate total field count (frontmatter + body)
    assert_eq!(doc.fields().len(), 3); // title, author, body
}
```

## Implementation Divergence from DESIGN.md

The current implementation differs from `DESIGN.md` recommendations in several ways:

### 1. Line Ending Support
- **DESIGN.md**: Check both `\n` and `\r\n`
- **Current**: Only checks `\n`
- **Impact**: Windows users may encounter issues

### 2. Error Handling Philosophy
- **DESIGN.md**: "Graceful degradation" - treat errors as warnings
- **Current**: Fail-fast with explicit errors
- **Rationale**: Prevents silent data corruption from malformed YAML

### 3. Delimiter Search Strategy
- **DESIGN.md**: Line-by-line iteration with `line.trim() == "---"`
- **Current**: String search for `"\n---\n"` substring
- **Trade-off**: Current approach is simpler but less flexible

### 4. Body Whitespace Handling
- **DESIGN.md**: Use `trim_start()` to remove leading whitespace
- **Current**: Preserves all whitespace including leading newline
- **Impact**: Body may start with `\n` when frontmatter is present

## Extended YAML Metadata Standard (Proposed)

This section formalizes a proposed extension to the current frontmatter-only approach, allowing **inline metadata sections** throughout the document body. This feature is **NOT YET IMPLEMENTED** and serves as a design specification for future development.

### Motivation

The current single-frontmatter design limits documents to a flat metadata structure at the beginning. Many use cases require:
- **Structured sub-documents**: Breaking content into logical sections with their own metadata
- **Repeated elements**: Collections of similar items (e.g., multiple products, blog posts, or chapters)
- **Hierarchical content**: Documents that naturally contain nested structures

### Design Overview

The extended standard allows metadata blocks to appear anywhere in the document using a **tag directive** syntax:

```markdown
---
title: Global Metadata
author: John Doe
---

This is the main document body.

---
!sub_documents
title: First Sub-Document
tags: [example, demo]
---

Body of the *first sub-document* with **markdown formatting**.

---
!sub_documents
title: Second Sub-Document
tags: [test]
---

Body of the second sub-document.
```

**Resulting structure:**

```json
{
  "title": "Global Metadata",
  "author": "John Doe",
  "body": "This is the main document body.",
  "sub_documents": [
    {
      "title": "First Sub-Document",
      "tags": ["example", "demo"],
      "body": "Body of the *first sub-document* with **markdown formatting**."
    },
    {
      "title": "Second Sub-Document",
      "tags": ["test"],
      "body": "Body of the second sub-document."
    }
  ]
}
```

### Syntax Specification

#### 1. Tag Directive Format

**Grammar:**
```
metadata_block ::= "---" NEWLINE tag_directive? yaml_content "---" NEWLINE body_content
tag_directive ::= "!" attribute_name NEWLINE
attribute_name ::= [a-z_][a-z0-9_]*
```

**Rules:**
- Tag directive MUST appear on the first line after opening `---`
- Tag directive MUST start with `!` followed by the attribute name
- Attribute name MUST be a valid YAML key (lowercase letters, digits, underscores)
- Attribute name MUST NOT be a reserved field (e.g., `body`)
- If no tag directive is present, the block is treated as global frontmatter

#### 2. Body Content Extraction

For tagged metadata blocks:
- **Body starts**: Immediately after the closing `---` delimiter
- **Body ends**: At the start of the next metadata block OR end of document
- **Body trimming**: Leading/trailing whitespace handling follows same rules as global body

**Example body extraction:**

```markdown
---
!items
name: Item 1
---

Content for item 1.
Next paragraph.

---
!items
name: Item 2
---

Content for item 2.
```

Item 1 body: `"Content for item 1.\nNext paragraph.\n"`  
Item 2 body: `"Content for item 2."`

#### 3. Collection Semantics

**Array aggregation:**
- All tagged blocks with the same attribute name are collected into an array
- Array preserves document order
- Each entry is an object containing metadata fields + body

**Global vs. tagged:**
- First block without tag directive → global frontmatter
- Subsequent untagged blocks → error (only one global frontmatter allowed)
- Tagged blocks can appear before or after global frontmatter body

### Parsing Algorithm

**High-level steps:**

1. **Scan document for all `---` delimiters**
   - Track positions of opening/closing pairs
   - Identify tag directives

2. **Parse global frontmatter** (if present)
   - First block without tag directive
   - Extract YAML fields into global map
   - Extract body up to next metadata block (or EOF)

3. **Parse tagged metadata blocks**
   - For each tagged block:
     - Extract attribute name from tag directive
     - Parse YAML content
     - Extract body content up to next block
     - Append to array under attribute name

4. **Assemble final structure**
   - Merge global fields with tagged arrays
   - Validate no conflicts (e.g., global field and tagged array with same name)

### Edge Cases and Validation

#### 1. Multiple Global Frontmatter Blocks

```markdown
---
title: First
---

Body

---
author: Second
---

More body
```

**Behavior**: ERROR - Only one untagged frontmatter block allowed.

#### 2. Empty Tagged Block

```markdown
---
!items
---

Body content
```

**Behavior**: Valid - creates entry with empty metadata and specified body.

#### 3. Tagged Block Without Body

```markdown
---
!items
name: Item
---
```

**Behavior**: Valid - creates entry with empty string body.

#### 4. Name Collision

```markdown
---
items: "global value"
---

Body

---
!items
name: Sub-item
---

Sub-body
```

**Behavior**: ERROR - Tagged attribute name conflicts with global field.

#### 5. Reserved Field Names

```markdown
---
!body
content: Test
---
```

**Behavior**: ERROR - Cannot use reserved field name `body` as tag directive.

#### 6. Invalid Tag Syntax

```markdown
---
!Invalid-Name
title: Test
---
```

**Behavior**: ERROR - Tag names must follow `[a-z_][a-z0-9_]*` pattern.

#### 7. Nested Tagged Blocks

```markdown
---
!outer
title: Outer
---

Body with nested:

---
!inner
title: Inner
---
```

**Behavior**: Sequential, not nested - both `outer` and `inner` arrays created at top level. No hierarchical nesting supported.

### Data Structure Changes

#### Extended ParsedDocument

```rust
// Conceptual structure (not implemented)
pub struct ParsedDocument {
    fields: HashMap<String, serde_yaml::Value>,
}
```

**Internal representation:**
- Global fields and arrays stored in same `HashMap`
- Tagged collections represented as `serde_yaml::Value::Sequence`
- Each array element is a `serde_yaml::Value::Mapping` with fields + body

**Access patterns:**
```rust
// Access global field
doc.get_field("title")

// Access tagged collection
doc.get_field("sub_documents")
    .and_then(|v| v.as_sequence())
    
// Access specific item in collection
if let Some(seq) = doc.get_field("items").and_then(|v| v.as_sequence()) {
    for item in seq {
        let title = item.get("title").and_then(|v| v.as_str());
        let body = item.get("body").and_then(|v| v.as_str());
    }
}
```

### Backward Compatibility

**Guarantees:**
- Documents with only global frontmatter parse identically
- No tag directive means no behavior change
- Existing ParsedDocument API remains unchanged

**Migration path:**
- Old documents continue to work without modification
- New documents can opt-in by using tag directives
- Templates can check for presence of tagged arrays with `get_field()`

### Performance Considerations

**Complexity:**
- Document scan: O(n) where n = document length
- Metadata extraction: O(m) where m = number of metadata blocks
- Total: O(n + m) linear time complexity

**Memory:**
- Single pass parsing (no backtracking required)
- Metadata blocks stored as YAML values (reuses existing infrastructure)

### Security Considerations

**Potential issues:**
1. **Deep nesting**: While not structurally nested, many tagged blocks could create large arrays
   - **Mitigation**: Add configurable limit on array size per attribute

2. **Tag name injection**: Malicious tag names could conflict with template variables
   - **Mitigation**: Strict validation of tag names (already in spec)

3. **Body content isolation**: Tagged bodies might contain metadata-like syntax
   - **Mitigation**: Bodies are treated as opaque strings (no recursive parsing)

### Template Integration

Templates can leverage tagged collections using standard iteration:

```typst
// Typst template example
#set document(title: {{ title | String }})

{{ body | Content }}

#for item in {{ items | Dict }}
  #heading(level: 2, item.title)
  #eval(item.body)
#endfor
```

### Testing Requirements

When implementing, the following test cases must be covered:

1. **Basic tagged block**: Single tag directive with metadata and body
2. **Multiple instances**: Same tag directive used multiple times
3. **Mixed global and tagged**: Global frontmatter + tagged blocks
4. **Empty metadata**: Tagged block with no YAML fields
5. **Empty body**: Tagged block with no body content
6. **Adjacent blocks**: Back-to-back tagged blocks with different tags
7. **Order preservation**: Verify array maintains document order
8. **Error: multiple global**: Second untagged block should fail
9. **Error: name collision**: Tagged name conflicts with global field
10. **Error: reserved name**: Using `body` as tag directive
11. **Error: invalid syntax**: Malformed tag directives
12. **Complex YAML**: Nested structures within tagged metadata
13. **Cross-platform**: Line ending variations (`\n` vs `\r\n`)

### Open Design Questions

1. **Hierarchical nesting**: Should tagged blocks support parent-child relationships?
   - Current design: No nesting, all collections are flat
   - Alternative: Allow `!parent.child` syntax for nested structures

2. **Body concatenation**: Should adjacent same-tag blocks merge bodies?
   - Current design: No, each creates separate array entry
   - Alternative: Concatenate bodies if metadata is identical

3. **Global body placement**: Where does global body end when tagged blocks present?
   - Current design: Up to first tagged block
   - Alternative: Only content before first `---` after frontmatter

4. **Type coercion**: Should single tagged block create array or object?
   - Current design: Always creates array for consistency
   - Alternative: Single item → object, multiple → array

### Future Enhancements

Beyond the extended metadata standard:

1. **Cross-platform line endings**: Support both `\n` and `\r\n`
2. **Graceful degradation option**: Add flag for non-fatal YAML errors
3. **Empty frontmatter handling**: Optimize for `---\n---\n` edge case
4. **Body trimming option**: Configurable whitespace normalization
5. **Diagnostic context**: Include line/column numbers in error messages
6. **Metadata inheritance**: Tagged blocks inherit global fields unless overridden
7. **Custom delimiters**: Allow alternatives to `---` for metadata blocks
8. **Schema validation**: JSON Schema or similar for metadata structure validation

## Related Files

- **Implementation**: `quillmark-core/src/parse.rs`
- **Architecture**: `DESIGN.md` (Parsing and Document Decomposition section)
- **Example documents**: `quillmark-fixtures/resources/frontmatter_demo.md`
- **Integration**: Used by `Workflow::render()` as first step in pipeline
