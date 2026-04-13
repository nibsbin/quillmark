use std::collections::BTreeMap;

use serde::Serialize;

use crate::value::QuillValue;

use super::{CardSchema, FieldSchema, QuillConfig, UiContainerSchema, UiFieldSchema};

#[derive(Debug, Clone, Serialize)]
struct PublicSchema {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    example: Option<String>,
    fields: BTreeMap<String, PublicField>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    cards: BTreeMap<String, PublicCard>,
}

#[derive(Debug, Clone, Serialize)]
struct PublicField {
    #[serde(rename = "type")]
    field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<QuillValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    examples: Option<Vec<QuillValue>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "enum")]
    enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, PublicField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<PublicField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui: Option<PublicUiField>,
}

#[derive(Debug, Clone, Serialize)]
struct PublicCard {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    fields: BTreeMap<String, PublicField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui: Option<PublicUiContainer>,
}

#[derive(Debug, Clone, Serialize)]
struct PublicUiField {
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    multiline: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct PublicUiContainer {
    #[serde(skip_serializing_if = "Option::is_none")]
    hide_body: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_title: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn map_ui_field(ui: &UiFieldSchema) -> PublicUiField {
    PublicUiField {
        group: ui.group.clone(),
        order: ui.order,
        compact: ui.compact,
        multiline: ui.multiline,
    }
}

fn map_ui_container(ui: &UiContainerSchema) -> PublicUiContainer {
    PublicUiContainer {
        hide_body: ui.hide_body,
        default_title: ui.default_title.clone(),
    }
}

fn map_field(field: &FieldSchema) -> PublicField {
    PublicField {
        field_type: field.r#type.as_yaml_str().to_string(),
        title: field.title.clone(),
        description: field.description.clone(),
        required: field.required,
        default: field.default.clone(),
        examples: field.examples.as_ref().and_then(|values| {
            values.as_array().map(|arr| {
                arr.iter()
                    .map(|value| QuillValue::from_json(value.clone()))
                    .collect()
            })
        }),
        enum_values: field.enum_values.clone(),
        properties: field.properties.as_ref().map(|properties| {
            properties
                .iter()
                .map(|(name, schema)| (name.clone(), map_field(schema)))
                .collect()
        }),
        items: field.items.as_ref().map(|items| Box::new(map_field(items))),
        ui: field.ui.as_ref().map(map_ui_field),
    }
}

fn map_card(card: &CardSchema) -> PublicCard {
    PublicCard {
        title: card.title.clone(),
        description: card.description.clone(),
        fields: card
            .fields
            .iter()
            .map(|(name, field)| (name.clone(), map_field(field)))
            .collect(),
        ui: card.ui.as_ref().map(map_ui_container),
    }
}

impl QuillConfig {
    /// Emit the public schema contract as a YAML string.
    pub fn public_schema_yaml(&self) -> Result<String, serde_yaml::Error> {
        let schema = PublicSchema {
            name: self.name.clone(),
            description: self.main().description.clone(),
            example: self.example_markdown.clone(),
            fields: self
                .main()
                .fields
                .iter()
                .map(|(name, field)| (name.clone(), map_field(field)))
                .collect(),
            cards: self
                .card_definitions()
                .iter()
                .map(|card| (card.name.clone(), map_card(card)))
                .collect(),
        };

        serde_yaml::to_string(&schema)
    }
}

#[cfg(test)]
mod tests {
    use crate::quill::QuillConfig;

    fn config_from_yaml(yaml: &str) -> QuillConfig {
        QuillConfig::from_yaml(yaml).expect("valid quill yaml")
    }

    #[test]
    fn emits_minimal_public_schema() {
        let config = config_from_yaml(
            r#"
Quill:
  name: test_schema
  version: "1.0"
  backend: typst
  description: Test schema

main:
  fields:
    memo_for:
      type: string
      description: Memo recipient
"#,
        );

        let yaml = config.public_schema_yaml().unwrap();
        assert!(yaml.contains("name: test_schema"));
        assert!(yaml.contains("fields:"));
        assert!(yaml.contains("memo_for:"));
        assert!(yaml.contains("type: string"));
    }

    #[test]
    fn omits_cards_when_absent() {
        let config = config_from_yaml(
            r#"
Quill:
  name: no_cards
  version: "1.0"
  backend: typst
  description: No cards

main:
  fields:
    title:
      type: string
"#,
        );

        let yaml = config.public_schema_yaml().unwrap();
        assert!(!yaml.contains("cards:"));
    }

    #[test]
    fn includes_cards_ui_and_enum() {
        let config = config_from_yaml(
            r#"
Quill:
  name: card_schema
  version: "1.0"
  backend: typst
  description: Card schema

main:
  fields:
    status:
      type: string
      enum: [draft, final]
      ui:
        group: Meta

cards:
  indorsement:
    title: Indorsement
    fields:
      signature_block:
        type: string
"#,
        );

        let yaml = config.public_schema_yaml().unwrap();
        assert!(yaml.contains("enum:"));
        assert!(yaml.contains("ui:"));
        assert!(yaml.contains("cards:"));
        assert!(yaml.contains("indorsement:"));
        assert!(!yaml.contains("x-"));
        assert!(!yaml.contains("CARDS:"));
    }

    #[test]
    fn includes_example_when_present() {
        let mut config = config_from_yaml(
            r#"
Quill:
  name: with_example
  version: "1.0"
  backend: typst
  description: Has example

main:
  fields:
    body:
      type: markdown
"#,
        );
        config.example_markdown = Some("---\nQUILL: test\n---\n\n# Heading".to_string());

        let yaml = config.public_schema_yaml().unwrap();
        assert!(yaml.contains("example:"));
        assert!(yaml.contains("QUILL: test"));
    }

    #[test]
    fn round_trips_with_serde_yaml_value() {
        let config = config_from_yaml(
            r#"
Quill:
  name: round_trip
  version: "1.0"
  backend: typst
  description: Round trip

main:
  fields:
    recipients:
      type: array
      items:
        type: object
        properties:
          name:
            type: string
            required: true
"#,
        );

        let yaml = config.public_schema_yaml().unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(
            parsed.get("name").and_then(|v| v.as_str()),
            Some("round_trip")
        );
        assert!(parsed.get("fields").is_some());
    }
}
