//! # Document Module
//!
//! Parsing functionality for markdown documents with YAML frontmatter.
//!
//! ## Overview
//!
//! The `document` module provides the [`Document::from_markdown`] function for parsing
//! markdown documents into a typed in-memory model.
//!
//! ## Key Types
//!
//! - [`Document`]: Typed in-memory Quillmark document — frontmatter, body, and cards.
//! - [`Card`]: A single `CARD:` block with a tag, typed fields, and a body.
//!
//! ## Examples
//!
//! ### Basic Parsing
//!
//! ```
//! use quillmark_core::Document;
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
//! let doc = Document::from_markdown(markdown).unwrap();
//! let title = doc.frontmatter()
//!     .get("title")
//!     .and_then(|v| v.as_str())
//!     .unwrap_or("Untitled");
//! assert_eq!(title, "My Document");
//! assert_eq!(doc.cards().len(), 0);
//! ```
//!
//! ### Accessing the plate wire format
//!
//! ```
//! use quillmark_core::Document;
//!
//! let doc = Document::from_markdown(
//!     "---\nQUILL: my_quill\ntitle: Hi\n---\n\nBody here.\n"
//! ).unwrap();
//! let json = doc.to_plate_json();
//! assert_eq!(json["QUILL"], "my_quill");
//! assert_eq!(json["title"], "Hi");
//! assert_eq!(json["BODY"], "\nBody here.\n");
//! assert!(json["CARDS"].is_array());
//! ```
//!
//! ## Error Handling
//!
//! [`Document::from_markdown`] returns errors for:
//! - Malformed YAML syntax
//! - Unclosed frontmatter blocks
//! - Multiple global frontmatter blocks
//! - Both QUILL and CARD specified in the same block
//! - Reserved field name usage
//! - Name collisions
//!
//! See [PARSE.md](https://github.com/nibsbin/quillmark/blob/main/designs/PARSE.md) for
//! comprehensive documentation of the Extended YAML Metadata Standard.

use indexmap::IndexMap;

use crate::error::ParseError;
use crate::value::QuillValue;
use crate::version::QuillReference;
use crate::Diagnostic;

pub mod assemble;
pub mod edit;
pub mod emit;
pub mod fences;
pub mod limits;
pub mod sentinel;

pub use edit::EditError;

#[cfg(test)]
mod tests;

/// Parse result carrying both the parsed document and any non-fatal warnings
/// (e.g. near-miss sentinel lints emitted per spec §4.2).
#[derive(Debug)]
pub struct ParseOutput {
    /// The successfully parsed document.
    pub document: Document,
    /// Non-fatal warnings collected during parsing.
    pub warnings: Vec<Diagnostic>,
}

/// A single `CARD:` block parsed from a Quillmark Markdown document.
///
/// Every card has a `tag` (the value of its `CARD:` sentinel), typed `fields`
/// (all YAML key-value pairs from the fence body, excluding the `CARD` key
/// itself), and a `body` (the Markdown text that follows the closing `---`).
///
/// ## Card body absence
///
/// If a card block has no trailing Markdown content (e.g. the next block or
/// EOF immediately follows the closing fence), `body` is the empty string `""`.
/// It is never `None`; callers that need to distinguish "absent" from "empty"
/// should check `card.body().is_empty()`.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    tag: String,
    fields: IndexMap<String, QuillValue>,
    body: String,
}

impl Card {
    /// Create a `Card` directly from typed parts.
    ///
    /// Used by `assemble.rs`, `normalize.rs`, and the `Workflow`.
    /// Does **not** validate the tag name or field names — callers are
    /// responsible for providing already-valid data.  For user-facing
    /// construction use [`Card::new`] (defined in `edit.rs`).
    pub fn new_internal(tag: String, fields: IndexMap<String, QuillValue>, body: String) -> Self {
        Self { tag, fields, body }
    }

    /// The card tag (value of the `CARD:` sentinel).
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Typed fields from this card's YAML fence (excluding the `CARD` key).
    pub fn fields(&self) -> &IndexMap<String, QuillValue> {
        &self.fields
    }

