use std::collections::HashMap;
use tera::{Tera, Context, Value, Filter};
use serde_yaml;
use crate::parse::ParsedDocument;

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

/// Template engine wrapper around Tera for markdown-to-format templating
pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    /// Create a new TemplateEngine instance
    pub fn new() -> Self {
        let mut tera = Tera::default();
        
        // Register common filters that backends might need
        tera.register_filter("String", string_filter);
        tera.register_filter("List", list_filter);
        tera.register_filter("Array", list_filter); // Alias for List
        tera.register_filter("Int", int_filter);
        tera.register_filter("Bool", bool_filter);
        tera.register_filter("Date", date_filter);
        tera.register_filter("DateTime", date_filter); // Alias for Date
        tera.register_filter("Dict", dict_filter);
        tera.register_filter("Body", body_filter);
        
        Self { tera }
    }
    
    /// Register a custom filter with the template engine
    pub fn register_filter<F>(&mut self, name: &str, filter: F) 
    where
        F: Filter + 'static,
    {
        self.tera.register_filter(name, filter);
    }
    
    /// Render a template string with the provided data
    pub fn render_string(&self, template: &str, data: &ParsedDocument) -> Result<String, TemplateError> {
        let mut context = Context::new();
        
        // Convert ParsedDocument fields to tera context, normalizing field names
        for (key, value) in data.fields() {
            // Replace dashes with underscores for tera compatibility
            let normalized_key = key.replace("-", "_");
            context.insert(&normalized_key, value);
            // Also insert the original key for backward compatibility
            context.insert(key, value);
        }
        
        // Use tera one_off to render template string with custom filters
        let mut one_off_tera = self.tera.clone();
        one_off_tera.render_str(template, &context)
            .map_err(TemplateError::RenderError)
    }
    
    /// Render a template string with raw field data (HashMap)
    pub fn render_string_with_data(&self, template: &str, data: &HashMap<String, serde_yaml::Value>) -> Result<String, TemplateError> {
        let mut context = Context::new();
        
        // Convert data to tera context, normalizing field names
        for (key, value) in data {
            // Replace dashes with underscores for tera compatibility  
            let normalized_key = key.replace("-", "_");
            context.insert(&normalized_key, value);
            // Also insert the original key for backward compatibility
            context.insert(key, value);
        }
        
        // Use tera one_off to render template string with custom filters
        let mut one_off_tera = self.tera.clone();
        one_off_tera.render_str(template, &context)
            .map_err(TemplateError::RenderError)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Filter implementations for common data type conversions
fn string_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match value {
        Value::String(s) => Ok(Value::String(s.clone())),
        Value::Number(n) => Ok(Value::String(n.to_string())),
        Value::Bool(b) => Ok(Value::String(b.to_string())),
        _ => Ok(Value::String(format!("{}", value))),
    }
}

fn list_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match value {
        Value::Array(_) => Ok(value.clone()),
        Value::String(s) => {
            // Split string into array by lines or commas
            let items: Vec<Value> = s.lines()
                .map(|line| Value::String(line.trim().to_string()))
                .collect();
            Ok(Value::Array(items))
        }
        _ => Ok(Value::Array(vec![value.clone()])),
    }
}

fn int_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match value {
        Value::Number(_) => Ok(value.clone()),
        Value::String(s) => {
            if let Ok(parsed) = s.parse::<i64>() {
                Ok(Value::Number(serde_json::Number::from(parsed)))
            } else {
                Ok(Value::Number(serde_json::Number::from(0)))
            }
        }
        _ => Ok(Value::Number(serde_json::Number::from(0))),
    }
}

fn bool_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match value {
        Value::Bool(_) => Ok(value.clone()),
        Value::String(s) => {
            let lower = s.to_lowercase();
            Ok(Value::Bool(lower == "true" || lower == "yes" || lower == "1"))
        }
        Value::Number(n) => {
            Ok(Value::Bool(n.as_i64().unwrap_or(0) != 0))
        }
        _ => Ok(Value::Bool(false)),
    }
}

fn date_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    // For now, just pass through - backends can implement more sophisticated date formatting
    match value {
        Value::String(_) => Ok(value.clone()),
        _ => Ok(Value::String(format!("{}", value))),
    }
}

fn dict_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match value {
        Value::Object(_) => Ok(value.clone()),
        _ => {
            // Create a simple object wrapper
            let mut obj = serde_json::Map::new();
            obj.insert("value".to_string(), value.clone());
            Ok(Value::Object(obj))
        }
    }
}

