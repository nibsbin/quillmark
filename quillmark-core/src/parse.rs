//! # Parsing Module
//!
//! Parsing functionality for markdown documents with YAML frontmatter.
//!
//! ## Overview
//!
//! The `parse` module provides the [`decompose`] function for parsing markdown documents
//! and the [`ParsedDocument`] type for accessing parsed content.
//!
//! ## Key Types
//!
//! - [`ParsedDocument`]: Container for parsed frontmatter fields and body content
//! - [`BODY_FIELD`]: Constant for the field name storing document body
//!
//! ## Examples
//!
//! ### Basic Parsing
//!
//! ```
//! use quillmark_core::decompose;
//!
//! let markdown = r#"---
//! title: My Document
//! author: John Doe
//! ---
//!
//! # Introduction
//!
//! Document content here.
//! "#;
//!
//! let doc = decompose(markdown).unwrap();
//! let title = doc.get_field("title")
//!     .and_then(|v| v.as_str())
//!     .unwrap_or("Untitled");
//! ```
//!
//! ### Extended Metadata with Tags
//!
//! ```
//! use quillmark_core::decompose;
//!
//! let markdown = r#"---
//! catalog_title: Product Catalog
//! ---
//!
//! # Products
//!
//! ---
//! SCOPE: products
//! name: Widget
//! price: 19.99
//! ---
//!
//! A versatile widget for all occasions.
//! "#;
//!
//! let doc = decompose(markdown).unwrap();
//!
//! // Access tagged collections
//! if let Some(products) = doc.get_field("products")
//!     .and_then(|v| v.as_sequence())
//! {
//!     for product in products {
//!         let name = product.get("name").and_then(|v| v.as_str()).unwrap();
//!         let price = product.get("price").and_then(|v| v.as_f64()).unwrap();
//!         println!("{}: ${}", name, price);
//!     }
//! }
//! ```
//!
//! ## Error Handling
//!
//! The [`decompose`] function returns errors for:
//! - Malformed YAML syntax
//! - Unclosed frontmatter blocks
//! - Multiple global frontmatter blocks
//! - Both QUILL and SCOPE specified in the same block
//! - Reserved field name usage
//! - Name collisions
//!
//! See [PARSE.md](https://github.com/nibsbin/quillmark/blob/main/designs/PARSE.md) for comprehensive documentation of the Extended YAML Metadata Standard.

use std::collections::HashMap;

use crate::value::QuillValue;

/// The field name used to store the document body
pub const BODY_FIELD: &str = "body";

/// Reserved tag name for quill specification
pub const QUILL_TAG: &str = "quill";

/// A parsed markdown document with frontmatter
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    fields: HashMap<String, QuillValue>,
    quill_tag: Option<String>,
}

impl ParsedDocument {
    /// Create a new ParsedDocument with the given fields
    pub fn new(fields: HashMap<String, QuillValue>) -> Self {
        Self {
            fields,
            quill_tag: None,
        }
    }

    /// Create a ParsedDocument from fields and optional quill tag
    pub fn with_quill_tag(fields: HashMap<String, QuillValue>, quill_tag: Option<String>) -> Self {
        Self { fields, quill_tag }
    }

    /// Create a ParsedDocument from markdown string
    pub fn from_markdown(markdown: &str) -> Result<Self, crate::error::ParseError> {
        decompose(markdown).map_err(|e| crate::error::ParseError::from(e))
    }

    /// Get the quill tag if specified (from QUILL key)
    pub fn quill_tag(&self) -> Option<&str> {
        self.quill_tag.as_deref()
    }

    /// Get the document body
    pub fn body(&self) -> Option<&str> {
        self.fields.get(BODY_FIELD).and_then(|v| v.as_str())
    }

    /// Get a specific field
    pub fn get_field(&self, name: &str) -> Option<&QuillValue> {
        self.fields.get(name)
    }

    /// Get all fields (including body)
    pub fn fields(&self) -> &HashMap<String, QuillValue> {
        &self.fields
    }
}

#[derive(Debug)]
struct MetadataBlock {
    start: usize, // Position of opening "---"
    end: usize,   // Position after closing "---\n"
    yaml_content: String,
    tag: Option<String>,        // Field name from SCOPE key
    quill_name: Option<String>, // Quill name from QUILL key
}

