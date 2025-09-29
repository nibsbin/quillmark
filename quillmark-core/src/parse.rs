use std::collections::HashMap;

/// The field name used to store the document body
pub const BODY_FIELD: &str = "body";

/// A parsed markdown document with frontmatter
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    fields: HashMap<String, serde_yaml::Value>,
}

impl ParsedDocument {
    /// Create a new ParsedDocument with the given fields
    pub fn new(fields: HashMap<String, serde_yaml::Value>) -> Self {
        Self { fields }
    }

    /// Get the document body
    pub fn body(&self) -> Option<&str> {
        self.fields.get(BODY_FIELD)
            .and_then(|v| v.as_str())
    }

    /// Get a specific field
    pub fn get_field(&self, name: &str) -> Option<&serde_yaml::Value> {
        self.fields.get(name)
    }

    /// Get all fields (including body)
    pub fn fields(&self) -> &HashMap<String, serde_yaml::Value> {
        &self.fields
    }
}

/// Decompose markdown into frontmatter fields and body
pub fn decompose(markdown: &str) -> Result<ParsedDocument, Box<dyn std::error::Error + Send + Sync>> {
    let mut fields = HashMap::new();
    
    // Check if we have frontmatter
    if markdown.starts_with("---\n") {
        // Find the end of frontmatter
        let rest = &markdown[4..];
        if let Some(end_pos) = rest.find("\n---\n") {
            let frontmatter = &rest[..end_pos];
            let body = &rest[end_pos + 5..]; // Skip past the closing ---\n
            
            // Parse YAML frontmatter
            let yaml_fields: HashMap<String, serde_yaml::Value> = serde_yaml::from_str(frontmatter)
                .map_err(|e| format!("Invalid YAML frontmatter: {}", e))?;
            
            fields.extend(yaml_fields);
            fields.insert(BODY_FIELD.to_string(), serde_yaml::Value::String(body.to_string()));
        } else {
            return Err("Frontmatter started but not closed with ---".into());
        }
    } else {
        // No frontmatter, entire content is body
        fields.insert(BODY_FIELD.to_string(), serde_yaml::Value::String(markdown.to_string()));
    }

    Ok(ParsedDocument::new(fields))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let markdown = "# Hello World\n\nThis is a test.";
        let doc = decompose(markdown).unwrap();
        
        assert_eq!(doc.body(), Some(markdown));
        assert_eq!(doc.fields().len(), 1);
    }

    #[test]
    fn test_with_frontmatter() {
        let markdown = r#"---
title: Test Document
author: Test Author
---

# Hello World

This is the body."#;
        
        let doc = decompose(markdown).unwrap();
        
        assert_eq!(doc.body(), Some("\n# Hello World\n\nThis is the body."));
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Test Document");
        assert_eq!(doc.get_field("author").unwrap().as_str().unwrap(), "Test Author");
        assert_eq!(doc.fields().len(), 3); // title, author, body
    }

    #[test]
    fn test_complex_yaml_frontmatter() {
        let markdown = r#"---
title: Complex Document
tags:
  - test
  - yaml
metadata:
  version: 1.0
  nested:
    field: value
---

Content here."#;
        
        let doc = decompose(markdown).unwrap();
        
        assert_eq!(doc.body(), Some("\nContent here."));
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Complex Document");
        
        let tags = doc.get_field("tags").unwrap().as_sequence().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str().unwrap(), "test");
        assert_eq!(tags[1].as_str().unwrap(), "yaml");
    }

    #[test]
    fn test_invalid_yaml() {
        let markdown = r#"---
title: [invalid yaml
author: missing close bracket
---

Content here."#;
        
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid YAML frontmatter"));
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let markdown = r#"---
title: Test
author: Test Author

Content without closing ---"#;
        
        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not closed"));
    }
}