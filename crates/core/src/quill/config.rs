//! Quill configuration parsing and normalization.
use std::collections::HashMap;
use std::error::Error as StdError;

use serde::{Deserialize, Serialize};

use crate::error::{Diagnostic, Severity};
use crate::value::QuillValue;

use super::{CardSchema, FieldSchema, UiContainerSchema, UiFieldSchema};

/// Top-level configuration for a Quillmark project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuillConfig {
    /// Quill package name
    pub name: String,
    /// Ordered card schemas where index 0 is always the main card.
    pub cards: Vec<CardSchema>,
    /// Backend to use for rendering (e.g., "typst", "html")
    pub backend: String,
    /// Version of the Quillmark spec
    pub version: String,
    /// Author of the project
    pub author: String,
    /// Example data file for preview
    pub example_file: Option<String>,
    /// Plate file (template)
    pub plate_file: Option<String>,
    /// Additional unstructured metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, QuillValue>,
    /// Typst specific configuration
    #[serde(default)]
    pub typst_config: HashMap<String, QuillValue>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CardSchemaDef {
    pub title: Option<String>,
    pub description: Option<String>,
    pub fields: Option<serde_json::Map<String, serde_json::Value>>,
    pub ui: Option<UiContainerSchema>,
}

impl QuillConfig {
    /// Returns the main document card schema (`cards[0]`).
    pub fn main(&self) -> &CardSchema {
        &self.cards[0]
    }

    /// Returns all named card definitions (everything except main).
    pub fn card_definitions(&self) -> &[CardSchema] {
        &self.cards[1..]
    }

    /// Returns named card definitions as a map keyed by card name.
    pub fn card_definitions_map(&self) -> HashMap<String, CardSchema> {
        self.card_definitions()
            .iter()
            .map(|card| (card.name.clone(), card.clone()))
            .collect()
    }

    /// Returns a named card definition by name.
    pub fn card_definition(&self, name: &str) -> Option<&CardSchema> {
        self.card_definitions()
            .iter()
            .find(|card| card.name == name)
    }

    /// Extract default values from the main card's field schemas.
    ///
    /// Returns a HashMap of field names to their default QuillValues,
    /// reading directly from `FieldSchema.default` without round-tripping
    /// through JSON Schema.
    pub fn extract_defaults(&self) -> HashMap<String, QuillValue> {
        let mut defaults = HashMap::new();
        for (field_name, field_schema) in &self.main().fields {
            if let Some(ref default_value) = field_schema.default {
                defaults.insert(field_name.clone(), default_value.clone());
            }
        }
        defaults
    }

    /// Extract example values from the main card's field schemas.
    ///
    /// Returns a HashMap of field names to their example values,
    /// reading directly from `FieldSchema.examples` without round-tripping
    /// through JSON Schema.
    pub fn extract_examples(&self) -> HashMap<String, Vec<QuillValue>> {
        let mut examples = HashMap::new();
        for (field_name, field_schema) in &self.main().fields {
            if let Some(ref examples_value) = field_schema.examples {
                if let Some(examples_array) = examples_value.as_array() {
                    let examples_vec: Vec<QuillValue> = examples_array
                        .iter()
                        .map(|v| QuillValue::from_json(v.clone()))
                        .collect();
                    if !examples_vec.is_empty() {
                        examples.insert(field_name.clone(), examples_vec);
                    }
                }
            }
        }
        examples
    }

    /// Coerce document fields to match expected schema types, reading types
    /// directly from `FieldSchema` without round-tripping through JSON Schema.
    ///
    /// Handles both main card fields (top-level) and card instances in the
    /// `CARDS` array.
    pub fn coerce_fields(
        &self,
        fields: &HashMap<String, QuillValue>,
    ) -> HashMap<String, QuillValue> {
        let mut coerced = HashMap::new();

        for (field_name, field_value) in fields {
            if let Some(field_schema) = self.main().fields.get(field_name) {
                coerced.insert(
                    field_name.clone(),
                    Self::coerce_value(field_value, field_schema),
                );
            } else {
                coerced.insert(field_name.clone(), field_value.clone());
            }
        }

        // Coerce card instances in CARDS array
        if let Some(cards_value) = coerced.get("CARDS") {
            if let Some(cards_array) = cards_value.as_array() {
                let coerced_cards = self.coerce_cards_array(cards_array);
                coerced.insert(
                    "CARDS".to_string(),
                    QuillValue::from_json(serde_json::Value::Array(coerced_cards)),
                );
            }
        }

        coerced
    }

