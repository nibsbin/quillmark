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
//! - Both QUILL and CARD specified in the same block
//! - Reserved field name usage
//! - Name collisions
//!
//! See [PARSE.md](https://github.com/nibsbin/quillmark/blob/main/designs/PARSE.md) for comprehensive documentation of the Extended YAML Metadata Standard.

use std::collections::HashMap;

use crate::value::QuillValue;

/// The field name used to store the document body
pub const BODY_FIELD: &str = "BODY";

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
        decompose(markdown)
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
    start: usize,                          // Position of opening "---"
    end: usize,                            // Position after closing "---\n"
    yaml_value: Option<serde_json::Value>, // Parsed YAML as JSON (None if empty or parse failed)
    tag: Option<String>,                   // Field name from CARD key
    quill_name: Option<String>,            // Quill name from QUILL key
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

/// Check if a position is inside a fenced code block
///
/// This uses strict fence detection per EXTENDED_MARKDOWN.md specification:
/// - Only exactly 3 backticks (```) are valid fences
/// - Tildes (~~~) are NOT treated as fences
/// - 4+ backticks are NOT treated as fences
fn is_inside_fenced_block(markdown: &str, pos: usize) -> bool {
    let before = &markdown[..pos];
    let mut in_fence = false;

    // Check if document starts with exactly ```
    if is_exact_fence_at(before, 0) {
        in_fence = !in_fence;
    }

    // Scan for fence toggles after newlines
    for (i, _) in before.match_indices('\n') {
        if is_exact_fence_at(before, i + 1) {
            in_fence = !in_fence;
        }
    }

    in_fence
}

/// Check if position starts exactly 3 backticks (not 2, not 4+)
///
/// Strict specification: only exactly ``` is a valid fence marker.
fn is_exact_fence_at(text: &str, pos: usize) -> bool {
    if pos >= text.len() {
        return false;
    }
    let remaining = &text[pos..];
    if !remaining.starts_with("```") {
        return false;
    }
    // Ensure it's exactly 3 backticks (4th char is not a backtick)
    remaining.len() == 3 || remaining.as_bytes().get(3) != Some(&b'`')
}

/// Creates serde_saphyr Options with security budgets configured.
///
/// Uses MAX_YAML_DEPTH from error.rs to limit nesting depth at the parser level,
/// which is more robust than heuristic-based pre-parse checks.
fn yaml_parse_options() -> serde_saphyr::Options {
    let budget = serde_saphyr::Budget {
        max_depth: crate::error::MAX_YAML_DEPTH,
        ..Default::default()
    };
    serde_saphyr::Options {
        budget: Some(budget),
        ..Default::default()
    }
}