/// Validate tag name follows pattern [a-z_][a-z0-9_]*
fn is_valid_tag_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    if !first.is_ascii_lowercase() && first != '_' {
        return false;
    }

    for ch in chars {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_' {
            return false;
        }
    }

    true
}

/// Find all metadata blocks in the document
fn find_metadata_blocks(
    markdown: &str,
) -> Result<Vec<MetadataBlock>, Box<dyn std::error::Error + Send + Sync>> {
    let mut blocks = Vec::new();
    let mut pos = 0;

    while pos < markdown.len() {
        // Look for opening "---\n" or "---\r\n"
        let search_str = &markdown[pos..];
        let delimiter_result = if let Some(p) = search_str.find("---\n") {
            Some((p, 4, "\n"))
        } else if let Some(p) = search_str.find("---\r\n") {
            Some((p, 5, "\r\n"))
        } else {
            None
        };

        if let Some((delimiter_pos, delimiter_len, _line_ending)) = delimiter_result {
            let abs_pos = pos + delimiter_pos;
            let content_start = abs_pos + delimiter_len; // After "---\n" or "---\r\n"

            // Check if opening --- is followed by a blank line (horizontal rule, not metadata)
            let followed_by_blank = if content_start < markdown.len() {
                markdown[content_start..].starts_with('\n')
                    || markdown[content_start..].starts_with("\r\n")
            } else {
                false
            };

            if followed_by_blank {
                // This is a horizontal rule in the body, skip it
                pos = abs_pos + 3; // Skip past "---"
                continue;
            }

            // Found potential metadata block opening
            // Look for closing "\n---\n" or "\r\n---\r\n" etc., OR "\n---" / "\r\n---" at end of document
            let rest = &markdown[content_start..];

            // First try to find delimiters with trailing newlines
            let closing_patterns = ["\n---\n", "\r\n---\r\n", "\n---\r\n", "\r\n---\n"];
            let closing_with_newline = closing_patterns
                .iter()
                .filter_map(|delim| rest.find(delim).map(|p| (p, delim.len())))
                .min_by_key(|(p, _)| *p);

            // Also check for closing at end of document (no trailing newline)
            let closing_at_eof = ["\n---", "\r\n---"]
                .iter()
                .filter_map(|delim| {
                    rest.find(delim).and_then(|p| {
                        if p + delim.len() == rest.len() {
                            Some((p, delim.len()))
                        } else {
                            None
                        }
                    })
                })
                .min_by_key(|(p, _)| *p);

            let closing_result = match (closing_with_newline, closing_at_eof) {
                (Some((p1, _l1)), Some((p2, _))) if p2 < p1 => closing_at_eof,
                (Some(_), Some(_)) => closing_with_newline,
                (Some(_), None) => closing_with_newline,
                (None, Some(_)) => closing_at_eof,
                (None, None) => None,
            };

            if let Some((closing_pos, closing_len)) = closing_result {
                let abs_closing_pos = content_start + closing_pos;
                let content = &markdown[content_start..abs_closing_pos];

                // Check YAML size limit
                if content.len() > crate::error::MAX_YAML_SIZE {
                    return Err(format!(
                        "YAML block too large: {} bytes (max: {} bytes)",
                        content.len(),
                        crate::error::MAX_YAML_SIZE
                    )
                    .into());
                }

                // Check if the block is contiguous (no blank lines in the YAML content)
                if content.contains("\n\n") || content.contains("\r\n\r\n") {
                    // Not a contiguous block
                    if abs_pos == 0 {
                        // Started at beginning but has blank lines - this is an error
                        return Err("Frontmatter started but not closed with ---".into());
                    }
                    // Otherwise treat as horizontal rule in body
                    pos = abs_pos + 3;
                    continue;
                }

                // Parse YAML content to check for reserved keys (QUILL, SCOPE)
                // First, try to parse as YAML
                let (tag, quill_name, yaml_content) = if !content.is_empty() {
                    // Try to parse the YAML to check for reserved keys
                    match serde_yaml::from_str::<serde_yaml::Value>(content) {
                        Ok(yaml_value) => {
                            if let Some(mapping) = yaml_value.as_mapping() {
                                let quill_key = serde_yaml::Value::String("QUILL".to_string());
                                let scope_key = serde_yaml::Value::String("SCOPE".to_string());

                                let has_quill = mapping.contains_key(&quill_key);
                                let has_scope = mapping.contains_key(&scope_key);

                                if has_quill && has_scope {
                                    return Err(
                                        "Cannot specify both QUILL and SCOPE in the same block"
                                            .into(),
                                    );
                                }

                                if has_quill {
                                    // Extract quill name
                                    let quill_value = mapping.get(&quill_key).unwrap();
                                    let quill_name_str = quill_value
                                        .as_str()
                                        .ok_or_else(|| "QUILL value must be a string")?;

                                    if !is_valid_tag_name(quill_name_str) {
                                        return Err(format!(
                                            "Invalid quill name '{}': must match pattern [a-z_][a-z0-9_]*",
                                            quill_name_str
                                        )
                                        .into());
                                    }

                                    // Remove QUILL from the YAML content for processing
                                    let mut new_mapping = mapping.clone();
                                    new_mapping.remove(&quill_key);
                                    let new_yaml = serde_yaml::to_string(&new_mapping)
                                        .map_err(|e| format!("Failed to serialize YAML: {}", e))?;

                                    (None, Some(quill_name_str.to_string()), new_yaml)
                                } else if has_scope {
                                    // Extract scope field name
                                    let scope_value = mapping.get(&scope_key).unwrap();
                                    let field_name = scope_value
                                        .as_str()
                                        .ok_or_else(|| "SCOPE value must be a string")?;

                                    if !is_valid_tag_name(field_name) {
                                        return Err(format!(
                                            "Invalid field name '{}': must match pattern [a-z_][a-z0-9_]*",
                                            field_name
                                        )
                                        .into());
                                    }

                                    if field_name == BODY_FIELD {
                                        return Err(format!(
                                            "Cannot use reserved field name '{}' as SCOPE value",
                                            BODY_FIELD
                                        )
                                        .into());
                                    }

                                    // Remove SCOPE from the YAML content for processing
                                    let mut new_mapping = mapping.clone();
                                    new_mapping.remove(&scope_key);
                                    let new_yaml = serde_yaml::to_string(&new_mapping)
                                        .map_err(|e| format!("Failed to serialize YAML: {}", e))?;

                                    (Some(field_name.to_string()), None, new_yaml)
                                } else {
                                    // No reserved keys, treat as normal YAML
                                    (None, None, content.to_string())
                                }
                            } else {
                                // Not a mapping, treat as normal YAML
                                (None, None, content.to_string())
                            }
                        }
                        Err(_) => {
                            // If YAML parsing fails here, we'll catch it later
                            (None, None, content.to_string())
                        }
                    }
                } else {
                    (None, None, content.to_string())
                };

                blocks.push(MetadataBlock {
                    start: abs_pos,
                    end: abs_closing_pos + closing_len, // After closing delimiter
                    yaml_content,
                    tag,
                    quill_name,
                });

                pos = abs_closing_pos + closing_len;
            } else if abs_pos == 0 {
                // Frontmatter started but not closed
                return Err("Frontmatter started but not closed with ---".into());
            } else {
                // Not a valid metadata block, skip this position
                pos = abs_pos + 3;
            }
        } else {
            break;
        }
    }

    Ok(blocks)
}

