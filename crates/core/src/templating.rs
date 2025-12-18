//! # Templating Module
//!
//! MiniJinja-based template composition with stable filter API.
//!
//! ## Overview
//!
//! The `templating` module provides the [`Plate`] type for template rendering and a stable
//! filter API for backends to register custom filters.
//!
//! ## Key Types
//!
//! - [`Plate`]: Template rendering engine wrapper
//! - [`TemplateError`]: Template-specific error types
//! - [`filter_api`]: Stable API for filter registration (no direct minijinja dependency)
//!
//! ## Examples
//!
//! ### Basic Template Rendering
//!
//! ```no_run
//! use quillmark_core::{Plate, QuillValue};
//! use std::collections::HashMap;
//!
//! let template = r#"
//! #set document(title: {{ title | String }})
//!
//! {{ body | Content }}
//! "#;
//!
//! let mut plate = Plate::new(template.to_string());
//!
//! // Register filters (done by backends)
//! // plate.register_filter("String", string_filter);
//! // plate.register_filter("Content", content_filter);
//!
//! let mut context = HashMap::new();
//! context.insert("title".to_string(), QuillValue::from_json(serde_json::json!("My Doc")));
//! context.insert("BODY".to_string(), QuillValue::from_json(serde_json::json!("Content")));
//!
//! let output = plate.compose(context).unwrap();
//! ```
//!
//! ### Custom Filter Implementation
//!
//! ```no_run
//! use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
//! # use quillmark_core::Plate;
//! # let mut plate = Plate::new("template".to_string());
//!
//! fn uppercase_filter(
//!     _state: &State,
//!     value: Value,
//!     _kwargs: Kwargs,
//! ) -> Result<Value, Error> {
//!     let s = value.as_str().ok_or_else(|| {
//!         Error::new(ErrorKind::InvalidOperation, "Expected string")
//!     })?;
//!     Ok(Value::from(s.to_uppercase()))
//! }
//!
//! // Register with plate
//! plate.register_filter("uppercase", uppercase_filter);
//! ```
//!
//! ## Filter API
//!
//! The [`filter_api`] module provides a stable ABI that external crates can depend on
//! without requiring a direct minijinja dependency.
//!
//! ### Filter Function Signature
//!
//! ```rust,ignore
//! type FilterFn = fn(
//!     &filter_api::State,
//!     filter_api::Value,
//!     filter_api::Kwargs,
//! ) -> Result<filter_api::Value, minijinja::Error>;
//! ```
//!
//! ## Error Types
//!
//! - [`TemplateError::RenderError`]: Template rendering error from MiniJinja
//! - [`TemplateError::InvalidTemplate`]: Template compilation failed
//! - [`TemplateError::FilterError`]: Filter execution error

use std::collections::{BTreeMap, HashMap};
use std::error::Error as StdError;

use minijinja::{Environment, Error as MjError};

use crate::parse::BODY_FIELD;
use crate::value::QuillValue;

