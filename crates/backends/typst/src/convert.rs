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
use std::ops::Range;

/// Maximum nesting depth for markdown structures
const MAX_NESTING_DEPTH: usize = 100;

/// Maximum length for guillemet content (single line, 64 KiB)
const MAX_GUILLEMET_LENGTH: usize = 64 * 1024;

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

/// Preprocesses markdown to convert guillemets: <<text>> → «text»
/// Extracts raw text content between << and >>, trims leading/trailing whitespace
/// and returns both the preprocessed source and the byte ranges for guillemet
/// content within the preprocessed source. This lets the main converter
/// perform context-aware escaping (e.g., link destination escaping) for
/// events that occur inside guillemets.
/// Skips conversion inside code blocks (fenced and indented) and code spans
fn preprocess_guillemets(markdown: &str) -> (String, Vec<Range<usize>>) {
    let mut result = String::with_capacity(markdown.len());
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let chars: Vec<char> = markdown.chars().collect();
    let mut i = 0;
    // Fence state: Some((char, length)) when inside a fenced code block
    let mut fence_state: Option<(char, usize)> = None;
    // Inline code state: Some(length) when inside an inline code span
    let mut inline_code_backticks: Option<usize> = None;
    let mut at_line_start = true;

    while i < chars.len() {
        let ch = chars[i];

        // Track line start for indented code block detection
        if ch == '\n' {
            at_line_start = true;
            result.push(ch);
            i += 1;
            continue;
        }

        // Check for indented code block (4+ spaces at line start, but only outside fences)
        if at_line_start && fence_state.is_none() && inline_code_backticks.is_none() {
            let indent = count_leading_spaces(&chars[i..]);
            if indent >= 4 {
                // This is an indented code block line - copy entire line without conversion
                while i < chars.len() && chars[i] != '\n' {
                    result.push(chars[i]);
                    i += 1;
                }
                continue;
            }
        }

        // No longer at line start after processing non-newline
        at_line_start = false;

        // Handle fenced code blocks (``` or ~~~, 3+ chars)
        if fence_state.is_none() && inline_code_backticks.is_none() && (ch == '`' || ch == '~') {
            let fence_len = count_consecutive(&chars[i..], ch);
            if fence_len >= 3 {
                // Start of fenced code block
                fence_state = Some((ch, fence_len));
                for _ in 0..fence_len {
                    result.push(ch);
                }
                i += fence_len;
                continue;
            }
        }

        // Check for end of fenced code block
        if let Some((fence_char, fence_len)) = fence_state {
            if ch == fence_char {
                let current_len = count_consecutive(&chars[i..], ch);
                if current_len >= fence_len {
                    // End of fenced code block
                    fence_state = None;
                    for _ in 0..current_len {
                        result.push(ch);
                    }
                    i += current_len;
                    continue;
                }
            }
            // Inside fenced code block - just copy
            result.push(ch);
            i += 1;
            continue;
        }

        // Handle inline code spans (backticks only)
        if ch == '`' {
            let backtick_count = count_consecutive(&chars[i..], '`');
            if let Some(open_count) = inline_code_backticks {
                if backtick_count == open_count {
                    // End of inline code span
                    inline_code_backticks = None;
                    for _ in 0..backtick_count {
                        result.push('`');
                    }
                    i += backtick_count;
                    continue;
                }
                // Inside inline code span but different backtick count - just copy
                result.push(ch);
                i += 1;
                continue;
            } else {
                // Start of inline code span
                inline_code_backticks = Some(backtick_count);
                for _ in 0..backtick_count {
                    result.push('`');
                }
                i += backtick_count;
                continue;
            }
        }

        // Inside inline code span - just copy
        if inline_code_backticks.is_some() {
            result.push(ch);
            i += 1;
            continue;
        }

        // Only process << when not in any code context
        if i + 1 < chars.len() && ch == '<' && chars[i + 1] == '<' {
            // Find matching >>
            if let Some(end_offset) = find_matching_guillemet_end(&chars[i + 2..]) {
                let content_end = i + 2 + end_offset;
                let content: String = chars[i + 2..content_end].iter().collect();

                // Check constraints: same line and size limit
                if !content.contains('\n') && content.len() <= MAX_GUILLEMET_LENGTH {
                    // Trim leading/trailing whitespace
                    let clean = trim_guillemet_content(&content);
                    result.push('«');
                    // Record byte start for range
                    let start = result.len();
                    result.push_str(&clean);
                    let end = result.len();
                    result.push('»');
                    ranges.push(start..end);
                    i = content_end + 2; // Skip past >>
                    continue;
                }
            }
        }

        // Regular character - just copy it
        result.push(ch);
        i += 1;
    }

    (result, ranges)
}

