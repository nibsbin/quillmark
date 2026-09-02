//! The portable values shape: [`Quill::project`].
//!
//! [`resolve`](Quill::resolve) answers what the render projection *would* use,
//! blank-filling every declared field and tagging the rung it came from. This
//! answers what the document *carries*, with content leaves as their codec's
//! text, so a consumer edits plain values and sends them back. The two share the
//! render floor's reading of a value and differ after it.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::config::Leniency;
use super::{CardSchema, Quill, QuillConfig};
use crate::normalize::normalize_field_name;
use crate::reader::{project_value, ProjectMode};
use crate::{Card, Document};

/// A document's values as a consumer reads and edits them: the declared fields
/// the document carries, content leaves as their codec's text (`richtext`
/// markdown, `plaintext` literal text), every other value as the render floor
/// reads it, bodies as markdown.
///
/// **Sparse**: an absent field is absent here too, never materialized from its
/// `default:`. **A projection, never a storage format** — markdown does not
/// carry anchors, island ids, or content-only marks, and `$quill`, `$seed`,
/// `!must_fill` markers, YAML comments and undeclared fields are not carried.
/// Persist a document through [`StoredDocument`](crate::document).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct DocumentValues {
    /// Emitted in declaration order; key order carries no contract on the way
    /// in.
    #[serde(default)]
    pub fields: IndexMap<String, JsonValue>,
    /// The main body's markdown. Absent on input reads as the empty body.
    #[serde(default)]
    pub body: String,
    /// In document order.
    #[serde(default)]
    pub cards: Vec<CardValues>,
    /// The main card's `$ext`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Map<String, JsonValue>>,
}

/// `kind` is the stored `$kind`, `""` for a kindless card. A kind naming no
/// schema carries its fields verbatim: there is no declared type to project
/// them through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CardValues {
    pub kind: String,
    #[serde(default)]
    pub fields: IndexMap<String, JsonValue>,
    #[serde(default)]
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Map<String, JsonValue>>,
}

