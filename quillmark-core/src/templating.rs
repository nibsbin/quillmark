use std::collections::HashMap;
use tera::{Tera, Context, Filter};
use serde_yaml;

/// Error types for template rendering
#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("Template rendering failed: {0}")]
    RenderError(#[from] tera::Error),
    #[error("Invalid template content: {0}")]
    InvalidTemplate(String),
    #[error("Filter error: {0}")]
    FilterError(String),
}

/// Glue class for template rendering - provides interface for backends to interact with templates
pub struct Glue {
    tera: Tera,
}

impl Glue {
    /// Create a new Glue instance
    pub fn new() -> Self {
        let tera = Tera::default();
        Self { tera }
    }
    
    /// Register a custom filter with the template engine
    pub fn register_filter<F>(&mut self, name: &str, filter: F) 
    where
        F: Filter + 'static,
    {
        self.tera.register_filter(name, filter);
    }
    
    
    /// Compose template with context from markdown decomposition
    pub fn compose(&mut self, template: &str, context: HashMap<String, serde_yaml::Value>) -> Result<String, TemplateError> {
        let mut tera_context = Context::new();
        
        // Convert data to tera context, normalizing field names
        for (key, value) in context {
            // Replace dashes with underscores for tera compatibility  
            let normalized_key = key.replace("-", "_");
            tera_context.insert(&normalized_key, &value);
            // Also insert the original key for backward compatibility
            tera_context.insert(&key, &value);
        }
        
        // Render the template
        self.tera.render_str(template, &tera_context)
            .map_err(TemplateError::RenderError)
    }
}

impl Default for Glue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_glue_creation() {
        let _glue = Glue::new();
        // Just verify it doesn't panic
        assert!(true);
    }
    
    #[test]
    fn test_glue_default() {
        let _glue = Glue::default();
        // Just verify it doesn't panic
        assert!(true);
    }
    
    #[test]
    fn test_compose_simple_template() {
        let mut glue = Glue::new();
        let mut context = HashMap::new();
        context.insert("name".to_string(), serde_yaml::Value::String("World".to_string()));
        context.insert("BODY".to_string(), serde_yaml::Value::String("Hello content".to_string()));
        
        let template = "Hello {{ name }}! Body: {{ BODY }}";
        
        let result = glue.compose(template, context).unwrap();
        assert!(result.contains("Hello World!"));
        assert!(result.contains("Body: Hello content"));
    }
    
    #[test]
    fn test_field_with_dash() {
        let mut glue = Glue::new();
        let mut context = HashMap::new();
        context.insert("letterhead-title".to_string(), serde_yaml::Value::String("TEST VALUE".to_string()));
        context.insert("BODY".to_string(), serde_yaml::Value::String("body".to_string()));
        
        let template = r#"Field: {{ letterhead_title }}"#;
        let result = glue.compose(template, context).unwrap();
        assert!(result.contains("TEST VALUE"));
    }
}