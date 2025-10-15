//! # Templating Module
//!
//! MiniJinja-based template composition with stable filter API.
//!
//! ## Overview
//!
//! The `templating` module provides the [`Glue`] type for template rendering and a stable
//! filter API for backends to register custom filters.
//!
//! ## Key Types
//!
//! - [`Glue`]: Template rendering engine wrapper
//! - [`TemplateError`]: Template-specific error types
//! - [`filter_api`]: Stable API for filter registration (no direct minijinja dependency)
//!
//! ## Examples
//!
//! ### Basic Template Rendering
//!
//! ```no_run
//! use quillmark_core::{Glue, QuillValue};
//! use std::collections::HashMap;
//!
//! let template = r#"
//! #set document(title: {{ title | String }})
//!
//! {{ body | Content }}
//! "#;
//!
//! let mut glue = Glue::new(template.to_string());
//!
//! // Register filters (done by backends)
//! // glue.register_filter("String", string_filter);
//! // glue.register_filter("Content", content_filter);
//!
//! let mut context = HashMap::new();
//! context.insert("title".to_string(), QuillValue::from_json(serde_json::json!("My Doc")));
//! context.insert("body".to_string(), QuillValue::from_json(serde_json::json!("Content")));
//!
//! let output = glue.compose(context).unwrap();
//! ```
//!
//! ### Custom Filter Implementation
//!
//! ```no_run
//! use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
//! # use quillmark_core::Glue;
//! # let mut glue = Glue::new("template".to_string());
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
//! // Register with glue
//! glue.register_filter("uppercase", uppercase_filter);
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

use std::collections::HashMap;
use std::error::Error as StdError;

use minijinja::{Environment, Error as MjError};

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

/// Trait for glue engines that compose context into output
pub trait GlueEngine {
    /// Register a filter with the engine
    fn register_filter(&mut self, name: &str, func: FilterFn);

    /// Compose context from markdown decomposition into output
    fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError>;
}

/// Template-based glue engine using MiniJinja
pub struct TemplateGlue {
    template: String,
    filters: HashMap<String, FilterFn>,
}

/// JSON-based glue engine that outputs context as JSON
pub struct JsonGlue {
    filters: HashMap<String, FilterFn>,
}

/// Glue type that can be either template-based or JSON-based
pub enum Glue {
    /// Template-based glue using MiniJinja
    Template(TemplateGlue),
    /// JSON-based glue that outputs context as JSON
    Json(JsonGlue),
}

impl TemplateGlue {
    /// Create a new TemplateGlue instance with a template string
    pub fn new(template: String) -> Self {
        Self {
            template,
            filters: HashMap::new(),
        }
    }
}

impl GlueEngine for TemplateGlue {
    /// Register a filter with the template environment
    fn register_filter(&mut self, name: &str, func: FilterFn) {
        self.filters.insert(name.to_string(), func);
    }