impl DocumentValues {
    /// Main fields only: no cards, empty body, no `ext`.
    pub fn new(fields: IndexMap<String, JsonValue>) -> Self {
        Self {
            fields,
            body: String::new(),
            cards: Vec::new(),
            ext: None,
        }
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn with_cards(mut self, cards: Vec<CardValues>) -> Self {
        self.cards = cards;
        self
    }

    pub fn with_ext(mut self, ext: serde_json::Map<String, JsonValue>) -> Self {
        self.ext = Some(ext);
        self
    }
}

impl CardValues {
    pub fn new(kind: impl Into<String>, fields: IndexMap<String, JsonValue>) -> Self {
        Self {
            kind: kind.into(),
            fields,
            body: String::new(),
            ext: None,
        }
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn with_ext(mut self, ext: serde_json::Map<String, JsonValue>) -> Self {
        self.ext = Some(ext);
        self
    }
}

impl Quill {
    /// The portable values of `doc` against this quill's schema.
    ///
    /// Total: never raises. A content leaf that decodes under neither encoding
    /// passes verbatim rather than failing the whole read — the load that
    /// admitted it already reported it (`conform::*` on
    /// [`Parsed`](crate::Parsed) warnings), and an ingestion must open a
    /// document it can repair.
    pub fn project(&self, doc: &Document) -> DocumentValues {
        let config = self.config();
        let cards = doc
            .cards()
            .iter()
            .map(|card| {
                let schema = card.kind().and_then(|k| config.card_kind(k));
                CardValues {
                    kind: card.kind().unwrap_or_default().to_string(),
                    fields: project_card_fields(schema, card),
                    body: card.body_markdown(),
                    ext: card.ext().cloned(),
                }
            })
            .collect();
        DocumentValues {
            fields: project_card_fields(Some(&config.main), doc.main()),
            body: doc.main().body_markdown(),
            cards,
            ext: doc.main().ext().cloned(),
        }
    }
}

/// One card's authored fields, in declaration order, each read through the
/// render floor and then projected at every content leaf below it.
///
/// The floor's reading is [`resolve`](Quill::resolve)'s (`Leniency::Render`,
/// keep-raw on refusal, NFC-normalized keys), so the two views cannot disagree
/// about what a document authored. A `None` schema is a card kind carrying no
/// declaration: its fields have no declared type to project through and ride
/// verbatim.
fn project_card_fields(schema: Option<&CardSchema>, card: &Card) -> IndexMap<String, JsonValue> {
    let payload = card.payload().to_index_map();
    let Some(schema) = schema else {
        return payload
            .into_iter()
            .map(|(name, value)| (name, value.as_json().clone()))
            .collect();
    };
    let mut authored: IndexMap<String, &crate::QuillValue> = IndexMap::new();
    for (raw_name, value) in payload.iter() {
        authored.insert(normalize_field_name(raw_name), value);
    }
    schema
        .fields
        .iter()
        .filter_map(|(name, field_schema)| {
            let value = authored.get(name.as_str())?;
            Some((name.clone(), project_field(name, value, field_schema)))
        })
        .collect()
}

/// One authored field's projection: the render floor's reading, then every
/// content leaf below it as its codec's text.
///
/// [`TypedWriter::set_values`](crate::TypedWriter::set_values) guards each cell
/// against this rather than against the stored bytes, so a value that projects
/// equal is not rewritten — the only comparison under which an anchor-bearing
/// content, a `!must_fill` marker, or a scalar shorthand the floor reads as
/// typed all count as unchanged.
pub(crate) fn project_field(
    name: &str,
    value: &crate::QuillValue,
    schema: &crate::FieldSchema,
) -> JsonValue {
    let conformed = QuillConfig::conform_value(value, schema, name, Leniency::Render)
        .unwrap_or_else(|_| value.clone());
    project_value(
        name,
        conformed.as_json(),
        schema,
        &mut Vec::new(),
        ProjectMode::Total,
    )
    .expect("ProjectMode::Total never raises")
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::quill::quill_from_yaml;

    pub(super) const QUILL: &str = r#"
quill:
  name: memo
  version: "1.0"
  backend: typst
  description: Values example
main:
  fields:
    subject: { type: richtext, inline: true }
    note: { type: plaintext }
    qty: { type: integer, default: 1 }
    paragraphs:
      type: array
      items: { type: richtext }
    letterhead:
      type: object
      properties:
        motto: { type: richtext }
        code: { type: string }
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI]
      variants:
        CUI:
          controlled_by: { type: plaintext }
          banner: { type: richtext }
card_kinds:
  line_item:
    fields:
      desc: { type: richtext, inline: true }
      qty: { type: integer }
"#;

    pub(super) const DOC: &str = "\
~~~card-yaml
$quill: memo@1.0
$kind: main
$ext:
  editor:
    title: Q3 memo
subject: Hello **world**
note: a *literal* line
paragraphs:
  - Para **one**
  - Para two
letterhead:
  motto: Fly *fight*
  code: \"9\"
classification:
  value: CUI
  controlled_by: \"OPR: 49 FW\"
  banner: \"**CUI** banner\"
~~~

Body prose.

~~~card-yaml
$kind: line_item
$ext:
  myapp:
    row_id: 17
desc: Widget __A__
qty: \"3\"
~~~
Item note.
";

    fn projected() -> DocumentValues {
        let quill = quill_from_yaml(QUILL);
        let doc = quill.parse(DOC).expect("document should parse").document;
        quill.project(&doc)
    }

