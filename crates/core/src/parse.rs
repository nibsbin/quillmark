//! # Parsing Module
//!
//! Parsing functionality for markdown documents with YAML frontmatter.
//!
//! ## Overview
//!
//! The `parse` module provides the [`ParsedDocument::from_markdown`] function for parsing markdown documents
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
//! use quillmark_core::ParsedDocument;
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
//! let doc = ParsedDocument::from_markdown(markdown).unwrap();
//! let title = doc.get_field("title")
//!     .and_then(|v| v.as_str())
//!     .unwrap_or("Untitled");
//! ```
//!
//! ## Error Handling
//!
//! The [`ParsedDocument::from_markdown`] function returns errors for:
//! - Malformed YAML syntax
//! - Unclosed frontmatter blocks
//! - Multiple global frontmatter blocks
//! - Both QUILL and SCOPE specified in the same block
//! - Reserved field name usage
//! - Name collisions
//!
//! See [PARSE.md](https://github.com/nibsbin/quillmark/blob/main/designs/PARSE.md) for comprehensive documentation of the Extended YAML Metadata Standard.

use std::collections::HashMap;

use crate::guillemet::{preprocess_guillemets, preprocess_markdown_guillemets};
use crate::value::QuillValue;

/// The field name used to store the document body
pub const BODY_FIELD: &str = "body";

/// Helper function to convert serde_yaml::Error with location extraction
fn yaml_error_to_string(e: serde_yaml::Error, context: &str) -> String {
    let mut msg = format!("{}: {}", context, e);

    if let Some(loc) = e.location() {
        msg.push_str(&format!(" at line {}, column {}", loc.line(), loc.column()));
    }

    msg
}

/// Recursively preprocesses guillemets in YAML values
///
/// Converts `<<text>>` to `«text»` in all string values within the YAML structure.
/// For non-string values (numbers, booleans, null), they are passed through unchanged.
/// For sequences and mappings, the function recurses into their elements.
fn preprocess_yaml_guillemets(value: serde_yaml::Value) -> serde_yaml::Value {
    match value {
        serde_yaml::Value::String(s) => serde_yaml::Value::String(preprocess_guillemets(&s)),
        serde_yaml::Value::Sequence(seq) => {
            serde_yaml::Value::Sequence(seq.into_iter().map(preprocess_yaml_guillemets).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let new_map: serde_yaml::Mapping = map
                .into_iter()
                .map(|(k, v)| (k, preprocess_yaml_guillemets(v)))
                .collect();
            serde_yaml::Value::Mapping(new_map)
        }
        // Pass through other types unchanged (numbers, booleans, null, tagged)
        other => other,
    }
}

/// Reserved tag name for quill specification
pub const QUILL_TAG: &str = "quill";

/// A parsed markdown document with frontmatter
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    fields: HashMap<String, QuillValue>,
    quill_tag: String,
}

impl ParsedDocument {
    /// Create a new ParsedDocument with the given fields
    pub fn new(fields: HashMap<String, QuillValue>) -> Self {
        Self {
            fields,
            quill_tag: "__default__".to_string(),
        }
    }

    /// Create a ParsedDocument from fields and quill tag
    pub fn with_quill_tag(fields: HashMap<String, QuillValue>, quill_tag: String) -> Self {
        Self { fields, quill_tag }
    }

    /// Create a ParsedDocument from markdown string
    pub fn from_markdown(markdown: &str) -> Result<Self, crate::error::ParseError> {
        decompose(markdown).map_err(|e| crate::error::ParseError::from(e))
    }

