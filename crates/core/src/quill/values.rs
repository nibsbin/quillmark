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
/// [`StoredDocument`](crate::document) remains persistence.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct DocumentValues {
    /// Main-card fields by name, emitted in declaration order. Key order
    /// carries no contract on the way in.
    #[serde(default)]
    pub fields: IndexMap<String, JsonValue>,
    /// The main body's markdown. Absent on input reads as the empty body.
    #[serde(default)]
    pub body: String,
    /// Every composable card, in document order.
    #[serde(default)]
    pub cards: Vec<CardValues>,
    /// The main card's `$ext`, when it carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Map<String, JsonValue>>,
}

/// One composable card's values. `kind` is the stored `$kind`, `""` for a
/// kindless card. A kind naming no schema carries its fields verbatim: there is
/// no declared type to project them through.
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
            let conformed =
                QuillConfig::conform_value(value, field_schema, name, Leniency::Render)
                    .unwrap_or_else(|_| (*value).clone());
            let projected = project_value(
                name,
                conformed.as_json(),
                field_schema,
                &mut Vec::new(),
                ProjectMode::Total,
            )
            .expect("ProjectMode::Total never raises");
            Some((name.clone(), projected))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quill::quill_from_yaml;

    const QUILL: &str = r#"
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

    const DOC: &str = "\
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
