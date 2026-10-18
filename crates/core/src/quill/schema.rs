//! Maps the [`FieldSchema`] / [`FieldType`] model to a JSON-Schema-shaped
//! [`QuillValue`]. Backend-agnostic: backends consume it to drive per-field
//! transforms such as markdown to backend markup.

use super::{FieldSchema, FieldType, QuillConfig, VARIANT_DISCRIMINANT_KEY};
use crate::value::QuillValue;

/// The `contentMediaType` marking a richtext field in the transform schema. The
/// value crossing the seam for such a field is canonical Content-JSON (an
/// object), not a string: backends classify on this media type to lower the
/// content rather than a scalar.
pub const CONTENT_MEDIA_TYPE: &str = "application/quillmark-content+json";

/// Transform-schema keyword marking a `text` or `richtext` field that admits
/// more than one line (`multiline: true` in Quill.yaml); absent, the field is
/// one line (`text`) or one paragraph (`richtext`). The blueprint spells the
/// same fact `richtext(multiline)<markdown>`; this key is the JSON Schema–shaped
/// wire for editor and backend consumers.
pub const QUILLMARK_MULTILINE_KEY: &str = "quillmark:multiline";

/// Transform-schema keywords carrying a field's `max:` in the unit its type
/// counts: elements on an array, characters on a text field. A cap warns and
/// never refuses, so it rides a `quillmark:` annotation rather than JSON
/// Schema's asserting `maxItems` / `maxLength`, which a standard validator
/// would enforce against a document the engine renders.
pub const QUILLMARK_MAX_ITEMS_KEY: &str = "quillmark:maxItems";
pub const QUILLMARK_MAX_LENGTH_KEY: &str = "quillmark:maxLength";

/// Transform-schema keyword marking a `text` field: the literal-codec
/// sibling of richtext. It rides the same [`CONTENT_MEDIA_TYPE`], so backends
/// lower it identically; this annotation only tells editors to mount a
/// formatting-free surface and to author/project through the literal codec.
pub const QUILLMARK_PLAIN_KEY: &str = "quillmark:plain";

/// Transform-schema keyword carrying an `enum` blank's label (`ui.blank_title`).
/// Emitted only when the author names one; absent, a consumer supplies its own
/// conventional label.
pub const QUILLMARK_BLANK_TITLE_KEY: &str = "quillmark:blank_title";

/// Transform-schema keyword carrying a `matrix`'s roster, its `[id, title]`
/// pairs in declaration order: the vocabulary a plate prints, held or not.
///
/// It also fixes the key order a held value lowers in. The backend's dictionary
/// keys sort, so a reorder-only update rebuilds byte-identical source; this
/// order is a property of the schema rather than of the data, so honoring it
/// costs that nothing. The keys it does not name, an open matrix's added items,
/// trail it sorted.
pub const QUILLMARK_ROSTER_KEY: &str = "quillmark:roster";

