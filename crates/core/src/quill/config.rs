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
    /// The root document schema
    pub document: CardSchema,
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
    /// Card definitions (reusable sub-schemas)
    pub cards: HashMap<String, CardSchema>,
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
                    // Always set ui.order based on position
                    if schema.ui.is_none() {
                        schema.ui = Some(UiFieldSchema {
                            group: None,
                            order: Some(order),
                            visible_when: None,
                            compact: None,
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

        // Extract [fields] section (optional) using shared helper
        let fields = if let Some(fields_val) = quill_yaml_val.get("fields") {
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
        };

        // Extract [cards] section (optional)
        let mut cards: HashMap<String, CardSchema> = HashMap::new();
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

                cards.insert(card_name.clone(), card_schema);
            }
        }

        // Create document schema from root fields
        let document = CardSchema {
            name: name.clone(),
            title: Some(name),
            description: Some(description),
            fields,
            ui: ui_section,
        };

        Ok((
            QuillConfig {
                document,
                backend,
                version,
                author,
                example_file,
                plate_file,
                cards,
                metadata,
                typst_config,
            },
            warnings,
        ))
    }
}
