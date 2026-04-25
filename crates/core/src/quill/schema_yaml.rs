use super::QuillConfig;

impl QuillConfig {
    /// Emit the public schema contract as a YAML string.
    ///
    /// Thin wrapper around [`QuillConfig::public_schema`]; the JSON value
    /// returned by that function is the single source of truth for the
    /// public wire format, and YAML is one of several encodings of it.
    pub fn public_schema_yaml(&self) -> Result<String, serde_saphyr::ser::Error> {
        serde_saphyr::to_string(&self.public_schema())
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

        let yaml = config.public_schema_yaml().unwrap();
        assert!(yaml.contains("name: test_schema"));
        assert!(yaml.contains("main:"));
        assert!(yaml.contains("memo_for:"));
        assert!(yaml.contains("type: string"));
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

        let yaml = config.public_schema_yaml().unwrap();
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

        let yaml = config.public_schema_yaml().unwrap();
        assert!(yaml.contains("page_count:"));
        assert!(yaml.contains("type: integer"));
    }

    #[test]
    fn includes_card_types_ui_and_enum() {
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

        let yaml = config.public_schema_yaml().unwrap();
        assert!(yaml.contains("enum:"));
        assert!(yaml.contains("ui:"));
        assert!(yaml.contains("card_types:"));
        assert!(yaml.contains("indorsement:"));
    }

    #[test]
    fn includes_example_when_present() {
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

        let yaml = config.public_schema_yaml().unwrap();
        assert!(yaml.contains("example:"));
        assert!(yaml.contains("QUILL: test"));
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

        let yaml = config.public_schema_yaml().unwrap();
        let parsed: serde_json::Value = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(
            parsed.get("name").and_then(|v| v.as_str()),
            Some("round_trip")
        );
        assert!(parsed.get("main").and_then(|v| v.get("fields")).is_some());
    }

    #[test]
    fn public_schema_value_matches_yaml_round_trip() {
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

        let value = config.public_schema();
        let yaml = config.public_schema_yaml().unwrap();
        let parsed: serde_json::Value = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(value, parsed);
    }
}
