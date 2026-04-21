//! Schema-aware form projection for form editors.
//!
//! This module provides [`FormProjection`] — a read-only snapshot of a
//! [`Document`] through its [`Quill`] schema. For each schema-declared field
//! the projection records the current value, the schema default, and the
//! source of the effective value.
//!
//! # Usage
//!
//! ```rust,no_run
//! # use quillmark_core::{Quill, Document, quill::form::{FormProjection, FormFieldSource}};
//! # fn example(quill: &Quill, doc: &Document) {
//! let projection = quill.project_form(doc);
//!
//! for (name, fv) in &projection.main.values {
//!     match fv.source {
//!         FormFieldSource::Document => println!("{name}: {:?}", fv.value),
//!         FormFieldSource::Default  => println!("{name}: (default) {:?}", fv.default),
//!         FormFieldSource::Missing  => println!("{name}: MISSING"),
//!     }
//! }
//! # }
//! ```
//!
//! # Re-projection after editing
//!
//! A `FormProjection` is a **read-only snapshot** of the document at the time
//! [`Quill::project_form`] is called. Subsequent edits to `doc` (e.g. via
//! [`Document::set_field`]) are not reflected in an existing `FormProjection`;
//! call `project_form` again to obtain an updated snapshot.
//!
//! # Unknown card tags
//!
//! Cards whose tag is not declared in the schema are **dropped** from
//! `FormProjection.cards`. Each such card produces one [`SerializableDiagnostic`]
//! in `FormProjection.diagnostics` with code `"form::unknown_card_tag"`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::error::{Diagnostic, SerializableDiagnostic, Severity};
use crate::value::QuillValue;

use super::{CardSchema, Quill};

/// Source of a field's effective value in a form projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FormFieldSource {
    /// Value was present in the document's frontmatter or card fields.
    Document,
    /// Value was absent from the document; the schema provides a default.
    Default,
    /// Value was absent from the document and the schema has no default.
    Missing,
}

/// A single field's projection within a [`FormCard`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormFieldValue {
    /// Current value from the document, if present.
    pub value: Option<QuillValue>,
    /// Schema default value, if declared.
    pub default: Option<QuillValue>,
    /// Where the effective value comes from.
    pub source: FormFieldSource,
}

/// A card projected through its schema — either the main document card or a
/// named card block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormCard {
    /// The schema that governs this card.
    pub schema: CardSchema,
    /// Projection of each schema-declared field.
    ///
    /// Keys follow `IndexMap` insertion order (schema field definition order).
    pub values: IndexMap<String, FormFieldValue>,
}

/// Read-only snapshot of a [`Document`] projected through a [`Quill`]'s schema.
///
/// Produced by [`Quill::project_form`]. Subsequent edits to the document
/// are **not** reflected here — call `project_form` again after editing.
///
/// # Unknown cards
///
/// Document cards whose tag is not declared in the schema are dropped and
/// each produces a [`SerializableDiagnostic`] with code `"form::unknown_card_tag"` in
/// `diagnostics`.
///
/// [`SerializableDiagnostic`]: crate::error::SerializableDiagnostic
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormProjection {
    /// Projection of the main document (frontmatter fields).
    pub main: FormCard,
    /// Projections of each recognised card, in document order.
    ///
    /// Cards with unknown tags are excluded; see `diagnostics`.
    pub cards: Vec<FormCard>,
    /// Diagnostics from unknown card tags and validation.
    ///
    /// Uses [`SerializableDiagnostic`] (fully serializable) rather than
    /// [`Diagnostic`] (non-deserializable due to boxed source chain) so that
    /// `FormProjection` can be fully round-tripped via `serde_json`,
    /// `serde_wasm_bindgen`, and `pyo3`.
    ///
    /// [`SerializableDiagnostic`]: crate::error::SerializableDiagnostic
    /// [`Diagnostic`]: crate::error::Diagnostic
    pub diagnostics: Vec<SerializableDiagnostic>,
}

// ── impl Quill ────────────────────────────────────────────────────────────────

