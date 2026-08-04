//! Schema-bound typed reader: the read twin of
//! [`TypedWriter`](crate::TypedWriter).
//!
//! The read surface's verbs split by read-vs-write, but the deeper fault line is
//! **interpret-vs-transport**. The verbatim [`payload().get`](crate::Card::payload)
//! (the WASM binding's `Document.getStored`) is *transport*: it returns the stored
//! value verbatim, schema-free and round-trippable, the disambiguation / debug
//! read. Projecting a field to
//! markdown is *interpretation*: a schema-shaped question ("this field's
//! richtext, as markdown") that a schema-free `Document` cannot answer without
//! guessing which fields are even richtext. So the projection has one door, and
//! it binds the schema.
//!
//! [`Quill::reader`](crate::Quill::reader) binds the schema (where the authority
//! already lives, the writer's twin) so a single verb interprets by the field's
//! declared type:
//!
//! ```ignore
//! let v = quill.reader(&doc);
//! v.get("subject")?;            // richtext → Some(Markdown(..))
//! v.get("qty")?;                // integer  → Some(Value(3))
//! v.get("absent")?;             // absent   → None
//! v.get("nope");                // unknown name → Err(UnknownField)
//! v.card(2)?.get("body")?;      // card field, kind resolves its schema
//! ```
//!
//! **absence returns; mismatch raises; an unknown name is a typo.** A `richtext`
//! field projects to markdown ([`ReadValue::Markdown`]) and a `plaintext` field
//! to its literal text ([`ReadValue::Plaintext`]); every other declared type
//! returns its canonical value verbatim ([`ReadValue::Value`]): the same
//! transport `Document` reads, now reached with schema authority. A present value
//! that does not decode under a content field raises
//! [`EditError::FieldDecode`]. A name the schema does not declare raises
//! [`EditError::UnknownField`], exactly as [`TypedWriter::set`](crate::TypedWriter::set)
//! rejects it on the write side.
//!
//! [`get_content`](TypedReader::get_content) is the same read at the other end of
//! the codec: the `Content` rather than the projection. A document that came
//! through the bound door ([`Quill::parse`](crate::Quill::parse) /
//! [`Quill::conform`](crate::Quill::conform)) rests at one form per codec, but
//! one the transport door left rests as authored, so the verbatim payload read
//! still answers "content object or string?" with "depends where this document
//! came from" and this one does not. Decoding needs the schema, not the payload:
//! a `richtext` string is markdown and a `plaintext` string is literal text, so
//! the same bytes decode two ways and only the declared type says which. That is
//! why the `Content` read binds the quill and
//! `Document` carries none.
//!
//! The body read stays quill-free: a body's type is a format fact, not a schema
//! fact, so [`body_markdown`](TypedReader::body_markdown) mirrors
//! [`Card::body_markdown`](crate::Card::body_markdown) rather than consulting the
//! schema.
//!
//! Like [`TypedWriter`](crate::TypedWriter), a bound reader holds `&Document`
//! and `&QuillConfig`, so
//! it cannot cross a binding boundary that carries no lifetimes (wasm-bindgen /
//! pyo3); those surfaces construct one per call from the quill handle.

use indexmap::IndexMap;
use quillmark_content::Content;

use crate::document::edit::{CODEC_PLAINTEXT, CODEC_RICHTEXT};
use crate::document::{Card, Document, EditError, RichtextDecodeError};
use crate::quill::{CardSchema, FieldSchema, FieldType, QuillConfig};
use crate::value::QuillValue;

/// The interpreted value at a field address: the output of [`TypedReader::get`].
/// A content field decodes to its codec's projection (`richtext` to markdown,
/// `plaintext` to literal text); every other declared type carries its canonical
/// value verbatim (the transport read, reached through the schema). Absence is
/// the `None` of the enclosing `Option`, not a variant here.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ReadValue {
    /// A `richtext` field projected to markdown (`export ∘ decode`): the lossy,
    /// on-demand view (content-only marks do not survive markdown).
    Markdown(String),
    /// A `plaintext` field projected through its literal codec (`to_plaintext ∘
    /// decode`): verbatim text, marks never interpreted (`*hi*` is four
    /// characters, not emphasis).
    Plaintext(String),
    /// A non-content field's canonical value, verbatim: the schema-free
    /// transport read a `Document` returns, delivered here with schema authority.
    Value(QuillValue),
}

