//! # Document Editor Surface
//!
//! Typed mutators for [`Document`] and [`Card`] with invariant enforcement.
//!
//! ## Invariants
//!
//! Every successful mutator call leaves the document in a state that:
//! - Contains no reserved key in `frontmatter` (`BODY`, `CARDS`, `QUILL`, `CARD`).
//! - Has every `card.tag` passing `sentinel::is_valid_tag_name`.
//! - Can be safely serialized via [`Document::to_plate_json`].
//!
//! **Mutators never modify `warnings`.**  Warnings are parse-time observations
//! and remain stable for the lifetime of the document.

use unicode_normalization::UnicodeNormalization;

use crate::document::sentinel::is_valid_tag_name;
use crate::document::{Card, Document};
use crate::value::QuillValue;
use crate::version::QuillReference;

// ── Reserved names ──────────────────────────────────────────────────────────

/// Reserved field names that may not appear in `Document.frontmatter` or
/// `Card.fields`.  These are the sentinel keys whose presence in user-visible
/// fields would corrupt the plate wire format or the parser's structural
/// invariants.
pub const RESERVED_NAMES: &[&str] = &["BODY", "CARDS", "QUILL", "CARD"];

/// Returns `true` if `name` is one of the four reserved sentinel names.
#[inline]
pub fn is_reserved_name(name: &str) -> bool {
    RESERVED_NAMES.contains(&name)
}

// ── Field name validation ───────────────────────────────────────────────────

/// Returns `true` if `name` is a valid frontmatter / card field name.
///
/// A valid field name matches `[a-z_][a-z0-9_]*` after NFC normalisation.
/// Upper-case identifiers are intentionally excluded; they are reserved for
/// sentinel keys (`QUILL`, `CARD`, `BODY`, `CARDS`).
pub fn is_valid_field_name(name: &str) -> bool {
    // NFC-normalize first so that, e.g., composed vs decomposed forms compare equal.
    let normalized: String = name.nfc().collect();
    if normalized.is_empty() {
        return false;
    }
    let mut chars = normalized.chars();
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

// ── EditError ────────────────────────────────────────────────────────────────

/// Errors returned by document and card mutators.
///
/// `EditError` is distinct from [`crate::error::ParseError`]: it carries no
/// source-location information because edits happen after parsing.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EditError {
    /// The supplied name is one of the four reserved sentinel keys
    /// (`BODY`, `CARDS`, `QUILL`, `CARD`).
    #[error("reserved name '{0}' cannot be used as a field name")]
    ReservedName(String),

    /// The supplied name does not match `[a-z_][a-z0-9_]*`.
    #[error("invalid field name '{0}': must match [a-z_][a-z0-9_]*")]
    InvalidFieldName(String),

    /// The supplied tag does not match `[a-z_][a-z0-9_]*`.
    #[error("invalid tag name '{0}': must match [a-z_][a-z0-9_]*")]
    InvalidTagName(String),

    /// A card index was out of the valid range.
    #[error("index {index} is out of range (len = {len})")]
    IndexOutOfRange { index: usize, len: usize },
}

// ── impl Document ────────────────────────────────────────────────────────────

impl Document {
    // ── Frontmatter mutators ─────────────────────────────────────────────────

    /// Set a frontmatter field by name.
    ///
    /// # Invariants enforced
    ///
    /// - `name` must not be one of the reserved sentinel names
    ///   (`BODY`, `CARDS`, `QUILL`, `CARD`).  Returns [`EditError::ReservedName`].
    /// - `name` must match `[a-z_][a-z0-9_]*` after NFC normalisation.
    ///   Returns [`EditError::InvalidFieldName`].
    ///
    /// # Validity
    ///
    /// After a successful call the document remains valid: `frontmatter`
    /// contains no reserved key and the value is stored at the correct key.
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn set_field(&mut self, name: &str, value: QuillValue) -> Result<(), EditError> {
        if is_reserved_name(name) {
            return Err(EditError::ReservedName(name.to_string()));
        }
        if !is_valid_field_name(name) {
            return Err(EditError::InvalidFieldName(name.to_string()));
        }
        self.frontmatter.insert(name.to_string(), value);
        Ok(())
    }

    /// Remove a frontmatter field by name, returning the value if it existed.
    ///
    /// # Invariants enforced
    ///
    /// None — reserved names cannot be present in `frontmatter` (the parser
    /// guarantees this), so passing a reserved name simply returns `None`.
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn remove_field(&mut self, name: &str) -> Option<QuillValue> {
        self.frontmatter.shift_remove(name)
    }

    /// Replace the QUILL reference.
    ///
    /// # Invariants enforced
    ///
    /// The `QuillReference` type guarantees structural validity; no further
    /// checks are needed here.
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn set_quill_ref(&mut self, reference: QuillReference) {
        self.quill_ref = reference;
    }