impl Quill {
    /// Project a document through this quill's schema.
    ///
    /// Returns a [`FormProjection`] — a read-only snapshot of the document's
    /// fields mapped against the schema. For each schema-declared field the
    /// projection records:
    ///
    /// - [`FormFieldSource::Document`] — value present in the document.
    /// - [`FormFieldSource::Default`] — value absent; schema default used.
    /// - [`FormFieldSource::Missing`] — value absent; no schema default.
    ///
    /// **Snapshot semantics.** Subsequent edits to `doc` are not reflected;
    /// call `project_form` again after editing.
    ///
    /// **Unknown cards.** Each card in `doc.cards()` whose tag is not declared
    /// in the quill schema is dropped from `FormProjection.cards`. A
    /// [`SerializableDiagnostic`] with code `"form::unknown_card_tag"` is
    /// appended to `FormProjection.diagnostics` for each such card.
    ///
    /// **Validation.** `QuillConfig::validate_document` is run over the
    /// document and any resulting errors are converted to diagnostics and
    /// appended to `FormProjection.diagnostics`. This is purely additive —
    /// the projection itself is never modified by validation failures.
    ///
    /// # Composing existing functions
    ///
    /// This method composes:
    /// - `QuillConfig::main` — to obtain the main card schema.
    /// - `QuillConfig::card_definition` — to look up card schemas by tag.
    /// - `QuillConfig::validate_document` — to gather validation diagnostics.
    ///
    /// Coercion (`coerce_frontmatter` / `coerce_card`) is **not** applied here
    /// because `project_form` is a projection of the document as-is; coercion
    /// is a lossy transformation and would change the field values visible to
    /// the form editor. Validation diagnostics already inform the consumer when
    /// values are type-mismatched.
    ///
    /// [`SerializableDiagnostic`]: crate::error::SerializableDiagnostic
    pub fn project_form(&self, doc: &Document) -> FormProjection {
        let mut diagnostics: Vec<SerializableDiagnostic> = Vec::new();

        // ── Main card projection ──────────────────────────────────────────────
        let main_schema = self.config.main();
        let main = project_card(main_schema, doc.frontmatter());

        // ── Per-card projections ──────────────────────────────────────────────
        let mut cards: Vec<FormCard> = Vec::new();

        for (index, card) in doc.cards().iter().enumerate() {
            let tag = card.tag();
            match self.config.card_definition(tag) {
                Some(card_schema) => {
                    cards.push(project_card(card_schema, card.fields()));
                }
                None => {
                    let diag = Diagnostic::new(
                        Severity::Warning,
                        format!(
                            "card at index {index} has unknown tag \"{tag}\"; \
                             it is not declared in the quill schema and has been \
                             excluded from the form projection"
                        ),
                    )
                    .with_code("form::unknown_card_tag".to_string());
                    diagnostics.push(SerializableDiagnostic::from(diag));
                }
            }
        }

        // ── Validation diagnostics ────────────────────────────────────────────
        if let Err(validation_errors) = self.config.validate_document(doc) {
            for err in validation_errors {
                let diag = Diagnostic::new(Severity::Error, err.to_string())
                    .with_code("form::validation_error".to_string());
                diagnostics.push(SerializableDiagnostic::from(diag));
            }
        }

        FormProjection {
            main,
            cards,
            diagnostics,
        }
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Build a [`FormCard`] by walking each schema-declared field and looking up
/// its value in `fields`.
fn project_card(
    schema: &CardSchema,
    fields: &IndexMap<String, QuillValue>,
) -> FormCard {
    let mut values: IndexMap<String, FormFieldValue> = IndexMap::new();

    // Collect field names sorted by their ui.order (if available), falling
    // back to alphabetical order.  This gives a stable, schema-defined order.
    let mut field_names: Vec<&str> = schema.fields.keys().map(String::as_str).collect();
    field_names.sort_by_key(|name| {
        schema
            .fields
            .get(*name)
            .and_then(|fs| fs.ui.as_ref())
            .and_then(|ui| ui.order)
            .unwrap_or(i32::MAX)
    });

    for field_name in field_names {
        let field_schema = &schema.fields[field_name];
        let default = field_schema.default.clone();

        let ffv = match fields.get(field_name) {
            Some(v) => FormFieldValue {
                value: Some(v.clone()),
                default,
                source: FormFieldSource::Document,
            },
            None => match default {
                Some(ref d) => FormFieldValue {
                    value: None,
                    default: Some(d.clone()),
                    source: FormFieldSource::Default,
                },
                None => FormFieldValue {
                    value: None,
                    default: None,
                    source: FormFieldSource::Missing,
                },
            },
        };

        values.insert(field_name.to_string(), ffv);
    }

    FormCard {
        schema: schema.clone(),
        values,
    }
}
