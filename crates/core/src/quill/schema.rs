//! Maps the [`FieldSchema`] / [`FieldType`] model to a JSON-Schema-shaped
//! [`QuillValue`]. Backend-agnostic: backends consume it to drive per-field
//! transforms such as markdown to backend markup.

use super::{FieldSchema, FieldType, QuillConfig};
use crate::value::QuillValue;

/// The `contentMediaType` marking a richtext field in the transform schema. The
/// value crossing the seam for such a field is canonical Content-JSON (an
/// object), not a string: backends classify on this media type to lower the
/// content rather than a scalar.
pub const CONTENT_MEDIA_TYPE: &str = "application/quillmark-content+json";

/// Transform-schema keyword marking a single-`Para` richtext field (`inline: true`
/// in Quill.yaml). The blueprint spells the same fact `richtext(inline)<markdown>`;
/// this key is the JSON Schema–shaped wire for editor and backend consumers.
pub const QUILLMARK_INLINE_KEY: &str = "quillmark:inline";

/// Transform-schema keyword marking a `plaintext` field: the literal-codec
/// sibling of richtext. It rides the same [`CONTENT_MEDIA_TYPE`], so backends
/// lower it identically; this annotation only tells editors to mount a
/// formatting-free surface and to author/project through the literal codec.
pub const QUILLMARK_PLAIN_KEY: &str = "quillmark:plain";

/// Transform-schema keyword carrying an `enum` blank's label (`ui.blank_title`).
/// Emitted only when the author names one; absent, a consumer supplies its own
/// conventional label.
pub const QUILLMARK_BLANK_TITLE_KEY: &str = "quillmark:blank_title";

/// Transform-schema keyword carrying whether a human must author the field
/// ([`FieldSchema::must_fill`](crate::quill::FieldSchema::must_fill)). Emitted
/// on every field, always resolved: the obligation is not derivable from this
/// projection, which carries no `default:`.
///
/// It is an authoring affordance, not a submit gate. An unfilled field renders,
/// and enforcement — if a consumer wants any — is that consumer's policy.
pub const QUILLMARK_MUST_FILL_KEY: &str = "quillmark:must_fill";

/// Transform-schema keyword carrying a field's conditional obligation
/// (`must_fill_when:`) as `{field, operator, operand?}`.
///
/// An **annotation, not a constraint**, and deliberately not the standard
/// `if`/`then` spelling. This projection's contract is that a stock
/// JSON-Schema validator accepts exactly what the engine accepts (the same
/// reason `enum` emits the blank), and the engine never *rejects* an
/// outstanding obligation — it warns, and the document renders. An enforcing
/// `if`/`then` would make a validator refuse documents the engine happily
/// renders, converting a warning into a hard failure at the one seam that is
/// supposed to agree with the engine.
///
/// So the rule crosses as data a consumer can act on at its own severity: an
/// editor reveals the obligation live as the condition field changes, and a
/// strict consumer routes it however it routes `validation::must_fill`.
pub const QUILLMARK_MUST_FILL_WHEN_KEY: &str = "quillmark:must_fill_when";

