use serde_yaml;
use std::collections::HashMap;
use std::error::Error;

/// Reserved field name for markdown body content
pub const BODY_FIELD: &str = "BODY";

/// Parsed markdown document with frontmatter fields and body
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDocument {
    /// Dictionary containing frontmatter fields and the BODY field
    pub fields: HashMap<String, serde_yaml::Value>,
}

impl ParsedDocument {
    /// Create a new ParsedDocument with just the body content
    pub fn new(body: String) -> Self {
        let mut fields = HashMap::new();
        fields.insert(BODY_FIELD.to_string(), serde_yaml::Value::String(body));
        Self { fields }
    }

    /// Create a new ParsedDocument with frontmatter and body content
    pub fn with_frontmatter(frontmatter: HashMap<String, serde_yaml::Value>, body: String) -> Self {
        let mut fields = frontmatter;
        fields.insert(BODY_FIELD.to_string(), serde_yaml::Value::String(body));
        Self { fields }
    }

    /// Get the markdown body content
    pub fn body(&self) -> Option<&str> {
        self.fields
            .get(BODY_FIELD)
            .and_then(|v| v.as_str())
    }

    /// Get a frontmatter field value
    pub fn get_field(&self, key: &str) -> Option<&serde_yaml::Value> {
        self.fields.get(key)
    }

    /// Get all fields as a reference to the internal HashMap
    pub fn fields(&self) -> &HashMap<String, serde_yaml::Value> {
        &self.fields
    }
}

/// Parse markdown content, handling YAML frontmatter if present
/// 
/// This function separates YAML frontmatter (if present) from the markdown body,
/// and returns a ParsedDocument containing both as a dictionary.
/// YAML frontmatter fields are mapped to dictionary fields, and the markdown body
/// is stored under the reserved BODY field.
pub fn decompose(markdown: &str) -> Result<ParsedDocument, Box<dyn Error + Send + Sync>> {
    // Check if the document starts with YAML frontmatter (---\n)
    if markdown.starts_with("---\n") || markdown.starts_with("---\r\n") {
        let lines: Vec<&str> = markdown.lines().collect();
        if lines.is_empty() {
            return Ok(ParsedDocument::new(markdown.to_string()));
        }

        // Find the end of frontmatter (second ---)
        let mut end_idx = None;
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_idx = Some(i);
                break;
            }
        }

        if let Some(end_idx) = end_idx {
            // Extract frontmatter (excluding the --- delimiters)
            let frontmatter_lines = &lines[1..end_idx];
            let frontmatter_str = frontmatter_lines.join("\n");
            
            // Extract body (everything after the closing ---)
            let body_lines = &lines[end_idx + 1..];
            let body = body_lines.join("\n").trim_start().to_string();

            // Parse YAML frontmatter
            if frontmatter_str.trim().is_empty() {
                // Empty frontmatter, just return the body
                return Ok(ParsedDocument::new(body));
            }

            match serde_yaml::from_str::<HashMap<String, serde_yaml::Value>>(&frontmatter_str) {
                Ok(frontmatter_map) => {
                    Ok(ParsedDocument::with_frontmatter(frontmatter_map, body))
                }
                Err(e) => {
                    // If frontmatter parsing fails, treat the entire content as body
                    eprintln!("Warning: Failed to parse YAML frontmatter: {}", e);
                    Ok(ParsedDocument::new(markdown.to_string()))
                }
            }
        } else {
            // No closing ---, treat entire content as body
            Ok(ParsedDocument::new(markdown.to_string()))
        }
    } else {
        // No frontmatter, entire content is body
        Ok(ParsedDocument::new(markdown.to_string()))
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_no_frontmatter() {
        let markdown = "# Hello World\n\nThis is a test.";
        let result = decompose(markdown).unwrap();
        
        assert_eq!(result.body(), Some(markdown));
        assert_eq!(result.fields().len(), 1);
        assert!(result.fields().contains_key(BODY_FIELD));
    }

    #[test]
    fn test_decompose_with_frontmatter() {
        let markdown = r#"---
title: Test Document
author: John Doe
tags:
  - test
  - markdown
---

# Hello World

This is the body content."#;
        
        let result = decompose(markdown).unwrap();
        
        assert_eq!(result.body(), Some("# Hello World\n\nThis is the body content."));
        assert_eq!(result.get_field("title").and_then(|v| v.as_str()), Some("Test Document"));
        assert_eq!(result.get_field("author").and_then(|v| v.as_str()), Some("John Doe"));
        
        // Check tags array
        let tags = result.get_field("tags").and_then(|v| v.as_sequence());
        assert!(tags.is_some());
        assert_eq!(tags.unwrap().len(), 2);
    }

    #[test]
    fn test_decompose_empty_frontmatter() {
        let markdown = r#"---
---

# Hello World

This is the body."#;
        
        let result = decompose(markdown).unwrap();
        
        assert_eq!(result.body(), Some("# Hello World\n\nThis is the body."));
        assert_eq!(result.fields().len(), 1); // Only BODY field
    }

    #[test]
    fn test_decompose_invalid_frontmatter() {
        let markdown = r#"---
invalid: yaml: content: [
---

# Hello World"#;
        
        let result = decompose(markdown).unwrap();
        
        // Should fallback to treating entire content as body when YAML is invalid
        assert!(result.body().unwrap().contains("---"));
        assert!(result.body().unwrap().contains("# Hello World"));
    }

    #[test]
    fn test_decompose_incomplete_frontmatter() {
        let markdown = r#"---
title: Test Document
author: John Doe

# Hello World"#;
        
        let result = decompose(markdown).unwrap();
        
        // No closing ---, should treat entire content as body
        assert!(result.body().unwrap().contains("---"));
        assert!(result.body().unwrap().contains("title: Test Document"));
    }

    #[test]
    fn test_parsed_document_methods() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_yaml::Value::String("Test".to_string()));
        fields.insert("count".to_string(), serde_yaml::Value::Number(serde_yaml::Number::from(42)));
        
        let doc = ParsedDocument::with_frontmatter(fields, "Body content".to_string());
        
        assert_eq!(doc.body(), Some("Body content"));
        assert_eq!(doc.get_field("title").and_then(|v| v.as_str()), Some("Test"));
        assert_eq!(doc.get_field("count").and_then(|v| v.as_i64()), Some(42));
        assert!(doc.get_field("nonexistent").is_none());
        assert_eq!(doc.fields().len(), 3); // title, count, BODY
    }
}