/// Decompose markdown into frontmatter fields and body
pub fn decompose(
    markdown: &str,
) -> Result<ParsedDocument, Box<dyn std::error::Error + Send + Sync>> {
    // Check input size limit
    if markdown.len() > crate::error::MAX_INPUT_SIZE {
        return Err(format!(
            "Input too large: {} bytes (max: {} bytes)",
            markdown.len(),
            crate::error::MAX_INPUT_SIZE
        )
        .into());
    }

    let mut fields = HashMap::new();

    // Find all metadata blocks
    let blocks = find_metadata_blocks(markdown)?;

    if blocks.is_empty() {
        // No metadata blocks, entire content is body
        fields.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::Value::String(markdown.to_string())),
        );
        return Ok(ParsedDocument::new(fields));
    }

    // Track which attributes are used for tagged blocks
    let mut tagged_attributes: HashMap<String, Vec<serde_yaml::Value>> = HashMap::new();
    let mut has_global_frontmatter = false;
    let mut global_frontmatter_index: Option<usize> = None;
    let mut quill_name: Option<String> = None;

    // First pass: identify global frontmatter, quill directive, and validate
    for (idx, block) in blocks.iter().enumerate() {
        // Check for quill directive
        if let Some(ref name) = block.quill_name {
            if quill_name.is_some() {
                return Err("Multiple quill directives found: only one allowed".into());
            }
            quill_name = Some(name.clone());
        }

        // Check for global frontmatter (no tag and no quill directive)
        if block.tag.is_none() && block.quill_name.is_none() {
            if has_global_frontmatter {
                return Err(
                    "Multiple global frontmatter blocks found: only one untagged block allowed"
                        .into(),
                );
            }
            has_global_frontmatter = true;
            global_frontmatter_index = Some(idx);
        }
    }

    // Parse global frontmatter if present
    if let Some(idx) = global_frontmatter_index {
        let block = &blocks[idx];

        // Parse YAML frontmatter
        let yaml_fields: HashMap<String, serde_yaml::Value> = if block.yaml_content.is_empty() {
            HashMap::new()
        } else {
            serde_yaml::from_str(&block.yaml_content)
                .map_err(|e| format!("Invalid YAML frontmatter: {}", e))?
        };

        // Check that all tagged blocks don't conflict with global fields
        // Exception: if the global field is an array, allow it (we'll merge later)
        for other_block in &blocks {
            if let Some(ref tag) = other_block.tag {
                if let Some(global_value) = yaml_fields.get(tag) {
                    // Check if the global value is an array
                    if global_value.as_sequence().is_none() {
                        return Err(format!(
                            "Name collision: global field '{}' conflicts with tagged attribute",
                            tag
                        )
                        .into());
                    }
                }
            }
        }

        // Convert YAML values to QuillValue at boundary
        for (key, value) in yaml_fields {
            fields.insert(key, QuillValue::from_yaml(value)?);
        }
    }

    // Process blocks with quill directives
    for block in &blocks {
        if block.quill_name.is_some() {
            // Quill directive blocks can have YAML content (becomes part of frontmatter)
            if !block.yaml_content.is_empty() {
                let yaml_fields: HashMap<String, serde_yaml::Value> =
                    serde_yaml::from_str(&block.yaml_content)
                        .map_err(|e| format!("Invalid YAML in quill block: {}", e))?;

                // Check for conflicts with existing fields
                for key in yaml_fields.keys() {
                    if fields.contains_key(key) {
                        return Err(format!(
                            "Name collision: quill block field '{}' conflicts with existing field",
                            key
                        )
                        .into());
                    }
                }

                // Convert YAML values to QuillValue at boundary
                for (key, value) in yaml_fields {
                    fields.insert(key, QuillValue::from_yaml(value)?);
                }
            }
        }
    }

    // Parse tagged blocks
    for (idx, block) in blocks.iter().enumerate() {
        if let Some(ref tag_name) = block.tag {
            // Check if this conflicts with global fields
            // Exception: if the global field is an array, allow it (we'll merge later)
            if let Some(existing_value) = fields.get(tag_name) {
                if existing_value.as_array().is_none() {
                    return Err(format!(
                        "Name collision: tagged attribute '{}' conflicts with global field",
                        tag_name
                    )
                    .into());
                }
            }

            // Parse YAML metadata
            let mut item_fields: HashMap<String, serde_yaml::Value> =
                if block.yaml_content.is_empty() {
                    HashMap::new()
                } else {
                    serde_yaml::from_str(&block.yaml_content).map_err(|e| {
                        format!("Invalid YAML in tagged block '{}': {}", tag_name, e)
                    })?
                };

            // Extract body for this tagged block
            let body_start = block.end;
            let body_end = if idx + 1 < blocks.len() {
                blocks[idx + 1].start
            } else {
                markdown.len()
            };
            let body = &markdown[body_start..body_end];

            // Add body to item fields
            item_fields.insert(
                BODY_FIELD.to_string(),
                serde_yaml::Value::String(body.to_string()),
            );

            // Convert HashMap to serde_yaml::Value::Mapping
            let item_value = serde_yaml::to_value(item_fields)?;

            // Add to collection
            tagged_attributes
                .entry(tag_name.clone())
                .or_insert_with(Vec::new)
                .push(item_value);
        }
    }

    // Extract global body
    // Body starts after global frontmatter or quill block (whichever comes first)
    // Body ends at the first scope block or EOF
    let first_non_scope_block_idx = blocks
        .iter()
        .position(|b| b.tag.is_none() && b.quill_name.is_none())
        .or_else(|| blocks.iter().position(|b| b.quill_name.is_some()));

    let (body_start, body_end) = if let Some(idx) = first_non_scope_block_idx {
        // Body starts after the first non-scope block (global frontmatter or quill)
        let start = blocks[idx].end;

        // Body ends at the first scope block after this, or EOF
        let end = blocks
            .iter()
            .skip(idx + 1)
            .find(|b| b.tag.is_some())
            .map(|b| b.start)
            .unwrap_or(markdown.len());

        (start, end)
    } else {
        // No global frontmatter or quill block - body is everything before the first scope block
        let end = blocks
            .iter()
            .find(|b| b.tag.is_some())
            .map(|b| b.start)
            .unwrap_or(0);

        (0, end)
    };

    let global_body = &markdown[body_start..body_end];

    fields.insert(
        BODY_FIELD.to_string(),
        QuillValue::from_json(serde_json::Value::String(global_body.to_string())),
    );

    // Add all tagged collections to fields (convert to QuillValue)
    // If a field already exists and is an array, merge the new items into it
    for (tag_name, items) in tagged_attributes {
        if let Some(existing_value) = fields.get(&tag_name) {
            // The existing value must be an array (checked earlier)
            if let Some(existing_array) = existing_value.as_array() {
                // Convert new items from YAML to JSON
                let new_items_json: Vec<serde_json::Value> = items
                    .into_iter()
                    .map(|yaml_val| {
                        serde_json::to_value(&yaml_val)
                            .map_err(|e| format!("Failed to convert YAML to JSON: {}", e))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // Combine existing and new items
                let mut merged_array = existing_array.clone();
                merged_array.extend(new_items_json);

                // Create QuillValue from merged JSON array
                let quill_value = QuillValue::from_json(serde_json::Value::Array(merged_array));
                fields.insert(tag_name, quill_value);
            } else {
                // This should not happen due to earlier validation, but handle it gracefully
                return Err(format!(
                    "Internal error: field '{}' exists but is not an array",
                    tag_name
                )
                .into());
            }
        } else {
            // No existing field, just create a new sequence
            let quill_value = QuillValue::from_yaml(serde_yaml::Value::Sequence(items))?;
            fields.insert(tag_name, quill_value);
        }
    }

    let mut parsed = ParsedDocument::new(fields);

    // Set quill tag if present
    if let Some(name) = quill_name {
        parsed.quill_tag = Some(name);
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let markdown = "# Hello World\n\nThis is a test.";
        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some(markdown));
        assert_eq!(doc.fields().len(), 1);
    }

    #[test]
    fn test_with_frontmatter() {
        let markdown = r#"---
title: Test Document
author: Test Author
---

# Hello World

This is the body."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\n# Hello World\n\nThis is the body."));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test Document"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "Test Author"
        );
        assert_eq!(doc.fields().len(), 3); // title, author, body
    }

    #[test]
    fn test_complex_yaml_frontmatter() {
        let markdown = r#"---
title: Complex Document
tags:
  - test
  - yaml
metadata:
  version: 1.0
  nested:
    field: value
---

Content here."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\nContent here."));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Complex Document"
        );

        let tags = doc.get_field("tags").unwrap().as_sequence().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str().unwrap(), "test");
        assert_eq!(tags[1].as_str().unwrap(), "yaml");
    }

    #[test]
    fn test_invalid_yaml() {
        let markdown = r#"---
title: [invalid yaml
author: missing close bracket
---

Content here."#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid YAML frontmatter"));
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let markdown = r#"---
title: Test
author: Test Author

