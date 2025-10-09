# Markdown Parsing and Decomposition

This document details the design and implementation of markdown document decomposition in Quillmark, implemented in `quillmark-core/src/parse.rs`.

## Overview

Quillmark uses a **frontmatter-aware markdown parser** that separates YAML metadata from document content. This enables template-driven document generation where frontmatter fields (like title, author, date) can be processed independently from the markdown body.

**Key capabilities:**
- Parse YAML frontmatter delimited by `---` markers
- **NEW**: Support inline metadata sections with SCOPE/QUILL keys (Extended YAML Metadata Standard)
- **NEW**: Aggregate scoped blocks into collections (arrays of objects)
- Extract frontmatter fields into a structured `HashMap`
- Preserve markdown body content separately
- Robust error handling with descriptive messages
- Cross-platform line ending support (`\n` and `\r\n`)
- Horizontal rule disambiguation (distinguish metadata from markdown syntax)

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

**High-Level Algorithm:**

1. **Metadata block discovery**
   - Scan entire document for all `---` delimiters
   - Support both `---\n` and `---\r\n` line endings
   - Distinguish metadata blocks from horizontal rules via blank line detection

2. **Block classification**
   - Check if opening `---` is followed by blank line → horizontal rule (skip)
   - Check if opening `---` is followed by content → metadata block (parse)
   - Validate contiguity (no blank lines within YAML content)

3. **Scope/Quill key extraction**
   - Parse YAML content to check for special keys `SCOPE` or `QUILL`
   - If `SCOPE` key present → tagged block (value becomes collection name)
   - If `QUILL` key present → quill specification block
   - If neither present → global frontmatter

4. **YAML parsing**
   - Parse YAML content with `serde_yaml::from_str`
   - Return descriptive error if YAML is malformed
   - Apply same parsing rigor to both frontmatter and tagged blocks

5. **Body extraction**
   - For global frontmatter: body starts after closing `---`, ends at first tagged block or EOF
   - For tagged blocks: body starts after closing `---`, ends at next block or EOF
   - Preserve all whitespace (including leading newlines)

6. **Collection aggregation**
   - Group all blocks with same tag name into arrays
   - Each array element contains metadata fields + body
   - Preserve document order

7. **Validation**
   - Check for multiple global frontmatter blocks → error
   - Check for name collisions (global field vs tagged attribute) → error
   - Check for reserved field names in tags (`body`) → error
   - Validate tag name syntax → error if invalid

8. **Result assembly**
   - Merge global fields, global body, and tagged collections
   - Return unified `ParsedDocument` with all fields in single HashMap

### Supporting Functions

**`find_metadata_blocks()`**

Scans the document and returns a list of all metadata blocks with their positions, content, and optional scope/quill keys.

**Key logic:**
- Pattern matching for `---\n` and `---\r\n`
- Blank line detection (opening `---` followed by `\n` or `\r\n` → horizontal rule)
- Contiguity validation (content between delimiters must have no blank lines)
- End-of-file delimiter support (closing `---` at EOF without trailing newline)
- SCOPE/QUILL key extraction from parsed YAML

**`is_valid_tag_name()`**

Validates scope names against the pattern `[a-z_][a-z0-9_]*`:
- Must start with lowercase letter or underscore
- Remaining chars must be lowercase letters, digits, or underscores

### Error Messages

The implementation provides specific, actionable error messages:

- `"Invalid YAML frontmatter: {error}"` - YAML parser rejected frontmatter
- `"Invalid YAML in scoped block '{scope}': {error}"` - YAML parser rejected scoped block
- `"Frontmatter started but not closed with ---"` - Missing closing delimiter
- `"Multiple global frontmatter blocks found: only one block without SCOPE allowed"` - Duplicate frontmatter
- `"Invalid scope name '{name}': must match pattern [a-z_][a-z0-9_]*"` - Invalid scope syntax
- `"Cannot use reserved field name '{name}' as SCOPE value"` - Protected field name
- `"Name collision: global field '{name}' conflicts with scoped attribute"` - Field/scope conflict
- `"Name collision: scoped attribute '{name}' conflicts with global field"` - Scope/field conflict
- `"Cannot specify both QUILL and SCOPE in the same block"` - Conflicting special keys

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

The test suite in `parse.rs` provides comprehensive coverage:

### 1. Basic Frontmatter Cases
- `test_no_frontmatter` - Document without any YAML metadata
- `test_with_frontmatter` - Standard frontmatter with title and author
- `test_complex_yaml_frontmatter` - Nested structures and arrays