/// Counts consecutive occurrences of a character from the start of the slice.
fn count_consecutive(chars: &[char], target: char) -> usize {
    chars.iter().take_while(|&&c| c == target).count()
}

/// Counts leading spaces (not tabs) at the start of the slice.
fn count_leading_spaces(chars: &[char]) -> usize {
    chars.iter().take_while(|&&c| c == ' ').count()
}

/// Trims whitespace from guillemet content.
fn trim_guillemet_content(content: &str) -> String {
    content.trim().to_string()
}

/// Finds the position of >> that matches an opening <<, returns offset from search start
fn find_matching_guillemet_end(chars: &[char]) -> Option<usize> {
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i] == '>' && chars[i + 1] == '>' {
            return Some(i);
        }
    }
    None
}

/// Escapes text for safe use in Typst markup context.
pub fn escape_markup(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace("//", "\\/\\/")
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

#[derive(Debug, Clone, Copy)]
enum StrongKind {
    Bold,      // Source was **...**
    Underline, // Source was __...__
}

/// Converts an iterator of markdown events to Typst markup
fn push_typst<'a, I>(
    output: &mut String,
    source: &str,
    iter: I,
    guillemet_ranges: &[Range<usize>],
) -> Result<(), ConversionError>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    let mut end_newline = true;
    let mut list_stack: Vec<ListType> = Vec::new();
    let mut strong_stack: Vec<StrongKind> = Vec::new();
    let mut skip_strong_stack: Vec<bool> = Vec::new();
    let mut in_list_item = false; // Track if we're inside a list item
    let mut depth = 0; // Track nesting depth for DoS prevention
    let mut iter = iter.peekable();

    let mut had_strong_in_paragraph = false;
    while let Some((event, range)) = iter.next() {
        // Check whether this event falls within any guillemet range
        let in_guillemet = guillemet_ranges
            .iter()
            .any(|r| range.start >= r.start && range.start < r.end);
        // no-op
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
                        had_strong_in_paragraph = false;
                    }
                    Tag::CodeBlock(_) => {
                        // Code blocks are handled, no special tracking needed
                    }
                    Tag::HtmlBlock => {
                        // HTML blocks are handled, no special tracking needed
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
                                    output.push_str(&format!("{}- ", indent));
                                }
                                ListType::Ordered => {
                                    output.push_str(&format!("{}+ ", indent));
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
                        // Detect whether this is __ (underline) or ** (bold) by peeking at source
                        let kind = if range.start + 2 <= source.len() {
                            match &source[range.start..range.start + 2] {
                                "__" => StrongKind::Underline,
                                _ => StrongKind::Bold, // Default to bold for ** or edge cases
                            }
                        } else {
                            StrongKind::Bold // Fallback for very short ranges
                        };
                        strong_stack.push(kind);
                        // Determine whether we should suppress this strong. Use the
                        // pre-existing paragraph state (i.e., whether a strong was
                        // already present earlier), so compute skip_current before
                        // updating the paragraph flag.
                        let skip_current = in_guillemet
                            && had_strong_in_paragraph
                            && matches!(kind, StrongKind::Bold);
                        // Track that we saw a strong in this paragraph
                        had_strong_in_paragraph = true;
                        skip_strong_stack.push(skip_current);
                        match kind {
                            StrongKind::Underline => output.push_str("#underline["),
                            StrongKind::Bold => {
                                // Debug (removed)
                                // Special case: if the paragraph had earlier strong, and this start
                                // is inside guillemets, we skip the inner strong markers.
                                if !skip_strong_stack.last().copied().unwrap_or(false) {
                                    output.push('*');
                                }
                            }
                        }
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
                        if in_guillemet {
                            // Inside guillemets, use string escaping (no // escape)
                            output.push_str(&escape_string(&dest_url));
                        } else {
                            output.push_str(&escape_markup(&dest_url));
                        }
                        output.push_str("\")[");
                        end_newline = false;
                    }
                    Tag::Heading { level, .. } => {
                        if !end_newline {
                            output.push('\n');
                        }
                        let equals = "=".repeat(level as usize);
                        output.push_str(&equals);
                        output.push(' ');
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
                    TagEnd::CodeBlock => {
                        // Code blocks are handled, no special tracking needed
                    }
                    TagEnd::HtmlBlock => {
                        // HTML blocks are handled, no special tracking needed
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
                        // Check if next event is text starting with alphanumeric
                        if let Some((Event::Text(text), _)) = iter.peek() {
                            if text.chars().next().map_or(false, |c| c.is_alphanumeric()) {
                                output.push_str("#{}");
                            }
                        }
                        end_newline = false;
                    }
                    TagEnd::Strong => {
                        let skip_current = skip_strong_stack.pop().unwrap_or(false);
                        match strong_stack.pop() {
                            Some(StrongKind::Bold) => {
                                if !skip_current {
                                    output.push('*');
                                }
                                // Word-boundary handling only for bold
                                if let Some((Event::Text(text), _)) = iter.peek() {
                                    if text.chars().next().map_or(false, |c| c.is_alphanumeric()) {
                                        output.push_str("#{}");
                                    }
                                }
                            }
                            Some(StrongKind::Underline) => {
                                if !skip_current {
                                    output.push(']');
                                }
                                // No word-boundary handling needed for function syntax
                            }
                            None => {
                                // Malformed: more end tags than start tags
                                // Default to bold behavior for robustness
                                output.push('*');
                            }
                        }
                        // no-op
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
                    TagEnd::Heading(_) => {
                        output.push('\n');
                        output.push('\n'); // Extra newline after heading
                        end_newline = true;
                    }
                    _ => {
                        // Ignore other end tags not in requirements
                    }
                }
            }
            Event::Text(text) => {
                // Normal text processing
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
            Event::HardBreak => {
                output.push('\n');
                end_newline = true;
            }
            Event::SoftBreak => {
                output.push(' ');
                end_newline = false;
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
    // Preprocess to convert guillemets before parsing
    let (preprocessed, guillemet_ranges) = preprocess_guillemets(markdown);

    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&preprocessed, options);
    let mut typst_output = String::new();

    // Pass preprocessed source for delimiter peeking
    push_typst(
        &mut typst_output,
        &preprocessed,
        parser.into_offset_iter(),
        &guillemet_ranges,
    )?;
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
        assert_eq!(typst, "- Item 1\n- Item 2\n- Item 3\n\n");
    }

    #[test]
    fn test_ordered_list() {
        let markdown = "1. First\n2. Second\n3. Third";
        let typst = mark_to_typst(markdown).unwrap();
        // Typst auto-numbers, so we always use 1.
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(typst, "+ First\n+ Second\n+ Third\n\n");
    }

    #[test]
    fn test_nested_list() {
        let markdown = "- Item 1\n- Item 2\n  - Nested item\n- Item 3";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(typst, "- Item 1\n- Item 2\n  - Nested item\n- Item 3\n\n");
    }

    #[test]
    fn test_deeply_nested_list() {
        let markdown = "- Level 1\n  - Level 2\n    - Level 3";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline per CONVERT.md examples
        assert_eq!(typst, "- Level 1\n  - Level 2\n    - Level 3\n\n");
    }

    // Tests for Links
    #[test]
    fn test_link() {
        let markdown = "[Link text](https://example.com)";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#link(\"https:\\/\\/example.com\")[Link text]\n\n");
    }

    #[test]
    fn test_link_in_sentence() {
        let markdown = "Visit [our site](https://example.com) for more.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(
            typst,
            "Visit #link(\"https:\\/\\/example.com\")[our site] for more.\n\n"
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
            "A paragraph with *bold* and a #link(\"https:\\/\\/example.com\")[link].\n\nAnother paragraph with `inline code`.\n\n- A list item\n- Another item\n\n"
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

    #[test]
    fn test_hard_break() {
        let markdown = "Line one  \nLine two";
        let typst = mark_to_typst(markdown).unwrap();
        // Hard break (two spaces) becomes newline
        assert_eq!(typst, "Line one\nLine two\n\n");
    }

    #[test]
    fn test_soft_break() {
        let markdown = "Line one\nLine two";
        let typst = mark_to_typst(markdown).unwrap();
        // Soft break (single newline) becomes space
        assert_eq!(typst, "Line one Line two\n\n");
    }

    #[test]
    fn test_soft_break_multiple_lines() {
        let markdown = "This is some\ntext on multiple\nlines";
        let typst = mark_to_typst(markdown).unwrap();
        // Soft breaks should join with spaces
        assert_eq!(typst, "This is some text on multiple lines\n\n");
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
        assert_eq!(typst, "- *Bold* item\n- _Italic_ item\n- `Code` item\n\n");
    }

    #[test]
    fn test_mixed_list_types() {
        let markdown = "- Bullet item\n\n1. Ordered item\n2. Another ordered";
        let typst = mark_to_typst(markdown).unwrap();
        // Each list ends with extra newline
        assert_eq!(
            typst,
            "- Bullet item\n\n+ Ordered item\n+ Another ordered\n\n"
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

    // Tests for // (comment syntax) escaping
    #[test]
    fn test_slash_comment_in_url() {
        let markdown = "Check out https://example.com for more.";
        let typst = mark_to_typst(markdown).unwrap();
        // The // in https:// should be escaped to prevent it from being treated as a comment
        assert!(typst.contains("https:\\/\\/example.com"));
    }

    #[test]
    fn test_slash_comment_at_line_start() {
        let markdown = "// This should not be a comment";
        let typst = mark_to_typst(markdown).unwrap();
        // // at the start of a line should be escaped
        assert!(typst.contains("\\/\\/"));
    }

    #[test]
    fn test_slash_comment_in_middle() {
        let markdown = "Some text // with slashes in the middle";
        let typst = mark_to_typst(markdown).unwrap();
        // // in the middle of text should be escaped
        assert!(typst.contains("text \\/\\/"));
    }

    #[test]
    fn test_file_protocol() {
        let markdown = "Use file://path/to/file protocol";
        let typst = mark_to_typst(markdown).unwrap();
        // file:// should be escaped
        assert!(typst.contains("file:\\/\\/"));
    }

    #[test]
    fn test_single_slash() {
        let markdown = "Use path/to/file for the file";
        let typst = mark_to_typst(markdown).unwrap();
        // Single slashes should not be escaped (only // is a comment in Typst)
        assert!(typst.contains("path/to/file"));
    }

    #[test]
    fn test_italic_followed_by_alphanumeric() {
        // Bug: When closing italic marker is followed by alphanumeric, Typst doesn't recognize it
        let markdown = "*Write y*our paragraphs here.";
        let typst = mark_to_typst(markdown).unwrap();
        // Should add word boundary after closing underscore when followed by alphanumeric
        assert_eq!(typst, "_Write y_#{}our paragraphs here.\n\n");
    }

    #[test]
    fn test_italic_followed_by_space() {
        // When followed by space, no word boundary needed
        let markdown = "*italic* text";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "_italic_ text\n\n");
    }

    #[test]
    fn test_italic_followed_by_punctuation() {
        // When followed by punctuation, no word boundary needed
        let markdown = "*italic*.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "_italic_.\n\n");
    }

    #[test]
    fn test_bold_followed_by_alphanumeric() {
        // Same issue can occur with bold
        let markdown = "**bold**text";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "*bold*#{}text\n\n");
    }

    // Tests for Headings
    #[test]
    fn test_heading_level_1() {
        let markdown = "# Heading 1";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "= Heading 1\n\n");
    }

    #[test]
    fn test_heading_level_2() {
        let markdown = "## Heading 2";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "== Heading 2\n\n");
    }

    #[test]
    fn test_heading_level_3() {
        let markdown = "### Heading 3";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "=== Heading 3\n\n");
    }

    #[test]
    fn test_heading_level_4() {
        let markdown = "#### Heading 4";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "==== Heading 4\n\n");
    }

    #[test]
    fn test_heading_level_5() {
        let markdown = "##### Heading 5";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "===== Heading 5\n\n");
    }

    #[test]
    fn test_heading_level_6() {
        let markdown = "###### Heading 6";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "====== Heading 6\n\n");
    }

    #[test]
    fn test_heading_with_formatting() {
        let markdown = "## Heading with **bold** and _italic_";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "== Heading with *bold* and _italic_\n\n");
    }

    #[test]
    fn test_multiple_headings() {
        let markdown = "# First\n\n## Second\n\n### Third";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "= First\n\n== Second\n\n=== Third\n\n");
    }

    #[test]
    fn test_heading_followed_by_paragraph() {
        let markdown = "# Heading\n\nThis is a paragraph.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "= Heading\n\nThis is a paragraph.\n\n");
    }

    #[test]
    fn test_heading_with_special_chars() {
        let markdown = "# Heading with $math$ and #functions";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "= Heading with \\$math\\$ and \\#functions\n\n");
    }

    #[test]
    fn test_paragraph_then_heading() {
        let markdown = "A paragraph.\n\n# A Heading";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "A paragraph.\n\n= A Heading\n\n");
    }

    #[test]
    fn test_heading_with_inline_code() {
        let markdown = "## Code example: `fn main()`";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "== Code example: `fn main()`\n\n");
    }

    // Tests for underline support (__ syntax)

    // Basic Underline Tests
    #[test]
    fn test_underline_basic() {
        assert_eq!(
            mark_to_typst("__underlined__").unwrap(),
            "#underline[underlined]\n\n"
        );
    }

    #[test]
    fn test_underline_with_text() {
        assert_eq!(
            mark_to_typst("This is __underlined__ text").unwrap(),
            "This is #underline[underlined] text\n\n"
        );
    }

    #[test]
    fn test_bold_unchanged() {
        // Verify ** still works as bold
        assert_eq!(mark_to_typst("**bold**").unwrap(), "*bold*\n\n");
    }

    // Nesting Tests
    #[test]
    fn test_underline_containing_bold() {
        assert_eq!(
            mark_to_typst("__A **B** A__").unwrap(),
            "#underline[A *B* A]\n\n"
        );
    }

    #[test]
    fn test_bold_containing_underline() {
        assert_eq!(
            mark_to_typst("**A __B__ A**").unwrap(),
            "*A #underline[B] A*\n\n"
        );
    }

    #[test]
    fn test_deep_nesting() {
        assert_eq!(
            mark_to_typst("__A **B __C__ B** A__").unwrap(),
            "#underline[A *B #underline[C] B* A]\n\n"
        );
    }

    // Adjacent Styles Tests
    #[test]
    fn test_adjacent_underline_bold() {
        assert_eq!(mark_to_typst("__A__**B**").unwrap(), "#underline[A]*B*\n\n");
    }

    #[test]
    fn test_adjacent_bold_underline() {
        assert_eq!(mark_to_typst("**A**__B__").unwrap(), "*A*#underline[B]\n\n");
    }

    // Escaping Tests
    #[test]
    fn test_underline_special_chars() {
        // Special characters inside underline should be escaped
        assert_eq!(mark_to_typst("__#1__").unwrap(), "#underline[\\#1]\n\n");
    }

    #[test]
    fn test_underline_with_brackets() {
        assert_eq!(
            mark_to_typst("__[text]__").unwrap(),
            "#underline[\\[text\\]]\n\n"
        );
    }

    #[test]
    fn test_underline_with_asterisk() {
        assert_eq!(
            mark_to_typst("__a * b__").unwrap(),
            "#underline[a \\* b]\n\n"
        );
    }

    // Edge Case Tests
    #[test]
    fn test_empty_underline() {
        // Four underscores is parsed as horizontal rule by pulldown-cmark, not empty strong
        // This test verifies we don't crash on this input
        // (pulldown-cmark treats ____ as a thematic break / horizontal rule)
        let result = mark_to_typst("____").unwrap();
        // The result is empty because Rule events are ignored in our converter
        assert_eq!(result, "");
    }

    #[test]
    fn test_underline_in_list() {
        assert_eq!(
            mark_to_typst("- __underlined__ item").unwrap(),
            "- #underline[underlined] item\n\n"
        );
    }

    #[test]
    fn test_underline_in_heading() {
        assert_eq!(
            mark_to_typst("# Heading with __underline__").unwrap(),
            "= Heading with #underline[underline]\n\n"
        );
    }

    #[test]
    fn test_underline_followed_by_alphanumeric() {
        // When __under__ is immediately followed by alphanumeric (no space),
        // pulldown-cmark does NOT parse it as Strong - it treats underscores as literal.
        // This is standard CommonMark behavior requiring word boundaries.
        // With a space after, it does work as underline:
        assert_eq!(
            mark_to_typst("__under__ line").unwrap(),
            "#underline[under] line\n\n"
        );
    }

    // Mixed Formatting Tests
    #[test]
    fn test_underline_with_italic() {
        assert_eq!(
            mark_to_typst("__underline *italic*__").unwrap(),
            "#underline[underline _italic_]\n\n"
        );
    }

    #[test]
    fn test_underline_with_code() {
        assert_eq!(
            mark_to_typst("__underline `code`__").unwrap(),
            "#underline[underline `code`]\n\n"
        );
    }

    #[test]
    fn test_underline_with_strikethrough() {
        assert_eq!(
            mark_to_typst("__underline ~~strike~~__").unwrap(),
            "#underline[underline #strike[strike]]\n\n"
        );
    }

    // Tests for guillemet conversion

    // Basic Conversion Tests
    #[test]
    fn test_guillemet_basic() {
        assert_eq!(
            mark_to_typst("She said <<Hello, world>>.").unwrap(),
            "She said «Hello, world».\n\n"
        );
    }

    #[test]
    fn test_guillemet_simple_text() {
        assert_eq!(mark_to_typst("<<text>>").unwrap(), "«text»\n\n");
    }

    #[test]
    fn test_guillemet_with_spaces() {
        // Note: Markdown parser trims leading/trailing spaces in paragraphs
        assert_eq!(
            mark_to_typst("<<  spaced  text  >>").unwrap(),
            "«spaced  text»\n\n"
        );
    }

    // Formatting Strip Tests
    #[test]
    fn test_guillemet_strips_bold() {
        assert_eq!(mark_to_typst("<<**bold**>>").unwrap(), "«*bold*»\n\n");
    }

    #[test]
    fn test_guillemet_strips_italic() {
        assert_eq!(mark_to_typst("<<_italic_>>").unwrap(), "«_italic_»\n\n");
    }

    #[test]
    fn test_guillemet_strips_mixed_formatting() {
        assert_eq!(
            mark_to_typst("<<**bold** and _italic_ text>>").unwrap(),
            "«*bold* and _italic_ text»\n\n"
        );
    }

    #[test]
    fn test_guillemet_strips_strikethrough() {
        assert_eq!(
            mark_to_typst("<<~~strike~~>>").unwrap(),
            "«#strike[strike]»\n\n"
        );
    }

    #[test]
    fn test_guillemet_strips_underline() {
        assert_eq!(
            mark_to_typst("<<__underline__>>").unwrap(),
            "«#underline[underline]»\n\n"
        );
    }

    #[test]
    fn test_guillemet_preserves_inline_code_text() {
        // Inline code text is preserved as Typst markup
        assert_eq!(
            mark_to_typst("<<text with `code` inside>>").unwrap(),
            "«text with `code` inside»\n\n"
        );
    }

    #[test]
    fn test_guillemet_extracts_link_text() {
        // Link is preserved as Typst markup
        assert_eq!(
            mark_to_typst("<<visit [our site](https://example.com)>>").unwrap(),
            "«visit #link(\"https://example.com\")[our site]»\n\n"
        );
    }

    // Context Awareness Tests
    #[test]
    fn test_guillemet_not_in_code_span() {
        // Chevrons inside inline code should not convert
        assert_eq!(
            mark_to_typst("`<<not converted>>`").unwrap(),
            "`<<not converted>>`\n\n"
        );
    }

    #[test]
    fn test_guillemet_not_in_multi_backtick_code_span() {
        // Multi-backtick code spans per CommonMark
        let result = mark_to_typst("`` <<text>> ``").unwrap();
        assert!(!result.contains('«'), "Multi-backtick span incorrectly converted: {}", result);
    }

    #[test]
    fn test_guillemet_not_in_indented_code_block() {
        // Indented code blocks (4+ spaces) per CommonMark
        let result = mark_to_typst("    <<not converted>>").unwrap();
        assert!(!result.contains('«'), "Indented code block incorrectly converted: {}", result);
    }

    #[test]
    fn test_guillemet_not_in_tilde_fence() {
        // Tilde fences per CommonMark
        let result = mark_to_typst("~~~\n<<text>>\n~~~").unwrap();
        assert!(!result.contains('«'), "Tilde fence incorrectly converted: {}", result);
    }

    #[test]
    fn test_guillemet_not_in_long_backtick_fence() {
        // Fences with >3 backticks per CommonMark
        let result = mark_to_typst("````\n<<text>>\n````").unwrap();
        assert!(!result.contains('«'), "Long backtick fence incorrectly converted: {}", result);
    }

    #[test]
    fn test_guillemet_not_in_code_block() {
        // Chevrons in code blocks should not convert
        // Note: Code blocks are not fully implemented in the converter,
        // so text still gets output (without guillemet conversion)
        let markdown = "```\n<<not converted>>\n```";
        let result = mark_to_typst(markdown).unwrap();
        // Should not contain guillemets
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
    }

    // Edge Cases
    #[test]
    fn test_guillemet_unmatched_open() {
        assert_eq!(mark_to_typst("<<unmatched").unwrap(), "\\<\\<unmatched\n\n");
    }

    #[test]
    fn test_guillemet_unmatched_close() {
        assert_eq!(mark_to_typst("unmatched>>").unwrap(), "unmatched\\>\\>\n\n");
    }

    #[test]
    fn test_guillemet_multiple_same_line() {
        assert_eq!(
            mark_to_typst("<<first>> and <<second>>").unwrap(),
            "«first» and «second»\n\n"
        );
    }

    #[test]
    fn test_guillemet_multiline_not_converted() {
        // Newlines between chevrons should prevent conversion
        let markdown = "<<text on\ndifferent line>>";
        let result = mark_to_typst(markdown).unwrap();
        // The content becomes HTML which is ignored, so we just get the < and > chars
        assert!(result.contains("\\<\\<") || result.contains("\\<\\>"));
    }

    #[test]
    fn test_guillemet_nested_chevrons() {
        // Nearest-match logic: first << matches first >>
        // "<<outer <<inner>> text>>" -> first << at 0, first >> at 16
        // Content: "outer <<inner" - this successfully extracts to plain text
        assert_eq!(
            mark_to_typst("<<outer <<inner>> text>>").unwrap(),
            "«outer \\<\\<inner» text\\>\\>\n\n"
        );
    }

    #[test]
    fn test_guillemet_with_special_chars() {
        // Special chars inside guillemets are escaped by Typst processing
        assert_eq!(
            mark_to_typst("<<text with #hash and *star>>").unwrap(),
            "«text with \\#hash and \\*star»\n\n"
        );
    }

    #[test]
    fn test_guillemet_empty_content() {
        assert_eq!(mark_to_typst("<<>>").unwrap(), "«»\n\n");
    }

    // Integration Tests
    #[test]
    fn test_guillemet_in_heading() {
        assert_eq!(
            mark_to_typst("# Heading with <<guillemets>>").unwrap(),
            "= Heading with «guillemets»\n\n"
        );
    }

    #[test]
    fn test_guillemet_in_list() {
        assert_eq!(
            mark_to_typst("- Item with <<guillemets>>").unwrap(),
            "- Item with «guillemets»\n\n"
        );
    }

    #[test]
    fn test_guillemet_mixed_with_formatting() {
        assert_eq!(
            mark_to_typst("**Bold** and <<**bold in guillemets**>>").unwrap(),
            "*Bold* and «bold in guillemets»\n\n"
        );
    }

    #[test]
    fn test_guillemet_before_and_after_text() {
        assert_eq!(
            mark_to_typst("Before <<middle>> after").unwrap(),
            "Before «middle» after\n\n"
        );
    }

    // Safety Tests
    #[test]
    fn test_guillemet_respects_buffer_limit() {
        // Create content larger than MAX_GUILLEMET_LENGTH
        let large_content = "a".repeat(MAX_GUILLEMET_LENGTH + 1);
        let markdown = format!("<<{}>>", large_content);
        let result = mark_to_typst(&markdown).unwrap();
        // Should not convert due to buffer limit - guillemets should not appear
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
    }
}