    #[test]
    fn content_leaves_project_at_every_depth() {
        let v = projected();
        assert_eq!(v.fields["subject"], serde_json::json!("Hello **world**"));
        assert_eq!(v.fields["note"], serde_json::json!("a *literal* line"));
        assert_eq!(
            v.fields["paragraphs"],
            serde_json::json!(["Para **one**", "Para two"]),
            "an array<richtext> exports at each element, not as stored content"
        );
        assert_eq!(
            v.fields["letterhead"],
            serde_json::json!({"motto": "Fly *fight*", "code": "9"}),
            "a content property projects; a scalar sibling rides verbatim"
        );
        assert_eq!(
            v.fields["classification"],
            serde_json::json!({
                "value": "CUI",
                "controlled_by": "OPR: 49 FW",
                "banner": "**CUI** banner",
            }),
            "each variant cell projects at its own codec, the discriminant verbatim"
        );
    }

    #[test]
    fn the_shape_is_sparse() {
        let v = projected();
        assert!(
            !v.fields.contains_key("qty"),
            "an absent field is absent here: its `default: 1` is never materialized"
        );
    }

    #[test]
    fn bodies_cards_and_ext_ride_the_shape() {
        let v = projected();
        assert_eq!(v.body, "Body prose.");
        assert_eq!(v.ext, Some(serde_json::json!({"editor": {"title": "Q3 memo"}}).as_object().unwrap().clone()));

        let card = &v.cards[0];
        assert_eq!(card.kind, "line_item");
        assert_eq!(card.body, "Item note.");
        assert_eq!(card.fields["desc"], serde_json::json!("Widget **A**"));
        assert_eq!(
            card.fields["qty"],
            serde_json::json!(3),
            "the render floor reads the authored `\"3\"` as the plate does"
        );
        assert_eq!(
            card.ext,
            Some(serde_json::json!({"myapp": {"row_id": 17}}).as_object().unwrap().clone()),
            "$ext is the consumer's own card key and survives the projection"
        );
    }

    #[test]
    fn undeclared_fields_are_not_carried() {
        let quill = quill_from_yaml(QUILL);
        let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: Hi\nstray: whatever\n~~~\n";
        let doc = quill.parse(md).expect("parses").document;
        let v = quill.project(&doc);
        assert!(v.fields.contains_key("subject"));
        assert!(!v.fields.contains_key("stray"));
    }

    #[test]
    fn an_unknown_kind_card_carries_its_fields_verbatim() {
        let quill = quill_from_yaml(QUILL);
        let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
                  ~~~card-yaml\n$kind: mystery\nfoo: bar\n~~~\nStray.\n";
        let doc = quill.parse(md).expect("parses").document;
        let card = &quill.project(&doc).cards[0];
        assert_eq!(card.kind, "mystery");
        assert_eq!(card.fields["foo"], serde_json::json!("bar"));
        assert_eq!(card.body, "Stray.");
    }

    #[test]
    fn a_leaf_that_decodes_under_neither_encoding_passes_verbatim() {
        let quill = quill_from_yaml(QUILL);
        let mut doc = quill.parse("~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n")
            .expect("parses")
            .document;
        doc.main_mut()
            .store_field(
                "paragraphs",
                crate::QuillValue::from_json(serde_json::json!([{"not": "content"}])),
            )
            .unwrap();
        let v = quill.project(&doc);
        assert_eq!(
            v.fields["paragraphs"],
            serde_json::json!([{"not": "content"}]),
            "project is total: a dirty leaf rides out rather than failing the read"
        );
    }

    #[test]
    fn the_shape_round_trips_through_serde() {
        let v = projected();
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(serde_json::from_value::<DocumentValues>(json).unwrap(), v);
    }

    #[test]
    fn an_unknown_key_is_refused_on_the_way_in() {
        assert!(
            serde_json::from_value::<DocumentValues>(serde_json::json!({"feilds": {}})).is_err(),
            "the hand-authored lane must fail loudly on a misspelled key"
        );
    }
}

#[cfg(test)]
mod roundtrip_tests {
    use super::tests::*;
    use super::*;
    use crate::document::EditError;
    use crate::quill::quill_from_yaml;