/// A [`Document`] bound to its [`QuillConfig`] for typed reads. Construct with
/// [`Quill::reader`](crate::Quill::reader). Reads target the main card; use
/// [`card`](Self::card) for a composable card. The read twin of
/// [`TypedWriter`](crate::TypedWriter).
pub struct TypedReader<'a> {
    config: &'a QuillConfig,
    doc: &'a Document,
}

impl<'a> TypedReader<'a> {
    /// Bind `doc` to `config`. Prefer [`Quill::reader`](crate::Quill::reader).
    pub fn new(config: &'a QuillConfig, doc: &'a Document) -> Self {
        Self { config, doc }
    }

    /// Read a main-card field, interpreted by its declared type: `richtext` to
    /// markdown ([`ReadValue::Markdown`]), `plaintext` to literal text
    /// ([`ReadValue::Plaintext`]), every other type verbatim
    /// ([`ReadValue::Value`]). `Ok(None)` when the field is absent;
    /// [`EditError::UnknownField`] for a name the schema does not declare (a typo,
    /// as on the write side); [`EditError::FieldDecode`] when a content
    /// field holds a value that does not decode (a scalar an opaque
    /// [`store_field`](crate::Card::store_field) wrote).
    pub fn get(&self, name: &str) -> Result<Option<ReadValue>, EditError> {
        read_field(self.doc.main(), Some(&self.config.main.fields), name)
    }

    /// Read a main-card content field as its [`Content`], decoded through the
    /// codec its declared type names: the [`Content`] twin of [`get`](Self::get),
    /// which returns a projection. Total over the storage form, so a field the
    /// writer committed as a canonical content object and one a markdown parse
    /// left as an authored string both read back as a [`Content`], and which
    /// lane built the document stops being the caller's business.
    ///
    /// `Ok(None)` when the field is absent;
    /// [`EditError::UnknownField`] for a name the schema does not declare;
    /// [`EditError::FieldNotContent`] for a declared type that is not a content
    /// leaf (an `integer` has no [`Content`] even when it holds a string, and an
    /// `array<richtext>` carries content without having one [`Content`]);
    /// [`EditError::FieldDecode`] when the stored value decodes under
    /// neither encoding.
    ///
    /// The codec is the schema's to name, which is why this read is here and not
    /// on `Document`: a `richtext` string is markdown, a `plaintext` string is
    /// literal text, and a quill-free read would have to guess.
    pub fn get_content(&self, name: &str) -> Result<Option<Content>, EditError> {
        read_content(self.doc.main(), Some(&self.config.main.fields), name)
    }

    /// The main body's markdown projection, the quill-free body read
    /// ([`Card::body_markdown`](crate::Card::body_markdown)). A body's type is a
    /// format fact, not a schema fact, so this consults no schema and never
    /// raises; the body is never absent.
    pub fn body_markdown(&self) -> String {
        self.doc.main().body_markdown()
    }

    /// A schema-bound reader for the composable card at `index`. The card's
    /// `$kind` resolves its [`CardSchema`]; an unknown kind carries no schema, so
    /// every field name on it is undeclared and reads with
    /// [`EditError::UnknownField`] (read such a card verbatim through
    /// [`Card::payload`]). [`EditError::IndexOutOfRange`] when `index` is out of
    /// range: a boundary error, not an absent field, as the card write verbs
    /// treat it.
    pub fn card(&self, index: usize) -> Result<CardReader<'_>, EditError> {
        let len = self.doc.cards().len();
        let card = self
            .doc
            .card(index)
            .ok_or(EditError::IndexOutOfRange { index, len })?;
        let schema = card.kind().and_then(|k| self.config.card_kind(k));
        Ok(CardReader { schema, card })
    }
}

/// A single composable card bound to its [`CardSchema`], from
/// [`TypedReader::card`]. Same `get` / `body_markdown` verbs as [`TypedReader`],
/// reading the card at its bound index.
pub struct CardReader<'a> {
    schema: Option<&'a CardSchema>,
    card: &'a Card,
}