Content without closing ---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not closed"));
    }

    // Extended metadata tests

    #[test]
    fn test_basic_tagged_block() {
        let markdown = r#"---
title: Main Document
---

Main body content.

---
SCOPE: items
name: Item 1
---

Body of item 1."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\nMain body content.\n\n"));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Main Document"
        );

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);

        let item = items[0].as_object().unwrap();
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");
        assert_eq!(
            item.get("body").unwrap().as_str().unwrap(),
            "\nBody of item 1."
        );
    }

    #[test]
    fn test_multiple_tagged_blocks() {
        let markdown = r#"---
SCOPE: items
name: Item 1
tags: [a, b]
---

First item body.

---
SCOPE: items
name: Item 2
tags: [c, d]
---

Second item body."#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 2);

        let item1 = items[0].as_object().unwrap();
        assert_eq!(item1.get("name").unwrap().as_str().unwrap(), "Item 1");

        let item2 = items[1].as_object().unwrap();
        assert_eq!(item2.get("name").unwrap().as_str().unwrap(), "Item 2");
    }

    #[test]
    fn test_mixed_global_and_tagged() {
        let markdown = r#"---
title: Global
author: John Doe
---

Global body.

---
SCOPE: sections
title: Section 1
---

Section 1 content.

---
SCOPE: sections
title: Section 2
---

Section 2 content."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Global");
        assert_eq!(doc.body(), Some("\nGlobal body.\n\n"));

        let sections = doc.get_field("sections").unwrap().as_sequence().unwrap();
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn test_empty_tagged_metadata() {
        let markdown = r#"---
SCOPE: items
---

Body without metadata."#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);

        let item = items[0].as_object().unwrap();
        assert_eq!(
            item.get("body").unwrap().as_str().unwrap(),
            "\nBody without metadata."
        );
    }

    #[test]
    fn test_tagged_block_without_body() {
        let markdown = r#"---
SCOPE: items
name: Item
---"#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);

        let item = items[0].as_object().unwrap();
        assert_eq!(item.get("body").unwrap().as_str().unwrap(), "");
    }

    #[test]
    fn test_name_collision_global_and_tagged() {
        let markdown = r#"---
items: "global value"
---

Body

---
SCOPE: items
name: Item
---

Item body"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("collision"));
    }

    #[test]
    fn test_global_array_merged_with_scope() {
        // When global frontmatter has an array field with the same name as a SCOPE,
        // the SCOPE items should be added to the array
        let markdown = r#"---
items:
  - name: Global Item 1
    value: 100
  - name: Global Item 2
    value: 200
---

Global body

---
SCOPE: items
name: Scope Item 1
value: 300
---

Scope item 1 body

---
SCOPE: items
name: Scope Item 2
value: 400
---

Scope item 2 body"#;

        let doc = decompose(markdown).unwrap();

        // Verify the items array has all 4 items (2 from global + 2 from SCOPE)
        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 4);

        // Verify first two items (from global array)
        let item1 = items[0].as_object().unwrap();
        assert_eq!(
            item1.get("name").unwrap().as_str().unwrap(),
            "Global Item 1"
        );
        assert_eq!(item1.get("value").unwrap().as_i64().unwrap(), 100);

        let item2 = items[1].as_object().unwrap();
        assert_eq!(
            item2.get("name").unwrap().as_str().unwrap(),
            "Global Item 2"
        );
        assert_eq!(item2.get("value").unwrap().as_i64().unwrap(), 200);

        // Verify last two items (from SCOPE blocks)
        let item3 = items[2].as_object().unwrap();
        assert_eq!(item3.get("name").unwrap().as_str().unwrap(), "Scope Item 1");
        assert_eq!(item3.get("value").unwrap().as_i64().unwrap(), 300);
        assert_eq!(
            item3.get("body").unwrap().as_str().unwrap(),
            "\nScope item 1 body\n\n"
        );

        let item4 = items[3].as_object().unwrap();
        assert_eq!(item4.get("name").unwrap().as_str().unwrap(), "Scope Item 2");
        assert_eq!(item4.get("value").unwrap().as_i64().unwrap(), 400);
        assert_eq!(
            item4.get("body").unwrap().as_str().unwrap(),
            "\nScope item 2 body"
        );
    }

    #[test]
    fn test_empty_global_array_with_scope() {
        // Edge case: global frontmatter has an empty array
        let markdown = r#"---
items: []
---

Global body

---
SCOPE: items
name: Item 1
---

Item 1 body"#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);

        let item = items[0].as_object().unwrap();
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");
    }

    #[test]
    fn test_reserved_field_name() {
        let markdown = r#"---
SCOPE: body
content: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_invalid_tag_syntax() {
        let markdown = r#"---
SCOPE: Invalid-Name
title: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid field name"));
    }

    #[test]
    fn test_multiple_global_frontmatter_blocks() {
        let markdown = r#"---
title: First
---

Body

---
author: Second
---

More body"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Multiple global frontmatter"));
    }

    #[test]
    fn test_adjacent_blocks_different_tags() {
        let markdown = r#"---
SCOPE: items
name: Item 1
---

Item 1 body

---
SCOPE: sections
title: Section 1
---

Section 1 body"#;

        let doc = decompose(markdown).unwrap();

        assert!(doc.get_field("items").is_some());
        assert!(doc.get_field("sections").is_some());

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);

        let sections = doc.get_field("sections").unwrap().as_sequence().unwrap();
        assert_eq!(sections.len(), 1);
    }

    #[test]
    fn test_order_preservation() {
        let markdown = r#"---
SCOPE: items
id: 1
---

First

---
SCOPE: items
id: 2
---

Second

---
SCOPE: items
id: 3
---

Third"#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 3);

        for (i, item) in items.iter().enumerate() {
            let mapping = item.as_object().unwrap();
            let id = mapping.get("id").unwrap().as_i64().unwrap();
            assert_eq!(id, (i + 1) as i64);
        }
    }

    #[test]
    fn test_product_catalog_integration() {
        let markdown = r#"---
title: Product Catalog
author: John Doe
date: 2024-01-01
---

This is the main catalog description.

---
SCOPE: products
name: Widget A
price: 19.99
sku: WID-001
---

The **Widget A** is our most popular product.

---
SCOPE: products
name: Gadget B
price: 29.99
sku: GAD-002
---

The **Gadget B** is perfect for professionals.

---
SCOPE: reviews
product: Widget A
rating: 5
---

"Excellent product! Highly recommended."

---
SCOPE: reviews
product: Gadget B
rating: 4
---

"Very good, but a bit pricey.""#;

        let doc = decompose(markdown).unwrap();

        // Verify global fields
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Product Catalog"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "John Doe"
        );
        assert_eq!(
            doc.get_field("date").unwrap().as_str().unwrap(),
            "2024-01-01"
        );

        // Verify global body
        assert!(doc.body().unwrap().contains("main catalog description"));

        // Verify products collection
        let products = doc.get_field("products").unwrap().as_sequence().unwrap();
        assert_eq!(products.len(), 2);

        let product1 = products[0].as_object().unwrap();
        assert_eq!(product1.get("name").unwrap().as_str().unwrap(), "Widget A");
        assert_eq!(product1.get("price").unwrap().as_f64().unwrap(), 19.99);

        // Verify reviews collection
        let reviews = doc.get_field("reviews").unwrap().as_sequence().unwrap();
        assert_eq!(reviews.len(), 2);

        let review1 = reviews[0].as_object().unwrap();
        assert_eq!(
            review1.get("product").unwrap().as_str().unwrap(),
            "Widget A"
        );
        assert_eq!(review1.get("rating").unwrap().as_i64().unwrap(), 5);

        // Total fields: title, author, date, body, products, reviews = 6
        assert_eq!(doc.fields().len(), 6);
    }

    #[test]
    fn taro_quill_directive() {
        let markdown = r#"---
QUILL: usaf_memo
memo_for: [ORG/SYMBOL]
memo_from: [ORG/SYMBOL]
---

This is the memo body."#;

        let doc = decompose(markdown).unwrap();

        // Verify quill tag is set
        assert_eq!(doc.quill_tag(), Some("usaf_memo"));

        // Verify fields from quill block become frontmatter
        assert_eq!(
            doc.get_field("memo_for").unwrap().as_sequence().unwrap()[0]
                .as_str()
                .unwrap(),
            "ORG/SYMBOL"
        );

        // Verify body
        assert_eq!(doc.body(), Some("\nThis is the memo body."));
    }

    #[test]
    fn test_quill_with_scope_blocks() {
        let markdown = r#"---
QUILL: document
title: Test Document
---

Main body.

---
SCOPE: sections
name: Section 1
---

Section 1 body."#;

        let doc = decompose(markdown).unwrap();

        // Verify quill tag
        assert_eq!(doc.quill_tag(), Some("document"));

        // Verify global field from quill block
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test Document"
        );

        // Verify scope blocks work
        let sections = doc.get_field("sections").unwrap().as_sequence().unwrap();
        assert_eq!(sections.len(), 1);

        // Verify body
        assert_eq!(doc.body(), Some("\nMain body.\n\n"));
    }

    #[test]
    fn test_multiple_quill_directives_error() {
        let markdown = r#"---
QUILL: first
---

---
QUILL: second
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Multiple quill directives"));
    }

    #[test]
    fn test_invalid_quill_name() {
        let markdown = r#"---
QUILL: Invalid-Name
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid quill name"));
    }

    #[test]
    fn test_quill_wrong_value_type() {
        let markdown = r#"---
QUILL: 123
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("QUILL value must be a string"));
    }

    #[test]
    fn test_scope_wrong_value_type() {
        let markdown = r#"---
SCOPE: 123
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("SCOPE value must be a string"));
    }

    #[test]
    fn test_both_quill_and_scope_error() {
        let markdown = r#"---
QUILL: test
SCOPE: items
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot specify both QUILL and SCOPE"));
    }
}
#[cfg(test)]
mod demo_file_test {
    use super::*;