    /// Get the quill tag (from QUILL key, or "__default__" if not specified)
    pub fn quill_tag(&self) -> &str {
        &self.quill_tag
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

    /// Create a new ParsedDocument with default values applied
    ///
    /// This method creates a new ParsedDocument with default values applied for any
    /// fields that are missing from the original document but have defaults specified.
    /// Existing fields are preserved and not overwritten.
    ///
    /// # Arguments
    ///
    /// * `defaults` - A HashMap of field names to their default QuillValues
    ///
    /// # Returns
    ///
    /// A new ParsedDocument with defaults applied for missing fields
    pub fn with_defaults(&self, defaults: &HashMap<String, QuillValue>) -> Self {
        let mut fields = self.fields.clone();

        for (field_name, default_value) in defaults {
            // Only apply default if field is missing
            if !fields.contains_key(field_name) {
                fields.insert(field_name.clone(), default_value.clone());
            }
        }

        Self {
            fields,
            quill_tag: self.quill_tag.clone(),
        }
    }

    /// Create a new ParsedDocument with coerced field values
    ///
    /// This method applies type coercions to field values based on the schema.
    /// Coercions include:
    /// - Singular values to arrays when schema expects array
    /// - String "true"/"false" to boolean
    /// - Numbers to boolean (0=false, non-zero=true)
    /// - String numbers to number type
    /// - Boolean to number (true=1, false=0)
    ///
    /// # Arguments
    ///
    /// * `schema` - A JSON Schema object defining expected field types
    ///
    /// # Returns
    ///
    /// A new ParsedDocument with coerced field values
    pub fn with_coercion(&self, schema: &QuillValue) -> Self {
        use crate::schema::coerce_document;

        let coerced_fields = coerce_document(schema, &self.fields);

        Self {
            fields: coerced_fields,
            quill_tag: self.quill_tag.clone(),
        }
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

            // Check if the delimiter is at the start of a line
            let is_start_of_line = if abs_pos == 0 {
                true
            } else {
                let char_before = markdown.as_bytes()[abs_pos - 1];
                char_before == b'\n' || char_before == b'\r'
            };

            if !is_start_of_line {
                pos = abs_pos + 1;
                continue;
            }

            let content_start = abs_pos + delimiter_len; // After "---\n" or "---\r\n"

            // Check if this --- is a horizontal rule (blank lines above AND below)
            let preceded_by_blank = if abs_pos > 0 {
                // Check if there's a blank line before the ---
                let before = &markdown[..abs_pos];
                before.ends_with("\n\n") || before.ends_with("\r\n\r\n")
            } else {
                false
            };

            let followed_by_blank = if content_start < markdown.len() {
                markdown[content_start..].starts_with('\n')
                    || markdown[content_start..].starts_with("\r\n")
            } else {
                false
            };

            // Horizontal rule: blank lines both above and below
            if preceded_by_blank && followed_by_blank {
                // This is a horizontal rule in the body, skip it
                pos = abs_pos + 3; // Skip past "---"
                continue;
            }

            // Check if followed by non-blank line (or if we're at document start)
            // This starts a metadata block
            if followed_by_blank {
                // --- followed by blank line but NOT preceded by blank line
                // This is NOT a metadata block opening, skip it
                pos = abs_pos + 3;
                continue;
            }

            // Found potential metadata block opening (followed by non-blank line)
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
fn decompose(markdown: &str) -> Result<ParsedDocument, Box<dyn std::error::Error + Send + Sync>> {
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
        // Preprocess guillemets in markdown body
        let preprocessed_body = preprocess_markdown_guillemets(markdown);
        fields.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::Value::String(preprocessed_body)),
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
                .map_err(|e| yaml_error_to_string(e, "Invalid YAML frontmatter"))?
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
        // Preprocess guillemets in all string values
        for (key, value) in yaml_fields {
            let preprocessed = preprocess_yaml_guillemets(value);
            fields.insert(key, QuillValue::from_yaml(preprocessed)?);
        }
    }

    // Process blocks with quill directives
    for block in &blocks {
        if block.quill_name.is_some() {
            // Quill directive blocks can have YAML content (becomes part of frontmatter)
            if !block.yaml_content.is_empty() {
                let yaml_fields: HashMap<String, serde_yaml::Value> =
                    serde_yaml::from_str(&block.yaml_content)
                        .map_err(|e| yaml_error_to_string(e, "Invalid YAML in quill block"))?;

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
                // Preprocess guillemets in all string values
                for (key, value) in yaml_fields {
                    let preprocessed = preprocess_yaml_guillemets(value);
                    fields.insert(key, QuillValue::from_yaml(preprocessed)?);
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
            let mut item_fields: HashMap<String, serde_yaml::Value> = if block
                .yaml_content
                .is_empty()
            {
                HashMap::new()
            } else {
                serde_yaml::from_str(&block.yaml_content).map_err(|e| {
                    yaml_error_to_string(e, &format!("Invalid YAML in tagged block '{}'", tag_name))
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

            // Preprocess guillemets in the tagged block body (markdown-aware)
            let preprocessed_body = preprocess_markdown_guillemets(body);

            // Add preprocessed body to item fields
            item_fields.insert(
                BODY_FIELD.to_string(),
                serde_yaml::Value::String(preprocessed_body),
            );

            // Preprocess guillemets in YAML string values
            let preprocessed_fields: HashMap<String, serde_yaml::Value> = item_fields
                .into_iter()
                .map(|(k, v)| (k, preprocess_yaml_guillemets(v)))
                .collect();

            // Convert HashMap to serde_yaml::Value::Mapping
            let item_value = serde_yaml::to_value(preprocessed_fields)?;

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

    // Preprocess guillemets in markdown body
    let preprocessed_global_body = preprocess_markdown_guillemets(global_body);

    fields.insert(
        BODY_FIELD.to_string(),
        QuillValue::from_json(serde_json::Value::String(preprocessed_global_body)),
    );

    // Add all tagged collections to fields (convert to QuillValue)
    // If a field already exists and is an array, merge the new items into it
    for (tag_name, items) in tagged_attributes {
        if let Some(existing_value) = fields.get(&tag_name) {
            // The existing value must be an array (checked earlier)
            if let Some(existing_array) = existing_value.as_array() {
                // Convert new items from YAML to JSON
                // Note: guillemets in items were already preprocessed when the items were created
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
            // Note: guillemets in items were already preprocessed when the items were created
            let quill_value = QuillValue::from_yaml(serde_yaml::Value::Sequence(items))?;
            fields.insert(tag_name, quill_value);
        }
    }

    let quill_tag = quill_name.unwrap_or_else(|| "__default__".to_string());
    let parsed = ParsedDocument::with_quill_tag(fields, quill_tag);

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
        // Verify default quill tag is set
        assert_eq!(doc.quill_tag(), "__default__");
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
                                           // Verify default quill tag is set when no QUILL directive
        assert_eq!(doc.quill_tag(), "__default__");
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
    fn test_with_defaults_empty_document() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("draft")),
        );
        defaults.insert(
            "version".to_string(),
            QuillValue::from_json(serde_json::json!(1)),
        );

        // Create an empty parsed document
        let doc = ParsedDocument::new(HashMap::new());
        let doc_with_defaults = doc.with_defaults(&defaults);

        // Check that defaults were applied
        assert_eq!(
            doc_with_defaults
                .get_field("status")
                .unwrap()
                .as_str()
                .unwrap(),
            "draft"
        );
        assert_eq!(
            doc_with_defaults
                .get_field("version")
                .unwrap()
                .as_number()
                .unwrap()
                .as_i64()
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_with_defaults_preserves_existing_values() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("draft")),
        );

        // Create document with existing status
        let mut fields = HashMap::new();
        fields.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("published")),
        );
        let doc = ParsedDocument::new(fields);

        let doc_with_defaults = doc.with_defaults(&defaults);

        // Existing value should be preserved
        assert_eq!(
            doc_with_defaults
                .get_field("status")
                .unwrap()
                .as_str()
                .unwrap(),
            "published"
        );
    }

    #[test]
    fn test_with_defaults_partial_application() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("draft")),
        );
        defaults.insert(
            "version".to_string(),
            QuillValue::from_json(serde_json::json!(1)),
        );

        // Create document with only one field
        let mut fields = HashMap::new();
        fields.insert(
            "status".to_string(),
            QuillValue::from_json(serde_json::json!("published")),
        );
        let doc = ParsedDocument::new(fields);

        let doc_with_defaults = doc.with_defaults(&defaults);

        // Existing field preserved, missing field gets default
        assert_eq!(
            doc_with_defaults
                .get_field("status")
                .unwrap()
                .as_str()
                .unwrap(),
            "published"
        );
        assert_eq!(
            doc_with_defaults
                .get_field("version")
                .unwrap()
                .as_number()
                .unwrap()
                .as_i64()
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_with_defaults_no_defaults() {
        use std::collections::HashMap;

        let defaults = HashMap::new(); // Empty defaults map

        let doc = ParsedDocument::new(HashMap::new());
        let doc_with_defaults = doc.with_defaults(&defaults);

        // No defaults should be applied
        assert!(doc_with_defaults.fields().is_empty());
    }

    #[test]
    fn test_with_defaults_complex_types() {
        use std::collections::HashMap;

        let mut defaults = HashMap::new();
        defaults.insert(
            "tags".to_string(),
            QuillValue::from_json(serde_json::json!(["default", "tag"])),
        );

        let doc = ParsedDocument::new(HashMap::new());
        let doc_with_defaults = doc.with_defaults(&defaults);

        // Complex default value should be applied
        let tags = doc_with_defaults
            .get_field("tags")
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str().unwrap(), "default");
        assert_eq!(tags[1].as_str().unwrap(), "tag");
    }

    #[test]
    fn test_with_coercion_singular_to_array() {
        use std::collections::HashMap;

        let schema = QuillValue::from_json(serde_json::json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "tags": {"type": "array"}
            }
        }));

        let mut fields = HashMap::new();
        fields.insert(
            "tags".to_string(),
            QuillValue::from_json(serde_json::json!("single-tag")),
        );
        let doc = ParsedDocument::new(fields);

        let coerced_doc = doc.with_coercion(&schema);

        let tags = coerced_doc.get_field("tags").unwrap();
        assert!(tags.as_array().is_some());
        let tags_array = tags.as_array().unwrap();
        assert_eq!(tags_array.len(), 1);
        assert_eq!(tags_array[0].as_str().unwrap(), "single-tag");
    }

    #[test]
    fn test_with_coercion_string_to_boolean() {
        use std::collections::HashMap;

        let schema = QuillValue::from_json(serde_json::json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "active": {"type": "boolean"}
            }
        }));

        let mut fields = HashMap::new();
        fields.insert(
            "active".to_string(),
            QuillValue::from_json(serde_json::json!("true")),
        );
        let doc = ParsedDocument::new(fields);

        let coerced_doc = doc.with_coercion(&schema);

        assert_eq!(
            coerced_doc.get_field("active").unwrap().as_bool().unwrap(),
            true
        );
    }

    #[test]
    fn test_with_coercion_string_to_number() {
        use std::collections::HashMap;

        let schema = QuillValue::from_json(serde_json::json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "count": {"type": "number"}
            }
        }));

        let mut fields = HashMap::new();
        fields.insert(
            "count".to_string(),
            QuillValue::from_json(serde_json::json!("42")),
        );
        let doc = ParsedDocument::new(fields);

        let coerced_doc = doc.with_coercion(&schema);

        assert_eq!(
            coerced_doc.get_field("count").unwrap().as_i64().unwrap(),
            42
        );
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
        assert_eq!(doc.quill_tag(), "usaf_memo");

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
        assert_eq!(doc.quill_tag(), "document");

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

    #[test]
    fn test_blank_lines_in_frontmatter() {
        // New parsing standard: blank lines are allowed within YAML blocks
        let markdown = r#"---
title: Test Document
author: Test Author

description: This has a blank line above it
tags:
  - one
  - two
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
        assert_eq!(
            doc.get_field("description").unwrap().as_str().unwrap(),
            "This has a blank line above it"
        );

        let tags = doc.get_field("tags").unwrap().as_sequence().unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_blank_lines_in_scope_blocks() {
        // Blank lines should be allowed in SCOPE blocks too
        let markdown = r#"---
SCOPE: items
name: Item 1

price: 19.99

tags:
  - electronics
  - gadgets
---

Body of item 1."#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);

        let item = items[0].as_object().unwrap();
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");
        assert_eq!(item.get("price").unwrap().as_f64().unwrap(), 19.99);

        let tags = item.get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_horizontal_rule_with_blank_lines_above_and_below() {
        // Horizontal rule: blank lines both above AND below the ---
        let markdown = r#"---
title: Test
---

First paragraph.

---

Second paragraph."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");

        // The body should contain the horizontal rule (---) as part of the content
        let body = doc.body().unwrap();
        assert!(body.contains("First paragraph."));
        assert!(body.contains("---"));
        assert!(body.contains("Second paragraph."));
    }

    #[test]
    fn test_horizontal_rule_not_preceded_by_blank() {
        // --- not preceded by blank line but followed by blank line is NOT a horizontal rule
        // It's also NOT a valid metadata block opening (since it's followed by blank)
        let markdown = r#"---
title: Test
---

First paragraph.
---

Second paragraph."#;

        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // The second --- should be in the body as text (not a horizontal rule since no blank above)
        assert!(body.contains("---"));
    }

    #[test]
    fn test_multiple_blank_lines_in_yaml() {
        // Multiple blank lines should also be allowed
        let markdown = r#"---
title: Test


author: John Doe


version: 1.0
---

Body content."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "John Doe"
        );
        assert_eq!(doc.get_field("version").unwrap().as_f64().unwrap(), 1.0);
    }

    #[test]
    fn test_html_comment_interaction() {
        let markdown = r#"<!---
---> the rest of the page content

---
key: value
---
"#;
        let doc = decompose(markdown).unwrap();

        // The comment should be ignored (or at least not cause a parse error)
        // The frontmatter should be parsed
        let key = doc.get_field("key").and_then(|v| v.as_str());
        assert_eq!(key, Some("value"));
    }
}
#[cfg(test)]
mod demo_file_test {
    use super::*;

