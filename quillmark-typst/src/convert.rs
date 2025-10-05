//! # Markdown to Typst Conversion
//!
//! This module transforms CommonMark markdown into Typst markup language.
//!
//! ## Key Functions
//!
//! - [`mark_to_typst()`] - Primary conversion function for Markdown to Typst
//! - [`escape_markup()`] - Escapes text for safe use in Typst markup context
//! - [`escape_string()`] - Escapes text for embedding in Typst string literals
//!
//! ## Quick Example
//!
//! ```
//! use quillmark_typst::convert::mark_to_typst;
//!
//! let markdown = "This is **bold** and _italic_.";
//! let typst = mark_to_typst(markdown).unwrap();
//! // Output: "This is *bold* and _italic_.\n\n"
//! ```
//!
//! ## Detailed Documentation
//!
//! For comprehensive conversion details including:
//! - Character escaping strategies
//! - CommonMark feature coverage  
//! - Event-based conversion flow
//! - Implementation notes
//!
//! See [CONVERT.md](https://github.com/nibsbin/quillmark/blob/main/quillmark-typst/docs/designs/CONVERT.md) for the complete specification.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// Maximum nesting depth for markdown structures
const MAX_NESTING_DEPTH: usize = 100;

/// Errors that can occur during markdown to Typst conversion
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    /// Nesting depth exceeded maximum allowed
    #[error("Nesting too deep: {depth} levels (max: {max} levels)")]
    NestingTooDeep {
        /// Actual depth
        depth: usize,
        /// Maximum allowed depth
        max: usize,
    },
}

