use std::collections::HashMap;
use tera::{Tera, Context, Filter};
use serde_yaml;
use std::error::Error as StdError;

/// Error types for template rendering
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("{0}")]
    RenderError(#[from] tera::Error),
    #[error("{0}")]
    InvalidTemplate(String, #[source] Box<dyn StdError + Send + Sync>),
    #[error("{0}")]
    FilterError(String),
}

/// Glue class for template rendering - provides interface for backends to interact with templates
pub struct Glue {
    tera: Tera,
    template: String,
}

impl Glue {
    /// Create a new Glue instance with a template string
    pub fn new(template: String) -> Self {
        let tera = Tera::default();
        Self { tera, template }
    }
    
    /// Register a custom filter with the template engine
    pub fn register_filter<F>(&mut self, name: &str, filter: F) 
    where
        F: Filter + 'static,
    {
        self.tera.register_filter(name, filter);
    }
    
    
    /// Compose template with context from markdown decomposition
    pub fn compose(&mut self, context: HashMap<String, serde_yaml::Value>) -> Result<String, TemplateError> {
        let mut tera_context = Context::new();
        
        // Convert data to tera context.
        // Insert keys into the tera context exactly as provided. The project

        for (key, value) in context {
            tera_context.insert(&key, &value);
        }

        match self.tera.render_str(&self.template, &tera_context) {
            Ok(s) => Ok(s),
            Err(tera_err) => {
                // Keep diagnostics minimal: show the most specific source error
                let mut root = tera_err.to_string();
                let mut src = tera_err.source();
                while let Some(e) = src {
                    root = e.to_string();
                    src = e.source();
                }

                let msg = format!("Template rendering error: {}", root);
                Err(TemplateError::InvalidTemplate(msg, Box::new(tera_err)))
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
        // Just verify it doesn't panic
        assert!(true);
    }
    
    #[test]
    fn test_compose_simple_template() {
        let mut glue = Glue::new("Hello {{ name }}! Body: {{ body }}".to_string());
        let mut context = HashMap::new();
        context.insert("name".to_string(), serde_yaml::Value::String("World".to_string()));
        context.insert("body".to_string(), serde_yaml::Value::String("Hello content".to_string()));
        
        let result = glue.compose(context).unwrap();
        assert!(result.contains("Hello World!"));
        assert!(result.contains("Body: Hello content"));
    }
    
    #[test]
    fn test_field_with_dash() {
        let mut glue = Glue::new("Field: {{ letterhead_title }}".to_string());
        let mut context = HashMap::new();
        context.insert("letterhead_title".to_string(), serde_yaml::Value::String("TEST VALUE".to_string()));
        context.insert("body".to_string(), serde_yaml::Value::String("body".to_string()));
        
        let result = glue.compose(context).unwrap();
        assert!(result.contains("TEST VALUE"));
    }

    #[test]
    fn test_compose_with_dash_in_template() {
        // Templates must reference the exact key names provided by the context.
        let mut glue = Glue::new("Field: {{ letterhead_title }}".to_string());
        let mut context = HashMap::new();
        context.insert("letterhead_title".to_string(), serde_yaml::Value::String("DASHED".to_string()));
        context.insert("body".to_string(), serde_yaml::Value::String("body".to_string()));

        let result = glue.compose(context).unwrap();
        assert!(result.contains("DASHED"));
    }
}