    #[test]
    fn test_extended_metadata_demo_file() {
        let markdown = include_str!("../../fixtures/resources/extended_metadata_demo.md");
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

    // Tests for guillemet preprocessing in parsing
    #[test]
    fn test_guillemet_in_body_no_frontmatter() {
        let markdown = "Use <<raw content>> here.";
        let doc = decompose(markdown).unwrap();

        // Body should have guillemets converted
        assert_eq!(doc.body(), Some("Use «raw content» here."));
    }

    #[test]
    fn test_guillemet_in_body_with_frontmatter() {
        let markdown = r#"---
title: Test
---

Use <<raw content>> here."#;
        let doc = decompose(markdown).unwrap();

        // Body should have guillemets converted
        assert_eq!(doc.body(), Some("\nUse «raw content» here."));
    }

    #[test]
    fn test_guillemet_in_yaml_string() {
        let markdown = r#"---
title: Test <<with chevrons>>
---

Body content."#;
        let doc = decompose(markdown).unwrap();

        // YAML string values should have guillemets converted
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test «with chevrons»"
        );
    }

    #[test]
    fn test_guillemet_in_yaml_array() {
        let markdown = r#"---
items:
  - "<<first>>"
  - "<<second>>"
---

Body."#;
        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items[0].as_str().unwrap(), "«first»");
        assert_eq!(items[1].as_str().unwrap(), "«second»");
    }

    #[test]
    fn test_guillemet_in_yaml_nested() {
        let markdown = r#"---
metadata:
  description: "<<nested value>>"
---

Body."#;
        let doc = decompose(markdown).unwrap();

        let metadata = doc.get_field("metadata").unwrap().as_object().unwrap();
        assert_eq!(
            metadata.get("description").unwrap().as_str().unwrap(),
            "«nested value»"
        );
    }

    #[test]
    fn test_guillemet_in_body_skips_code_blocks() {
        let markdown = r#"```
<<not converted>>
```

<<converted>>"#;
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // Code block content should NOT be converted
        assert!(body.contains("<<not converted>>"));
        // Regular content should be converted
        assert!(body.contains("«converted»"));
    }

    #[test]
    fn test_guillemet_in_body_skips_inline_code() {
        let markdown = "`<<not converted>>` and <<converted>>";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // Inline code should NOT be converted
        assert!(body.contains("`<<not converted>>`"));
        // Regular content should be converted
        assert!(body.contains("«converted»"));
    }

    #[test]
    fn test_guillemet_in_tagged_block_body() {
        let markdown = r#"---
title: Main
---

Main body.

---
SCOPE: items
name: Item 1
---

Use <<raw>> here."#;
        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        let item = items[0].as_object().unwrap();
        let item_body = item.get("body").unwrap().as_str().unwrap();
        // Tagged block body should have guillemets converted
        assert!(item_body.contains("«raw»"));
    }

    #[test]
    fn test_guillemet_in_tagged_block_yaml() {
        let markdown = r#"---
title: Main
---

Main body.

---
SCOPE: items
description: "<<tagged yaml>>"
---

Item body."#;
        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        let item = items[0].as_object().unwrap();
        // Tagged block YAML should have guillemets converted
        assert_eq!(
            item.get("description").unwrap().as_str().unwrap(),
            "«tagged yaml»"
        );
    }

    #[test]
    fn test_guillemet_not_converted_in_yaml_numbers() {
        // Numbers should not be affected
        let markdown = r#"---
count: 42
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("count").unwrap().as_i64().unwrap(), 42);
    }

    #[test]
    fn test_guillemet_not_converted_in_yaml_booleans() {
        // Booleans should not be affected
        let markdown = r#"---
active: true
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("active").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_guillemet_multiline_not_converted() {
        // Multiline guillemets should not be converted
        let markdown = "<<text\nacross lines>>";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // Should NOT contain guillemets since content spans lines
        assert!(!body.contains('«'));
        assert!(!body.contains('»'));
    }

    #[test]
    fn test_guillemet_unmatched_not_converted() {
        let markdown = "<<unmatched";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // Unmatched should remain as-is
        assert_eq!(body, "<<unmatched");
    }
}

// Additional robustness tests
#[cfg(test)]
mod robustness_tests {
    use super::*;

    // Edge cases for delimiter handling

    #[test]
    fn test_empty_document() {
        let doc = decompose("").unwrap();
        assert_eq!(doc.body(), Some(""));
        assert_eq!(doc.quill_tag(), "__default__");
    }

    #[test]
    fn test_only_whitespace() {
        let doc = decompose("   \n\n   \t").unwrap();
        assert_eq!(doc.body(), Some("   \n\n   \t"));
    }

    #[test]
    fn test_only_dashes() {
        // Just "---" at document start without newline is not treated as frontmatter opener
        // (requires "---\n" to start a frontmatter block)
        let result = decompose("---");
        // This is NOT an error - "---" alone without newline is just body content
        assert!(result.is_ok());
        assert_eq!(result.unwrap().body(), Some("---"));
    }

    #[test]
    fn test_dashes_in_middle_of_line() {
        // --- not at start of line should not be treated as delimiter
        let markdown = "some text --- more text";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.body(), Some("some text --- more text"));
    }

    #[test]
    fn test_four_dashes() {
        // ---- is not a valid delimiter
        let markdown = "----\ntitle: Test\n----\n\nBody";
        let doc = decompose(markdown).unwrap();
        // Should treat entire content as body
        assert!(doc.body().unwrap().contains("----"));
    }

    #[test]
    fn test_crlf_line_endings() {
        // Windows-style line endings
        let markdown = "---\r\ntitle: Test\r\n---\r\n\r\nBody content.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
        assert!(doc.body().unwrap().contains("Body content."));
    }

    #[test]
    fn test_mixed_line_endings() {
        // Mix of \n and \r\n
        let markdown = "---\ntitle: Test\r\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
    }

    #[test]
    fn test_frontmatter_at_eof_no_trailing_newline() {
        // Frontmatter closed at EOF without trailing newline
        let markdown = "---\ntitle: Test\n---";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test");
        assert_eq!(doc.body(), Some(""));
    }

    #[test]
    fn test_empty_frontmatter() {
        // Empty frontmatter block - requires content between delimiters
        // "---\n---" is not valid because --- followed by --- (blank line then ---)
        // is treated as horizontal rule logic, not empty frontmatter
        // A valid empty frontmatter would be "---\n \n---" (with whitespace content)
        let markdown = "---\n \n---\n\nBody content.";
        let doc = decompose(markdown).unwrap();
        assert!(doc.body().unwrap().contains("Body content."));
        // Should only have body field
        assert_eq!(doc.fields().len(), 1);
    }

    #[test]
    fn test_whitespace_only_frontmatter() {
        // Frontmatter with only whitespace
        let markdown = "---\n   \n\n   \n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert!(doc.body().unwrap().contains("Body."));
    }

    // Unicode handling

    #[test]
    fn test_unicode_in_yaml_keys() {
        let markdown = "---\ntitre: Bonjour\nタイトル: こんにちは\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("titre").unwrap().as_str().unwrap(), "Bonjour");
        assert_eq!(
            doc.get_field("タイトル").unwrap().as_str().unwrap(),
            "こんにちは"
        );
    }

    #[test]
    fn test_unicode_in_yaml_values() {
        let markdown = "---\ntitle: 你好世界 🎉\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "你好世界 🎉"
        );
    }

    #[test]
    fn test_unicode_in_body() {
        let markdown = "---\ntitle: Test\n---\n\n日本語テキスト with emoji 🚀";
        let doc = decompose(markdown).unwrap();
        assert!(doc.body().unwrap().contains("日本語テキスト"));
        assert!(doc.body().unwrap().contains("🚀"));
    }

    // YAML edge cases

    #[test]
    fn test_yaml_multiline_string() {
        let markdown = r#"---
description: |
  This is a
  multiline string
  with preserved newlines.
---

Body."#;
        let doc = decompose(markdown).unwrap();
        let desc = doc.get_field("description").unwrap().as_str().unwrap();
        assert!(desc.contains("multiline string"));
        assert!(desc.contains('\n'));
    }

    #[test]
    fn test_yaml_folded_string() {
        let markdown = r#"---
description: >
  This is a folded
  string that becomes
  a single line.
---

Body."#;
        let doc = decompose(markdown).unwrap();
        let desc = doc.get_field("description").unwrap().as_str().unwrap();
        // Folded strings join lines with spaces
        assert!(desc.contains("folded"));
    }

    #[test]
    fn test_yaml_null_value() {
        let markdown = "---\noptional: null\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert!(doc.get_field("optional").unwrap().is_null());
    }

    #[test]
    fn test_yaml_empty_string_value() {
        let markdown = "---\nempty: \"\"\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("empty").unwrap().as_str().unwrap(), "");
    }

    #[test]
    fn test_yaml_special_characters_in_string() {
        let markdown = "---\nspecial: \"colon: here, and [brackets]\"\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(
            doc.get_field("special").unwrap().as_str().unwrap(),
            "colon: here, and [brackets]"
        );
    }

    #[test]
    fn test_yaml_nested_objects() {
        let markdown = r#"---
config:
  database:
    host: localhost
    port: 5432
  cache:
    enabled: true
---

Body."#;
        let doc = decompose(markdown).unwrap();
        let config = doc.get_field("config").unwrap().as_object().unwrap();
        let db = config.get("database").unwrap().as_object().unwrap();
        assert_eq!(db.get("host").unwrap().as_str().unwrap(), "localhost");
        assert_eq!(db.get("port").unwrap().as_i64().unwrap(), 5432);
    }

    // SCOPE block edge cases

    #[test]
    fn test_scope_with_empty_body() {
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
    fn test_scope_consecutive_blocks() {
        let markdown = r#"---
SCOPE: a
id: 1
---
---
SCOPE: a
id: 2
---"#;
        let doc = decompose(markdown).unwrap();
        let items = doc.get_field("a").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_scope_with_body_containing_dashes() {
        let markdown = r#"---
SCOPE: items
name: Item
---

Some text with --- dashes in it."#;
        let doc = decompose(markdown).unwrap();
        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        let item = items[0].as_object().unwrap();
        let body = item.get("body").unwrap().as_str().unwrap();
        assert!(body.contains("--- dashes"));
    }

    // QUILL directive edge cases

    #[test]
    fn test_quill_with_underscore_prefix() {
        let markdown = "---\nQUILL: _internal\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.quill_tag(), "_internal");
    }

    #[test]
    fn test_quill_with_numbers() {
        let markdown = "---\nQUILL: form_8_v2\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.quill_tag(), "form_8_v2");
    }

    #[test]
    fn test_quill_with_additional_fields() {
        let markdown = r#"---
QUILL: my_quill
title: Document Title
author: John Doe
---

Body content."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.quill_tag(), "my_quill");
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Document Title"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "John Doe"
        );
    }

    // Error handling

    #[test]
    fn test_invalid_scope_name_uppercase() {
        let markdown = "---\nSCOPE: ITEMS\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid field name"));
    }

    #[test]
    fn test_invalid_scope_name_starts_with_number() {
        let markdown = "---\nSCOPE: 123items\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_scope_name_with_hyphen() {
        let markdown = "---\nSCOPE: my-items\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_quill_name_uppercase() {
        let markdown = "---\nQUILL: MyQuill\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_syntax_error_missing_colon() {
        let markdown = "---\ntitle Test\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_syntax_error_bad_indentation() {
        let markdown = "---\nitems:\n- one\n - two\n---\n\nBody.";
        let result = decompose(markdown);
        // Bad indentation may or may not be an error depending on YAML parser
        // Just ensure it doesn't panic
        let _ = result;
    }

    // Body extraction edge cases

    #[test]
    fn test_body_with_leading_newlines() {
        let markdown = "---\ntitle: Test\n---\n\n\n\nBody with leading newlines.";
        let doc = decompose(markdown).unwrap();
        // Body should preserve leading newlines after frontmatter
        assert!(doc.body().unwrap().starts_with('\n'));
    }

    #[test]
    fn test_body_with_trailing_newlines() {
        let markdown = "---\ntitle: Test\n---\n\nBody.\n\n\n";
        let doc = decompose(markdown).unwrap();
        // Body should preserve trailing newlines
        assert!(doc.body().unwrap().ends_with('\n'));
    }

    #[test]
    fn test_no_body_after_frontmatter() {
        let markdown = "---\ntitle: Test\n---";
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.body(), Some(""));
    }

    // Tag name validation

    #[test]
    fn test_valid_tag_name_single_underscore() {
        assert!(is_valid_tag_name("_"));
    }

    #[test]
    fn test_valid_tag_name_underscore_prefix() {
        assert!(is_valid_tag_name("_private"));
    }

    #[test]
    fn test_valid_tag_name_with_numbers() {
        assert!(is_valid_tag_name("item1"));
        assert!(is_valid_tag_name("item_2"));
    }

    #[test]
    fn test_invalid_tag_name_empty() {
        assert!(!is_valid_tag_name(""));
    }

    #[test]
    fn test_invalid_tag_name_starts_with_number() {
        assert!(!is_valid_tag_name("1item"));
    }

    #[test]
    fn test_invalid_tag_name_uppercase() {
        assert!(!is_valid_tag_name("Items"));
        assert!(!is_valid_tag_name("ITEMS"));
    }

    #[test]
    fn test_invalid_tag_name_special_chars() {
        assert!(!is_valid_tag_name("my-items"));
        assert!(!is_valid_tag_name("my.items"));
        assert!(!is_valid_tag_name("my items"));
    }

    // Guillemet preprocessing in YAML

    #[test]
    fn test_guillemet_in_yaml_preserves_non_strings() {
        let markdown = r#"---
count: 42
price: 19.99
active: true
items:
  - first
  - 100
  - true
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("count").unwrap().as_i64().unwrap(), 42);
        assert_eq!(doc.get_field("price").unwrap().as_f64().unwrap(), 19.99);
        assert_eq!(doc.get_field("active").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_guillemet_double_conversion_prevention() {
        // Ensure «» in input doesn't get double-processed
        let markdown = "---\ntitle: Already «converted»\n---\n\nBody.";
        let doc = decompose(markdown).unwrap();
        // Should remain as-is (not double-escaped)
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Already «converted»"
        );
    }
}