/// The `{type: string, enum: ["", …]}` an enum projects to: a plain enum's own
/// cell, and the discriminant cell of a variant-bearing one. That container is a
/// mapping and therefore not a cell, so this is where its choice lands.
fn discriminant_schema(field: &FieldSchema) -> serde_json::Value {
    let mut schema = serde_json::Map::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("string".to_string()),
    );
    // The model layer keeps `""` out of `values:` (it is not a choice), but this
    // projection describes what is *wire-valid*, and the blank is: without it a
    // standard JSON-Schema validator rejects a value the engine accepts.
    schema.insert(
        "enum".to_string(),
        serde_json::Value::Array(
            std::iter::once(String::new())
                .chain(field.domain().iter().cloned())
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    if let Some(blank_title) = field.ui.as_ref().and_then(|u| u.blank_title.as_ref()) {
        schema.insert(
            QUILLMARK_BLANK_TITLE_KEY.to_string(),
            serde_json::Value::String(blank_title.clone()),
        );
    }
    serde_json::Value::Object(schema)
}

/// Build a JSON-Schema-shaped descriptor of a [`QuillConfig`]'s main + card fields.
///
/// The descriptor marks richtext fields with `contentMediaType:
/// application/quillmark-content+json` (see [`CONTENT_MEDIA_TYPE`]) and
/// date/date-time fields with the corresponding JSON Schema `format`. A
/// `max:` crosses as [`QUILLMARK_MAX_ITEMS_KEY`] or [`QUILLMARK_MAX_LENGTH_KEY`].
///
/// `$body` is injected into a kind's `properties` only when that kind's
/// `body.enabled` is not `false`. A body-disabled kind's `$body` is absent,
/// not present-and-empty, so `form-field(field:)` rejects `$body` addresses on
/// that kind at compile time, as authored body content on the same kind drops
/// under `Quill::validate`'s `validation::body_disabled` warning.
pub fn build_transform_schema(config: &QuillConfig) -> QuillValue {
    /// An optional cell's wire admits `null` beside its type, as the JSON-Schema
    /// `type: [<type>, "null"]` union, and an enum's list gains `null` beside
    /// its blank.
    fn field_to_schema(field: &FieldSchema) -> serde_json::Value {
        let mut schema = declared_schema(field);
        if field.optional {
            if let Some(ty) = schema.get("type").cloned() {
                schema["type"] = serde_json::json!([ty, "null"]);
            }
            if let Some(serde_json::Value::Array(members)) = schema.get_mut("enum") {
                members.insert(0, serde_json::Value::Null);
            }
        }
        schema
    }

    fn declared_schema(field: &FieldSchema) -> serde_json::Value {
        let mut schema = serde_json::Map::new();
        // A variant-bearing enum crosses as the container it rests as, every
        // world's fields flattened beside the discriminant: at schema time there
        // is no live world, so the union is the only projection available
        // (`SCHEMAS.md` § "Schema emission").
        if let Some(variants) = &field.variants {
            let mut properties = serde_json::Map::new();
            properties.insert(
                VARIANT_DISCRIMINANT_KEY.to_string(),
                discriminant_schema(field),
            );
            for fields in variants.values() {
                for (name, variant_field) in fields {
                    // Every repetition is the same declaration
                    // (`quill::variant_field_collision`).
                    if properties.contains_key(name) {
                        continue;
                    }
                    properties.insert(name.clone(), field_to_schema(variant_field));
                }
            }
            schema.insert(
                "type".to_string(),
                serde_json::Value::String("object".to_string()),
            );
            schema.insert("properties".to_string(), properties.into());
            return serde_json::Value::Object(schema);
        }
        match field.r#type {
            FieldType::Enum { .. } => return discriminant_schema(field),
            FieldType::RichText { multiline } => {
                // The content crosses the seam as a JSON object (canonical
                // Content-JSON), not a string; `type: object` + the richtext
                // media type is how a backend classifies it to lower the content.
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("object".to_string()),
                );
                schema.insert(
                    "contentMediaType".to_string(),
                    serde_json::Value::String(CONTENT_MEDIA_TYPE.to_string()),
                );
                if multiline {
                    schema.insert(
                        QUILLMARK_MULTILINE_KEY.to_string(),
                        serde_json::Value::Bool(true),
                    );
                }
                if let Some(max) = field.max {
                    schema.insert(QUILLMARK_MAX_LENGTH_KEY.to_string(), max.into());
                }
            }
            FieldType::Text { multiline } => {
                // Text rides the *same* content and media type as richtext, so
                // a backend classifies and lowers it identically: no backend edit.
                // The distinction (literal codec, no formatting) is carried by the
                // `quillmark:plain` annotation, which only editors consult.
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("object".to_string()),
                );
                schema.insert(
                    "contentMediaType".to_string(),
                    serde_json::Value::String(CONTENT_MEDIA_TYPE.to_string()),
                );
                schema.insert(QUILLMARK_PLAIN_KEY.to_string(), serde_json::Value::Bool(true));
                if multiline {
                    schema.insert(
                        QUILLMARK_MULTILINE_KEY.to_string(),
                        serde_json::Value::Bool(true),
                    );
                }
                if let Some(max) = field.max {
                    schema.insert(QUILLMARK_MAX_LENGTH_KEY.to_string(), max.into());
                }
            }
            FieldType::Number => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("number".to_string()),
                );
            }
            FieldType::Integer => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("integer".to_string()),
                );
            }
            FieldType::Boolean => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("boolean".to_string()),
                );
            }
            FieldType::Array => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("array".to_string()),
                );
                // The element schema is emitted recursively, so a scalar
                // element yields `items: { type: text }` (and a richtext element
                // carries its `contentMediaType`), while an object element yields
                // `items: {type: object, properties: …}`.
                if let Some(items) = &field.items {
                    schema.insert("items".to_string(), field_to_schema(items));
                }
                if let Some(max) = field.max {
                    schema.insert(QUILLMARK_MAX_ITEMS_KEY.to_string(), max.into());
                }
            }
            FieldType::Object => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("object".to_string()),
                );
                if let Some(properties) = &field.properties {
                    let mut props = serde_json::Map::new();
                    for (name, prop) in properties {
                        props.insert(name.clone(), field_to_schema(prop));
                    }
                    schema.insert("properties".to_string(), serde_json::Value::Object(props));
                }
            }
            // Each roster id's node is the member schema, so
            // `qualifications.flight_cc.detail` resolves as a typed dictionary's
            // leaf does; an open matrix's added items are `additionalProperties`,
            // where `title` is a cell. None is required: a value holds only the
            // members it ticks.
            FieldType::Matrix { ref roster, .. } => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("object".to_string()),
                );
                schema.insert(
                    QUILLMARK_ROSTER_KEY.to_string(),
                    serde_json::Value::Array(
                        roster
                            .iter()
                            .map(|(id, title)| serde_json::json!([id, title]))
                            .collect(),
                    ),
                );
                if let Some(member) = &field.member {
                    let node = field_to_schema(member);
                    let props: serde_json::Map<String, serde_json::Value> =
                        roster.keys().map(|id| (id.clone(), node.clone())).collect();
                    schema.insert("properties".to_string(), serde_json::Value::Object(props));
                }
                if let Some(item) = &field.added_item {
                    schema.insert("additionalProperties".to_string(), field_to_schema(item));
                }
            }
            // Distinct markers for the two date types drive the Typst backend's
            // per-type lowering (3-component vs 6-component `datetime(..)`). This
            // is the internal transform schema; the marker precedent is
            // `quillmark:multiline`.
            FieldType::Date => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("string".to_string()),
                );
                schema.insert(
                    "format".to_string(),
                    serde_json::Value::String("date".to_string()),
                );
            }
            FieldType::DateTime => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("string".to_string()),
                );
                schema.insert(
                    "format".to_string(),
                    serde_json::Value::String("date-time".to_string()),
                );
            }
        }
        serde_json::Value::Object(schema)
    }

    let mut properties = serde_json::Map::new();
    for (name, field) in &config.main.fields {
        properties.insert(name.clone(), field_to_schema(field));
    }
    if config.main.body_enabled() {
        properties.insert(
            "$body".to_string(),
            serde_json::json!({
                "type": "object",
                "contentMediaType": CONTENT_MEDIA_TYPE,
                QUILLMARK_MULTILINE_KEY: true,
            }),
        );
    }

    let mut defs = serde_json::Map::new();
    for card in &config.card_kinds {
        let mut card_properties = serde_json::Map::new();
        for (name, field) in &card.fields {
            card_properties.insert(name.clone(), field_to_schema(field));
        }
        if card.body_enabled() {
            card_properties.insert(
                "$body".to_string(),
                serde_json::json!({
                    "type": "object",
                    "contentMediaType": CONTENT_MEDIA_TYPE,
                    QUILLMARK_MULTILINE_KEY: true,
                }),
            );
        }
        defs.insert(
            format!("{}_card", card.name),
            serde_json::json!({
                "type": "object",
                "properties": card_properties,
            }),
        );
    }

    QuillValue::from_json(serde_json::json!({
        "type": "object",
        "properties": properties,
        "$defs": defs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_from_yaml(yaml: &str) -> QuillValue {
        let config = QuillConfig::from_yaml(yaml).expect("yaml parses");
        build_transform_schema(&config)
    }

    #[test]
    fn enum_carries_its_domain_at_every_depth() {
        let yaml = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI]
    endorsements:
      type: array
      items:
        type: object
        properties:
          action:
            type: enum
            values: [approve, disapprove]
"#;
        let schema = build_from_yaml(yaml);
        let json = schema.as_json();
        // The blank leads the domain: the projection describes what is
        // wire-valid, and every enum accepts its blank.
        let domain = serde_json::json!({
            "type": "string",
            "enum": ["", "UNCLASSIFIED", "CUI"],
        });
        assert_eq!(json["properties"]["classification"], domain);
        // The domain survives the recursion into an array's element object: a
        // consumer building a validator sees it, blank and all, at the leaf too.
        assert_eq!(
            json["properties"]["endorsements"]["items"]["properties"]["action"],
            serde_json::json!({
                "type": "string",
                "enum": ["", "approve", "disapprove"],
            })
        );
    }

    const BLANK_TITLE_YAML: &str = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI]
      default: ""
      ui:
        blank_title: "(no marking)"
    action:
      type: enum
      values: [approve, disapprove]
"#;

    /// One `max:` in Quill.yaml, two keywords on the wire: the unit a reader
    /// of the declaration takes from the sibling `type:` is in the keyword's
    /// name here, and no asserting JSON Schema keyword refuses what the engine
    /// renders.
    #[test]
    fn max_crosses_as_the_keyword_that_names_its_unit() {
        let json = build_from_yaml(
            r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    citation: { type: richtext, multiline: true, max: 1350 }
    ssn: { type: text, max: 9 }
    statements:
      type: array
      max: 4
      items: { type: text, max: 350 }
    notes:
      type: array
      items: { type: text }
"#,
        )
        .as_json()
        .clone();
        let fields = &json["properties"];
        assert_eq!(fields["citation"]["quillmark:maxLength"], 1350);
        assert_eq!(fields["ssn"]["quillmark:maxLength"], 9);
        assert_eq!(fields["statements"]["quillmark:maxItems"], 4);
        assert_eq!(fields["statements"]["items"]["quillmark:maxLength"], 350);
        for (field, keyword) in [("citation", "quillmark:maxItems"), ("statements", "quillmark:maxLength")] {
            assert!(fields[field].get(keyword).is_none(), "{field} carries no {keyword}");
        }
        assert!(fields["notes"].get("quillmark:maxItems").is_none(), "an uncapped array");
        assert!(fields["notes"]["items"].get("quillmark:maxLength").is_none(), "uncapped text");
        for keyword in ["maxItems", "maxLength"] {
            assert!(fields["statements"].get(keyword).is_none(), "a cap never asserts");
        }
    }

    #[test]
    fn blank_title_labels_the_blank_only_where_the_author_names_one() {
        let json = build_from_yaml(BLANK_TITLE_YAML).as_json().clone();

        assert_eq!(
            json["properties"]["classification"],
            serde_json::json!({
                "type": "string",
                "enum": ["", "UNCLASSIFIED", "CUI"],
                "quillmark:blank_title": "(no marking)",
            })
        );
        assert_eq!(
            json["properties"]["action"],
            serde_json::json!({
                "type": "string",
                "enum": ["", "approve", "disapprove"],
            })
        );
    }

    /// The declaration view emits what the author wrote, so a quill round-tripping
    /// through it still loads: injecting the blank there would emit `values: ["", …]`,
    /// which `quill::enum_blank_member` rejects.
    #[test]
    fn blank_title_does_not_cross_into_the_declaration_view() {
        let config = QuillConfig::from_yaml(BLANK_TITLE_YAML).expect("yaml parses");
        let emitted = config.schema();

        assert_eq!(
            emitted["main"]["fields"]["classification"]["values"],
            serde_json::json!(["UNCLASSIFIED", "CUI"])
        );
        assert!(!emitted.to_string().contains("quillmark:"), "{emitted}");
    }

    #[test]
    fn each_field_shape_projects_its_schema() {
        let yaml = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    refs:
      type: array
      items:
        type: object
        properties:
          org: { type: text }
          year: { type: integer }
    counts:
      type: array
      items: { type: integer }
    sections:
      type: array
      items: { type: richtext, multiline: true }
    address:
      type: object
      properties:
        street: { type: text }
        city: { type: text }
    subject:
      type: richtext
      multiline: false
    lines:
      type: array
      items:
        type: richtext
        multiline: false
    label:
      type: text
"#;
        let content = serde_json::json!({ "type": "object", "contentMediaType": CONTENT_MEDIA_TYPE });
        let multiline = serde_json::json!({
            "type": "object",
            "contentMediaType": CONTENT_MEDIA_TYPE,
            QUILLMARK_MULTILINE_KEY: true,
        });
        let text = serde_json::json!({
            "type": "object",
            "contentMediaType": CONTENT_MEDIA_TYPE,
            QUILLMARK_PLAIN_KEY: true,
        });
        let cases = [
            (
                "refs",
                serde_json::json!({
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "org": text,
                            "year": { "type": "integer" },
                        },
                    },
                }),
            ),
            (
                "counts",
                serde_json::json!({ "type": "array", "items": { "type": "integer" } }),
            ),
            (
                "sections",
                serde_json::json!({ "type": "array", "items": multiline }),
            ),
            (
                "address",
                serde_json::json!({
                    "type": "object",
                    "properties": { "street": text, "city": text },
                }),
            ),
            ("subject", content.clone()),
            (
                "lines",
                serde_json::json!({ "type": "array", "items": content }),
            ),
            ("label", text),
        ];
        let schema = build_from_yaml(yaml);
        for (field, expected) in cases {
            assert_eq!(schema.as_json()["properties"][field], expected, "{field}");
        }
    }

    #[test]
    fn injects_body_as_markdown_for_main_and_each_card_kind() {
        let yaml = r#"
quill:
  name: example
  version: 0.1.0
  backend: typst
  description: example

main:
  fields:
    title:
      type: text

card_kinds:
  indorsement:
    fields:
      signature_block:
        type: text
  note:
    fields:
      author:
        type: text
"#;

        let schema = build_from_yaml(yaml);
        let json = schema.as_json();

        let main_body = &json["properties"]["$body"];
        assert_eq!(main_body["type"], "object");
        assert_eq!(main_body["contentMediaType"], CONTENT_MEDIA_TYPE);
        assert_eq!(main_body[QUILLMARK_MULTILINE_KEY], true);

        for def_name in ["indorsement_card", "note_card"] {
            let card_body = &json["$defs"][def_name]["properties"]["$body"];
            assert_eq!(
                card_body["type"], "object",
                "{def_name} $body type should be object"
            );
            assert_eq!(
                card_body["contentMediaType"], CONTENT_MEDIA_TYPE,
                "{def_name} $body should be richtext"
            );
            assert_eq!(
                card_body[QUILLMARK_MULTILINE_KEY], true,
                "{def_name} $body should lower as a block"
            );
        }
    }

    #[test]
    fn body_disabled_kind_omits_body_from_schema() {
        let yaml = r#"
quill:
  name: example
  version: 0.1.0
  backend: typst
  description: example

main:
  body:
    enabled: false
  fields:
    title:
      type: text

card_kinds:
  indorsement:
    body:
      enabled: false
    fields:
      signature_block:
        type: text
  note:
    fields:
      author:
        type: text
"#;

        let schema = build_from_yaml(yaml);
        let json = schema.as_json();

        assert!(
            json["properties"].get("$body").is_none(),
            "body-disabled main should not carry $body"
        );
        assert!(
            json["$defs"]["indorsement_card"]["properties"]
                .get("$body")
                .is_none(),
            "body-disabled card kind should not carry $body"
        );
        assert!(
            json["$defs"]["note_card"]["properties"]
                .get("$body")
                .is_some(),
            "body-enabled card kind should still carry $body"
        );
    }
}