### 2. Extended Metadata Standard Cases
- `test_basic_tagged_block` - Single scoped block with metadata and body
- `test_multiple_tagged_blocks` - Multiple blocks with same SCOPE (array creation)
- `test_mixed_global_and_tagged` - Global frontmatter combined with scoped blocks
- `test_empty_tagged_metadata` - Scoped block with no YAML fields
- `test_tagged_block_without_body` - Scoped block with no body content
- `test_adjacent_blocks_different_tags` - Multiple different collections
- `test_order_preservation` - Verify array maintains document order
- `test_complex_yaml_in_tagged_block` - Nested YAML within scoped blocks

### 3. Error Cases
- `test_invalid_yaml` - Malformed YAML syntax in frontmatter
- `test_invalid_yaml_in_tagged_block` - Malformed YAML in scoped block
- `test_unclosed_frontmatter` - Missing closing delimiter
- `test_multiple_global_frontmatter_blocks` - Multiple blocks without SCOPE (error)
- `test_name_collision_global_and_tagged` - Field/scope name conflicts
- `test_reserved_field_name` - Using `body` as SCOPE value
- `test_invalid_tag_syntax` - Invalid scope names (uppercase, hyphens, etc.)

### 4. Horizontal Rule Disambiguation Cases
- `test_horizontal_rule_disambiguation` - `---` with blank line before it in body

### 5. Integration Cases
- `test_product_catalog_integration` - Real-world product catalog example
- `test_extended_metadata_demo_file` - Full demo file with multiple collections

### Validation Approach

Tests verify:
- **Structural integrity**: Field count matches expected values
- **Field access**: Individual fields can be retrieved correctly
- **Body separation**: Body content excludes metadata blocks
- **Error messages**: Errors contain expected keywords and are descriptive
- **Type preservation**: YAML types (strings, sequences, maps) are accessible
- **Collection aggregation**: Tagged blocks create proper arrays
- **Order preservation**: Arrays maintain document order
- **Name validation**: Tag names follow required pattern
- **Collision detection**: Conflicts are properly detected and reported

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

## Implementation Status and DESIGN.md Alignment

The current implementation has evolved beyond the `DESIGN.md` recommendations to support the Extended YAML Metadata Standard:

### 1. Line Ending Support ✅
- **DESIGN.md**: Check both `\n` and `\r\n`
- **Current**: **IMPLEMENTED** - Checks both `\n` and `\r\n` for full cross-platform compatibility
- **Status**: Fully aligned with DESIGN.md

### 2. Error Handling Philosophy
- **DESIGN.md**: "Graceful degradation" - treat errors as warnings
- **Current**: Fail-fast with explicit errors
- **Rationale**: Prevents silent data corruption from malformed YAML; provides clear diagnostics
- **Status**: Intentional divergence for data integrity

### 3. Delimiter Search Strategy
- **DESIGN.md**: Line-by-line iteration with `line.trim() == "---"`
- **Current**: Pattern matching for `---\n` and `---\r\n` with contiguity validation
- **Trade-off**: More sophisticated approach supports extended metadata standard
- **Status**: Enhanced beyond DESIGN.md for new features

### 4. Body Whitespace Handling
- **DESIGN.md**: Use `trim_start()` to remove leading whitespace
- **Current**: Preserves all whitespace including leading newline
- **Impact**: Body may start with `\n` when frontmatter is present (preserves user formatting)
- **Status**: Intentional divergence for content fidelity

### 5. Extended Metadata Standard ✅
- **DESIGN.md**: Not mentioned
- **Current**: **FULLY IMPLEMENTED** - Tag directives, collection aggregation, horizontal rule disambiguation
- **Status**: New capability beyond original DESIGN.md scope

## Extended YAML Metadata Standard (Implemented)

This section documents the implemented extension to the frontmatter-only approach, allowing **inline metadata sections** throughout the document body. This feature is **FULLY IMPLEMENTED** and production-ready as of the implementation in `quillmark-core/src/parse.rs`.

### Motivation

The current single-frontmatter design limits documents to a flat metadata structure at the beginning. Many use cases require:
- **Structured sub-documents**: Breaking content into logical sections with their own metadata
- **Repeated elements**: Collections of similar items (e.g., multiple products, blog posts, or chapters)
- **Hierarchical content**: Documents that naturally contain nested structures

### Design Overview

The extended standard allows metadata blocks to appear anywhere in the document using **SCOPE** and **QUILL** special keys:

```markdown
---
title: Global Metadata
author: John Doe
---

This is the main document body.

---
SCOPE: sub_documents
title: First Sub-Document
tags: [example, demo]
---

Body of the *first sub-document* with **markdown formatting**.

---
SCOPE: sub_documents
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

#### 1. SCOPE/QUILL Key Format

**Grammar:**
```
metadata_block ::= "---" NEWLINE yaml_content "---" NEWLINE body_content
yaml_content ::= (yaml_line NEWLINE)+  // No blank lines allowed
scope_key ::= "SCOPE: " scope_name
quill_key ::= "QUILL: " quill_name
scope_name ::= [a-z_][a-z0-9_]*
```

**Disambiguation from markdown horizontal rules:**
- Metadata blocks MUST be contiguous (no blank lines between opening `---` and closing `---`)
- `---` preceded by blank line in body content is treated as horizontal rule, not metadata delimiter
- This ensures clear distinction between YAML metadata and markdown syntax

**Rules:**
- The YAML content is parsed normally
- If a `SCOPE` key is found in the parsed YAML, the block becomes a scoped collection item
- If a `QUILL` key is found, it specifies which quill template to use
- A block MUST NOT contain both `SCOPE` and `QUILL` keys
- Scope name MUST be a valid identifier (lowercase letters, digits, underscores)
- Scope name MUST NOT be a reserved field (e.g., `body`)
- If neither `SCOPE` nor `QUILL` is present, the block is treated as global frontmatter
- YAML metadata blocks (both frontmatter and scoped) are parsed with identical rules and rigor
- Frontmatter fields are stored in global scope; scoped fields are stored in arrays under the scope name
- The `SCOPE` and `QUILL` keys are removed from the final metadata after processing

#### 2. Body Content Extraction

For scoped metadata blocks:
- **Body starts**: Immediately after the closing `---` delimiter
- **Body ends**: At the start of the next metadata block OR end of document
- **Body trimming**: Leading/trailing whitespace handling follows same rules as global body

**Example body extraction:**

```markdown
---
SCOPE: items
name: Item 1
---

Content for item 1.
Next paragraph.

---
SCOPE: items
name: Item 2
---

Content for item 2.
```

Item 1 body: `"Content for item 1.\nNext paragraph.\n"`  
Item 2 body: `"Content for item 2."`

#### 3. Collection Semantics

**Array aggregation:**
- All scoped blocks with the same scope name are collected into an array
- Array preserves document order
- Each entry is an object containing metadata fields + body

**Global vs. scoped:**
- First block without SCOPE/QUILL key → global frontmatter
- Subsequent blocks without SCOPE/QUILL → error (only one global frontmatter allowed)
- Scoped blocks can appear before or after global frontmatter body

### Parsing Algorithm

**High-level steps:**

1. **Scan document for all `---` delimiters**
   - Track positions of opening/closing pairs
   - Check for contiguity: validate no blank lines between opening `---` and closing `---`
   - Distinguish metadata blocks from horizontal rules (metadata blocks are contiguous)

2. **Parse global frontmatter** (if present)
   - First contiguous block without SCOPE/QUILL key
   - Extract YAML fields into global map
   - Extract body up to next metadata block (or EOF)

3. **Parse scoped metadata blocks**
   - For each block:
     - Verify block is contiguous (no blank lines in YAML content)
     - Parse YAML content
     - Check for SCOPE or QUILL key
     - Extract scope name from SCOPE value
     - Remove SCOPE/QUILL key from the metadata
     - Extract body content up to next block
     - Append to array under scope name

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

**Behavior**: ERROR - Only one block without SCOPE/QUILL allowed.

#### 2. Empty Scoped Block

```markdown
---
SCOPE: items
---

Body content
```

**Behavior**: Valid - creates entry with empty metadata and specified body.

#### 3. Scoped Block Without Body

```markdown
---
SCOPE: items
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
SCOPE: items
name: Sub-item
---

Sub-body
```

**Behavior**: ERROR - Scoped attribute name conflicts with global field.

#### 5. Reserved Field Names

```markdown
---
SCOPE: body
content: Test
---
```

**Behavior**: ERROR - Cannot use reserved field name `body` as SCOPE value.

#### 6. Invalid Scope Syntax

```markdown
---
SCOPE: Invalid-Name
title: Test
---
```

**Behavior**: ERROR - Scope names must follow `[a-z_][a-z0-9_]*` pattern.

#### 7. Nested Scoped Blocks

```markdown
---
SCOPE: outer
title: Outer
---

Body with nested:

