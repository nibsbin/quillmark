use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error as StdError;

use minijinja::{Environment, Error as MjError};
use serde_yaml;

/// Error types for template rendering
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("{0}")]
    RenderError(#[from] minijinja::Error),
    #[error("{0}")]
    InvalidTemplate(String, #[source] Box<dyn StdError + Send + Sync>),
    #[error("{0}")]
    FilterError(String),
}

/// Public filter ABI that external crates can depend on (no direct minijinja dep required)
pub mod filter_api {
    pub use minijinja::{Error, ErrorKind, State};
    pub use minijinja::value::{Kwargs, Value};

    /// Trait alias for closures/functions used as filters (thread-safe, 'static)
    pub trait DynFilter: Send + Sync + 'static {}
    impl<T> DynFilter for T where T: Send + Sync + 'static {}
}

/// Glue class for template rendering - provides interface for backends to interact with templates
pub struct Glue {
    env: Environment<'static>,
    template: String,
}

impl Glue {
    /// Create a new Glue instance with a template string
    pub fn new(template: String) -> Self {
        let env = Environment::new();
        Self { env, template }
    }

    /// Register a MiniJinja-compatible filter function or closure.
    ///
    /// External crates should import `filter_api::{State, Value, Kwargs, Error}` and implement:
    /// `fn MyFilter(_s: &State, v: Value, k: Kwargs) -> Result<Value, Error>`
    /// and then call `glue.register_filter("My", MyFilter)`.
    ///
    /// Accepts either `&'static str` or `String` (owned) for the filter name.
    pub fn register_filter<N, F>(&mut self, name: N, filter: F)
    where
        N: Into<Cow<'static, str>>,
        // This bound matches what minijinja requires for add_filter (Fn(&State, Value, Kwargs) -> Result<Value, Error>)
        F: filter_api::DynFilter
            + Fn(&filter_api::State, filter_api::Value, filter_api::Kwargs)
                -> Result<filter_api::Value, MjError>,
    {
        self.env.add_filter(name, filter);
    }

    /// Compose template with context from markdown decomposition
    pub fn compose(
        &mut self,
        context: HashMap<String, serde_yaml::Value>,
    ) -> Result<String, TemplateError> {
        match self.env.render_named_str("inline", &self.template, &context) {
            Ok(s) => Ok(s),
            Err(err) => {
                // Keep diagnostics minimal: show the most specific source error
                let mut root = err.to_string();
                let mut src = err.source();
                while let Some(e) = src {
                    root = e.to_string();
                    src = e.source();
                }
                let msg = format!("Template rendering error: {}", root);
                Err(TemplateError::InvalidTemplate(msg, Box::new(err)))
            }
        }
    }
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
            serde_yaml::Value::String("World".to_string()),
        );
        context.insert(
            "body".to_string(),
            serde_yaml::Value::String("Hello content".to_string()),
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
            serde_yaml::Value::String("TEST VALUE".to_string()),
        );
        context.insert(
            "body".to_string(),
            serde_yaml::Value::String("body".to_string()),
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
            serde_yaml::Value::String("DASHED".to_string()),
        );
        context.insert(
            "body".to_string(),
            serde_yaml::Value::String("body".to_string()),
        );

        let result = glue.compose(context).unwrap();
        assert!(result.contains("DASHED"));
    }
}