    #[test]
    fn test_extended_metadata_demo_file() {
        let markdown = include_str!("../../quillmark-fixtures/resources/extended_metadata_demo.md");
        let doc = decompose(markdown).unwrap();

        // Verify global fields
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Extended Metadata Demo"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "Quillmark Team"
        );
        // version is parsed as a number by YAML
        assert_eq!(doc.get_field("version").unwrap().as_f64().unwrap(), 1.0);

        // Verify body
        assert!(doc
            .body()
            .unwrap()
            .contains("extended YAML metadata standard"));

        // Verify features collection
        let features = doc.get_field("features").unwrap().as_sequence().unwrap();
        assert_eq!(features.len(), 3);

        // Verify use_cases collection
        let use_cases = doc.get_field("use_cases").unwrap().as_sequence().unwrap();
        assert_eq!(use_cases.len(), 2);

        // Check first feature
        let feature1 = features[0].as_object().unwrap();
        assert_eq!(
            feature1.get("name").unwrap().as_str().unwrap(),
            "Tag Directives"
        );
    }

    #[test]
    fn test_input_size_limit() {
        // Create markdown larger than MAX_INPUT_SIZE (10 MB)
        let size = crate::error::MAX_INPUT_SIZE + 1;
        let large_markdown = "a".repeat(size);

        let result = decompose(&large_markdown);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Input too large"));
    }

    #[test]
    fn test_yaml_size_limit() {
        // Create YAML block larger than MAX_YAML_SIZE (1 MB)
        let mut markdown = String::from("---\n");

        // Create a very large YAML field
        let size = crate::error::MAX_YAML_SIZE + 1;
        markdown.push_str("data: \"");
        markdown.push_str(&"x".repeat(size));
        markdown.push_str("\"\n---\n\nBody");

        let result = decompose(&markdown);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("YAML block too large"));
    }

    #[test]
    fn test_input_within_size_limit() {
        // Create markdown just under the limit
        let size = 1000; // Much smaller than limit
        let markdown = format!("---\ntitle: Test\n---\n\n{}", "a".repeat(size));

        let result = decompose(&markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_yaml_within_size_limit() {
        // Create YAML block well within the limit
        let markdown = "---\ntitle: Test\nauthor: John Doe\n---\n\nBody content";

        let result = decompose(&markdown);
        assert!(result.is_ok());
    }
}