    /// I1: the cycle writes nothing at all, on every document the bound door
    /// admits — markers, comments, `$ext`, dirty leaves and scalar shorthands
    /// included.
    #[track_caller]
    fn assert_cycle_is_a_no_op(quill: &Quill, md: &str) {
        let mut doc = quill.parse(md).expect("document should parse").document;
        let before = serde_json::to_string(&doc).unwrap();
        let values = quill.project(&doc);
        quill
            .writer(&mut doc)
            .set_values(&values)
            .unwrap_or_else(|e| panic!("set_values refused its own projection: {e:?}"));
        assert_eq!(
            serde_json::to_string(&doc).unwrap(),
            before,
            "set_values(project(doc)) moved bytes"
        );
    }

    #[test]
    fn the_untouched_cycle_is_a_byte_no_op() {
        let quill = quill_from_yaml(QUILL);
        assert_cycle_is_a_no_op(&quill, DOC);
    }

    #[test]
    fn the_cycle_preserves_what_a_re_import_cannot_reproduce() {
        let quill = quill_from_yaml(QUILL);
        for (what, md) in [
            (
                "a must-fill marker",
                "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: !must_fill Draft\n~~~\n",
            ),
            (
                "a YAML comment",
                "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n# keep me\nsubject: Hi\n~~~\n",
            ),
            (
                "an undeclared field",
                "~~~card-yaml\n$quill: memo@1.0\n$kind: main\nstray: whatever\n~~~\n",
            ),
            (
                "a scalar shorthand the floor reads as typed",
                "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
                 ~~~card-yaml\n$kind: line_item\nqty: \"3\"\n~~~\n",
            ),
            (
                "a kindless card",
                "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n~~~card-yaml\nfoo: bar\n~~~\n",
            ),
            (
                "an unknown-kind card",
                "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
                 ~~~card-yaml\n$kind: mystery\nfoo: bar\n~~~\nStray.\n",
            ),
        ] {
            let mut doc = quill.parse(md).expect("parses").document;
            let before = serde_json::to_string(&doc).unwrap();
            let values = quill.project(&doc);
            quill.writer(&mut doc).set_values(&values).expect(what);
            assert_eq!(
                serde_json::to_string(&doc).unwrap(),
                before,
                "the cycle did not preserve {what}"
            );
        }
    }

    /// I2: one write canonicalises, and the canonical form is a fixed point.
    #[test]
    fn a_write_canonicalises_once_then_holds_still() {
        let quill = quill_from_yaml(QUILL);
        let blank = || {
            quill
                .parse("~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n")
                .expect("parses")
                .document
        };
        let mut values = DocumentValues::default();
        values.fields.insert("subject".into(), serde_json::json!("Widget __A__"));
        values.fields.insert("qty".into(), serde_json::json!("3"));

        let mut d1 = blank();
        quill.writer(&mut d1).set_values(&values).unwrap();
        let p = quill.project(&d1);
        assert_eq!(
            p.fields["subject"],
            serde_json::json!("Widget **A**"),
            "the write canonicalises markdown"
        );
        assert_eq!(p.fields["qty"], serde_json::json!(3), "and the scalar");

        let mut d2 = blank();
        quill.writer(&mut d2).set_values(&p).unwrap();
        assert_eq!(
            serde_json::to_string(&d1).unwrap(),
            serde_json::to_string(&d2).unwrap()
        );
        assert_eq!(quill.project(&d2), p, "the canonical form is a fixed point");
    }

    #[test]
    fn a_declared_field_absent_from_the_shape_is_removed() {
        let quill = quill_from_yaml(QUILL);
        let mut doc = quill
            .parse("~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: Hi\nnote: keep\n~~~\n")
            .expect("parses")
            .document;
        let mut values = quill.project(&doc);
        values.fields.shift_remove("note");
        quill.writer(&mut doc).set_values(&values).unwrap();
        assert_eq!(quill.reader(&doc).get("note").unwrap(), None);
        assert!(doc.main().payload().get("subject").is_some());
    }