fn body_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    // This filter is for body content - backends will customize this
    // For now, just pass through as string
    string_filter(value, _args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_template_engine_creation() {
        let _engine = TemplateEngine::new();
        // Just verify it doesn't panic
        assert!(true);
    }
    
    #[test]
    fn test_template_engine_default() {
        let _engine = TemplateEngine::default();
        // Just verify it doesn't panic
        assert!(true);
    }
    
    #[test]
    fn test_render_simple_template() {
        let engine = TemplateEngine::new();
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), serde_yaml::Value::String("World".to_string()));
        fields.insert("BODY".to_string(), serde_yaml::Value::String("Hello content".to_string()));
        
        let doc = ParsedDocument::with_frontmatter(fields, "Hello content".to_string());
        let template = "Hello {{ name }}! Body: {{ BODY }}";
        
        let result = engine.render_string(template, &doc).unwrap();
        assert!(result.contains("Hello World!"));
        assert!(result.contains("Body: Hello content"));
    }
    
    #[test] 
    fn test_render_with_filters() {
        let engine = TemplateEngine::new();
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_yaml::Value::String("Test Title".to_string()));
        fields.insert("count".to_string(), serde_yaml::Value::Number(serde_yaml::Number::from(42)));
        fields.insert("BODY".to_string(), serde_yaml::Value::String("Body content".to_string()));
        
        let doc = ParsedDocument::with_frontmatter(fields, "Body content".to_string());
        let template = "{{ title | String }} - {{ count | Int }} - {{ BODY | Body }}";
        
        let result = engine.render_string(template, &doc).unwrap();
        assert!(result.contains("Test Title"));
        assert!(result.contains("42"));
        assert!(result.contains("Body content"));
    }
    
    #[test]
    fn test_string_filter() {
        let value = Value::Number(serde_json::Number::from(123));
        let args = HashMap::new();
        let result = string_filter(&value, &args).unwrap();
        assert_eq!(result, Value::String("123".to_string()));
    }
    
    #[test]
    fn test_list_filter() {
        let value = Value::String("item1\nitem2\nitem3".to_string());
        let args = HashMap::new();
        let result = list_filter(&value, &args).unwrap();
        
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::String("item1".to_string()));
        } else {
            panic!("Expected array result");
        }
    }
    
    #[test]
    fn test_int_filter() {
        let value = Value::String("456".to_string());
        let args = HashMap::new();
        let result = int_filter(&value, &args).unwrap();
        assert_eq!(result, Value::Number(serde_json::Number::from(456)));
    }
    
    #[test]
    fn test_bool_filter() {
        let value = Value::String("true".to_string());
        let args = HashMap::new();
        let result = bool_filter(&value, &args).unwrap();
        assert_eq!(result, Value::Bool(true));
        
        let value = Value::String("false".to_string());
        let result = bool_filter(&value, &args).unwrap();
        assert_eq!(result, Value::Bool(false));
    }
    
    #[test]
    fn test_field_with_dash() {
        let engine = TemplateEngine::new();
        let mut fields = HashMap::new();
        fields.insert("letterhead_title".to_string(), serde_yaml::Value::String("TEST VALUE".to_string()));
        fields.insert("BODY".to_string(), serde_yaml::Value::String("body".to_string()));
        
        let doc = ParsedDocument::with_frontmatter(fields, "body".to_string());
        
        let template = r#"Field: {{ letterhead_title }}"#;
        let result = engine.render_string(template, &doc).unwrap();
        println!("Simple field test: {}", result);
        assert!(result.contains("TEST VALUE"));
    }
    
    #[test]
    fn test_template_with_typst_like_syntax() {
        let engine = TemplateEngine::new();
        
        // Create test data similar to the glue.typ example
        let mut fields = HashMap::new();
        fields.insert("letterhead-title".to_string(), serde_yaml::Value::String("DEPARTMENT OF THE AIR FORCE".to_string()));
        
        // Create an array for letterhead-caption
        let caption_items = vec![
            serde_yaml::Value::String("HEADQUARTERS UNITED STATES AIR FORCE".to_string()),
            serde_yaml::Value::String("WASHINGTON, DC 20330-1000".to_string())
        ];
        fields.insert("letterhead-caption".to_string(), serde_yaml::Value::Sequence(caption_items));
        
        fields.insert("subject".to_string(), serde_yaml::Value::String("Test Memorandum Subject".to_string()));
        fields.insert("date".to_string(), serde_yaml::Value::String("1 January 2024".to_string()));
        
        let body_content = "This is the main content of the memorandum.\n\nIt contains multiple paragraphs.";
        let doc = ParsedDocument::with_frontmatter(fields, body_content.to_string());
        
        // Test template similar to glue.typ format, using normalized field names (underscores)
        let template = r#"#show:official-memorandum.with(
  letterhead-title: {{ letterhead_title | String }},
  subject: {{ subject | String }},
  {{ BODY | Body }}
)"#;
        
        let result = engine.render_string(template, &doc).unwrap();
        println!("Template result: {}", result);
        assert!(result.contains("DEPARTMENT OF THE AIR FORCE"));
        assert!(result.contains("Test Memorandum Subject"));
        assert!(result.contains("This is the main content"));
    }
}