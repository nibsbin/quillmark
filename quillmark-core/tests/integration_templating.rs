//! Integration test demonstrating templating with realistic frontmatter
use quillmark_core::{templating::Glue, parse::decompose};

#[test]
fn test_integration_template_rendering() {
    // Simulate parsing markdown with frontmatter (like what a backend would get)
    let markdown = r#"---
letterhead_title: "DEPARTMENT OF THE AIR FORCE"
letterhead_caption:
  - "HEADQUARTERS UNITED STATES AIR FORCE"  
  - "WASHINGTON, DC 20330-1000"
date: "1 January 2024"
memo_for:
  - "ALL PERSONNEL"
memo_from:
  - "COMMANDER"
subject: "New Policies and Procedures"
---

# Important Notice

This memorandum outlines the new policies and procedures that will take effect immediately.

## Key Points

1. **Policy Changes**: All personnel must comply with new guidelines
2. **Implementation**: Effective immediately
3. **Questions**: Contact your supervisor

Please review these changes carefully and ensure compliance."#;
    
    let doc = decompose(markdown).expect("Failed to parse markdown");
    
    // Template without filters since those will be implemented in quillmark-typst
    let template = r#"#import "@preview/tonguetoquill-usaf-memo:0.1.1": official-memorandum

#show:official-memorandum.with(
  // Letterhead configuration  
  letterhead-title: {{ letterhead_title }},
  letterhead-caption: {{ letterhead_caption }},
  
  // Frontmatter
  date: {{ date }}, 
  memo-for: {{ memo_for }},
  memo-from: {{ memo_from }},
  
  // Subject line
  subject: {{ subject }},

  {{ body }}
)
"#;
    
    // Create a glue instance with the template
    let mut glue = Glue::new(template.to_string());
    
    // Render the template using compose method with the parsed document fields
    let result = glue.compose(doc.fields().clone()).expect("Failed to render template");
    
    println!("=== Rendered Typst Template ===");
    println!("{}", result);
    println!("=== End of Template ===");
    
    // Show that the template rendered correctly with proper values
    assert!(result.contains("DEPARTMENT OF THE AIR FORCE"));
    assert!(result.contains("New Policies and Procedures"));
    assert!(result.contains("This memorandum outlines"));
    println!("✅ Template rendered successfully with all expected content!");
}