    #[test]
    fn an_undeclared_name_in_the_shape_is_refused_with_its_path() {
        let quill = quill_from_yaml(QUILL);
        let mut doc = quill
            .parse("~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n")
            .expect("parses")
            .document;
        let mut values = DocumentValues::default();
        values.fields.insert("nope".into(), serde_json::json!("x"));
        let errors = quill.writer(&mut doc).set_values(&values).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0.to_string(), "main.nope");
        assert!(matches!(errors[0].1, EditError::UnknownField { .. }));
    }

    #[test]
    fn every_refusal_arrives_at_once_and_nothing_is_written() {
        let quill = quill_from_yaml(QUILL);
        let mut doc = quill
            .parse("~~~card-yaml\n$quill: memo@1.0\n$kind: main\nsubject: Hi\n~~~\n")
            .expect("parses")
            .document;
        let before = serde_json::to_string(&doc).unwrap();
        let mut values = DocumentValues::default();
        values.fields.insert("nope".into(), serde_json::json!("x"));
        values.fields.insert("alsonope".into(), serde_json::json!("y"));
        values.cards.push(CardValues::new(
            "line_item",
            [("bad".to_string(), serde_json::json!(1))].into_iter().collect(),
        ));
        let errors = quill.writer(&mut doc).set_values(&values).unwrap_err();
        let paths: Vec<String> = errors.iter().map(|(p, _)| p.to_string()).collect();
        assert_eq!(paths, ["main.nope", "main.alsonope", "cards.line_item[0].bad"]);
        assert_eq!(
            serde_json::to_string(&doc).unwrap(),
            before,
            "an all-or-nothing batch leaves the document untouched"
        );
    }

    #[test]
    fn the_card_list_is_replaced_and_truncated() {
        let quill = quill_from_yaml(QUILL);
        let mut doc = quill
            .parse(
                "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n\n\
                 ~~~card-yaml\n$kind: line_item\nqty: 1\n~~~\n\n\
                 ~~~card-yaml\n$kind: line_item\nqty: 2\n~~~\n",
            )
            .expect("parses")
            .document;
        let mut values = quill.project(&doc);
        values.cards.truncate(1);
        quill.writer(&mut doc).set_values(&values).unwrap();
        assert_eq!(doc.cards().len(), 1);
        assert_eq!(
            quill.reader(&doc).card(0).unwrap().get("qty").unwrap(),
            Some(crate::ReadValue::Value(crate::QuillValue::from_json(
                serde_json::json!(1)
            )))
        );
    }

    #[test]
    fn a_new_card_is_appended_from_the_shape() {
        let quill = quill_from_yaml(QUILL);
        let mut doc = quill
            .parse("~~~card-yaml\n$quill: memo@1.0\n$kind: main\n~~~\n")
            .expect("parses")
            .document;
        let mut values = quill.project(&doc);
        values.cards.push(
            CardValues::new(
                "line_item",
                [("desc".to_string(), serde_json::json!("Widget **A**"))]
                    .into_iter()
                    .collect(),
            )
            .with_body("Note."),
        );
        quill.writer(&mut doc).set_values(&values).unwrap();
        assert_eq!(doc.cards().len(), 1);
        assert_eq!(quill.project(&doc).cards[0].body, "Note.");
    }

    #[test]
    fn an_absent_ext_is_untouched_and_an_empty_one_removes() {
        let quill = quill_from_yaml(QUILL);
        let md = "~~~card-yaml\n$quill: memo@1.0\n$kind: main\n$ext:\n  app:\n    k: 1\n~~~\n";

        let mut doc = quill.parse(md).expect("parses").document;
        let mut values = quill.project(&doc);
        values.ext = None;
        quill.writer(&mut doc).set_values(&values).unwrap();
        assert!(doc.main().ext().is_some(), "an absent ext leaves $ext alone");

        let mut doc = quill.parse(md).expect("parses").document;
        let mut values = quill.project(&doc);
        values.ext = Some(serde_json::Map::new());
        quill.writer(&mut doc).set_values(&values).unwrap();
        assert!(doc.main().ext().is_none(), "an empty ext removes $ext");
    }
}
