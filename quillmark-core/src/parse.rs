#[doc = include_str!("../docs/parse.md")]

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
        self.fields.get(BODY_FIELD).and_then(|v| v.as_str())
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

#[derive(Debug)]
struct MetadataBlock {
    start: usize,        // Position of opening "---"
    end: usize,          // Position after closing "---\n"
    yaml_content: String,
    tag: Option<String>, // Tag directive if present
}

/// Validate tag name follows pattern [a-z_][a-z0-9_]*
fn is_valid_tag_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    
    if !first.is_ascii_lowercase() && first != '_' {
        return false;
    }
    
    for ch in chars {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_' {
            return false;
        }
    }
    
    true
}

/// Find all metadata blocks in the document
fn find_metadata_blocks(markdown: &str) -> Result<Vec<MetadataBlock>, Box<dyn std::error::Error + Send + Sync>> {
    let mut blocks = Vec::new();
    let mut pos = 0;
    
    while pos < markdown.len() {
        // Look for opening "---\n" or "---\r\n"
        let search_str = &markdown[pos..];
        let delimiter_result = if let Some(p) = search_str.find("---\n") {
            Some((p, 4, "\n"))
        } else if let Some(p) = search_str.find("---\r\n") {
            Some((p, 5, "\r\n"))
        } else {
            None
        };
        
        if let Some((delimiter_pos, delimiter_len, _line_ending)) = delimiter_result {
            let abs_pos = pos + delimiter_pos;
            let content_start = abs_pos + delimiter_len; // After "---\n" or "---\r\n"
            
            // Check if opening --- is followed by a blank line (horizontal rule, not metadata)
            let followed_by_blank = if content_start < markdown.len() {
                markdown[content_start..].starts_with('\n') || markdown[content_start..].starts_with("\r\n")
            } else {
                false
            };
            
            if followed_by_blank {
                // This is a horizontal rule in the body, skip it
                pos = abs_pos + 3; // Skip past "---"
                continue;
            }
            
            // Found potential metadata block opening
            // Look for closing "\n---\n" or "\r\n---\r\n" etc., OR "\n---" / "\r\n---" at end of document
            let rest = &markdown[content_start..];
            
            // First try to find delimiters with trailing newlines
            let closing_patterns = ["\n---\n", "\r\n---\r\n", "\n---\r\n", "\r\n---\n"];
            let closing_with_newline = closing_patterns
                .iter()
                .filter_map(|delim| rest.find(delim).map(|p| (p, delim.len())))
                .min_by_key(|(p, _)| *p);
            
            // Also check for closing at end of document (no trailing newline)
            let closing_at_eof = ["\n---", "\r\n---"]
                .iter()
                .filter_map(|delim| {
                    rest.find(delim).and_then(|p| {
                        if p + delim.len() == rest.len() {
                            Some((p, delim.len()))
                        } else {
                            None
                        }
                    })
                })
                .min_by_key(|(p, _)| *p);
            
            let closing_result = match (closing_with_newline, closing_at_eof) {
                (Some((p1, _l1)), Some((p2, _))) if p2 < p1 => closing_at_eof,
                (Some(_), Some(_)) => closing_with_newline,
                (Some(_), None) => closing_with_newline,
                (None, Some(_)) => closing_at_eof,
                (None, None) => None,
            };
            
            if let Some((closing_pos, closing_len)) = closing_result {
                let abs_closing_pos = content_start + closing_pos;
                let content = &markdown[content_start..abs_closing_pos];
                
                // Check if the block is contiguous (no blank lines in the YAML content)
                if content.contains("\n\n") || content.contains("\r\n\r\n") {
                    // Not a contiguous block
                    if abs_pos == 0 {
                        // Started at beginning but has blank lines - this is an error
                        return Err("Frontmatter started but not closed with ---".into());
                    }
                    // Otherwise treat as horizontal rule in body
                    pos = abs_pos + 3;
                    continue;
                }
                
                // Extract tag directive if present
                let (tag, yaml_content) = if content.starts_with('!') {
                    if let Some(newline_pos) = content.find(|c| c == '\n' || c == '\r') {
                        let tag_line = &content[1..newline_pos];
                        // Skip newline(s) after tag
                        let yaml_start = if content[newline_pos..].starts_with("\r\n") {
                            newline_pos + 2
                        } else {
                            newline_pos + 1
                        };
                        let yaml = if yaml_start < content.len() {
                            &content[yaml_start..]
                        } else {
                            ""
                        };
                        (Some(tag_line.trim().to_string()), yaml.to_string())
                    } else {
                        // Tag directive with no YAML content (entire content is just tag)
                        (Some(content[1..].trim().to_string()), String::new())
                    }
                } else {
                    (None, content.to_string())
                };
                
                // Validate tag name if present
                if let Some(ref tag_name) = tag {
                    if !is_valid_tag_name(tag_name) {
                        return Err(format!("Invalid tag name '{}': must match pattern [a-z_][a-z0-9_]*", tag_name).into());
                    }
                    if tag_name == BODY_FIELD {
                        return Err(format!("Cannot use reserved field name '{}' as tag directive", BODY_FIELD).into());
                    }
                }
                
                blocks.push(MetadataBlock {
                    start: abs_pos,
                    end: abs_closing_pos + closing_len, // After closing delimiter
                    yaml_content,
                    tag,
                });
                
                pos = abs_closing_pos + closing_len;
            } else if abs_pos == 0 {
                // Frontmatter started but not closed
                return Err("Frontmatter started but not closed with ---".into());
            } else {
                // Not a valid metadata block, skip this position
                pos = abs_pos + 3;
            }
        } else {
            break;
        }
    }
    
    Ok(blocks)
}