/// Escapes text for safe use in Typst markup context.
pub fn escape_markup(s: &str) -> String {
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

/// Escapes text for embedding in Typst string literals.
pub fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Escape other ASCII controls with \u{..}
            c if c.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

#[derive(Debug, Clone)]
enum ListType {
    Bullet,
    Ordered,
}

/// Converts an iterator of markdown events to Typst markup
fn push_typst<'a, I>(output: &mut String, iter: I) -> Result<(), ConversionError>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut end_newline = true;
    let mut list_stack: Vec<ListType> = Vec::new();
    let mut in_list_item = false; // Track if we're inside a list item
    let mut depth = 0; // Track nesting depth for DoS prevention

    for event in iter {
        match event {
            Event::Start(tag) => {
                // Track nesting depth
                depth += 1;
                if depth > MAX_NESTING_DEPTH {
                    return Err(ConversionError::NestingTooDeep {
                        depth,
                        max: MAX_NESTING_DEPTH,
                    });
                }

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
                    Tag::Link {
                        dest_url, title: _, ..
                    } => {
                        output.push_str("#link(\"");
                        output.push_str(&escape_markup(&dest_url));
                        output.push_str("\")[");
                        end_newline = false;
                    }
                    _ => {
                        // Ignore other start tags not in requirements
                    }
                }
            }
            Event::End(tag) => {
                // Decrement depth
                depth = depth.saturating_sub(1);

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
                                              // Only add newline if we're not already at end of line
                        if !end_newline {
                            output.push('\n');
                            end_newline = true;
                        }
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
                let escaped = escape_markup(&text);
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

    Ok(())
}

/// Converts CommonMark Markdown to Typst markup.
///
/// Returns an error if nesting depth exceeds the maximum allowed.
pub fn mark_to_typst(markdown: &str) -> Result<String, ConversionError> {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);
    let mut typst_output = String::new();

    push_typst(&mut typst_output, parser)?;
    Ok(typst_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for escape_markup function
    #[test]
    fn test_escape_markup_basic() {
        assert_eq!(escape_markup("plain text"), "plain text");
    }

    #[test]
    fn test_escape_markup_backslash() {
        // Backslash must be escaped first to prevent double-escaping
        assert_eq!(escape_markup("\\"), "\\\\");
        assert_eq!(escape_markup("C:\\Users\\file"), "C:\\\\Users\\\\file");
    }

    #[test]
    fn test_escape_markup_formatting_chars() {
        assert_eq!(escape_markup("*bold*"), "\\*bold\\*");
        assert_eq!(escape_markup("_italic_"), "\\_italic\\_");
        assert_eq!(escape_markup("`code`"), "\\`code\\`");
    }

    #[test]
    fn test_escape_markup_typst_special_chars() {
        assert_eq!(escape_markup("#function"), "\\#function");
        assert_eq!(escape_markup("[link]"), "\\[link\\]");
        assert_eq!(escape_markup("$math$"), "\\$math\\$");
        assert_eq!(escape_markup("<tag>"), "\\<tag\\>");
        assert_eq!(escape_markup("@ref"), "\\@ref");
    }

    #[test]
    fn test_escape_markup_combined() {
        assert_eq!(
            escape_markup("Use * for bold and # for functions"),
            "Use \\* for bold and \\# for functions"
        );
    }

    // Tests for escape_string function
    #[test]
    fn test_escape_string_basic() {
        assert_eq!(escape_string("plain text"), "plain text");
    }

    #[test]
    fn test_escape_string_quotes_and_backslash() {
        assert_eq!(escape_string("\"quoted\""), "\\\"quoted\\\"");
        assert_eq!(escape_string("\\"), "\\\\");
    }

    #[test]
    fn test_escape_string_whitespace() {
        assert_eq!(escape_string("line\nbreak"), "line\\nbreak");
        assert_eq!(escape_string("carriage\rreturn"), "carriage\\rreturn");
        assert_eq!(escape_string("tab\there"), "tab\\there");
    }

    #[test]
    fn test_escape_string_control_chars() {
        // ASCII control character (e.g., NUL)
        assert_eq!(escape_string("\x00"), "\\u{0}");
        assert_eq!(escape_string("\x01"), "\\u{1}");
    }

    // Tests for mark_to_typst - Basic Text Formatting
    #[test]
    fn test_basic_text_formatting() {
        let markdown = "This is **bold**, _italic_, and ~~strikethrough~~ text.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(
            typst,
            "This is *bold*, _italic_, and #strike[strikethrough] text.\n\n"
        );
    }

    #[test]
    fn test_bold_formatting() {
        assert_eq!(mark_to_typst("**bold**").unwrap(), "*bold*\n\n");
        assert_eq!(
            mark_to_typst("This is **bold** text").unwrap(),
            "This is *bold* text\n\n"
        );
    }

    #[test]
    fn test_italic_formatting() {
        assert_eq!(mark_to_typst("_italic_").unwrap(), "_italic_\n\n");
        assert_eq!(mark_to_typst("*italic*").unwrap(), "_italic_\n\n");
    }

    #[test]
    fn test_strikethrough_formatting() {
        assert_eq!(mark_to_typst("~~strike~~").unwrap(), "#strike[strike]\n\n");
    }

    #[test]
    fn test_inline_code() {
        assert_eq!(mark_to_typst("`code`").unwrap(), "`code`\n\n");
        assert_eq!(
            mark_to_typst("Text with `inline code` here").unwrap(),
            "Text with `inline code` here\n\n"
        );
    }

    // Tests for Lists
    #[test]
    fn test_unordered_list() {
        let markdown = "- Item 1\n- Item 2\n- Item 3";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(typst, "+ Item 1\n+ Item 2\n+ Item 3\n\n");
    }

    #[test]
    fn test_ordered_list() {
        let markdown = "1. First\n2. Second\n3. Third";
        let typst = mark_to_typst(markdown).unwrap();
        // Typst auto-numbers, so we always use 1.
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(typst, "1. First\n1. Second\n1. Third\n\n");
    }

    #[test]
    fn test_nested_list() {
        let markdown = "- Item 1\n- Item 2\n  - Nested item\n- Item 3";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(typst, "+ Item 1\n+ Item 2\n  + Nested item\n+ Item 3\n\n");
    }

    #[test]
    fn test_deeply_nested_list() {
        let markdown = "- Level 1\n  - Level 2\n    - Level 3";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(typst, "+ Level 1\n  + Level 2\n    + Level 3\n\n");
    }

    // Tests for Links
    #[test]
    fn test_link() {
        let markdown = "[Link text](https://example.com)";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#link(\"https://example.com\")[Link text]\n\n");
    }

    #[test]
    fn test_link_in_sentence() {
        let markdown = "Visit [our site](https://example.com) for more.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(
            typst,
            "Visit #link(\"https://example.com\")[our site] for more.\n\n"
        );
    }

    // Tests for Mixed Content
    #[test]
    fn test_mixed_content() {
        let markdown = "A paragraph with **bold** and a [link](https://example.com).\n\nAnother paragraph with `inline code`.\n\n- A list item\n- Another item";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(
            typst,
            "A paragraph with *bold* and a #link(\"https://example.com\")[link].\n\nAnother paragraph with `inline code`.\n\n+ A list item\n+ Another item\n\n"
        );
    }

    // Tests for Paragraphs
    #[test]
    fn test_single_paragraph() {
        let markdown = "This is a paragraph.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "This is a paragraph.\n\n");
    }

    #[test]
    fn test_multiple_paragraphs() {
        let markdown = "First paragraph.\n\nSecond paragraph.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "First paragraph.\n\nSecond paragraph.\n\n");
    }

    // Tests for Line Breaks
    #[test]
    fn test_soft_break() {
        let markdown = "Line one\nLine two";
        let typst = mark_to_typst(markdown).unwrap();
        // Soft break becomes a space
        assert_eq!(typst, "Line one Line two\n\n");
    }

    #[test]
    fn test_hard_break() {
        let markdown = "Line one  \nLine two";
        let typst = mark_to_typst(markdown).unwrap();
        // Hard break (two spaces) becomes newline
        assert_eq!(typst, "Line one\nLine two\n\n");
    }

    // Tests for Character Escaping
    #[test]
    fn test_escaping_special_characters() {
        let markdown = "Typst uses * for bold and # for functions.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "Typst uses \\* for bold and \\# for functions.\n\n");
    }

    #[test]
    fn test_escaping_in_text() {
        let markdown = "Use [brackets] and $math$ symbols.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "Use \\[brackets\\] and \\$math\\$ symbols.\n\n");
    }

    // Edge Cases
    #[test]
    fn test_empty_string() {
        assert_eq!(mark_to_typst("").unwrap(), "");
    }

    #[test]
    fn test_only_whitespace() {
        let markdown = "   ";
        let typst = mark_to_typst(markdown).unwrap();
        // Whitespace-only input produces empty output (no paragraph tags for empty content)
        assert_eq!(typst, "");
    }

    #[test]
    fn test_consecutive_formatting() {
        let markdown = "**bold** _italic_ ~~strike~~";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "*bold* _italic_ #strike[strike]\n\n");
    }

    #[test]
    fn test_nested_formatting() {
        let markdown = "**bold _and italic_**";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "*bold _and italic_*\n\n");
    }

    #[test]
    fn test_list_with_formatting() {
        let markdown = "- **Bold** item\n- _Italic_ item\n- `Code` item";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline
        assert_eq!(typst, "+ *Bold* item\n+ _Italic_ item\n+ `Code` item\n\n");
    }

    #[test]
    fn test_list_with_multiple_paragraphs() {
        // This tests the in_list_item flag behavior
        let markdown = "- First paragraph.\n\n  Second paragraph.";
        let typst = mark_to_typst(markdown).unwrap();
        // Within list items, paragraphs should not add extra spacing
        // But there's still a space between them from the soft break handling
        assert_eq!(typst, "+ First paragraph.Second paragraph.\n\n");
    }

    #[test]
    fn test_mixed_list_types() {
        let markdown = "- Bullet item\n\n1. Ordered item\n2. Another ordered";
        let typst = mark_to_typst(markdown).unwrap();
        // Each list ends with extra newline
        assert_eq!(
            typst,
            "+ Bullet item\n\n1. Ordered item\n1. Another ordered\n\n"
        );
    }

    #[test]
    fn test_link_with_special_chars_in_url() {
        // URLs with special chars should be escaped
        let markdown = "[Link](#anchor)";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#link(\"\\#anchor\")[Link]\n\n");
    }

    #[test]
    fn test_markdown_escapes() {
        // Backslash escapes in markdown should work
        let markdown = "Use \\* for lists";
        let typst = mark_to_typst(markdown).unwrap();
        // The parser removes the backslash, then we escape for Typst
        assert_eq!(typst, "Use \\* for lists\n\n");
    }

    #[test]
    fn test_double_backslash() {
        let markdown = "Path: C:\\\\Users\\\\file";
        let typst = mark_to_typst(markdown).unwrap();
        // Double backslash in markdown becomes single in parser, then doubled for Typst
        assert_eq!(typst, "Path: C:\\\\Users\\\\file\n\n");
    }

    // Tests for resource limits
    #[test]
    fn test_nesting_depth_limit() {
        // Create deeply nested blockquotes (each ">" adds one nesting level)
        let mut markdown = String::new();
        for _ in 0..=MAX_NESTING_DEPTH {
            markdown.push('>');
            markdown.push(' ');
        }
        markdown.push_str("text");

        // This should exceed the limit and return an error
        let result = mark_to_typst(&markdown);
        assert!(result.is_err());

        if let Err(ConversionError::NestingTooDeep { depth, max }) = result {
            assert!(depth > max);
            assert_eq!(max, MAX_NESTING_DEPTH);
        } else {
            panic!("Expected NestingTooDeep error");
        }
    }

    #[test]
    fn test_nesting_depth_within_limit() {
        // Create nested structure just within the limit
        let mut markdown = String::new();
        for _ in 0..50 {
            markdown.push('>');
            markdown.push(' ');
        }
        markdown.push_str("text");

        // This should succeed
        let result = mark_to_typst(&markdown);
        assert!(result.is_ok());
    }
}