/// Find all metadata blocks in the document
fn find_metadata_blocks(markdown: &str) -> Result<Vec<MetadataBlock>, crate::error::ParseError> {
    let mut blocks = Vec::new();
    let mut pos = 0;

    while pos < markdown.len() {
        // Look for opening "---\n" or "---\r\n"
        let search_str = &markdown[pos..];
        let delimiter_result = search_str
            .find("---\n")
            .map(|p| (p, 4, "\n"))
            .or_else(|| search_str.find("---\r\n").map(|p| (p, 5, "\r\n")));

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

            // Skip if inside a fenced code block
            if is_inside_fenced_block(markdown, abs_pos) {
                pos = abs_pos + 3;
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
                    return Err(crate::error::ParseError::InputTooLarge {
                        size: content.len(),
                        max: crate::error::MAX_YAML_SIZE,
                    });
                }

                // Parse YAML content to check for reserved keys (QUILL, CARD)
                // Uses configured budget to limit nesting depth (prevents stack overflow)
                // Normalize: treat whitespace-only content as empty frontmatter
                let content = content.trim();
                let (tag, quill_name, yaml_value) = if !content.is_empty() {
                    // Try to parse the YAML with security budgets
                    match serde_saphyr::from_str_with_options::<serde_json::Value>(
                        content,
                        yaml_parse_options(),
                    ) {
                        Ok(parsed_yaml) => {
                            if let Some(mapping) = parsed_yaml.as_object() {
                                let quill_key = "QUILL";
                                let card_key = "CARD";

                                let has_quill = mapping.contains_key(quill_key);
                                let has_card = mapping.contains_key(card_key);

                                if has_quill && has_card {
                                    return Err(crate::error::ParseError::InvalidStructure(
                                        "Cannot specify both QUILL and CARD in the same block"
                                            .to_string(),
                                    ));
                                }

                                // Check for reserved field names (BODY, CARDS)
                                const RESERVED_FIELDS: &[&str] = &["BODY", "CARDS"];
                                for reserved in RESERVED_FIELDS {
                                    if mapping.contains_key(*reserved) {
                                        return Err(crate::error::ParseError::InvalidStructure(
                                            format!(
                                                "Reserved field name '{}' cannot be used in YAML frontmatter",
                                                reserved
                                            ),
                                        ));
                                    }
                                }

                                if has_quill {
                                    // Extract quill name
                                    let quill_value = mapping.get(quill_key).unwrap();
                                    let quill_name_str = quill_value
                                        .as_str()
                                        .ok_or("QUILL value must be a string")?;

                                    if !is_valid_tag_name(quill_name_str) {
                                        return Err(crate::error::ParseError::InvalidStructure(format!(
                                            "Invalid quill name '{}': must match pattern [a-z_][a-z0-9_]*",
                                            quill_name_str
                                        )));
                                    }

                                    // Remove QUILL from the YAML value for processing
                                    let mut new_mapping = mapping.clone();
                                    new_mapping.remove(quill_key);
                                    let new_value = if new_mapping.is_empty() {
                                        None
                                    } else {
                                        Some(serde_json::Value::Object(new_mapping))
                                    };

                                    (None, Some(quill_name_str.to_string()), new_value)
                                } else if has_card {
                                    // Extract card field name
                                    let card_value = mapping.get(card_key).unwrap();
                                    let field_name =
                                        card_value.as_str().ok_or("CARD value must be a string")?;

                                    if !is_valid_tag_name(field_name) {
                                        return Err(crate::error::ParseError::InvalidStructure(format!(
                                            "Invalid card field name '{}': must match pattern [a-z_][a-z0-9_]*",
                                            field_name
                                        )));
                                    }

                                    // Remove CARD from the YAML value for processing
                                    let mut new_mapping = mapping.clone();
                                    new_mapping.remove(card_key);
                                    let new_value = if new_mapping.is_empty() {
                                        None
                                    } else {
                                        Some(serde_json::Value::Object(new_mapping))
                                    };

                                    (Some(field_name.to_string()), None, new_value)
                                } else {
                                    // No reserved keys, keep the parsed YAML
                                    (None, None, Some(parsed_yaml))
                                }
                            } else {
                                // Not a mapping, keep the parsed YAML (could be null for whitespace)
                                (None, None, Some(parsed_yaml))
                            }
                        }
                        Err(e) => {
                            // Calculate line number for the start of this block
                            let block_start_line = markdown[..abs_pos].lines().count() + 1;
                            return Err(crate::error::ParseError::YamlErrorWithLocation {
                                message: e.to_string(),
                                line: block_start_line,
                                block_index: blocks.len(),
                            });
                        }
                    }
                } else {
                    // Empty content
                    (None, None, None)
                };

                blocks.push(MetadataBlock {
                    start: abs_pos,
                    end: abs_closing_pos + closing_len, // After closing delimiter
                    yaml_value,
                    tag,
                    quill_name,
                });

                // Check card count limit to prevent memory exhaustion
                if blocks.len() > crate::error::MAX_CARD_COUNT {
                    return Err(crate::error::ParseError::InputTooLarge {
                        size: blocks.len(),
                        max: crate::error::MAX_CARD_COUNT,
                    });
                }

                pos = abs_closing_pos + closing_len;
            } else if abs_pos == 0 {
                // Frontmatter started but not closed
                return Err(crate::error::ParseError::InvalidStructure(
                    "Frontmatter started but not closed with ---".to_string(),
                ));
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
fn decompose(markdown: &str) -> Result<ParsedDocument, crate::error::ParseError> {
    // Check input size limit
    if markdown.len() > crate::error::MAX_INPUT_SIZE {
        return Err(crate::error::ParseError::InputTooLarge {
            size: markdown.len(),
            max: crate::error::MAX_INPUT_SIZE,
        });
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

    // Collect all card items into unified CARDS array
    let mut cards_array: Vec<serde_json::Value> = Vec::new();
    let mut global_frontmatter_index: Option<usize> = None;
    let mut quill_name: Option<String> = None;

    // First pass: identify global frontmatter, quill directive, and validate
    for (idx, block) in blocks.iter().enumerate() {
        if idx == 0 {
            // Top-level frontmatter: can have QUILL or neither (not considered a card)
            if let Some(ref name) = block.quill_name {
                quill_name = Some(name.clone());
            }
            // If it has neither QUILL nor CARD, it's global frontmatter
            if block.tag.is_none() && block.quill_name.is_none() {
                global_frontmatter_index = Some(idx);
            }
        } else {
            // Inline blocks (idx > 0): MUST have CARD, cannot have QUILL
            if block.quill_name.is_some() {
                return Err(crate::error::ParseError::InvalidStructure("QUILL directive can only appear in the top-level frontmatter, not in inline blocks. Use CARD instead.".to_string()));
            }
            if block.tag.is_none() {
                // Inline block without CARD
                return Err(crate::error::ParseError::missing_card_directive());
            }
        }
    }

    // Parse global frontmatter if present
    if let Some(idx) = global_frontmatter_index {
        let block = &blocks[idx];

        // Get parsed JSON fields directly (already parsed in find_metadata_blocks)
        let json_fields: HashMap<String, serde_json::Value> = match &block.yaml_value {
            Some(serde_json::Value::Object(mapping)) => mapping
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            Some(serde_json::Value::Null) => {
                // Null value (from whitespace-only YAML) - treat as empty mapping
                HashMap::new()
            }
            Some(_) => {
                // Non-mapping, non-null YAML (e.g., scalar, sequence) - this is an error for frontmatter
                return Err(crate::error::ParseError::InvalidStructure(
                    "Invalid YAML frontmatter: expected a mapping".to_string(),
                ));
            }
            None => HashMap::new(),
        };

        // Convert JSON values to QuillValue at boundary
        for (key, value) in json_fields {
            fields.insert(key, QuillValue::from_json(value));
        }
    }

    // Process blocks with quill directives
    for block in &blocks {
        if block.quill_name.is_some() {
            // Quill directive blocks can have YAML content (becomes part of frontmatter)
            if let Some(ref json_val) = block.yaml_value {
                let json_fields: HashMap<String, serde_json::Value> = match json_val {
                    serde_json::Value::Object(mapping) => mapping
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    serde_json::Value::Null => {
                        // Null value (from whitespace-only YAML) - treat as empty mapping
                        HashMap::new()
                    }
                    _ => {
                        return Err(crate::error::ParseError::InvalidStructure(
                            "Invalid YAML in quill block: expected a mapping".to_string(),
                        ));
                    }
                };

                // Check for conflicts with existing fields
                for key in json_fields.keys() {
                    if fields.contains_key(key) {
                        return Err(crate::error::ParseError::InvalidStructure(format!(
                            "Name collision: quill block field '{}' conflicts with existing field",
                            key
                        )));
                    }
                }

                // Convert JSON values to QuillValue at boundary
                for (key, value) in json_fields {
                    fields.insert(key, QuillValue::from_json(value));
                }
            }
        }
    }

    // Parse tagged blocks (CARD blocks)
    for (idx, block) in blocks.iter().enumerate() {
        if let Some(ref tag_name) = block.tag {
            // Get YAML metadata directly (already parsed in find_metadata_blocks)
            // Get JSON metadata directly (already parsed in find_metadata_blocks)
            let mut item_fields: serde_json::Map<String, serde_json::Value> =
                match &block.yaml_value {
                    Some(serde_json::Value::Object(mapping)) => mapping.clone(),
                    Some(serde_json::Value::Null) => {
                        // Null value (from whitespace-only YAML) - treat as empty mapping
                        serde_json::Map::new()
                    }
                    Some(_) => {
                        return Err(crate::error::ParseError::InvalidStructure(format!(
                            "Invalid YAML in card block '{}': expected a mapping",
                            tag_name
                        )));
                    }
                    None => serde_json::Map::new(),
                };

            // Extract body for this card block
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
                serde_json::Value::String(body.to_string()),
            );

            // Add CARD discriminator field
            item_fields.insert(
                "CARD".to_string(),
                serde_json::Value::String(tag_name.clone()),
            );

            // Add to CARDS array
            cards_array.push(serde_json::Value::Object(item_fields));
        }
    }

    // Extract global body
    // Body starts after global frontmatter or quill block (whichever comes first)
    // Body ends at the first card block or EOF
    let first_non_card_block_idx = blocks
        .iter()
        .position(|b| b.tag.is_none() && b.quill_name.is_none())
        .or_else(|| blocks.iter().position(|b| b.quill_name.is_some()));

    let (body_start, body_end) = if let Some(idx) = first_non_card_block_idx {
        // Body starts after the first non-card block (global frontmatter or quill)
        let start = blocks[idx].end;

        // Body ends at the first card block after this, or EOF
        let end = blocks
            .iter()
            .skip(idx + 1)
            .find(|b| b.tag.is_some())
            .map(|b| b.start)
            .unwrap_or(markdown.len());

        (start, end)
    } else {
        // No global frontmatter or quill block - body is everything before the first card block
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

    // Always add CARDS array to fields (may be empty)
    fields.insert(
        "CARDS".to_string(),
        QuillValue::from_json(serde_json::Value::Array(cards_array)),
    );

    // Check field count limit to prevent memory exhaustion
    if fields.len() > crate::error::MAX_FIELD_COUNT {
        return Err(crate::error::ParseError::InputTooLarge {
            size: fields.len(),
            max: crate::error::MAX_FIELD_COUNT,
        });
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
        assert_eq!(doc.fields().len(), 4); // title, author, body, CARDS
                                           // Verify default quill tag is set when no QUILL directive
        assert_eq!(doc.quill_tag(), "__default__");
    }

    #[test]
    fn test_whitespace_frontmatter() {
        // Frontmatter with only whitespace should be treated as empty/valid
        // and not error out or be treated as null YAML
        let markdown = "---\n   \n---\n\n# Hello";
        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\n# Hello"));
        // Should have default fields (BODY + CARDS) but no others
        // (unless defaults are applied later, but decompose returns basics)
        assert!(doc.get_field("title").is_none());
        assert_eq!(doc.fields().len(), 2); // BODY, CARDS
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

        assert!(coerced_doc.get_field("active").unwrap().as_bool().unwrap());
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
        // Error message now includes location context
        assert!(result.unwrap_err().to_string().contains("YAML error"));
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
CARD: items
name: Item 1
---

Body of item 1."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\nMain body content.\n\n"));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Main Document"
        );

        // Cards are now in CARDS array with CARD discriminator
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");
        assert_eq!(
            item.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "\nBody of item 1."
        );
    }

    #[test]
    fn test_multiple_tagged_blocks() {
        let markdown = r#"---
CARD: items
name: Item 1
tags: [a, b]
---

First item body.

---
CARD: items
name: Item 2
tags: [c, d]
---

Second item body."#;

        let doc = decompose(markdown).unwrap();

        // Cards are in CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 2);

        let item1 = cards[0].as_object().unwrap();
        assert_eq!(item1.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item1.get("name").unwrap().as_str().unwrap(), "Item 1");

        let item2 = cards[1].as_object().unwrap();
        assert_eq!(item2.get("CARD").unwrap().as_str().unwrap(), "items");
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
CARD: sections
title: Section 1
---

Section 1 content.

---
CARD: sections
title: Section 2
---

Section 2 content."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Global");
        assert_eq!(doc.body(), Some("\nGlobal body.\n\n"));

        // Cards are in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(
            cards[0]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "sections"
        );
    }

    #[test]
    fn test_empty_tagged_metadata() {
        let markdown = r#"---
CARD: items
---

Body without metadata."#;

        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(
            item.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "\nBody without metadata."
        );
    }

    #[test]
    fn test_tagged_block_without_body() {
        let markdown = r#"---
CARD: items
name: Item
---"#;

        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get(BODY_FIELD).unwrap().as_str().unwrap(), "");
    }

    #[test]
    fn test_name_collision_global_and_tagged() {
        let markdown = r#"---
items: "global value"
---

Body

---
CARD: items
name: Item
---

Item body"#;

        let result = decompose(markdown);
        assert!(result.is_ok(), "Name collision should be allowed now");
    }

    #[test]
    fn test_card_name_collision_with_array_field() {
        // CARD type names CAN now conflict with frontmatter field names
        let markdown = r#"---
items:
  - name: Global Item 1
    value: 100
---

Global body

---
CARD: items
name: Scope Item 1
---

Scope item 1 body"#;

        let result = decompose(markdown);
        assert!(
            result.is_ok(),
            "Collision with array field should be allowed"
        );
    }

    #[test]
    fn test_empty_global_array_with_card() {
        // CARD type names CAN now conflict with frontmatter field names
        let markdown = r#"---
items: []
---

Global body

---
CARD: items
name: Item 1
---

Item 1 body"#;

        let result = decompose(markdown);
        assert!(
            result.is_ok(),
            "Collision with empty array field should be allowed"
        );
    }

    #[test]
    fn test_reserved_field_body_rejected() {
        let markdown = r#"---
CARD: section
BODY: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err(), "BODY is a reserved field name");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Reserved field name"));
    }

    #[test]
    fn test_reserved_field_cards_rejected() {
        let markdown = r#"---
title: Test
CARDS: []
---"#;

        let result = decompose(markdown);
        assert!(result.is_err(), "CARDS is a reserved field name");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Reserved field name"));
    }

    #[test]
    fn test_delimiter_inside_fenced_code_block_backticks() {
        let markdown = r#"---
title: Test
---
Here is some code:

```yaml
---
fake: frontmatter
---
```

More content.
"#;

        let doc = decompose(markdown).unwrap();
        // The --- inside the code block should NOT be parsed as metadata
        assert!(doc.body().unwrap().contains("fake: frontmatter"));
        assert!(doc.get_field("fake").is_none());
    }

    #[test]
    fn test_tildes_are_not_fences() {
        // Per EXTENDED_MARKDOWN.md: tildes (~~~) are NOT treated as fences
        // So --- inside ~~~ WILL be parsed as a metadata block
        let markdown = r#"---
title: Test
---
Here is some code:

~~~yaml
---
CARD: code_example
fake: frontmatter
---
~~~

More content.
"#;

        let doc = decompose(markdown).unwrap();
        // The --- should be parsed as a CARD block since tildes aren't fences
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0].get("fake").unwrap().as_str().unwrap(),
            "frontmatter"
        );
    }

    #[test]
    fn test_four_backticks_are_not_fences() {
        // Per EXTENDED_MARKDOWN.md: only exactly 3 backticks are valid fences
        // 4+ backticks are NOT treated as fences
        let markdown = r#"---
title: Test
---
Here is some code:

````yaml
---
CARD: code_example
fake: frontmatter
---
````

More content.
"#;

        let doc = decompose(markdown).unwrap();
        // The --- should be parsed as a CARD block since 4 backticks aren't a fence
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0].get("fake").unwrap().as_str().unwrap(),
            "frontmatter"
        );
    }

    #[test]
    fn test_invalid_tag_syntax() {
        let markdown = r#"---
CARD: Invalid-Name
title: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid card field name"));
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

        // Verify the error message contains CARD hint
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("CARD"),
            "Error should mention CARD directive: {}",
            err_str
        );
        assert!(
            err_str.contains("missing"),
            "Error should indicate missing directive: {}",
            err_str
        );
    }

    #[test]
    fn test_adjacent_blocks_different_tags() {
        let markdown = r#"---
CARD: items
name: Item 1
---

Item 1 body

---
CARD: sections
title: Section 1
---

Section 1 body"#;

        let doc = decompose(markdown).unwrap();

        // All cards in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 2);

        // First card is "items" type
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get("name").unwrap().as_str().unwrap(), "Item 1");

        // Second card is "sections" type
        let section = cards[1].as_object().unwrap();
        assert_eq!(section.get("CARD").unwrap().as_str().unwrap(), "sections");
        assert_eq!(section.get("title").unwrap().as_str().unwrap(), "Section 1");
    }

    #[test]
    fn test_order_preservation() {
        let markdown = r#"---
CARD: items
id: 1
---

First

---
CARD: items
id: 2
---

Second

---
CARD: items
id: 3
---

Third"#;

        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 3);

        for (i, card) in cards.iter().enumerate() {
            let mapping = card.as_object().unwrap();
            assert_eq!(mapping.get("CARD").unwrap().as_str().unwrap(), "items");
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
CARD: products
name: Widget A
price: 19.99
sku: WID-001
---

The **Widget A** is our most popular product.

---
CARD: products
name: Gadget B
price: 29.99
sku: GAD-002
---

The **Gadget B** is perfect for professionals.

---
CARD: reviews
product: Widget A
rating: 5
---

"Excellent product! Highly recommended."

---
CARD: reviews
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

        // All cards in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 4); // 2 products + 2 reviews

        // First 2 are products
        let product1 = cards[0].as_object().unwrap();
        assert_eq!(product1.get("CARD").unwrap().as_str().unwrap(), "products");
        assert_eq!(product1.get("name").unwrap().as_str().unwrap(), "Widget A");
        assert_eq!(product1.get("price").unwrap().as_f64().unwrap(), 19.99);

        let product2 = cards[1].as_object().unwrap();
        assert_eq!(product2.get("CARD").unwrap().as_str().unwrap(), "products");
        assert_eq!(product2.get("name").unwrap().as_str().unwrap(), "Gadget B");

        // Last 2 are reviews
        let review1 = cards[2].as_object().unwrap();
        assert_eq!(review1.get("CARD").unwrap().as_str().unwrap(), "reviews");
        assert_eq!(
            review1.get("product").unwrap().as_str().unwrap(),
            "Widget A"
        );
        assert_eq!(review1.get("rating").unwrap().as_i64().unwrap(), 5);

        // Total fields: title, author, date, body, CARDS = 5
        assert_eq!(doc.fields().len(), 5);
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
    fn test_quill_with_card_blocks() {
        let markdown = r#"---
QUILL: document
title: Test Document
---

Main body.

---
CARD: sections
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

        // Verify card blocks work via CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "sections"
        );

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
        // QUILL in inline block is now an error (must appear in top-level frontmatter only)
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("top-level frontmatter"));
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
    fn test_card_wrong_value_type() {
        let markdown = r#"---
CARD: 123
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("CARD value must be a string"));
    }

    #[test]
    fn test_both_quill_and_card_error() {
        let markdown = r#"---
QUILL: test
CARD: items
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot specify both QUILL and CARD"));
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
        // Blank lines should be allowed in CARD blocks too
        let markdown = r#"---
CARD: items
name: Item 1

price: 19.99

tags:
  - electronics
  - gadgets
---

Body of item 1."#;

        let doc = decompose(markdown).unwrap();

        // Cards are in CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);

        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
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

        // All cards are now in unified CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 5); // 3 features + 2 use_cases

        // Count features and use_cases cards
        let features_count = cards
            .iter()
            .filter(|c| {
                c.as_object()
                    .unwrap()
                    .get("CARD")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    == "features"
            })
            .count();
        let use_cases_count = cards
            .iter()
            .filter(|c| {
                c.as_object()
                    .unwrap()
                    .get("CARD")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    == "use_cases"
            })
            .count();
        assert_eq!(features_count, 3);
        assert_eq!(use_cases_count, 2);

        // Check first card is a feature
        let feature1 = cards[0].as_object().unwrap();
        assert_eq!(feature1.get("CARD").unwrap().as_str().unwrap(), "features");
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
        assert!(err_msg.contains("Input too large"));
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

        let result = decompose(markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_yaml_depth_limit() {
        // Create deeply nested YAML that exceeds MAX_YAML_DEPTH (100 levels)
        // This tests serde-saphyr's Budget.max_depth enforcement
        let mut yaml_content = String::new();
        for i in 0..110 {
            yaml_content.push_str(&"  ".repeat(i));
            yaml_content.push_str(&format!("level{}: value\n", i));
        }

        let markdown = format!("---\n{}---\n\nBody", yaml_content);
        let result = decompose(&markdown);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // serde-saphyr returns "budget exceeded" or similar for depth violations
        assert!(
            err_msg.to_lowercase().contains("budget")
                || err_msg.to_lowercase().contains("depth")
                || err_msg.contains("YAML"),
            "Expected depth/budget error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_yaml_depth_within_limit() {
        // Create reasonably nested YAML (should succeed)
        let markdown = r#"---
level1:
  level2:
    level3:
      level4:
        value: test
---

Body content"#;

        let result = decompose(markdown);
        assert!(result.is_ok());
    }

    // Tests for guillemet preservation in parsing (guillemets are NOT converted during parsing)
    // Guillemet conversion now happens in process_plate, not during parsing
    #[test]
    fn test_chevrons_preserved_in_body_no_frontmatter() {
        let markdown = "Use <<raw content>> here.";
        let doc = decompose(markdown).unwrap();

        // Body should preserve chevrons (conversion happens later in process_plate)
        assert_eq!(doc.body(), Some("Use <<raw content>> here."));
    }

    #[test]
    fn test_chevrons_preserved_in_body_with_frontmatter() {
        let markdown = r#"---
title: Test
---

Use <<raw content>> here."#;
        let doc = decompose(markdown).unwrap();

        // Body should preserve chevrons
        assert_eq!(doc.body(), Some("\nUse <<raw content>> here."));
    }

    #[test]
    fn test_chevrons_preserved_in_yaml_string() {
        let markdown = r#"---
title: Test <<with chevrons>>
---

Body content."#;
        let doc = decompose(markdown).unwrap();

        // YAML string values should preserve chevrons
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test <<with chevrons>>"
        );
    }

    #[test]
    fn test_chevrons_preserved_in_yaml_array() {
        let markdown = r#"---
items:
  - "<<first>>"
  - "<<second>>"
---

Body."#;
        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items[0].as_str().unwrap(), "<<first>>");
        assert_eq!(items[1].as_str().unwrap(), "<<second>>");
    }

    #[test]
    fn test_chevrons_preserved_in_yaml_nested() {
        let markdown = r#"---
metadata:
  description: "<<nested value>>"
---

Body."#;
        let doc = decompose(markdown).unwrap();

        let metadata = doc.get_field("metadata").unwrap().as_object().unwrap();
        assert_eq!(
            metadata.get("description").unwrap().as_str().unwrap(),
            "<<nested value>>"
        );
    }

    #[test]
    fn test_chevrons_preserved_in_code_blocks() {
        let markdown = r#"```
<<in code block>>
```

<<outside code block>>"#;
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // All chevrons should be preserved (no conversion during parsing)
        assert!(body.contains("<<in code block>>"));
        assert!(body.contains("<<outside code block>>"));
    }

    #[test]
    fn test_chevrons_preserved_in_inline_code() {
        let markdown = "`<<in inline code>>` and <<outside inline code>>";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // All chevrons should be preserved
        assert!(body.contains("`<<in inline code>>`"));
        assert!(body.contains("<<outside inline code>>"));
    }

    #[test]
    fn test_chevrons_preserved_in_tagged_block_body() {
        let markdown = r#"---
title: Main
---

Main body.

---
CARD: items
name: Item 1
---

Use <<raw>> here."#;
        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        let item_body = item.get(BODY_FIELD).unwrap().as_str().unwrap();
        // Tagged block body should preserve chevrons
        assert!(item_body.contains("<<raw>>"));
    }

    #[test]
    fn test_chevrons_preserved_in_tagged_block_yaml() {
        let markdown = r#"---
title: Main
---

Main body.

---
CARD: items
description: "<<tagged yaml>>"
---

Item body."#;
        let doc = decompose(markdown).unwrap();

        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        // Tagged block YAML should preserve chevrons
        assert_eq!(
            item.get("description").unwrap().as_str().unwrap(),
            "<<tagged yaml>>"
        );
    }

    #[test]
    fn test_yaml_numbers_not_affected() {
        // Numbers should not be affected
        let markdown = r#"---
count: 42
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert_eq!(doc.get_field("count").unwrap().as_i64().unwrap(), 42);
    }

    #[test]
    fn test_yaml_booleans_not_affected() {
        // Booleans should not be affected
        let markdown = r#"---
active: true
---

Body."#;
        let doc = decompose(markdown).unwrap();
        assert!(doc.get_field("active").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_multiline_chevrons_preserved() {
        // Multiline chevrons should be preserved as-is
        let markdown = "<<text\nacross lines>>";
        let doc = decompose(markdown).unwrap();

        let body = doc.body().unwrap();
        // Should contain the original chevrons
        assert!(body.contains("<<text"));
        assert!(body.contains("across lines>>"));
    }

    #[test]
    fn test_unmatched_chevrons_preserved() {
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
        // Should have body and CARDS fields
        assert_eq!(doc.fields().len(), 2);
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

    // CARD block edge cases

    #[test]
    fn test_card_with_empty_body() {
        let markdown = r#"---
CARD: items
name: Item
---"#;
        let doc = decompose(markdown).unwrap();
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 1);
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        assert_eq!(item.get(BODY_FIELD).unwrap().as_str().unwrap(), "");
    }

    #[test]
    fn test_card_consecutive_blocks() {
        let markdown = r#"---
CARD: a
id: 1
---
---
CARD: a
id: 2
---"#;
        let doc = decompose(markdown).unwrap();
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(
            cards[0]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "a"
        );
        assert_eq!(
            cards[1]
                .as_object()
                .unwrap()
                .get("CARD")
                .unwrap()
                .as_str()
                .unwrap(),
            "a"
        );
    }

    #[test]
    fn test_card_with_body_containing_dashes() {
        let markdown = r#"---
CARD: items
name: Item
---

Some text with --- dashes in it."#;
        let doc = decompose(markdown).unwrap();
        let cards = doc.get_field("CARDS").unwrap().as_sequence().unwrap();
        let item = cards[0].as_object().unwrap();
        assert_eq!(item.get("CARD").unwrap().as_str().unwrap(), "items");
        let body = item.get(BODY_FIELD).unwrap().as_str().unwrap();
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
        let markdown = "---\nCARD: ITEMS\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid card field name"));
    }

    #[test]
    fn test_invalid_scope_name_starts_with_number() {
        let markdown = "---\nCARD: 123items\n---\n\nBody.";
        let result = decompose(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_scope_name_with_hyphen() {
        let markdown = "---\nCARD: my-items\n---\n\nBody.";
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
        assert!(doc.get_field("active").unwrap().as_bool().unwrap());
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

    #[test]
    fn test_allowed_card_field_collision() {
        let markdown = r#"---
my_card: "some global value"
---

---
CARD: my_card
title: "My Card"
---
Body
"#;
        // This should SUCCEED according to new PARSE.md
        let doc = decompose(markdown).unwrap();

        // Verify global field exists
        assert_eq!(
            doc.get_field("my_card").unwrap().as_str().unwrap(),
            "some global value"
        );

        // Verify Card exists in CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert!(!cards.is_empty());
        let card = cards
            .iter()
            .find(|v| v.get("CARD").and_then(|c| c.as_str()) == Some("my_card"))
            .expect("Card not found");
        assert_eq!(card.get("title").unwrap().as_str().unwrap(), "My Card");
    }

    #[test]
    fn test_yaml_custom_tags_in_frontmatter() {
        // User-defined YAML tags like !fill should be accepted and ignored
        let markdown = r#"---
memo_from: !fill 2d lt example
regular_field: normal value
---

Body content."#;
        let doc = decompose(markdown).unwrap();

        // The tag !fill should be ignored, value parsed as string "2d lt example"
        assert_eq!(
            doc.get_field("memo_from").unwrap().as_str().unwrap(),
            "2d lt example"
        );
        // Regular fields should still work
        assert_eq!(
            doc.get_field("regular_field").unwrap().as_str().unwrap(),
            "normal value"
        );
        assert_eq!(doc.body(), Some("\nBody content."));
    }

    /// Test the exact example from EXTENDED_MARKDOWN.md (lines 92-127)
    #[test]
    fn test_spec_example() {
        let markdown = r#"---
title: My Document
QUILL: blog_post
---
Main document body.

***

More content after horizontal rule.

---
CARD: section
heading: Introduction
---
Introduction content.

---
CARD: section
heading: Conclusion
---
Conclusion content.
"#;

        let doc = decompose(markdown).unwrap();

        // Verify global fields
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "My Document"
        );
        assert_eq!(doc.quill_tag(), "blog_post");

        // Verify body contains horizontal rule (*** preserved)
        let body = doc.body().unwrap();
        assert!(body.contains("Main document body."));
        assert!(body.contains("***"));
        assert!(body.contains("More content after horizontal rule."));

        // Verify CARDS array
        let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
        assert_eq!(cards.len(), 2);

        // First card
        let card1 = cards[0].as_object().unwrap();
        assert_eq!(card1.get("CARD").unwrap().as_str().unwrap(), "section");
        assert_eq!(
            card1.get("heading").unwrap().as_str().unwrap(),
            "Introduction"
        );
        assert_eq!(
            card1.get("BODY").unwrap().as_str().unwrap(),
            "Introduction content.\n\n"
        );

        // Second card
        let card2 = cards[1].as_object().unwrap();
        assert_eq!(card2.get("CARD").unwrap().as_str().unwrap(), "section");
        assert_eq!(
            card2.get("heading").unwrap().as_str().unwrap(),
            "Conclusion"
        );
        assert_eq!(
            card2.get("BODY").unwrap().as_str().unwrap(),
            "Conclusion content.\n"
        );
    }
}