/// Decompose markdown into frontmatter fields and body
pub fn decompose(
    markdown: &str,
) -> Result<ParsedDocument, Box<dyn std::error::Error + Send + Sync>> {
    let mut fields = HashMap::new();
    
    // Find all metadata blocks
    let blocks = find_metadata_blocks(markdown)?;
    
    if blocks.is_empty() {
        // No metadata blocks, entire content is body
        fields.insert(
            BODY_FIELD.to_string(),
            serde_yaml::Value::String(markdown.to_string()),
        );
        return Ok(ParsedDocument::new(fields));
    }
    
    // Track which attributes are used for tagged blocks
    let mut tagged_attributes: HashMap<String, Vec<serde_yaml::Value>> = HashMap::new();
    let mut has_global_frontmatter = false;
    let mut global_frontmatter_index: Option<usize> = None;
    
    // First pass: identify global frontmatter and validate
    for (idx, block) in blocks.iter().enumerate() {
        if block.tag.is_none() {
            if has_global_frontmatter {
                return Err("Multiple global frontmatter blocks found: only one untagged block allowed".into());
            }
            has_global_frontmatter = true;
            global_frontmatter_index = Some(idx);
        }
    }
    
    // Parse global frontmatter if present
    if let Some(idx) = global_frontmatter_index {
        let block = &blocks[idx];
        
        // Parse YAML frontmatter
        let yaml_fields: HashMap<String, serde_yaml::Value> = if block.yaml_content.is_empty() {
            HashMap::new()
        } else {
            serde_yaml::from_str(&block.yaml_content)
                .map_err(|e| format!("Invalid YAML frontmatter: {}", e))?
        };
        
        // Check that all tagged blocks don't conflict with global fields
        for other_block in &blocks {
            if let Some(ref tag) = other_block.tag {
                if yaml_fields.contains_key(tag) {
                    return Err(format!("Name collision: global field '{}' conflicts with tagged attribute", tag).into());
                }
            }
        }
        
        fields.extend(yaml_fields);
    }
    
    // Parse tagged blocks
    for (idx, block) in blocks.iter().enumerate() {
        if let Some(ref tag_name) = block.tag {
            // Check if this conflicts with global fields
            if fields.contains_key(tag_name) {
                return Err(format!("Name collision: tagged attribute '{}' conflicts with global field", tag_name).into());
            }
            
            // Parse YAML metadata
            let mut item_fields: HashMap<String, serde_yaml::Value> = if block.yaml_content.is_empty() {
                HashMap::new()
            } else {
                serde_yaml::from_str(&block.yaml_content)
                    .map_err(|e| format!("Invalid YAML in tagged block '{}': {}", tag_name, e))?
            };
            
            // Extract body for this tagged block
            let body_start = block.end;
            let body_end = if idx + 1 < blocks.len() {
                blocks[idx + 1].start
            } else {
                markdown.len()
            };
            let body = &markdown[body_start..body_end];
            
            // Add body to item fields
            item_fields.insert(
                BODY_FIELD.to_string(),
                serde_yaml::Value::String(body.to_string()),
            );
            
            // Convert HashMap to serde_yaml::Value::Mapping
            let item_value = serde_yaml::to_value(item_fields)?;
            
            // Add to collection
            tagged_attributes.entry(tag_name.clone())
                .or_insert_with(Vec::new)
                .push(item_value);
        }
    }
    
    // Extract global body
    let (body_start, body_end) = if let Some(idx) = global_frontmatter_index {
        // Global body starts after frontmatter
        let start = blocks[idx].end;
        
        // Global body ends at the first tagged block after the frontmatter, or EOF
        let end = blocks.iter()
            .skip(idx + 1)
            .find(|b| b.tag.is_some())
            .map(|b| b.start)
            .unwrap_or(markdown.len());
        
        (start, end)
    } else {
        // No global frontmatter - body is everything before the first tagged block
        let end = blocks.iter()
            .find(|b| b.tag.is_some())
            .map(|b| b.start)
            .unwrap_or(0);
        
        (0, end)
    };
    
    let global_body = &markdown[body_start..body_end];
    
    fields.insert(
        BODY_FIELD.to_string(),
        serde_yaml::Value::String(global_body.to_string()),
    );
    
    // Add all tagged collections to fields
    for (tag_name, items) in tagged_attributes {
        fields.insert(tag_name, serde_yaml::Value::Sequence(items));
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
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Test Document"
        );
        assert_eq!(
            doc.get_field("author").unwrap().as_str().unwrap(),
            "Test Author"
        );
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
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Complex Document"
        );

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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid YAML frontmatter"));
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

    // Extended metadata tests

    #[test]
    fn test_basic_tagged_block() {
        let markdown = r#"---
title: Main Document
---

Main body content.

---
!items
name: Item 1
---

Body of item 1."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(doc.body(), Some("\nMain body content.\n\n"));
        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Main Document"
        );

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);
        
        let item = items[0].as_mapping().unwrap();
        assert_eq!(
            item.get(&serde_yaml::Value::String("name".to_string()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Item 1"
        );
        assert_eq!(
            item.get(&serde_yaml::Value::String("body".to_string()))
                .unwrap()
                .as_str()
                .unwrap(),
            "\nBody of item 1."
        );
    }

    #[test]
    fn test_multiple_tagged_blocks() {
        let markdown = r#"---
!items
name: Item 1
tags: [a, b]
---

First item body.

---
!items
name: Item 2
tags: [c, d]
---

Second item body."#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 2);
        
        let item1 = items[0].as_mapping().unwrap();
        assert_eq!(
            item1.get(&serde_yaml::Value::String("name".to_string()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Item 1"
        );
        
        let item2 = items[1].as_mapping().unwrap();
        assert_eq!(
            item2.get(&serde_yaml::Value::String("name".to_string()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Item 2"
        );
    }

    #[test]
    fn test_mixed_global_and_tagged() {
        let markdown = r#"---
title: Global
author: John Doe
---

Global body.

---
!sections
title: Section 1
---

Section 1 content.

---
!sections
title: Section 2
---

Section 2 content."#;

        let doc = decompose(markdown).unwrap();

        assert_eq!(
            doc.get_field("title").unwrap().as_str().unwrap(),
            "Global"
        );
        assert_eq!(doc.body(), Some("\nGlobal body.\n\n"));

        let sections = doc.get_field("sections").unwrap().as_sequence().unwrap();
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn test_empty_tagged_metadata() {
        let markdown = r#"---
!items
---

Body without metadata."#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);
        
        let item = items[0].as_mapping().unwrap();
        assert_eq!(
            item.get(&serde_yaml::Value::String("body".to_string()))
                .unwrap()
                .as_str()
                .unwrap(),
            "\nBody without metadata."
        );
    }

    #[test]
    fn test_tagged_block_without_body() {
        let markdown = r#"---
!items
name: Item
---"#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);
        
        let item = items[0].as_mapping().unwrap();
        assert_eq!(
            item.get(&serde_yaml::Value::String("body".to_string()))
                .unwrap()
                .as_str()
                .unwrap(),
            ""
        );
    }

    #[test]
    fn test_name_collision_global_and_tagged() {
        let markdown = r#"---
items: "global value"
---

Body

---
!items
name: Item
---

Item body"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("collision"));
    }

    #[test]
    fn test_reserved_field_name() {
        let markdown = r#"---
!body
content: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_invalid_tag_syntax() {
        let markdown = r#"---
!Invalid-Name
title: Test
---"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid tag name"));
    }

    #[test]
    fn test_multiple_global_frontmatter_blocks() {
        let markdown = r#"---
title: First
---

Body

---
author: Second
---

More body"#;

        let result = decompose(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Multiple global frontmatter"));
    }

    #[test]
    fn test_adjacent_blocks_different_tags() {
        let markdown = r#"---
!items
name: Item 1
---

Item 1 body

---
!sections
title: Section 1
---

Section 1 body"#;

        let doc = decompose(markdown).unwrap();

        assert!(doc.get_field("items").is_some());
        assert!(doc.get_field("sections").is_some());
        
        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 1);
        
        let sections = doc.get_field("sections").unwrap().as_sequence().unwrap();
        assert_eq!(sections.len(), 1);
    }

    #[test]
    fn test_order_preservation() {
        let markdown = r#"---
!items
id: 1
---

First

---
!items
id: 2
---

Second

---
!items
id: 3
---

Third"#;

        let doc = decompose(markdown).unwrap();

        let items = doc.get_field("items").unwrap().as_sequence().unwrap();
        assert_eq!(items.len(), 3);
        
        for (i, item) in items.iter().enumerate() {
            let mapping = item.as_mapping().unwrap();
            let id = mapping.get(&serde_yaml::Value::String("id".to_string()))
                .unwrap()
                .as_i64()
                .unwrap();
            assert_eq!(id, (i + 1) as i64);
        }
    }

    #[test]
    fn test_product_catalog_integration() {
        let markdown = r#"---
title: Product Catalog
author: John Doe
date: 2024-01-01
---

This is the main catalog description.

---
!products
name: Widget A
price: 19.99
sku: WID-001
---

The **Widget A** is our most popular product.

---
!products
name: Gadget B
price: 29.99
sku: GAD-002
---

The **Gadget B** is perfect for professionals.

---
!reviews
product: Widget A
rating: 5
---

"Excellent product! Highly recommended."

---
!reviews
product: Gadget B
rating: 4
---

"Very good, but a bit pricey.""#;

        let doc = decompose(markdown).unwrap();
        
        // Verify global fields
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Product Catalog");
        assert_eq!(doc.get_field("author").unwrap().as_str().unwrap(), "John Doe");
        assert_eq!(doc.get_field("date").unwrap().as_str().unwrap(), "2024-01-01");
        
        // Verify global body
        assert!(doc.body().unwrap().contains("main catalog description"));
        
        // Verify products collection
        let products = doc.get_field("products").unwrap().as_sequence().unwrap();
        assert_eq!(products.len(), 2);
        
        let product1 = products[0].as_mapping().unwrap();
        assert_eq!(
            product1.get(&serde_yaml::Value::String("name".to_string()))
                .unwrap().as_str().unwrap(),
            "Widget A"
        );
        assert_eq!(
            product1.get(&serde_yaml::Value::String("price".to_string()))
                .unwrap().as_f64().unwrap(),
            19.99
        );
        
        // Verify reviews collection
        let reviews = doc.get_field("reviews").unwrap().as_sequence().unwrap();
        assert_eq!(reviews.len(), 2);
        
        let review1 = reviews[0].as_mapping().unwrap();
        assert_eq!(
            review1.get(&serde_yaml::Value::String("product".to_string()))
                .unwrap().as_str().unwrap(),
            "Widget A"
        );
        assert_eq!(
            review1.get(&serde_yaml::Value::String("rating".to_string()))
                .unwrap().as_i64().unwrap(),
            5
        );
        
        // Total fields: title, author, date, body, products, reviews = 6
        assert_eq!(doc.fields().len(), 6);
    }
}
#[cfg(test)]
mod demo_file_test {
    use super::*;

    #[test]
    fn test_extended_metadata_demo_file() {
        let markdown = include_str!("../../quillmark-fixtures/resources/extended_metadata_demo.md");
        let doc = decompose(markdown).unwrap();
        
        // Verify global fields
        assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "Extended Metadata Demo");
        assert_eq!(doc.get_field("author").unwrap().as_str().unwrap(), "Quillmark Team");
        // version is parsed as a number by YAML
        assert_eq!(doc.get_field("version").unwrap().as_f64().unwrap(), 1.0);
        
        // Verify body
        assert!(doc.body().unwrap().contains("extended YAML metadata standard"));
        
        // Verify features collection
        let features = doc.get_field("features").unwrap().as_sequence().unwrap();
        assert_eq!(features.len(), 3);
        
        // Verify use_cases collection
        let use_cases = doc.get_field("use_cases").unwrap().as_sequence().unwrap();
        assert_eq!(use_cases.len(), 2);
        
        // Check first feature
        let feature1 = features[0].as_mapping().unwrap();
        assert_eq!(
            feature1.get(&serde_yaml::Value::String("name".to_string()))
                .unwrap().as_str().unwrap(),
            "Tag Directives"
        );
    }
}
