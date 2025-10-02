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

## Future Enhancements

Potential improvements aligned with `DESIGN.md`:

1. **Cross-platform line endings**: Support both `\n` and `\r\n`
2. **Graceful degradation option**: Add flag for non-fatal YAML errors
3. **Empty frontmatter handling**: Optimize for `---\n---\n` edge case
4. **Body trimming option**: Configurable whitespace normalization
5. **Diagnostic context**: Include line/column numbers in error messages

## Related Files

- **Implementation**: `quillmark-core/src/parse.rs`
- **Architecture**: `DESIGN.md` (Parsing and Document Decomposition section)
- **Example documents**: `quillmark-fixtures/resources/frontmatter_demo.md`
- **Integration**: Used by `Workflow::render()` as first step in pipeline
