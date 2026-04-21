//! # Document Module
//!
//! Parsing functionality for markdown documents with YAML frontmatter.
//!
//! ## Overview
//!
//! The `document` module provides the [`ParsedDocument::from_markdown`] function for parsing markdown documents
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
//! QUILL: my_quill
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

use crate::error::ParseError;
use crate::value::QuillValue;
use crate::version::QuillReference;
use crate::Diagnostic;

pub mod assemble;
pub mod fences;
pub mod limits;
pub mod sentinel;

#[cfg(test)]
mod tests;

/// The field name used to store the document body
pub const BODY_FIELD: &str = "BODY";

/// Parse result carrying both the parsed document and any non-fatal warnings
/// (e.g. near-miss sentinel lints emitted per spec §4.2).
#[derive(Debug)]
pub struct ParseOutput {
    /// The successfully parsed document.
    pub document: ParsedDocument,
    /// Non-fatal warnings collected during parsing.
    pub warnings: Vec<Diagnostic>,
}

/// A parsed markdown document with frontmatter
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    fields: HashMap<String, QuillValue>,
    quill_ref: QuillReference,
}

impl ParsedDocument {
    /// Create a ParsedDocument from fields and quill reference
    pub fn new(fields: HashMap<String, QuillValue>, quill_ref: QuillReference) -> Self {
        Self { fields, quill_ref }
    }

    /// Create a ParsedDocument from markdown string, discarding any warnings.
    pub fn from_markdown(markdown: &str) -> Result<Self, ParseError> {
        assemble::decompose(markdown)
    }

    /// Create a ParsedDocument from markdown string, returning warnings alongside the document.
    pub fn from_markdown_with_warnings(markdown: &str) -> Result<ParseOutput, ParseError> {
        assemble::decompose_with_warnings(markdown)
            .map(|(document, warnings)| ParseOutput { document, warnings })
    }

    /// Get the quill reference (name + version selector)
    pub fn quill_reference(&self) -> &QuillReference {
        &self.quill_ref
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
            quill_ref: self.quill_ref.clone(),
        }
    }
}