    /// Replace the global Markdown body.
    ///
    /// The new body replaces whatever was stored; no structural validation is
    /// applied (body content is opaque Markdown text).
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn replace_body(&mut self, body: impl Into<String>) {
        self.body = body.into();
    }

    // ── Card mutators ────────────────────────────────────────────────────────

    /// Return a mutable reference to the card at `index`, or `None` if out of range.
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn card_mut(&mut self, index: usize) -> Option<&mut Card> {
        self.cards.get_mut(index)
    }

    /// Append a card to the end of the card list.
    ///
    /// Currently trivial — reserved for future cross-card invariant checks
    /// (e.g. duplicate-tag detection).  The `Result` return type is API
    /// future-proofing.
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn push_card(&mut self, card: Card) -> Result<(), EditError> {
        self.cards.push(card);
        Ok(())
    }

    /// Insert a card at `index`.
    ///
    /// # Invariants enforced
    ///
    /// `index` must be in `0..=len`.  An `index > len` returns
    /// [`EditError::IndexOutOfRange`].
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn insert_card(&mut self, index: usize, card: Card) -> Result<(), EditError> {
        let len = self.cards.len();
        if index > len {
            return Err(EditError::IndexOutOfRange { index, len });
        }
        self.cards.insert(index, card);
        Ok(())
    }

    /// Remove and return the card at `index`, or `None` if out of range.
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn remove_card(&mut self, index: usize) -> Option<Card> {
        if index >= self.cards.len() {
            return None;
        }
        Some(self.cards.remove(index))
    }

    /// Move the card at `from` to position `to`.
    ///
    /// If `from == to`, this is a no-op and returns `Ok(())`.
    ///
    /// # Invariants enforced
    ///
    /// Both `from` and `to` must be in `0..len`.  Either being out of range
    /// returns [`EditError::IndexOutOfRange`] with the offending index.
    ///
    /// # Warnings
    ///
    /// This method never modifies `warnings`.
    pub fn move_card(&mut self, from: usize, to: usize) -> Result<(), EditError> {
        let len = self.cards.len();
        if from >= len {
            return Err(EditError::IndexOutOfRange { index: from, len });
        }
        if to >= len {
            return Err(EditError::IndexOutOfRange { index: to, len });
        }
        if from == to {
            return Ok(());
        }
        let card = self.cards.remove(from);
        self.cards.insert(to, card);
        Ok(())
    }
}

// ── impl Card ────────────────────────────────────────────────────────────────

impl Card {
    /// Create a new, empty card with the given tag.
    ///
    /// # Invariants enforced
    ///
    /// `tag` must match `[a-z_][a-z0-9_]*`.  An invalid tag returns
    /// [`EditError::InvalidTagName`].
    ///
    /// The new card has no fields and an empty body.
    pub fn new(tag: impl Into<String>) -> Result<Self, EditError> {
        let tag = tag.into();
        if !is_valid_tag_name(&tag) {
            return Err(EditError::InvalidTagName(tag));
        }
        Ok(Card::new_internal(tag, indexmap::IndexMap::new(), String::new()))
    }

    /// Set a card field by name.
    ///
    /// # Invariants enforced
    ///
    /// - `name` must not be one of the reserved sentinel names.
    ///   Returns [`EditError::ReservedName`].
    /// - `name` must match `[a-z_][a-z0-9_]*` after NFC normalisation.
    ///   Returns [`EditError::InvalidFieldName`].
    ///
    /// # Validity
    ///
    /// After a successful call the card remains valid: `fields` contains no
    /// reserved key and the value is stored at the correct key.
    ///
    /// # Warnings
    ///
    /// Card mutators never modify the parent document's `warnings`.
    pub fn set_field(&mut self, name: &str, value: QuillValue) -> Result<(), EditError> {
        if is_reserved_name(name) {
            return Err(EditError::ReservedName(name.to_string()));
        }
        if !is_valid_field_name(name) {
            return Err(EditError::InvalidFieldName(name.to_string()));
        }
        self.fields.insert(name.to_string(), value);
        Ok(())
    }

    /// Remove a card field by name, returning the value if it existed.
    ///
    /// Reserved names cannot be present in `fields` (the parser guarantees
    /// this), so passing a reserved name simply returns `None`.
    ///
    /// # Warnings
    ///
    /// Card mutators never modify the parent document's `warnings`.
    pub fn remove_field(&mut self, name: &str) -> Option<QuillValue> {
        self.fields.shift_remove(name)
    }

    /// Replace the card's Markdown body.
    ///
    /// # Warnings
    ///
    /// Card mutators never modify the parent document's `warnings`.
    pub fn set_body(&mut self, body: impl Into<String>) {
        self.body = body.into();
    }
}