/// Build a JSON-Schema-shaped descriptor of a [`QuillConfig`]'s main + card fields.
///
/// The descriptor marks richtext fields with `contentMediaType:
/// application/quillmark-content+json` (see [`CONTENT_MEDIA_TYPE`]) and
/// date/date-time fields with the corresponding JSON Schema `format`.
///
/// `$body` is injected into a kind's `properties` only when that kind's
/// `body.enabled` is not `false`. A body-disabled kind's `$body` is absent,
/// not present-and-empty: absence cascades through the `__meta__` address
/// tables so `form-field(field:)` rejects `$body` addresses on that
/// kind at compile time, matching `Quill::validate`'s hard error on authored
/// body content for the same kind.
pub fn build_transform_schema(config: &QuillConfig) -> QuillValue {
    fn field_to_schema(field: &FieldSchema) -> serde_json::Value {
        let mut schema = serde_json::Map::new();
        // In the prelude, not a type arm: the enum arm returns early below, and
        // an enum is the type an obligation matters most for. The *derived*
        // answer crosses, not the raw `Option` — a consumer reading this
        // projection should never have to re-run the `default:` derivation.
        schema.insert(
            QUILLMARK_MUST_FILL_KEY.to_string(),
            serde_json::Value::Bool(field.must_fill()),
        );
        // Beside it, and in the same prelude, for the same reason: the enum arm
        // returns early, and a conditional obligation is most often keyed on an
        // enum — so an enum field is exactly where a rule must still cross.
        if let Some(when) = &field.must_fill_when {
            let mut rule = serde_json::Map::new();
            rule.insert(
                "field".to_string(),
                serde_json::Value::String(when.field.clone()),
            );
            rule.insert(
                "operator".to_string(),
                serde_json::Value::String(when.condition.operator().to_string()),
            );
            if let Some(operand) = when.condition.operand() {
                rule.insert("operand".to_string(), operand);
            }
            schema.insert(
                QUILLMARK_MUST_FILL_WHEN_KEY.to_string(),
                serde_json::Value::Object(rule),
            );
        }
        // A finite domain projects to the idiomatic JSON-Schema spelling
        // `{type: string, enum: [...]}`: exactly what a backend dispatches on
        // today (a plain string), plus the domain. Keyed on the domain, as the
        // render floor, the pdfform widget and the blueprint annotation all are,
        // so a `FieldSchema` built outside the loader projects its domain too.
        if let Some(values) = &field.enum_values {
            schema.insert(
                "type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            // The model layer keeps `""` out of `values:` (it is not a choice),
            // but this projection describes what is *wire-valid*, and the blank
            // is: without it a standard JSON-Schema validator rejects a value
            // the engine accepts.
            schema.insert(
                "enum".to_string(),
                serde_json::Value::Array(
                    std::iter::once(String::new())
                        .chain(values.iter().cloned())
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
            return serde_json::Value::Object(schema);
        }
        match field.r#type {
            FieldType::String => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("string".to_string()),
                );
            }
            FieldType::RichText { inline } => {
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
                if inline {
                    schema.insert(
                        QUILLMARK_INLINE_KEY.to_string(),
                        serde_json::Value::Bool(true),
                    );
                }
            }
            FieldType::PlainText { inline } => {
                // Plaintext rides the *same* content and media type as richtext, so
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
                if inline {
                    schema.insert(
                        QUILLMARK_INLINE_KEY.to_string(),
                        serde_json::Value::Bool(true),
                    );
                }
            }
            // A loaded `enum` always carries `values:`, so the domain branch
            // above claims it; this arm is the domain-less residue.
            FieldType::Enum => {
                schema.insert(
                    "type".to_string(),
                    serde_json::Value::String("string".to_string()),
                );
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
                // element yields `items: {type: string}` (and a richtext element
                // carries its `contentMediaType`), while an object element yields
                // `items: {type: object, properties: …}`.
                if let Some(items) = &field.items {
                    schema.insert("items".to_string(), field_to_schema(items));
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
            // Distinct markers for the two date types drive the Typst backend's
            // per-type lowering (3-component vs 6-component `datetime(..)`). This
            // is the internal transform schema; the marker precedent is
            // `quillmark:inline`.
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
            serde_json::json!({ "type": "object", "contentMediaType": CONTENT_MEDIA_TYPE }),
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
                serde_json::json!({ "type": "object", "contentMediaType": CONTENT_MEDIA_TYPE }),
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
    fn must_fill_is_resolved_and_reaches_every_type() {
        let yaml = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    severity:   { type: enum, values: [low, high] }
    status:     { type: string, default: draft }
    confirmed:  { type: string, default: draft, must_fill: true }
    optional:   { type: string, must_fill: false }
"#;
        let json = build_from_yaml(yaml).as_json().clone();
        let flag = |name: &str| json["properties"][name][QUILLMARK_MUST_FILL_KEY].clone();

        // The enum arm returns before the type match, so a key placed in a type
        // arm would miss the one type an obligation matters most for.
        assert_eq!(flag("severity"), serde_json::json!(true));
        // Derived, not raw: a consumer reading this projection sees no
        // `default:` and so cannot re-run the derivation itself.
        assert_eq!(flag("status"), serde_json::json!(false));
        assert_eq!(flag("confirmed"), serde_json::json!(true));
        assert_eq!(flag("optional"), serde_json::json!(false));
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
            QUILLMARK_MUST_FILL_KEY: true,
        });
        assert_eq!(json["properties"]["classification"], domain);
        // The domain survives the recursion into an array's element object: a
        // consumer building a validator sees it, blank and all, at the leaf too.
        assert_eq!(
            json["properties"]["endorsements"]["items"]["properties"]["action"],
            serde_json::json!({
                "type": "string",
                "enum": ["", "approve", "disapprove"],
                QUILLMARK_MUST_FILL_KEY: true,
            })
        );
    }

    #[test]
    fn typed_table_emits_items_with_object_and_properties() {
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
          org: { type: string }
          year: { type: integer }
"#;
        let schema = build_from_yaml(yaml);
        let json = schema.as_json();
        let refs = &json["properties"]["refs"];
        assert_eq!(refs["type"], "array");
        assert_eq!(refs["items"]["type"], "object");
        assert_eq!(refs["items"]["properties"]["org"]["type"], "string");
        assert_eq!(refs["items"]["properties"]["year"]["type"], "integer");
    }

    #[test]
    fn scalar_array_emits_items_with_element_type() {
        let yaml = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    counts:
      type: array
      items: { type: integer }
"#;
        let schema = build_from_yaml(yaml);
        let json = schema.as_json();
        let counts = &json["properties"]["counts"];
        assert_eq!(counts["type"], "array");
        assert_eq!(counts["items"]["type"], "integer");
    }

    #[test]
    fn markdown_array_emits_items_with_content_media_type() {
        let yaml = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    sections:
      type: array
      items: { type: richtext }
"#;
        let schema = build_from_yaml(yaml);
        let json = schema.as_json();
        let sections = &json["properties"]["sections"];
        assert_eq!(sections["type"], "array");
        assert_eq!(sections["items"]["type"], "object");
        assert_eq!(sections["items"]["contentMediaType"], CONTENT_MEDIA_TYPE);
    }

    #[test]
    fn typed_dict_emits_object_with_properties() {
        let yaml = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    address:
      type: object
      properties:
        street: { type: string }
        city: { type: string }
"#;
        let schema = build_from_yaml(yaml);
        let json = schema.as_json();
        let address = &json["properties"]["address"];
        assert_eq!(address["type"], "object");
        assert_eq!(address["properties"]["street"]["type"], "string");
        assert_eq!(address["properties"]["city"]["type"], "string");
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
      type: string

card_kinds:
  indorsement:
    fields:
      signature_block:
        type: string
  note:
    fields:
      author:
        type: string
"#;

        let schema = build_from_yaml(yaml);
        let json = schema.as_json();

        let main_body = &json["properties"]["$body"];
        assert_eq!(main_body["type"], "object");
        assert_eq!(main_body["contentMediaType"], CONTENT_MEDIA_TYPE);

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
        }
    }

    #[test]
    fn inline_richtext_emits_quillmark_inline() {
        let yaml = r#"
quill:
  name: x
  version: 1.0.0
  backend: typst
  description: x
main:
  fields:
    subject:
      type: richtext
      inline: true
"#;
        let schema = build_from_yaml(yaml);
        let json = schema.as_json();
        let subject = &json["properties"]["subject"];
        assert_eq!(subject["type"], "object");
        assert_eq!(subject["contentMediaType"], CONTENT_MEDIA_TYPE);
        assert_eq!(subject[QUILLMARK_INLINE_KEY], true);
    }

    #[test]
    fn inline_richtext_array_items_emit_quillmark_inline() {
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
        type: richtext
        inline: true
"#;
        let schema = build_from_yaml(yaml);
        let json = schema.as_json();
        let items = &json["properties"]["refs"]["items"];
        assert_eq!(items[QUILLMARK_INLINE_KEY], true);
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
      type: string

card_kinds:
  indorsement:
    body:
      enabled: false
    fields:
      signature_block:
        type: string
  note:
    fields:
      author:
        type: string
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