impl CardReader<'_> {
    /// The card's `$kind`, if any.
    pub fn kind(&self) -> Option<&str> {
        self.card.kind()
    }

    /// Read a field on this card, interpreted by its declared type: the card
    /// twin of [`TypedReader::get`]. Resolves the field against the card's
    /// [`CardSchema`]; a name the schema does not declare (or any name when the
    /// card kind is unknown) reads with [`EditError::UnknownField`].
    pub fn get(&self, name: &str) -> Result<Option<ReadValue>, EditError> {
        read_field(self.card, self.schema.map(|s| &s.fields), name)
    }

    /// Read a content field on this card as its [`Content`]: the card twin
    /// of [`TypedReader::get_content`], carrying the same outcomes.
    pub fn get_content(&self, name: &str) -> Result<Option<Content>, EditError> {
        read_content(self.card, self.schema.map(|s| &s.fields), name)
    }

    /// This card's body markdown: the card twin of [`TypedReader::body_markdown`],
    /// quill-free and never raising.
    pub fn body_markdown(&self) -> String {
        self.card.body_markdown()
    }
}

/// The shared read dispatch behind [`TypedReader::get`] and [`CardReader::get`]:
/// resolve `name` against `fields_schema` (an unknown name, or every name when
/// the whole schema is `None` (an unknown card kind) is
/// [`EditError::UnknownField`]), then interpret by the field's declared type. A
/// content field projects through its codec (`richtext` via
/// [`Card::field_markdown`], `plaintext` via [`Card::field_plaintext`]) each
/// carrying the projection's absent (`None`) / mismatch
/// ([`EditError::FieldDecode`]) outcomes; every other type returns its
/// canonical value verbatim, `None` when absent.
fn read_field(
    card: &Card,
    fields_schema: Option<&IndexMap<String, FieldSchema>>,
    name: &str,
) -> Result<Option<ReadValue>, EditError> {
    let schema = fields_schema
        .and_then(|m| m.get(name))
        .ok_or_else(|| EditError::UnknownField(name.to_string()))?;
    match schema.r#type {
        FieldType::RichText { .. } => project(
            card.field_markdown(name),
            name,
            CODEC_RICHTEXT,
            ReadValue::Markdown,
        ),
        FieldType::PlainText { .. } => project(
            card.field_plaintext(name),
            name,
            CODEC_PLAINTEXT,
            ReadValue::Plaintext,
        ),
        _ => Ok(card
            .payload()
            .get(name)
            .map(|v| ReadValue::Value(v.clone()))),
    }
}

/// The shared [`Content`] dispatch behind [`TypedReader::get_content`] and
/// [`CardReader::get_content`]: resolve `name` against `fields_schema`, then
/// decode through the codec the declared type names ([`Card::field_richtext`] for
/// `richtext`, [`Card::field_plaintext_content`] for `plaintext`). Every other
/// type is [`EditError::FieldNotContent`], answered from the schema before the
/// payload is read: whether a field has a [`Content`] is a declared-type fact, so a
/// `string` field holding markdown-looking text is still not content. The two
/// content leaves are the whole domain, so an `array<richtext>` lands here as
/// well: it carries content and still has no single [`Content`].
fn read_content(
    card: &Card,
    fields_schema: Option<&IndexMap<String, FieldSchema>>,
    name: &str,
) -> Result<Option<Content>, EditError> {
    let schema = fields_schema
        .and_then(|m| m.get(name))
        .ok_or_else(|| EditError::UnknownField(name.to_string()))?;
    // The codec rides out of the dispatch: it is the declared type's, not the
    // stored shape's, and the same bytes decode two ways.
    let (decoded, codec) = match schema.r#type {
        FieldType::RichText { .. } => (card.field_richtext(name), CODEC_RICHTEXT),
        FieldType::PlainText { .. } => (card.field_plaintext_content(name), CODEC_PLAINTEXT),
        ref other => {
            return Err(EditError::FieldNotContent {
                field: name.to_string(),
                declared: other.as_str().to_string(),
            })
        }
    };
    match decoded {
        None => Ok(None),
        Some(Ok(content)) => Ok(Some(content)),
        Some(Err(e)) => Err(EditError::FieldDecode {
            field: name.to_string(),
            codec: codec.to_string(),
            message: e.into_message(),
        }),
    }
}

