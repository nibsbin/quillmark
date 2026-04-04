//! Quill schema and core type definitions.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::value::QuillValue;
/// Semantic constants for field schema keys used in parsing and JSON Schema generation.
/// Using constants provides IDE support (find references, autocomplete) and ensures
/// consistency between parsing and output.
pub mod field_key {
    /// Short label for the field
    pub const TITLE: &str = "title";
    /// Field type (string, number, boolean, array, etc.)
    pub const TYPE: &str = "type";
    /// Detailed field description
    pub const DESCRIPTION: &str = "description";
    /// Default value for the field
    pub const DEFAULT: &str = "default";
    /// Example values for the field
    pub const EXAMPLES: &str = "examples";
    /// UI-specific metadata
    pub const UI: &str = "ui";
    /// Whether the field is required
    pub const REQUIRED: &str = "required";
    /// Enum values for string fields
    pub const ENUM: &str = "enum";
    /// Date format specifier (JSON Schema)
    pub const FORMAT: &str = "format";
}

/// Semantic constants for UI schema keys
pub mod ui_key {
    /// Group name for field organization
    pub const GROUP: &str = "group";
    /// Display order within the UI
    pub const ORDER: &str = "order";
    /// Whether the field or specific component is hide-body (no body editor)
    pub const HIDE_BODY: &str = "hide_body";
    /// Compact rendering hint for UI consumers
    pub const COMPACT: &str = "compact";
    /// Multi-line text box hint for string and markdown fields
    pub const MULTILINE: &str = "multiline";
}

/// UI-specific metadata for field rendering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiFieldSchema {
    /// Group name for organizing fields (e.g., "Personal Info", "Preferences")
    pub group: Option<String>,
    /// Order of the field in the UI (automatically generated based on field position in Quill.yaml)
    pub order: Option<i32>,
    /// Compact rendering hint: when true, the UI should render this field in a compact style
    pub compact: Option<bool>,
    /// Multi-line text box hint: when true, the UI should start with a larger text box.
    /// Valid on `string` fields (plain text with newlines preserved) and `markdown` fields.
    pub multiline: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiContainerSchema {
    /// Whether to hide the body editor for this element (metadata only)
    pub hide_body: Option<bool>,
}

/// Schema definition for a card type (composable content blocks)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardSchema {
    /// Card type name (e.g., "indorsements")
    pub name: String,
    /// Short label for the card type
    pub title: Option<String>,
    /// Detailed description of this card type
    pub description: Option<String>,
    /// List of fields in the card
    pub fields: HashMap<String, FieldSchema>,
    /// UI layout hints
    pub ui: Option<UiContainerSchema>,
}

/// Field type hint enum for type-safe field type definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// String type
    #[serde(alias = "str")]
    String,
    /// Numeric type
    Number,
    /// Boolean type
    Boolean,
    /// Array type
    Array,
    /// Dictionary/object type
    Object,
    /// Date type (formatted as string with date format)
    Date,
    /// DateTime type (formatted as string with date-time format)
    DateTime,
    /// Markdown type (string with markdown content, contentMediaType: text/markdown)
    Markdown,
}

impl FieldType {
    /// Parse a FieldType from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "string" | "str" => Some(FieldType::String),
            "number" => Some(FieldType::Number),
            "boolean" => Some(FieldType::Boolean),
            "array" => Some(FieldType::Array),
            "object" | "dict" => Some(FieldType::Object),
            "date" => Some(FieldType::Date),
            "datetime" => Some(FieldType::DateTime),
            "markdown" => Some(FieldType::Markdown),
            _ => None,
        }
    }

    /// Get the canonical string representation for this type
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Number => "number",
            FieldType::Boolean => "boolean",
            FieldType::Array => "array",
            FieldType::Object => "dict",
            FieldType::Date => "date",
            FieldType::DateTime => "datetime",
            FieldType::Markdown => "markdown",
        }
    }
}

/// Schema definition for a template field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    /// Short label for the field (used in JSON Schema title)
    pub title: Option<String>,
    /// Field type (required)
    pub r#type: FieldType,
    /// Detailed description of the field (used in JSON Schema description)
    pub description: Option<String>,
    /// Default value for the field
    pub default: Option<QuillValue>,
    /// Example values for the field
    pub examples: Option<QuillValue>,
    /// UI layout hints
    pub ui: Option<UiFieldSchema>,
    /// Whether this field is required (fields are optional by default)
    pub required: bool,
    /// Enum values for string fields (restricts valid values)
    pub enum_values: Option<Vec<String>>,
    /// Properties for dict/object types (nested field schemas)
    pub properties: Option<HashMap<String, Box<FieldSchema>>>,
    /// Item schema for array types
    pub items: Option<Box<FieldSchema>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldSchemaDef {
    pub title: Option<String>,
    pub r#type: FieldType,
    pub description: Option<String>,
    pub default: Option<QuillValue>,
    pub examples: Option<QuillValue>,
    pub ui: Option<UiFieldSchema>,
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    // Nested schema support
    // Nested schema support
    pub properties: Option<serde_json::Map<String, serde_json::Value>>,
    pub items: Option<serde_json::Value>,
}

impl FieldSchema {
    /// Create a new FieldSchema with default values
    pub fn new(name: String, r#type: FieldType, description: Option<String>) -> Self {
        Self {
            name,
            title: None,
            r#type,
            description,
            default: None,
            examples: None,
            ui: None,
            required: false,
            enum_values: None,
            properties: None,
            items: None,
        }
    }

    /// Parse a FieldSchema from a QuillValue
    pub fn from_quill_value(key: String, value: &QuillValue) -> Result<Self, String> {
        let def: FieldSchemaDef = serde_json::from_value(value.clone().into_json())
            .map_err(|e| format!("Failed to parse field schema: {}", e))?;

        Ok(Self {
            name: key,
            title: def.title,
            r#type: def.r#type,
            description: def.description,
            default: def.default,
            examples: def.examples,
            ui: def.ui,
            required: def.required,
            enum_values: def.enum_values,
            properties: if let Some(props) = def.properties {
                let mut p = HashMap::new();
                for (key, value) in props {
                    p.insert(
                        key.clone(),
                        Box::new(FieldSchema::from_quill_value(
                            key,
                            &QuillValue::from_json(value),
                        )?),
                    );
                }
                Some(p)
            } else {
                None
            },
            items: if let Some(item_def) = def.items {
                Some(Box::new(FieldSchema::from_quill_value(
                    "items".to_string(),
                    &QuillValue::from_json(item_def),
                )?))
            } else {
                None
            },
        })
    }
}