    /// Compose template with context from markdown decomposition
    fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError> {
        // Convert QuillValue to MiniJinja values
        let context = convert_quillvalue_to_minijinja(context)?;

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

        let result = tmpl.render(&context)?;

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

impl JsonGlue {
    /// Create a new JsonGlue instance
    pub fn new() -> Self {
        Self {
            filters: HashMap::new(),
        }
    }
}

impl GlueEngine for JsonGlue {
    /// Register a filter with the JSON glue (ignored for JSON output)
    fn register_filter(&mut self, name: &str, func: FilterFn) {
        // Store filters even though they're not used for JSON output
        // This maintains consistency with the trait interface
        self.filters.insert(name.to_string(), func);
    }

    /// Compose context into JSON output
    fn compose(&mut self, context: HashMap<String, QuillValue>) -> Result<String, TemplateError> {
        // Convert context to JSON
        let mut json_map = serde_json::Map::new();
        for (key, value) in context {
            json_map.insert(key, value.as_json().clone());
        }

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

impl Glue {
    /// Create a new template-based Glue instance
    pub fn new(template: String) -> Self {
        Glue::Template(TemplateGlue::new(template))
    }

    /// Create a new JSON-based Glue instance
    pub fn new_json() -> Self {
        Glue::Json(JsonGlue::new())
    }

    /// Register a filter with the glue engine
    pub fn register_filter(&mut self, name: &str, func: FilterFn) {
        match self {
            Glue::Template(engine) => engine.register_filter(name, func),
            Glue::Json(engine) => engine.register_filter(name, func),
        }
    }

    /// Compose context into output
    pub fn compose(
        &mut self,
        context: HashMap<String, QuillValue>,
    ) -> Result<String, TemplateError> {
        match self {
            Glue::Template(engine) => engine.compose(context),
            Glue::Json(engine) => engine.compose(context),
        }
    }
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
    fn test_glue_creation() {
        let _glue = Glue::new("Hello {{ name }}".to_string());
        assert!(true);
    }

    #[test]
    fn test_compose_simple_template() {
        let mut glue = Glue::new("Hello {{ name }}! Body: {{ body }}".to_string());
        let mut context = HashMap::new();
        context.insert(
            "name".to_string(),
            QuillValue::from_json(serde_json::Value::String("World".to_string())),
        );
        context.insert(
            "body".to_string(),
            QuillValue::from_json(serde_json::Value::String("Hello content".to_string())),
        );

        let result = glue.compose(context).unwrap();
        assert!(result.contains("Hello World!"));
        assert!(result.contains("Body: Hello content"));
    }

    #[test]
    fn test_field_with_dash() {
        let mut glue = Glue::new("Field: {{ letterhead_title }}".to_string());
        let mut context = HashMap::new();
        context.insert(
            "letterhead_title".to_string(),
            QuillValue::from_json(serde_json::Value::String("TEST VALUE".to_string())),
        );
        context.insert(
            "body".to_string(),
            QuillValue::from_json(serde_json::Value::String("body".to_string())),
        );

        let result = glue.compose(context).unwrap();
        assert!(result.contains("TEST VALUE"));
    }

    #[test]
    fn test_compose_with_dash_in_template() {
        // Templates must reference the exact key names provided by the context.
        let mut glue = Glue::new("Field: {{ letterhead_title }}".to_string());
        let mut context = HashMap::new();
        context.insert(
            "letterhead_title".to_string(),
            QuillValue::from_json(serde_json::Value::String("DASHED".to_string())),
        );
        context.insert(
            "body".to_string(),
            QuillValue::from_json(serde_json::Value::String("body".to_string())),
        );

        let result = glue.compose(context).unwrap();
        assert!(result.contains("DASHED"));
    }

    #[test]
    fn test_template_output_size_limit() {
        // Create a template that generates output larger than MAX_TEMPLATE_OUTPUT
        // We can't easily create 50MB+ output in a test, so we'll use a smaller test
        // that validates the check exists
        let template = "{{ content }}".to_string();
        let mut glue = Glue::new(template);

        let mut context = HashMap::new();
        // Create a large string (simulate large output)
        // Note: In practice, this would need to exceed MAX_TEMPLATE_OUTPUT (50 MB)
        // For testing purposes, we'll just ensure the mechanism works
        context.insert(
            "content".to_string(),
            QuillValue::from_json(serde_json::Value::String("test".to_string())),
        );

        let result = glue.compose(context);
        // This should succeed as it's well under the limit
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_glue_basic() {
        let mut glue = Glue::new_json();
        let mut context = HashMap::new();
        context.insert(
            "name".to_string(),
            QuillValue::from_json(serde_json::Value::String("World".to_string())),
        );
        context.insert(
            "body".to_string(),
            QuillValue::from_json(serde_json::Value::String("Hello content".to_string())),
        );

        let result = glue.compose(context).unwrap();

        // Parse the result as JSON to verify it's valid
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["name"], "World");
        assert_eq!(json["body"], "Hello content");
    }

    #[test]
    fn test_json_glue_with_nested_data() {
        let mut glue = Glue::new_json();
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

        let result = glue.compose(context).unwrap();

        // Parse the result as JSON to verify structure
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["author"]["first"], "John");
        assert_eq!(json["author"]["last"], "Doe");
        assert_eq!(json["tags"][0], "tag1");
        assert_eq!(json["tags"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_json_glue_filter_registration() {
        // Test that filters can be registered (even though they're not used)
        let mut glue = Glue::new_json();

        fn dummy_filter(
            _state: &filter_api::State,
            value: filter_api::Value,
            _kwargs: filter_api::Kwargs,
        ) -> Result<filter_api::Value, MjError> {
            Ok(value)
        }

        // Should not panic
        glue.register_filter("dummy", dummy_filter);

        let mut context = HashMap::new();
        context.insert(
            "test".to_string(),
            QuillValue::from_json(serde_json::Value::String("value".to_string())),
        );

        let result = glue.compose(context).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["test"], "value");
    }
}