    /// Markdown body that follows this card's closing fence.
    ///
    /// Empty string when no trailing content is present.
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// A fully-parsed, typed in-memory Quillmark document.
///
/// `Document` is the canonical representation of a Quillmark Markdown file.
/// Markdown is one import format (and will be one export format in Phase 4);
/// the structured data here is primary.
///
/// ## Fields vs. plate wire format
///
/// `Document` stores:
/// - `frontmatter` — user-visible YAML fields (no `CARDS`, no `BODY` sentinel keys)
/// - `body` — global Markdown body between the frontmatter fence and the first card
/// - `cards` — ordered list of `Card` values
///
/// When a backend plate needs the legacy flat JSON shape, call
/// [`Document::to_plate_json`]. That method is the **only** place in core that
/// reconstructs `{"QUILL": ..., "CARDS": [...], "BODY": "..."}`.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    quill_ref: QuillReference,
    frontmatter: IndexMap<String, QuillValue>,
    body: String,
    cards: Vec<Card>,
    warnings: Vec<Diagnostic>,
}

impl Document {
    /// Create a `Document` directly from typed parts.
    ///
    /// This is used by `assemble.rs`, `normalize.rs`, and the `Workflow`.
    pub fn new_internal(
        quill_ref: QuillReference,
        frontmatter: IndexMap<String, QuillValue>,
        body: String,
        cards: Vec<Card>,
        warnings: Vec<Diagnostic>,
    ) -> Self {
        Self {
            quill_ref,
            frontmatter,
            body,
            cards,
            warnings,
        }
    }

    /// Parse a Quillmark Markdown document, discarding any non-fatal warnings.
    pub fn from_markdown(markdown: &str) -> Result<Self, ParseError> {
        assemble::decompose(markdown)
    }

    /// Parse a Quillmark Markdown document, returning warnings alongside the document.
    pub fn from_markdown_with_warnings(markdown: &str) -> Result<ParseOutput, ParseError> {
        assemble::decompose_with_warnings(markdown)
            .map(|(document, warnings)| ParseOutput { document, warnings })
    }

    // ── Accessors ──────────────────────────────────────────────────────────────

    /// The quill reference (`name@version-selector`).
    pub fn quill_reference(&self) -> &QuillReference {
        &self.quill_ref
    }

    /// User-visible YAML frontmatter fields.
    ///
    /// Does **not** include the `QUILL`, `CARDS`, or `BODY` sentinel keys;
    /// those are available via [`Document::quill_reference`], [`Document::cards`], and [`Document::body`].
    pub fn frontmatter(&self) -> &IndexMap<String, QuillValue> {
        &self.frontmatter
    }

    /// Global Markdown body between the frontmatter fence and the first card.
    ///
    /// Empty string when no body is present.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Ordered list of card blocks.
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// Non-fatal warnings collected during parsing.
    pub fn warnings(&self) -> &[Diagnostic] {
        &self.warnings
    }

    // ── Wire format ────────────────────────────────────────────────────────────

    /// Serialize this document to the JSON shape expected by backend plates.
    ///
    /// The output has the following top-level keys, which match what
    /// `lib.typ.template` reads at Typst runtime:
    ///
    /// ```json
    /// {
    ///   "QUILL": "<ref>",
    ///   "<field>": <value>,
    ///   ...
    ///   "BODY": "<global-body>",
    ///   "CARDS": [
    ///     { "CARD": "<tag>", "<field>": <value>, ..., "BODY": "<card-body>" },
    ///     ...
    ///   ]
    /// }
    /// ```
    ///
    /// This is the **only** place in `quillmark-core` that knows about the plate
    /// wire format. All internal consumers (workflow, backends) call this instead
    /// of constructing the shape by hand.
    pub fn to_plate_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        // QUILL first — plate authors expect this at the top.
        map.insert(
            "QUILL".to_string(),
            serde_json::Value::String(self.quill_ref.to_string()),
        );

        // Frontmatter fields in insertion order.
        for (key, value) in &self.frontmatter {
            map.insert(key.clone(), value.as_json().clone());
        }

        // Global body.
        map.insert(
            "BODY".to_string(),
            serde_json::Value::String(self.body.clone()),
        );

        // Cards array.
        let cards_array: Vec<serde_json::Value> = self
            .cards
            .iter()
            .map(|card| {
                let mut card_map = serde_json::Map::new();
                card_map.insert(
                    "CARD".to_string(),
                    serde_json::Value::String(card.tag.clone()),
                );
                for (key, value) in &card.fields {
                    card_map.insert(key.clone(), value.as_json().clone());
                }
                card_map.insert(
                    "BODY".to_string(),
                    serde_json::Value::String(card.body.clone()),
                );
                serde_json::Value::Object(card_map)
            })
            .collect();

        map.insert("CARDS".to_string(), serde_json::Value::Array(cards_array));

        serde_json::Value::Object(map)
    }

}