    fn coerce_cards_array(&self, cards_array: &[serde_json::Value]) -> Vec<serde_json::Value> {
        let mut coerced_cards = Vec::new();

        for card in cards_array {
            if let Some(card_obj) = card.as_object() {
                if let Some(card_type) = card_obj.get("CARD").and_then(|v| v.as_str()) {
                    if let Some(card_schema) = self.card_definition(card_type) {
                        let mut coerced_card = serde_json::Map::new();
                        for (k, v) in card_obj {
                            if let Some(field_schema) = card_schema.fields.get(k) {
                                let qv = QuillValue::from_json(v.clone());
                                coerced_card.insert(
                                    k.clone(),
                                    Self::coerce_value(&qv, field_schema).into_json(),
                                );
                            } else {
                                coerced_card.insert(k.clone(), v.clone());
                            }
                        }
                        coerced_cards.push(serde_json::Value::Object(coerced_card));
                        continue;
                    }
                }
            }
            coerced_cards.push(card.clone());
        }

        coerced_cards
    }

    fn coerce_value(value: &QuillValue, field_schema: &super::FieldSchema) -> QuillValue {
        use super::FieldType;

        let json_value = value.as_json();
        match field_schema.r#type {
            FieldType::Array => {
                // Ensure value is an array
                let arr = if let Some(a) = json_value.as_array() {
                    a.clone()
                } else {
                    vec![json_value.clone()]
                };

                // If items has properties, recursively coerce each element's properties
                let coerced_arr = if let Some(items_schema) = &field_schema.items {
                    if let Some(props) = &items_schema.properties {
                        arr.into_iter()
                            .map(|elem| {
                                if let Some(obj) = elem.as_object() {
                                    let mut coerced_elem = serde_json::Map::new();
                                    for (k, v) in obj {
                                        if let Some(prop_schema) = props.get(k) {
                                            let qv = QuillValue::from_json(v.clone());
                                            coerced_elem.insert(
                                                k.clone(),
                                                Self::coerce_value(&qv, prop_schema).into_json(),
                                            );
                                        } else {
                                            coerced_elem.insert(k.clone(), v.clone());
                                        }
                                    }
                                    serde_json::Value::Object(coerced_elem)
                                } else {
                                    elem
                                }
                            })
                            .collect()
                    } else {
                        arr
                    }
                } else {
                    arr
                };

                QuillValue::from_json(serde_json::Value::Array(coerced_arr))
            }
            FieldType::Boolean => {
                if let Some(b) = json_value.as_bool() {
                    return QuillValue::from_json(serde_json::Value::Bool(b));
                }
                if let Some(s) = json_value.as_str() {
                    let lower = s.to_lowercase();
                    if lower == "true" {
                        return QuillValue::from_json(serde_json::Value::Bool(true));
                    } else if lower == "false" {
                        return QuillValue::from_json(serde_json::Value::Bool(false));
                    }
                }
                if let Some(n) = json_value.as_i64() {
                    return QuillValue::from_json(serde_json::Value::Bool(n != 0));
                }
                if let Some(n) = json_value.as_f64() {
                    if n.is_nan() {
                        return QuillValue::from_json(serde_json::Value::Bool(false));
                    }
                    return QuillValue::from_json(serde_json::Value::Bool(n.abs() > f64::EPSILON));
                }
                value.clone()
            }
            FieldType::Number => {
                if json_value.is_number() {
                    return value.clone();
                }
                if let Some(s) = json_value.as_str() {
                    if let Ok(i) = s.parse::<i64>() {
                        return QuillValue::from_json(serde_json::Number::from(i).into());
                    }
                    if let Ok(f) = s.parse::<f64>() {
                        if let Some(num) = serde_json::Number::from_f64(f) {
                            return QuillValue::from_json(num.into());
                        }
                    }
                }
                if let Some(b) = json_value.as_bool() {
                    let n = if b { 1 } else { 0 };
                    return QuillValue::from_json(serde_json::Value::Number(
                        serde_json::Number::from(n),
                    ));
                }
                value.clone()
            }
            FieldType::String | FieldType::Date | FieldType::DateTime | FieldType::Markdown => {
                if json_value.is_string() {
                    return value.clone();
                }
                if let Some(arr) = json_value.as_array() {
                    if arr.len() == 1 {
                        if let Some(s) = arr[0].as_str() {
                            return QuillValue::from_json(serde_json::Value::String(s.to_string()));
                        }
                    }
                }
                value.clone()
            }
            FieldType::Object => value.clone(),
        }
    }

    /// Parse fields from a JSON Value map, assigning ui.order based on key_order.
    ///
    /// This helper ensures consistent field ordering logic for both top-level
    /// fields and card fields.
    ///
    /// # Arguments
    /// * `fields_map` - The JSON map containing field definitions
    /// * `key_order` - Vector of field names in their definition order
    /// * `context` - Context string for error messages (e.g., "field" or "card 'indorsement' field")
    fn parse_fields_with_order(
        fields_map: &serde_json::Map<String, serde_json::Value>,
        key_order: &[String],
        context: &str,
        warnings: &mut Vec<Diagnostic>,
    ) -> HashMap<String, FieldSchema> {
        let mut fields = HashMap::new();
        let mut fallback_counter = 0;

        for (field_name, field_value) in fields_map {
            // Determine order from key_order, or use fallback counter
            let order = if let Some(idx) = key_order.iter().position(|k| k == field_name) {
                idx as i32
            } else {
                let o = key_order.len() as i32 + fallback_counter;
                fallback_counter += 1;
                o
            };

            let quill_value = QuillValue::from_json(field_value.clone());
            match FieldSchema::from_quill_value(field_name.clone(), &quill_value) {
                Ok(mut schema) => {
                    // Reject standalone object/dict fields — object is only valid inside array items.
                    if schema.r#type == super::FieldType::Object {
                        warnings.push(
                            Diagnostic::new(
                                Severity::Warning,
                                format!(
                                    "Field '{}' uses standalone type: object, which is not supported. \
                                    Use separate fields with ui.group instead, or use type: array with items: {{type: object, properties: {{...}}}}.",
                                    field_name
                                ),
                            )
                            .with_code("quill::standalone_object_not_supported".to_string()),
                        );
                        continue;
                    }

                    // Always set ui.order based on position
                    if schema.ui.is_none() {
                        schema.ui = Some(UiFieldSchema {
                            group: None,
                            order: Some(order),
                            visible_when: None,
                            compact: None,
                            multiline: None,
                        });
                    } else if let Some(ui) = &mut schema.ui {
                        // Only set if not already set
                        if ui.order.is_none() {
                            ui.order = Some(order);
                        }
                    }

                    fields.insert(field_name.clone(), schema);
                }
                Err(e) => {
                    warnings.push(
                        Diagnostic::new(
                            Severity::Warning,
                            format!("Failed to parse {} '{}': {}", context, field_name, e),
                        )
                        .with_code("quill::field_parse_warning".to_string()),
                    );
                }
            }
        }

        fields
    }

    /// Parse QuillConfig from YAML content
    pub fn from_yaml(yaml_content: &str) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let (config, _warnings) = Self::from_yaml_with_warnings(yaml_content)?;
        Ok(config)
    }

    /// Parse QuillConfig from YAML content while collecting non-fatal warnings.
    pub fn from_yaml_with_warnings(
        yaml_content: &str,
    ) -> Result<(Self, Vec<Diagnostic>), Box<dyn StdError + Send + Sync>> {
        let mut warnings = Vec::new();

        // Parse YAML into serde_json::Value via serde_saphyr
        // Note: serde_json with "preserve_order" feature is required for this to work as expected
        let quill_yaml_val: serde_json::Value = serde_saphyr::from_str(yaml_content)
            .map_err(|e| format!("Failed to parse Quill.yaml: {}", e))?;

        // Extract [Quill] section (required)
        let quill_section = quill_yaml_val
            .get("Quill")
            .ok_or("Missing required 'Quill' section in Quill.yaml")?;

        // Extract required fields
        let name = quill_section
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required 'name' field in 'Quill' section")?
            .to_string();

        let backend = quill_section
            .get("backend")
            .and_then(|v| v.as_str())
            .ok_or("Missing required 'backend' field in 'Quill' section")?
            .to_string();

        let description = quill_section
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or("Missing required 'description' field in 'Quill' section")?;

        if description.trim().is_empty() {
            return Err("'description' field in 'Quill' section cannot be empty".into());
        }
        let description = description.to_string();

        // Extract optional fields (now version is required)
        let version_val = quill_section
            .get("version")
            .ok_or("Missing required 'version' field in 'Quill' section")?;

        // Handle version as string or number (YAML might parse 1.0 as number)
        let version = if let Some(s) = version_val.as_str() {
            s.to_string()
        } else if let Some(n) = version_val.as_f64() {
            n.to_string()
        } else {
            return Err("Invalid 'version' field format".into());
        };

        // Validate version format (semver: MAJOR.MINOR.PATCH or MAJOR.MINOR)
        use std::str::FromStr;
        crate::version::Version::from_str(&version)
            .map_err(|e| format!("Invalid version '{}': {}", version, e))?;

        let author = quill_section
            .get("author")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string()); // Default author

        let example_file = quill_section
            .get("example_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let plate_file = quill_section
            .get("plate_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let ui_section: Option<UiContainerSchema> = quill_section
            .get("ui")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok());

        // Extract additional metadata from [Quill] section (excluding standard fields)
        let mut metadata = HashMap::new();
        if let Some(table) = quill_section.as_object() {
            for (key, value) in table {
                // Skip standard fields that are stored in dedicated struct fields
                if key != "name"
                    && key != "backend"
                    && key != "description"
                    && key != "version"
                    && key != "author"
                    && key != "example_file"
                    && key != "plate_file"
                    && key != "ui"
                {
                    metadata.insert(key.clone(), QuillValue::from_json(value.clone()));
                }
            }
        }

        // Extract [typst] section (optional)
        let mut typst_config = HashMap::new();
        if let Some(typst_val) = quill_yaml_val.get("typst") {
            if let Some(table) = typst_val.as_object() {
                for (key, value) in table {
                    typst_config.insert(key.clone(), QuillValue::from_json(value.clone()));
                }
            }
        }

        let main_obj_opt = quill_yaml_val.get("main").and_then(|v| v.as_object());

        if quill_yaml_val.get("fields").is_some() {
            return Err("Root-level `fields` is not supported; use `main.fields` instead.".into());
        }

        // Extract main.fields (optional)
        let fields = if let Some(main_obj) = main_obj_opt {
            if let Some(fields_val) = main_obj.get("fields") {
                if let Some(fields_map) = fields_val.as_object() {
                    // With preserve_order feature, keys iterator respects insertion order
                    let field_order: Vec<String> = fields_map.keys().cloned().collect();
                    Self::parse_fields_with_order(
                        fields_map,
                        &field_order,
                        "field schema",
                        &mut warnings,
                    )
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        // Extract main.ui (optional)
        let main_ui: Option<UiContainerSchema> = main_obj_opt
            .and_then(|main_obj| main_obj.get("ui"))
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok());

        // Main card is always first.
        let mut cards: Vec<CardSchema> = vec![CardSchema {
            name: "main".to_string(),
            title: Some("main".to_string()),
            description: Some(description),
            fields,
            ui: main_ui.or(ui_section),
        }];

        // Extract [cards] section (optional)
        if let Some(cards_val) = quill_yaml_val.get("cards") {
            let cards_table = cards_val
                .as_object()
                .ok_or("'cards' section must be an object")?;

            for (card_name, card_value) in cards_table {
                // Parse card basic info using serde
                let card_def: CardSchemaDef = serde_json::from_value(card_value.clone())
                    .map_err(|e| format!("Failed to parse card '{}': {}", card_name, e))?;

                // Parse card fields
                let card_fields = if let Some(card_fields_table) =
                    card_value.get("fields").and_then(|v| v.as_object())
                {
                    let card_field_order: Vec<String> = card_fields_table.keys().cloned().collect();

                    Self::parse_fields_with_order(
                        card_fields_table,
                        &card_field_order,
                        &format!("card '{}' field", card_name),
                        &mut warnings,
                    )
                } else if let Some(_toml_fields) = &card_def.fields {
                    HashMap::new()
                } else {
                    HashMap::new()
                };

                let card_schema = CardSchema {
                    name: card_name.clone(),
                    title: card_def.title,
                    description: card_def.description,
                    fields: card_fields,
                    ui: card_def.ui,
                };

                cards.push(card_schema);
            }
        }

        Ok((
            QuillConfig {
                name,
                cards,
                backend,
                version,
                author,
                example_file,
                plate_file,
                metadata,
                typst_config,
            },
            warnings,
        ))
    }
}
