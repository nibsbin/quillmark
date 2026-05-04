use super::QuillConfig;

impl QuillConfig {
    /// YAML encoding of [`QuillConfig::schema`] (structural schema, no ui).
    pub fn schema_yaml(&self) -> Result<String, serde_saphyr::ser::Error> {
        serde_saphyr::to_string(&self.schema())
    }

    /// YAML encoding of [`QuillConfig::form_schema`] (schema + ui hints).
    pub fn form_schema_yaml(&self) -> Result<String, serde_saphyr::ser::Error> {
        serde_saphyr::to_string(&self.form_schema())
    }
}

#[cfg(test)]
mod tests {
    use crate::quill::QuillConfig;

    fn config_from_yaml(yaml: &str) -> QuillConfig {
        QuillConfig::from_yaml(yaml).expect("valid quill yaml")
    }

    #[test]
    fn emits_minimal_schema() {
        let config = config_from_yaml(
            r#"
quill:
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

        let yaml = config.schema_yaml().unwrap();
        assert!(yaml.contains("main:"));
        assert!(yaml.contains("memo_for:"));
        assert!(yaml.contains("type: string"));
        assert!(!yaml.contains("ref:"));
        assert!(!yaml.contains("example:"));
    }

    #[test]
    fn omits_card_types_when_absent() {
        let config = config_from_yaml(
            r#"
quill:
  name: no_card_types
  version: "1.0"
  backend: typst
  description: No card types

main:
  fields:
    title:
      type: string
"#,
        );

        let yaml = config.schema_yaml().unwrap();
        assert!(!yaml.contains("card_types:"));
    }

    #[test]
    fn emits_integer_field_type() {
        let config = config_from_yaml(
            r#"
quill:
  name: integer_schema
  version: "1.0"
  backend: typst
  description: Integer schema

main:
  fields:
    page_count:
      type: integer
"#,
        );

        let yaml = config.schema_yaml().unwrap();
        assert!(yaml.contains("page_count:"));
        assert!(yaml.contains("type: integer"));
    }

    #[test]
    fn schema_strips_ui_form_schema_keeps_it() {
        let config = config_from_yaml(
            r#"
quill:
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

card_types:
  indorsement:
    title: Indorsement
    fields:
      signature_block:
        type: string
"#,
        );

        let clean = config.schema_yaml().unwrap();
        assert!(clean.contains("enum:"));
        assert!(clean.contains("card_types:"));
        assert!(clean.contains("indorsement:"));
        assert!(!clean.contains("ui:"));

        let form = config.form_schema_yaml().unwrap();
        assert!(form.contains("ui:"));
        assert!(form.contains("group: Meta"));
    }

    #[test]
    fn omits_example_from_schema() {
        let mut config = config_from_yaml(
            r#"
quill:
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

        let yaml = config.schema_yaml().unwrap();
        assert!(!yaml.contains("example:"));
        let form = config.form_schema_yaml().unwrap();
        assert!(!form.contains("example:"));
    }

    #[test]
    fn round_trips_as_json_value() {
        let config = config_from_yaml(
            r#"
quill:
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

        let yaml = config.schema_yaml().unwrap();
        let parsed: serde_json::Value = serde_saphyr::from_str(&yaml).unwrap();
        assert!(parsed.get("ref").is_none());
        assert!(parsed.get("main").and_then(|v| v.get("fields")).is_some());
    }

    #[test]
    fn schema_value_matches_yaml_round_trip() {
        let config = config_from_yaml(
            r#"
quill:
  name: parity
  version: "1.0"
  backend: typst
  description: Parity check

main:
  fields:
    title:
      type: string
      required: true
    status:
      type: string
      enum: [draft, final]
      default: draft

card_types:
  attachment:
    fields:
      label:
        type: string
"#,
        );

        let value = config.schema();
        let yaml = config.schema_yaml().unwrap();
        let parsed: serde_json::Value = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(value, parsed);

        let form_value = config.form_schema();
        let form_yaml = config.form_schema_yaml().unwrap();
        let form_parsed: serde_json::Value = serde_saphyr::from_str(&form_yaml).unwrap();
        assert_eq!(form_value, form_parsed);
    }
}