---
SCOPE: inner
title: Inner
---
```

**Behavior**: Sequential, not nested - both `outer` and `inner` arrays created at top level. No hierarchical nesting supported.

#### 8. Horizontal Rule Ambiguity

The `---` delimiter is also valid markdown syntax for a horizontal rule. To disambiguate YAML metadata blocks from horizontal rules in body content, the following rules apply:

**Requirement: Contiguous YAML blocks**

YAML metadata blocks MUST be **contiguous** - no blank lines are allowed between the opening `---` and the closing `---`. This distinguishes metadata blocks from horizontal rules.

```markdown
---
title: Global
---

Body content

--- 
This is a horizontal rule (has blank line before it)

More body content
```

**Valid metadata block (contiguous):**
```markdown
---
SCOPE: items
name: Product
price: 99.99
---
Body for this item
```

**NOT a metadata block (has blank line, treated as horizontal rule + text):**
```markdown
---

SCOPE: items
name: Product
---
```

**Parsing behavior:**
- **Opening `---` followed by blank line**: Treated as horizontal rule in body content (not a metadata block)
- **Opening `---` followed by content**: Parsed as YAML metadata block
- **`---` preceded by blank line in body**: Treated as horizontal rule (not closing delimiter)

This rule ensures:
1. Metadata blocks are always recognized by their contiguous structure
2. Users can still use `---` horizontal rules in body content by adding a blank line before them
3. No ambiguity exists between metadata delimiters and markdown syntax

**Alternative horizontal rule syntaxes:**

To avoid potential confusion, users can use alternative markdown horizontal rule syntaxes:
- `***` (three or more asterisks)
- `___` (three or more underscores)
- `- - -` (three or more hyphens with spaces)

**Examples with mixed content:**

```markdown
---
title: Document with Rules
---

Introduction paragraph.

***
This uses asterisks for a horizontal rule.

---
SCOPE: sections
title: Section 1
---

Section 1 content.

___
Another horizontal rule using underscores.

---
SCOPE: sections  
title: Section 2
---

Section 2 content.
```

This approach maintains backward compatibility while providing clear disambiguation.

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
- Scoped collections represented as `serde_yaml::Value::Sequence`
- Each array element is a `serde_yaml::Value::Mapping` with fields + body

**Access patterns:**
```rust
// Access global field
doc.get_field("title")

// Access scoped collection
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
- No SCOPE/QUILL keys means no behavior change
- Existing ParsedDocument API remains unchanged

**Migration path:**
- Old documents continue to work without modification
- New documents can opt-in by using SCOPE/QUILL keys
- Templates can check for presence of scoped arrays with `get_field()`

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
1. **Deep nesting**: While not structurally nested, many scoped blocks could create large arrays
   - **Mitigation**: Add configurable limit on array size per attribute

2. **Scope name injection**: Malicious scope names could conflict with template variables
   - **Mitigation**: Strict validation of scope names (already in spec)

3. **Body content isolation**: Scoped bodies might contain metadata-like syntax
   - **Mitigation**: Bodies are treated as opaque strings (no recursive parsing)

### Template Integration

Templates can leverage scoped collections using standard iteration:

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

1. **Basic scoped block**: Single SCOPE key with metadata and body
2. **Multiple instances**: Same SCOPE key used multiple times
3. **Mixed global and scoped**: Global frontmatter + scoped blocks
4. **Empty metadata**: Scoped block with no YAML fields
5. **Empty body**: Scoped block with no body content
6. **Adjacent blocks**: Back-to-back scoped blocks with different scopes
7. **Order preservation**: Verify array maintains document order
8. **Error: multiple global**: Second block without SCOPE/QUILL should fail
9. **Error: name collision**: Scope name conflicts with global field
10. **Error: reserved name**: Using `body` as SCOPE value
11. **Error: invalid syntax**: Malformed scope names
12. **Complex YAML**: Nested structures within scoped metadata
13. **Cross-platform**: Line ending variations (`\n` vs `\r\n`)
14. **Horizontal rule disambiguation**: `---` with blank line before it in body is horizontal rule, not metadata
15. **Non-contiguous block**: Blank line between opening `---` and YAML content should error or be treated as horizontal rule
16. **Alternative horizontal rules**: Verify `***` and `___` work as horizontal rules in body content

### Open Design Questions

1. **Hierarchical nesting**: Should scoped blocks support parent-child relationships?
   - Current design: No nesting, all collections are flat
   - Alternative: Allow dotted scope names for nested structures

2. **Body concatenation**: Should adjacent same-scope blocks merge bodies?
   - Current design: No, each creates separate array entry
   - Alternative: Concatenate bodies if metadata is identical

3. **Global body placement**: Where does global body end when scoped blocks present?
   - Current design: Up to first scoped block
   - Alternative: Only content before first `---` after frontmatter

4. **Type coercion**: Should single scoped block create array or object?
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
