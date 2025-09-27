//! Integration test demonstrating templating with realistic frontmatter
use quillmark_core::{templating::TemplateEngine, parse::decompose};

#[test]
fn test_integration_template_rendering() {
    // Create a template engine
    let engine = TemplateEngine::new();
    
    // Simulate parsing markdown with frontmatter (like what a backend would get)
    let markdown = r#"---
letterhead-title: "DEPARTMENT OF THE AIR FORCE"
letterhead-caption:
  - "HEADQUARTERS UNITED STATES AIR FORCE"  
  - "WASHINGTON, DC 20330-1000"
date: "1 January 2024"
memo-for:
  - "ALL PERSONNEL"
memo-from:
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
    
    // Template similar to the real map.typ from examples/hello-quill/map.typ
    let template = r#"#import "@preview/tonguetoquill-usaf-memo:0.1.1": official-memorandum

#show:official-memorandum.with(
  // Letterhead configuration  
  letterhead-title: {{ letterhead_title | String }},
  letterhead-caption: {{ letterhead_caption | List }},
  
  // Frontmatter
  date: {{ date | Date }}, 
  memo-for: {{ memo_for | List }},
  memo-from: {{ memo_from | List }},
  
  // Subject line
  subject: {{ subject | String }},

  {{ BODY | Markup }}
)
"#;
    
    // Render the template
    let result = engine.render_string(template, &doc).expect("Failed to render template");
    
    println!("=== Rendered Typst Template ===");
    println!("{}", result);
    println!("=== End of Template ===");
    
    // Show that the template rendered correctly with proper values
    assert!(result.contains("DEPARTMENT OF THE AIR FORCE"));
    assert!(result.contains("New Policies and Procedures"));
    assert!(result.contains("This memorandum outlines"));
    println!("✅ Template rendered successfully with all expected content!");
}