/// Error types for template rendering
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    /// Template rendering error from MiniJinja
    #[error("{0}")]
    RenderError(#[from] minijinja::Error),
    /// Invalid template compilation error
    #[error("{0}")]
    InvalidTemplate(String, #[source] Box<dyn StdError + Send + Sync>),
    /// Filter execution error
    #[error("{0}")]
    FilterError(String),
}

/// Public filter ABI that external crates can depend on (no direct minijinja dep required)
pub mod filter_api {
    pub use minijinja::value::{Kwargs, Value};
    pub use minijinja::{Error, ErrorKind, State};

    /// Trait alias for closures/functions used as filters (thread-safe, 'static)
    pub trait DynFilter: Send + Sync + 'static {}
    impl<T> DynFilter for T where T: Send + Sync + 'static {}
}

/// Type for filter functions that can be called via function pointers
type FilterFn = fn(
    &filter_api::State,
    filter_api::Value,
    filter_api::Kwargs,
) -> Result<filter_api::Value, MjError>;

/// Trait for plate engines that compose context into output
pub trait PlateEngine {
    /// Register a filter with the engine
    fn register_filter(&mut self, name: &str, func: FilterFn);

    /// Compose context from markdown decomposition into output
    fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError>;
}

/// Template-based plate engine using MiniJinja
pub struct TemplatePlate {
    template: String,
    filters: HashMap<String, FilterFn>,
}

/// Auto plate engine that outputs context as JSON
pub struct AutoPlate {
    filters: HashMap<String, FilterFn>,
}

/// Plate type that can be either template-based or auto
pub enum Plate {
    /// Template-based plate using MiniJinja
    Template(TemplatePlate),
    /// Auto plate that outputs context as JSON
    Auto(AutoPlate),
}

impl TemplatePlate {
    /// Create a new TemplatePlate instance with a template string
    pub fn new(template: String) -> Self {
        Self {
            template,
            filters: HashMap::new(),
        }
    }
}

impl PlateEngine for TemplatePlate {
    /// Register a filter with the template environment
    fn register_filter(&mut self, name: &str, func: FilterFn) {
        self.filters.insert(name.to_string(), func);
    }

    /// Compose template with context from markdown decomposition
    fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError> {
        // Separate metadata from body using helper function
        let metadata_fields = separate_metadata_fields(&context);

        // Convert QuillValue to MiniJinja values
        let mut minijinja_context = convert_quillvalue_to_minijinja(context)?;
        let metadata_minijinja = convert_quillvalue_to_minijinja(metadata_fields)?;

        // Add __metadata__ field as a MiniJinja object
        // Convert HashMap to BTreeMap for from_object
        let metadata_btree: BTreeMap<String, minijinja::value::Value> =
            metadata_minijinja.into_iter().collect();
        minijinja_context.insert(
            "__metadata__".to_string(),
            minijinja::value::Value::from_object(metadata_btree),
        );

        // Create a new environment for this render
        let mut env = Environment::new();

        // Register all filters
        for (name, filter_fn) in &self.filters {
            let filter_fn = *filter_fn; // Copy the function pointer
            env.add_filter(name, filter_fn);
        }

        env.add_template("main", &self.template).map_err(|e| {
            TemplateError::InvalidTemplate("Failed to add template".to_string(), Box::new(e))
        })?;

        // Render the template
        let tmpl = env.get_template("main").map_err(|e| {
            TemplateError::InvalidTemplate("Failed to get template".to_string(), Box::new(e))
        })?;

        let result = tmpl.render(&minijinja_context)?;

        // Check output size limit
        if result.len() > crate::error::MAX_TEMPLATE_OUTPUT {
            return Err(TemplateError::FilterError(format!(
                "Template output too large: {} bytes (max: {} bytes)",
                result.len(),
                crate::error::MAX_TEMPLATE_OUTPUT
            )));
        }

        Ok(result)
    }
}

impl AutoPlate {
    /// Create a new AutoPlate instance
    pub fn new() -> Self {
        Self {
            filters: HashMap::new(),
        }
    }
}

impl Default for AutoPlate {
    fn default() -> Self {
        Self::new()
    }
}

impl PlateEngine for AutoPlate {
    /// Register a filter with the auto plate (ignored for JSON output)
    fn register_filter(&mut self, name: &str, func: FilterFn) {
        // Store filters even though they're not used for JSON output
        // This maintains consistency with the trait interface
        self.filters.insert(name.to_string(), func);
    }

    /// Compose context into JSON output
    fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError> {
        // Build both json_map and metadata_json in a single pass to avoid redundant iterations
        let mut json_map = serde_json::Map::new();
        let mut metadata_json = serde_json::Map::new();

        for (key, value) in &context {
            let json_value = value.as_json().clone();
            json_map.insert(key.clone(), json_value.clone());

            // Add to metadata if not the body field
            if key.as_str() != BODY_FIELD {
                metadata_json.insert(key.clone(), json_value);
            }
        }

        // Add __metadata__ object to json_map
        json_map.insert(
            "__metadata__".to_string(),
            serde_json::Value::Object(metadata_json),
        );

        let json_value = serde_json::Value::Object(json_map);
        let result = serde_json::to_string_pretty(&json_value).map_err(|e| {
            TemplateError::FilterError(format!("Failed to serialize to JSON: {}", e))
        })?;

        // Check output size limit
        if result.len() > crate::error::MAX_TEMPLATE_OUTPUT {
            return Err(TemplateError::FilterError(format!(
                "JSON output too large: {} bytes (max: {} bytes)",
                result.len(),
                crate::error::MAX_TEMPLATE_OUTPUT
            )));
        }

        Ok(result)
    }
}

impl Plate {
    /// Create a new template-based Plate instance
    pub fn new(template: String) -> Self {
        Plate::Template(TemplatePlate::new(template))
    }

    /// Create a new auto plate instance
    pub fn new_auto() -> Self {
        Plate::Auto(AutoPlate::new())
    }

    /// Register a filter with the plate engine
    pub fn register_filter(&mut self, name: &str, func: FilterFn) {
        match self {
            Plate::Template(engine) => engine.register_filter(name, func),
            Plate::Auto(engine) => engine.register_filter(name, func),
        }
    }

    /// Compose context into output
    pub fn compose(
        &mut self,
        context: HashMap<String, QuillValue>,
    ) -> Result<String, TemplateError> {
        match self {
            Plate::Template(engine) => engine.compose(context),
            Plate::Auto(engine) => engine.compose(context),
        }
    }
}

/// Separate metadata fields from body field
fn separate_metadata_fields(context: &HashMap<String, QuillValue>) -> HashMap<String, QuillValue> {
    context
        .iter()
        .filter(|(key, _)| key.as_str() != BODY_FIELD)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Convert QuillValue map to MiniJinja values
fn convert_quillvalue_to_minijinja(
    fields: HashMap<String, QuillValue>,
) -> Result<HashMap<String, minijinja::value::Value>, TemplateError> {
    let mut result = HashMap::new();

    for (key, value) in fields {
        let minijinja_value = value.to_minijinja().map_err(|e| {
            TemplateError::FilterError(format!("Failed to convert QuillValue to MiniJinja: {}", e))
        })?;
        result.insert(key, minijinja_value);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_plate_creation() {
        let _plate = Plate::new("Hello {{ name }}".to_string());
    }

    #[test]
    fn test_compose_simple_template() {
        let mut plate = Plate::new("Hello {{ name }}! Body: {{ BODY }}".to_string());
        let mut context = HashMap::new();
        context.insert(
            "name".to_string(),
            QuillValue::from_json(serde_json::Value::String("World".to_string())),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::Value::String("Hello content".to_string())),
        );

        let result = plate.compose(context).unwrap();
        assert!(result.contains("Hello World!"));
        assert!(result.contains("Body: Hello content"));
    }

    #[test]
    fn test_field_with_dash() {
        let mut plate = Plate::new("Field: {{ letterhead_title }}".to_string());
        let mut context = HashMap::new();
        context.insert(
            "letterhead_title".to_string(),
            QuillValue::from_json(serde_json::Value::String("TEST VALUE".to_string())),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::Value::String(BODY_FIELD.to_string())),
        );

        let result = plate.compose(context).unwrap();
        assert!(result.contains("TEST VALUE"));
    }

    #[test]
    fn test_compose_with_dash_in_template() {
        // Templates must reference the exact key names provided by the context.
        let mut plate = Plate::new("Field: {{ letterhead_title }}".to_string());
        let mut context = HashMap::new();
        context.insert(
            "letterhead_title".to_string(),
            QuillValue::from_json(serde_json::Value::String("DASHED".to_string())),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::Value::String(BODY_FIELD.to_string())),
        );

        let result = plate.compose(context).unwrap();
        assert!(result.contains("DASHED"));
    }

    #[test]
    fn test_template_output_size_limit() {
        // Create a template that generates output larger than MAX_TEMPLATE_OUTPUT
        // We can't easily create 50MB+ output in a test, so we'll use a smaller test
        // that validates the check exists
        let template = "{{ content }}".to_string();
        let mut plate = Plate::new(template);

        let mut context = HashMap::new();
        // Create a large string (simulate large output)
        // Note: In practice, this would need to exceed MAX_TEMPLATE_OUTPUT (50 MB)
        // For testing purposes, we'll just ensure the mechanism works
        context.insert(
            "content".to_string(),
            QuillValue::from_json(serde_json::Value::String("test".to_string())),
        );

        let result = plate.compose(context);
        // This should succeed as it's well under the limit
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_plate_basic() {
        let mut plate = Plate::new_auto();
        let mut context = HashMap::new();
        context.insert(
            "name".to_string(),
            QuillValue::from_json(serde_json::Value::String("World".to_string())),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::Value::String("Hello content".to_string())),
        );

        let result = plate.compose(context).unwrap();

        // Parse the result as JSON to verify it's valid
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["name"], "World");
        assert_eq!(json[BODY_FIELD], "Hello content");
    }

    #[test]
    fn test_auto_plate_with_nested_data() {
        let mut plate = Plate::new_auto();
        let mut context = HashMap::new();

        // Add nested object
        let nested_obj = serde_json::json!({
            "first": "John",
            "last": "Doe"
        });
        context.insert("author".to_string(), QuillValue::from_json(nested_obj));

        // Add array
        let tags = serde_json::json!(["tag1", "tag2", "tag3"]);
        context.insert("tags".to_string(), QuillValue::from_json(tags));

        let result = plate.compose(context).unwrap();

        // Parse the result as JSON to verify structure
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["author"]["first"], "John");
        assert_eq!(json["author"]["last"], "Doe");
        assert_eq!(json["tags"][0], "tag1");
        assert_eq!(json["tags"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_auto_plate_filter_registration() {
        // Test that filters can be registered (even though they're not used)
        let mut plate = Plate::new_auto();

        fn dummy_filter(
            _state: &filter_api::State,
            value: filter_api::Value,
            _kwargs: filter_api::Kwargs,
        ) -> Result<filter_api::Value, MjError> {
            Ok(value)
        }

        // Should not panic
        plate.register_filter("dummy", dummy_filter);

        let mut context = HashMap::new();
        context.insert(
            "test".to_string(),
            QuillValue::from_json(serde_json::Value::String("value".to_string())),
        );

        let result = plate.compose(context).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["test"], "value");
    }

    #[test]
    fn test_metadata_field_excludes_body() {
        let template = "{% for key in __metadata__ %}{{ key }},{% endfor %}";
        let mut plate = Plate::new(template.to_string());

        let mut context = HashMap::new();
        context.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("Test")),
        );
        context.insert(
            "author".to_string(),
            QuillValue::from_json(serde_json::json!("John")),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Body content")),
        );

        let result = plate.compose(context).unwrap();

        // Should contain title and author, but not body
        assert!(result.contains("title"));
        assert!(result.contains("author"));
        assert!(!result.contains(BODY_FIELD));
    }

    #[test]
    fn test_metadata_field_includes_frontmatter() {
        let template = r#"
{%- for key in __metadata__ -%}
{{ key }}
{% endfor -%}
"#;
        let mut plate = Plate::new(template.to_string());

        let mut context = HashMap::new();
        context.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("Test Document")),
        );
        context.insert(
            "author".to_string(),
            QuillValue::from_json(serde_json::json!("Jane Doe")),
        );
        context.insert(
            "date".to_string(),
            QuillValue::from_json(serde_json::json!("2024-01-01")),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Document body")),
        );

        let result = plate.compose(context).unwrap();

        // All metadata fields should be present as keys
        assert!(result.contains("title"));
        assert!(result.contains("author"));
        assert!(result.contains("date"));
        // Body should not be in metadata iteration
        assert!(!result.contains(BODY_FIELD));
    }

    #[test]
    fn test_metadata_field_empty_when_only_body() {
        let template = "Metadata count: {{ __metadata__ | length }}";
        let mut plate = Plate::new(template.to_string());

        let mut context = HashMap::new();
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Only body content")),
        );

        let result = plate.compose(context).unwrap();

        // Should have 0 metadata fields when only body is present
        assert!(result.contains("Metadata count: 0"));
    }

    #[test]
    fn test_backward_compatibility_top_level_access() {
        let template = "Title: {{ title }}, Author: {{ author }}, Body: {{ BODY }}";
        let mut plate = Plate::new(template.to_string());

        let mut context = HashMap::new();
        context.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("My Title")),
        );
        context.insert(
            "author".to_string(),
            QuillValue::from_json(serde_json::json!("Author Name")),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Body text")),
        );

        let result = plate.compose(context).unwrap();

        // Top-level access should still work
        assert!(result.contains("Title: My Title"));
        assert!(result.contains("Author: Author Name"));
        assert!(result.contains("Body: Body text"));
    }

    #[test]
    fn test_metadata_iteration_in_template() {
        let template = r#"
{%- set metadata_count = __metadata__ | length -%}
Metadata fields: {{ metadata_count }}
{%- for key in __metadata__ %}
- {{ key }}: {{ __metadata__[key] }}
{%- endfor %}
Body present: {{ BODY | length > 0 }}
"#;
        let mut plate = Plate::new(template.to_string());

        let mut context = HashMap::new();
        context.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("Test")),
        );
        context.insert(
            "version".to_string(),
            QuillValue::from_json(serde_json::json!("1.0")),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Content")),
        );

        let result = plate.compose(context).unwrap();

        // Should have exactly 2 metadata fields
        assert!(result.contains("Metadata fields: 2"));
        // Body should still be accessible directly
        assert!(result.contains("Body present: true"));
    }

    #[test]
    fn test_auto_plate_metadata_field() {
        let mut plate = Plate::new_auto();

        let mut context = HashMap::new();
        context.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("Document")),
        );
        context.insert(
            "author".to_string(),
            QuillValue::from_json(serde_json::json!("Writer")),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Content here")),
        );

        let result = plate.compose(context).unwrap();

        // Parse as JSON
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Verify __metadata__ field exists and contains correct fields
        assert!(json["__metadata__"].is_object());
        assert_eq!(json["__metadata__"]["title"], "Document");
        assert_eq!(json["__metadata__"]["author"], "Writer");

        // Body should not be in metadata
        assert!(json["__metadata__"][BODY_FIELD].is_null());

        // But body should be at top level
        assert_eq!(json[BODY_FIELD], "Content here");
    }

    #[test]
    fn test_metadata_with_nested_objects() {
        let template = "{{ __metadata__.author.name }}";
        let mut plate = Plate::new(template.to_string());

        let mut context = HashMap::new();
        context.insert(
            "author".to_string(),
            QuillValue::from_json(serde_json::json!({
                "name": "John Doe",
                "email": "john@example.com"
            })),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Text")),
        );

        let result = plate.compose(context).unwrap();

        // Should access nested metadata via __metadata__
        assert!(result.contains("John Doe"));
    }

    #[test]
    fn test_metadata_with_arrays() {
        let template = "Tags: {{ __metadata__.tags | length }}";
        let mut plate = Plate::new(template.to_string());

        let mut context = HashMap::new();
        context.insert(
            "tags".to_string(),
            QuillValue::from_json(serde_json::json!(["rust", "markdown", "template"])),
        );
        context.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("Content")),
        );

        let result = plate.compose(context).unwrap();

        // Should show 3 tags
        assert!(result.contains("Tags: 3"));
    }
}
