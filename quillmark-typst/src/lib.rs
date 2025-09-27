use quillmark_core::{Backend, OutputFormat, Options, RenderError, Artifact};
use pulldown_cmark::{Parser, Event, Tag, TagEnd};

/// Escapes text for safe use in Typst
fn escape_typst(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('$', "\\$")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('@', "\\@")
}

#[derive(Debug, Clone)]
enum ListType {
    Bullet,
    Ordered,
}

/// Converts an iterator of markdown events to Typst markup
fn push_typst<'a, I>(output: &mut String, iter: I)
where
    I: Iterator<Item = Event<'a>>,
{
    let mut end_newline = true;
    let mut list_stack: Vec<ListType> = Vec::new();
    let mut in_list_item = false; // Track if we're inside a list item
    
    for event in iter {
        match event {
            Event::Start(tag) => {
                match tag {
                    Tag::Paragraph => {
                        // Only add newlines for paragraphs that are NOT inside list items
                        if !in_list_item {
                            // Don't add extra newlines if we're already at start of line
                            if !end_newline {
                                output.push('\n');
                                end_newline = true;
                            }
                        }
                        // Typst doesn't need explicit paragraph tags for simple paragraphs
                    }
                    Tag::List(start_number) => {
                        if !end_newline {
                            output.push('\n');
                            end_newline = true;
                        }
                        
                        let list_type = if start_number.is_some() {
                            ListType::Ordered
                        } else {
                            ListType::Bullet
                        };
                        
                        list_stack.push(list_type);
                    }
                    Tag::Item => {
                        in_list_item = true; // We're now inside a list item
                        if let Some(list_type) = list_stack.last() {
                            let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                            
                            match list_type {
                                ListType::Bullet => {
                                    output.push_str(&format!("{}+ ", indent));
                                }
                                ListType::Ordered => {
                                    output.push_str(&format!("{}1. ", indent));
                                }
                            }
                            end_newline = false;
                        }
                    }
                    Tag::Emphasis => {
                        output.push('_');
                        end_newline = false;
                    }
                    Tag::Strong => {
                        output.push('*');
                        end_newline = false;
                    }
                    Tag::Strikethrough => {
                        output.push_str("#strike[");
                        end_newline = false;
                    }
                    Tag::Link { dest_url, title: _, .. } => {
                        output.push_str("#link(\"");
                        output.push_str(&escape_typst(&dest_url));
                        output.push_str("\")[");
                        end_newline = false;
                    }
                    _ => {
                        // Ignore other start tags not in requirements
                    }
                }
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Paragraph => {
                        // Only handle paragraph endings when NOT inside list items
                        if !in_list_item {
                            output.push('\n');
                            output.push('\n'); // Extra newline for paragraph separation
                            end_newline = true;
                        }
                        // For paragraphs inside list items, we don't add extra spacing
                    }
                    TagEnd::List(_) => {
                        list_stack.pop();
                        if list_stack.is_empty() {
                            output.push('\n');
                            end_newline = true;
                        }
                    }
                    TagEnd::Item => {
                        in_list_item = false; // We're no longer inside a list item
                        output.push('\n');
                        end_newline = true;
                    }
                    TagEnd::Emphasis => {
                        output.push('_');
                        end_newline = false;
                    }
                    TagEnd::Strong => {
                        output.push('*');
                        end_newline = false;
                    }
                    TagEnd::Strikethrough => {
                        output.push(']');
                        end_newline = false;
                    }
                    TagEnd::Link => {
                        output.push(']');
                        end_newline = false;
                    }
                    _ => {
                        // Ignore other end tags not in requirements
                    }
                }
            }
            Event::Text(text) => {
                let escaped = escape_typst(&text);
                output.push_str(&escaped);
                end_newline = escaped.ends_with('\n');
            }
            Event::Code(text) => {
                // Inline code
                output.push('`');
                output.push_str(&text);
                output.push('`');
                end_newline = false;
            }
            Event::SoftBreak => {
                output.push(' ');
                end_newline = false;
            }
            Event::HardBreak => {
                output.push('\n');
                end_newline = true;
            }
            _ => {
                // Ignore other events not specified in requirements
                // (html, math, footnotes, tables, etc.)
            }
        }
    }
}

/// Converts markdown to Typst markup
pub fn mark_to_typst(markdown: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    
    let parser = Parser::new_ext(markdown, options);
    let mut typst_output = String::new();
    
    push_typst(&mut typst_output, parser);
    typst_output
}

/// Typst backend implementation using puldown-cmark and Typst
pub struct TypstBackend;

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        // Typst can output PDF and SVG
        &[OutputFormat::Pdf, OutputFormat::Svg]
    }

    fn render(&self, markdown: &str, _opts: &Options) -> Result<Vec<Artifact>, RenderError> {
        // Convert markdown to Typst using our conversion logic
        let typst_content = mark_to_typst(markdown);
        
        // For now, return the Typst content as bytes
        // In a real implementation, this would compile with Typst to PDF/SVG
        Ok(vec![Artifact {
            bytes: typst_content.into_bytes(),
            output_format: OutputFormat::Pdf,
        }])
    }
}

impl Default for TypstBackend {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quillmark_core::OutputFormat;

    #[test]
    fn test_typst_backend_id() {
        let backend = TypstBackend::default();
        assert_eq!(backend.id(), "typst");
    }

    #[test]
    fn test_typst_backend_supported_formats() {
        let backend = TypstBackend::default();
        let formats = backend.supported_formats();
        assert!(formats.contains(&OutputFormat::Pdf));
        assert!(formats.contains(&OutputFormat::Svg));
        assert!(!formats.contains(&OutputFormat::Txt));
    }

    #[test]
    fn test_typst_backend_render() {
        let backend = TypstBackend::default();
        let options = Options {
            backend: Some("typst".to_string()),
            format: Some(OutputFormat::Pdf),
        };
        
        let result = backend.render("This is *emphasis* and **strong** text.", &options);
        assert!(result.is_ok());
        
        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].output_format, OutputFormat::Pdf);
        
        let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
        assert!(content.contains("_emphasis_"));
        assert!(content.contains("*strong*"));
    }

    #[test]
    fn test_mark_to_typst_basic() {
        let markdown = "This is **bold** and *italic* text.";
        let typst = mark_to_typst(markdown);
        assert!(typst.contains("*bold*"));
        assert!(typst.contains("_italic_"));
    }

    #[test]
    fn test_mark_to_typst_lists() {
        let markdown = "- First item\n- Second item\n  - Nested item";
        let typst = mark_to_typst(markdown);
        assert!(typst.contains("+ First item"));
        assert!(typst.contains("+ Second item"));
        assert!(typst.contains("  + Nested item"));
    }

    #[test]
    fn test_mark_to_typst_links() {
        let markdown = "[Rust](https://rust-lang.org)";
        let typst = mark_to_typst(markdown);
        assert!(typst.contains("#link(\"https://rust-lang.org\")[Rust]"));
    }

    #[test]
    fn test_mark_to_typst_strikethrough() {
        let markdown = "This has ~~strikethrough~~ text.";
        let typst = mark_to_typst(markdown);
        assert!(typst.contains("#strike[strikethrough]"));
    }

    #[test]
    fn test_escape_typst() {
        let input = "Hello *world* with #hash and $math$";
        let expected = "Hello \\*world\\* with \\#hash and \\$math\\$";
        assert_eq!(escape_typst(input), expected);
    }
}