/// Lift a codec projection ([`Card::field_markdown`] / [`Card::field_plaintext`])
/// into a [`ReadValue`]: `None` absent, `Some(Ok)` wrapped by `wrap`, `Some(Err)`
/// the [`EditError::FieldDecode`] naming `name` and the `codec` that ran.
fn project(
    projection: Option<Result<String, RichtextDecodeError>>,
    name: &str,
    codec: &str,
    wrap: fn(String) -> ReadValue,
) -> Result<Option<ReadValue>, EditError> {
    match projection {
        None => Ok(None),
        Some(Ok(text)) => Ok(Some(wrap(text))),
        Some(Err(e)) => Err(EditError::FieldDecode {
            field: name.to_string(),
            codec: codec.to_string(),
            message: e.into_message(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::version::QuillReference;
    use std::str::FromStr;

    const QUILL_YAML: &str = "\
quill:
  name: memo
  backend: typst
  version: 1.0.0
  description: Reader test quill
main:
  fields:
    subject:
      type: richtext
      inline: true
    note:
      type: plaintext
    qty:
      type: integer
card_kinds:
  note:
    fields:
      body:
        type: richtext
";

    fn config() -> QuillConfig {
        QuillConfig::from_yaml(QUILL_YAML).expect("valid quill")
    }

    fn blank_doc() -> Document {
        Document::new(QuillReference::from_str("memo@1.0.0").unwrap())
    }

    // Build a document through the writer, then read it back through the view.
    fn seeded_doc(config: &QuillConfig) -> Document {
        let mut doc = blank_doc();
        {
            let mut w = crate::TypedWriter::new(config, &mut doc);
            w.set("subject", "Hello **world**").unwrap();
            w.set("qty", "3").unwrap();
            w.add_card("note", [("body", "a *card*")], None, None).unwrap();
        }
        doc
    }

    #[test]
    fn richtext_field_projects_to_markdown() {
        let config = config();
        let doc = seeded_doc(&config);
        let view = TypedReader::new(&config, &doc);
        assert_eq!(
            view.get("subject").unwrap(),
            Some(ReadValue::Markdown("Hello **world**".to_string()))
        );
    }

    #[test]
    fn plaintext_field_projects_to_literal_text() {
        let config = config();
        let mut doc = blank_doc();
        {
            let mut w = crate::TypedWriter::new(&config, &mut doc);
            // Marks are literal under plaintext: `*hi*` is verbatim, not emphasis.
            w.set("note", "a *literal* line").unwrap();
        }
        let view = TypedReader::new(&config, &doc);
        assert_eq!(
            view.get("note").unwrap(),
            Some(ReadValue::Plaintext("a *literal* line".to_string()))
        );
    }

    #[test]
    fn scalar_field_returns_canonical_value() {
        let config = config();
        let doc = seeded_doc(&config);
        let view = TypedReader::new(&config, &doc);
        assert_eq!(
            view.get("qty").unwrap(),
            Some(ReadValue::Value(QuillValue::from_json(serde_json::json!(3))))
        );
    }

    #[test]
    fn absent_field_returns_none() {
        let config = config();
        let doc = blank_doc();
        let view = TypedReader::new(&config, &doc);
        assert_eq!(view.get("subject").unwrap(), None);
        assert_eq!(view.get("qty").unwrap(), None);
    }

    #[test]
    fn unknown_field_name_raises() {
        let config = config();
        let doc = blank_doc();
        let view = TypedReader::new(&config, &doc);
        assert!(matches!(
            view.get("nope"),
            Err(EditError::UnknownField(name)) if name == "nope"
        ));
    }

    #[test]
    fn richtext_field_holding_scalar_raises_mismatch() {
        let config = config();
        let mut doc = blank_doc();
        // An opaque write puts a bare number under the `subject` richtext field.
        doc.main_mut()
            .store_field("subject", QuillValue::from_json(serde_json::json!(3)))
            .unwrap();
        let view = TypedReader::new(&config, &doc);
        assert!(matches!(
            view.get("subject"),
            Err(EditError::FieldDecode { field, .. }) if field == "subject"
        ));
    }

    // A `plaintext` field parsed from markdown rests as an authored string, and
    // its codec is literal: `*literal*` is nine characters, not emphasis. The
    // object lane is covered above; this is the string lane, which decoded
    // through the markdown codec until it read its own declared type.
    #[test]
    fn parse_lane_plaintext_reads_through_the_literal_codec() {
        let config = config();
        let mut doc = blank_doc();
        doc.main_mut()
            .store_field(
                "note",
                QuillValue::from_json(serde_json::json!("a *literal* line")),
            )
            .unwrap();
        let view = TypedReader::new(&config, &doc);
        assert_eq!(
            view.get("note").unwrap(),
            Some(ReadValue::Plaintext("a *literal* line".to_string()))
        );
    }

    // The `Content` read is total over the storage form: the seeded (committed)
    // lane and the parsed (authored-string) lane return the same value.
    #[test]
    fn content_read_spans_both_storage_forms() {
        let config = config();
        let committed = seeded_doc(&config);
        let mut authored = blank_doc();
        authored
            .main_mut()
            .store_field(
                "subject",
                QuillValue::from_json(serde_json::json!("Hello **world**")),
            )
            .unwrap();

        let from_content = TypedReader::new(&config, &committed)
            .get_content("subject")
            .unwrap()
            .unwrap();
        let from_string = TypedReader::new(&config, &authored)
            .get_content("subject")
            .unwrap()
            .unwrap();
        assert_eq!(from_content.text, "Hello world");
        assert_eq!(from_content.text, from_string.text);
        assert_eq!(from_content.marks, from_string.marks);
    }

    // The codec follows the declared type, so the same stored bytes decode two
    // ways: markdown under `richtext`, literal under `plaintext`.
    #[test]
    fn content_read_decodes_by_declared_type() {
        let config = config();
        let mut doc = blank_doc();
        let text = serde_json::json!("a *literal* line");
        {
            let card = doc.main_mut();
            card.store_field("subject", QuillValue::from_json(text.clone())).unwrap();
            card.store_field("note", QuillValue::from_json(text)).unwrap();
        }
        let view = TypedReader::new(&config, &doc);
        // `richtext`: the asterisks are emphasis, so they leave the text.
        assert_eq!(view.get_content("subject").unwrap().unwrap().text, "a literal line");
        // `plaintext`: the asterisks are characters.
        assert_eq!(view.get_content("note").unwrap().unwrap().text, "a *literal* line");
    }

    #[test]
    fn content_read_absent_unknown_and_non_content() {
        let config = config();
        let doc = seeded_doc(&config);
        let view = TypedReader::new(&config, &doc);
        assert_eq!(view.get_content("note").unwrap(), None);
        assert!(matches!(
            view.get_content("nope"),
            Err(EditError::UnknownField(n)) if n == "nope"
        ));
        // A non-leaf declared type answers from the schema, not the payload:
        // `qty` holds 3 and is still not a content field.
        assert!(matches!(
            view.get_content("qty"),
            Err(EditError::FieldNotContent { field, declared })
                if field == "qty" && declared == "integer"
        ));
    }

    #[test]
    fn content_read_undecodable_value_raises() {
        let config = config();
        let mut doc = blank_doc();
        doc.main_mut()
            .store_field("subject", QuillValue::from_json(serde_json::json!(3)))
            .unwrap();
        let view = TypedReader::new(&config, &doc);
        assert!(matches!(
            view.get_content("subject"),
            Err(EditError::FieldDecode { field, .. }) if field == "subject"
        ));
    }

    #[test]
    fn card_content_reads_through_kind_schema() {
        let config = config();
        let doc = seeded_doc(&config);
        let view = TypedReader::new(&config, &doc);
        let card = view.card(0).unwrap();
        assert_eq!(card.get_content("body").unwrap().unwrap().text, "a card");
        assert!(matches!(
            card.get_content("nope"),
            Err(EditError::UnknownField(_))
        ));
    }

    #[test]
    fn card_field_reads_through_kind_schema() {
        let config = config();
        let doc = seeded_doc(&config);
        let view = TypedReader::new(&config, &doc);
        let card = view.card(0).unwrap();
        assert_eq!(card.kind(), Some("note"));
        assert_eq!(
            card.get("body").unwrap(),
            Some(ReadValue::Markdown("a *card*".to_string()))
        );
        assert!(matches!(card.get("nope"), Err(EditError::UnknownField(_))));
    }

    #[test]
    fn card_out_of_range_raises() {
        let config = config();
        let doc = blank_doc();
        let view = TypedReader::new(&config, &doc);
        assert!(matches!(
            view.card(9),
            Err(EditError::IndexOutOfRange { index: 9, len: 0 })
        ));
    }

    #[test]
    fn body_read_is_quill_free() {
        let config = config();
        let mut doc = blank_doc();
        doc.main_mut().revise_body("A **body**.").unwrap();
        let view = TypedReader::new(&config, &doc);
        assert_eq!(view.body_markdown(), "A **body**.");
    }
}
