use std::collections::HashMap;

use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime};

use crate::quill::formats::DATE_FORMAT;
use crate::quill::{CardSchema, FieldSchema, FieldType, QuillConfig};
use crate::value::QuillValue;

/// Validation error with a structured field path.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("missing required field `{path}`")]
    MissingRequired { path: String },

    #[error("field `{path}` has type `{actual}`, expected `{expected}`")]
    TypeMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("field `{path}` value `{value}` not in allowed set {allowed:?}")]
    EnumViolation {
        path: String,
        value: String,
        allowed: Vec<String>,
    },

    #[error("field `{path}` does not match expected format `{format}`")]
    FormatViolation { path: String, format: String },

    #[error("unknown card type `{card}` at `{path}`")]
    UnknownCard { path: String, card: String },

    #[error("card at `{path}` missing `CARD` discriminator")]
    MissingCardDiscriminator { path: String },
}

/// Validate a parsed document against the full config.
///
/// Validates main fields, all card instances, and enforces required fields.
/// Collects all errors rather than short-circuiting on the first.
pub fn validate_document(
    config: &QuillConfig,
    fields: &HashMap<String, QuillValue>,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = validate_fields_for_card(config.main(), fields, "");

    if let Some(cards_value) = fields.get("CARDS") {
        match cards_value.as_array() {
            Some(cards) => {
                for (index, card_value) in cards.iter().enumerate() {
                    let item_path = index_path("cards", index);
                    let Some(card_object) = card_value.as_object() else {
                        errors.push(ValidationError::TypeMismatch {
                            path: item_path,
                            expected: "object".to_string(),
                            actual: json_type_name(card_value).to_string(),
                        });
                        continue;
                    };

                    let Some(card_discriminator) = card_object.get("CARD") else {
                        errors.push(ValidationError::MissingCardDiscriminator { path: item_path });
                        continue;
                    };

                    let Some(card_name) = card_discriminator.as_str() else {
                        errors.push(ValidationError::TypeMismatch {
                            path: child_path(&item_path, "CARD"),
                            expected: "string".to_string(),
                            actual: json_type_name(card_discriminator).to_string(),
                        });
                        continue;
                    };

                    let Some(card_schema) = config.card_definition(card_name) else {
                        errors.push(ValidationError::UnknownCard {
                            path: item_path,
                            card: card_name.to_string(),
                        });
                        continue;
                    };

                    let mut card_fields = HashMap::new();
                    for (key, value) in card_object {
                        card_fields.insert(key.clone(), QuillValue::from_json(value.clone()));
                    }

                    let card_path = format!("cards.{card_name}[{index}]");
                    errors.extend(validate_fields_for_card(
                        card_schema,
                        &card_fields,
                        &card_path,
                    ));
                }
            }
            None => errors.push(ValidationError::TypeMismatch {
                path: "CARDS".to_string(),
                expected: "array".to_string(),
                actual: json_type_name(cards_value.as_json()).to_string(),
            }),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_fields_for_card(
    card: &CardSchema,
    fields: &HashMap<String, QuillValue>,
    base_path: &str,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut field_names: Vec<&String> = card.fields.keys().collect();
    field_names.sort();

    for field_name in field_names {
        let schema = &card.fields[field_name];
        let path = child_path(base_path, field_name);
        match fields.get(field_name) {
            Some(value) => errors.extend(validate_field(schema, value, &path)),
            None if schema.required => errors.push(ValidationError::MissingRequired { path }),
            None => {}
        }
    }

    errors
}

/// Validate a single value against a field schema at the given path.
/// Used internally; exposed for testing.
pub(crate) fn validate_field(
    field: &FieldSchema,
    value: &QuillValue,
    path: &str,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let type_valid = match field.r#type {
        FieldType::String | FieldType::Markdown => value.as_str().is_some(),
        FieldType::Number => value.as_json().is_number(),
        FieldType::Boolean => value.as_bool().is_some(),
        FieldType::Date => match value.as_str() {
            Some(text) => {
                if is_valid_date(text) {
                    true
                } else {
                    errors.push(ValidationError::FormatViolation {
                        path: path.to_string(),
                        format: "date".to_string(),
                    });
                    false
                }
            }
            None => false,
        },
        FieldType::DateTime => match value.as_str() {
            Some(text) => {
                if is_valid_datetime(text) {
                    true
                } else {
                    errors.push(ValidationError::FormatViolation {
                        path: path.to_string(),
                        format: "date-time".to_string(),
                    });
                    false
                }
            }
            None => false,
        },
        FieldType::Array => match value.as_array() {
            Some(items) => {
                if let Some(item_schema) = &field.items {
                    for (idx, item) in items.iter().enumerate() {
                        errors.extend(validate_field(
                            item_schema,
                            &QuillValue::from_json(item.clone()),
                            &index_path(path, idx),
                        ));
                    }
                }
                true
            }
            None => false,
        },
        FieldType::Object => match value.as_object() {
            Some(object) => {
                if let Some(properties) = &field.properties {
                    let mut property_names: Vec<&String> = properties.keys().collect();
                    property_names.sort();
                    for property_name in property_names {
                        let property_schema = &properties[property_name];
                        let property_path = child_path(path, property_name);
                        match object.get(property_name) {
                            Some(property_value) => errors.extend(validate_field(
                                property_schema,
                                &QuillValue::from_json(property_value.clone()),
                                &property_path,
                            )),
                            None if property_schema.required => {
                                errors.push(ValidationError::MissingRequired {
                                    path: property_path,
                                })
                            }
                            None => {}
                        }
                    }
                }
                true
            }
            None => false,
        },
    };

    // A Date/DateTime with a string value already emitted a FormatViolation;
    // skip the redundant TypeMismatch in that case.
    let format_error_already_reported =
        matches!(field.r#type, FieldType::Date | FieldType::DateTime) && value.as_str().is_some();

    if !type_valid && !format_error_already_reported {
        errors.push(ValidationError::TypeMismatch {
            path: path.to_string(),
            expected: expected_type_name(&field.r#type).to_string(),
            actual: json_type_name(value.as_json()).to_string(),
        });
    }

    if type_valid {
        if let (Some(allowed), Some(actual)) = (&field.enum_values, value.as_str()) {
            if !allowed.contains(&actual.to_string()) {
                errors.push(ValidationError::EnumViolation {
                    path: path.to_string(),
                    value: actual.to_string(),
                    allowed: allowed.clone(),
                });
            }
        }
    }

    errors
}

fn is_valid_date(value: &str) -> bool {
    Date::parse(value, &DATE_FORMAT).is_ok()
}

fn is_valid_datetime(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339).is_ok()
}

fn expected_type_name(field_type: &FieldType) -> &'static str {
    match field_type {
        FieldType::String | FieldType::Markdown | FieldType::Date | FieldType::DateTime => "string",
        FieldType::Number => "number",
        FieldType::Boolean => "boolean",
        FieldType::Array => "array",
        FieldType::Object => "object",
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn child_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}.{child}")
    }
}

fn index_path(parent: &str, index: usize) -> String {
    format!("{parent}[{index}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config_with(main_fields: &str, cards: &str) -> QuillConfig {
        let yaml = format!(
            r#"
Quill:
  name: native_validation
  backend: typst
  description: Native validator tests
  version: 1.0.0
main:
  fields:
{main_fields}
{cards}
"#
        );
        // Use _with_warnings so silently-dropped fields (e.g. unsupported
        // standalone `type: object`) fail loudly instead of passing vacuously.
        let (config, warnings) = QuillConfig::from_yaml_with_warnings(&yaml).unwrap();
        assert!(
            warnings.is_empty(),
            "config_with produced warnings (test schema is unsupported): {:?}",
            warnings
        );
        config
    }

    fn fields(entries: &[(&str, serde_json::Value)]) -> HashMap<String, QuillValue> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), QuillValue::from_json(v.clone())))
            .collect()
    }

    fn has_error<F>(errors: &[ValidationError], predicate: F) -> bool
    where
        F: Fn(&ValidationError) -> bool,
    {
        errors.iter().any(predicate)
    }

    #[test]
    fn validates_simple_string_field() {
        let config = config_with("    title:\n      type: string\n      required: true", "");
        let doc = fields(&[("title", json!("Memo"))]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn rejects_simple_string_type_mismatch() {
        let config = config_with("    title:\n      type: string", "");
        let doc = fields(&[("title", json!(9))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| matches!(
            e,
            ValidationError::TypeMismatch { path, expected, actual }
            if path == "title" && expected == "string" && actual == "number"
        )));
    }

    #[test]
    fn reports_missing_required_field() {
        let config = config_with(
            "    memo_for:\n      type: string\n      required: true",
            "",
        );
        let errors = validate_document(&config, &HashMap::new()).unwrap_err();
        assert!(has_error(&errors, |e| {
            matches!(e, ValidationError::MissingRequired { path } if path == "memo_for")
        }));
    }

    #[test]
    fn reports_required_field_wrong_type() {
        let config = config_with(
            "    memo_for:\n      type: string\n      required: true",
            "",
        );
        let doc = fields(&[("memo_for", json!(true))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| matches!(
            e,
            ValidationError::TypeMismatch { path, .. } if path == "memo_for"
        )));
    }

    #[test]
    fn validates_enum_value() {
        let config = config_with(
            "    status:\n      type: string\n      enum:\n        - draft\n        - final",
            "",
        );
        let doc = fields(&[("status", json!("draft"))]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn rejects_invalid_enum_value() {
        let config = config_with(
            "    status:\n      type: string\n      enum:\n        - draft\n        - final",
            "",
        );
        let doc = fields(&[("status", json!("invalid"))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| matches!(
            e,
            ValidationError::EnumViolation { path, value, .. }
            if path == "status" && value == "invalid"
        )));
    }

    #[test]
    fn validates_date_format() {
        let config = config_with("    signed_on:\n      type: date", "");
        let doc = fields(&[("signed_on", json!("2026-04-13"))]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn rejects_invalid_date_format() {
        let config = config_with("    signed_on:\n      type: date", "");
        let doc = fields(&[("signed_on", json!("13-04-2026"))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| {
            matches!(e, ValidationError::FormatViolation { path, format } if path == "signed_on" && format == "date")
        }));
    }

    #[test]
    fn validates_datetime_format() {
        let config = config_with("    created_at:\n      type: datetime", "");
        let doc = fields(&[("created_at", json!("2026-04-13T19:24:55Z"))]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn rejects_invalid_datetime_format() {
        let config = config_with("    created_at:\n      type: datetime", "");
        let doc = fields(&[("created_at", json!("2026-04-13 19:24:55"))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| matches!(
            e,
            ValidationError::FormatViolation { path, format }
            if path == "created_at" && format == "date-time"
        )));
    }

    #[test]
    fn markdown_accepts_any_string() {
        let config = config_with("    body:\n      type: markdown", "");
        let doc = fields(&[("body", json!("# Heading\n\nBody text"))]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn validates_array_of_strings() {
        let config = config_with(
            "    tags:\n      type: array\n      items:\n        type: string",
            "",
        );
        let doc = fields(&[("tags", json!(["a", "b"]))]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn rejects_invalid_array_element_type() {
        let config = config_with(
            "    tags:\n      type: array\n      items:\n        type: string",
            "",
        );
        let doc = fields(&[("tags", json!(["a", 2]))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| matches!(
            e,
            ValidationError::TypeMismatch { path, .. } if path == "tags[1]"
        )));
    }

    #[test]
    fn validates_array_of_objects() {
        let config = config_with(
            "    recipients:\n      type: array\n      items:\n        type: object\n        properties:\n          name:\n            type: string\n            required: true\n          org:\n            type: string",
            "",
        );
        let doc = fields(&[("recipients", json!([{ "name": "Sam", "org": "HQ" }]))]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn reports_missing_required_field_in_array_object() {
        let config = config_with(
            "    recipients:\n      type: array\n      items:\n        type: object\n        properties:\n          name:\n            type: string\n            required: true\n          org:\n            type: string",
            "",
        );
        let doc = fields(&[("recipients", json!([{ "org": "HQ" }]))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| {
            matches!(e, ValidationError::MissingRequired { path } if path == "recipients[0].name")
        }));
    }

    // NOTE: top-level `type: object` fields are explicitly unsupported by
    // the config parser (see `config::parse_fields_with_order`). Object
    // schemas only appear inside `array.items`; coverage for that shape lives
    // in `validates_array_of_objects` and
    // `reports_missing_required_field_in_array_object`.

    #[test]
    fn reports_type_mismatch_for_cards_when_not_array() {
        let config = config_with(
            "    title:\n      type: string",
            "cards:\n  indorsement:\n    fields:\n      signature_block:\n        type: string",
        );
        let doc = fields(&[("CARDS", json!("not-an-array"))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| {
            matches!(
                e,
                ValidationError::TypeMismatch { path, expected, actual }
                if path == "CARDS" && expected == "array" && actual == "string"
            )
        }));
    }

    #[test]
    fn accumulates_multiple_missing_required_errors() {
        let config = config_with(
            "    memo_for:\n      type: string\n      required: true\n    memo_from:\n      type: string\n      required: true",
            "",
        );
        let errors = validate_document(&config, &HashMap::new()).unwrap_err();
        let missing_paths: Vec<&str> = errors
            .iter()
            .filter_map(|e| match e {
                ValidationError::MissingRequired { path } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        assert!(missing_paths.contains(&"memo_for"));
        assert!(missing_paths.contains(&"memo_from"));
    }

    #[test]
    fn validates_card_with_valid_discriminator() {
        let config = config_with(
            "    title:\n      type: string",
            "cards:\n  indorsement:\n    fields:\n      signature_block:\n        type: string\n        required: true",
        );
        let doc = fields(&[(
            "CARDS",
            json!([{ "CARD": "indorsement", "signature_block": "Signed" }]),
        )]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn rejects_unknown_card_discriminator() {
        let config = config_with(
            "    title:\n      type: string",
            "cards:\n  indorsement:\n    fields:\n      signature_block:\n        type: string",
        );
        let doc = fields(&[("CARDS", json!([{ "CARD": "unknown" }]))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| {
            matches!(e, ValidationError::UnknownCard { path, card } if path == "cards[0]" && card == "unknown")
        }));
    }

    #[test]
    fn reports_missing_card_discriminator() {
        let config = config_with(
            "    title:\n      type: string",
            "cards:\n  indorsement:\n    fields:\n      signature_block:\n        type: string",
        );
        let doc = fields(&[("CARDS", json!([{ "signature_block": "Signed" }]))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| {
            matches!(e, ValidationError::MissingCardDiscriminator { path } if path == "cards[0]")
        }));
    }

    #[test]
    fn validates_multiple_card_instances_same_type() {
        let config = config_with(
            "    title:\n      type: string",
            "cards:\n  indorsement:\n    fields:\n      signature_block:\n        type: string\n        required: true",
        );
        let doc = fields(&[(
            "CARDS",
            json!([
                { "CARD": "indorsement", "signature_block": "A" },
                { "CARD": "indorsement", "signature_block": "B" }
            ]),
        )]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn validates_multiple_card_types_mixed() {
        let config = config_with(
            "    title:\n      type: string",
            "cards:\n  indorsement:\n    fields:\n      signature_block:\n        type: string\n        required: true\n  routing:\n    fields:\n      office:\n        type: string\n        required: true",
        );
        let doc = fields(&[(
            "CARDS",
            json!([
                { "CARD": "indorsement", "signature_block": "A" },
                { "CARD": "routing", "office": "HQ" }
            ]),
        )]);
        assert!(validate_document(&config, &doc).is_ok());
    }

    #[test]
    fn reports_card_field_paths_with_card_name_and_index() {
        let config = config_with(
            "    title:\n      type: string",
            "cards:\n  indorsement:\n    fields:\n      signature_block:\n        type: string\n        required: true",
        );
        let doc = fields(&[("CARDS", json!([{ "CARD": "indorsement" }]))]);
        let errors = validate_document(&config, &doc).unwrap_err();
        assert!(has_error(&errors, |e| {
            matches!(e, ValidationError::MissingRequired { path } if path == "cards.indorsement[0].signature_block")
        }));
    }
}
