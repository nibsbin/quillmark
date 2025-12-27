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
//! // Output: "This is #strong[bold] and #emph[italic].\n\n"
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
use quillmark_core::error::MAX_NESTING_DEPTH;
use std::ops::Range;

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
///
/// This function escapes all Typst-special characters to prevent:
/// - Markup injection (*, _, `, #, etc.)
/// - Layout manipulation (~, which is non-breaking space in Typst)
/// - Reference injection (@)
/// - Code/comment injection (//, $, etc.)
pub fn escape_markup(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace("//", "\\/\\/")
        .replace('~', "\\~") // Non-breaking space in Typst
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
fn push_typst<'a, I>(output: &mut String, source: &str, iter: I) -> Result<(), ConversionError>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    let mut end_newline = true;
    let mut list_stack: Vec<ListType> = Vec::new();
    let mut strong_stack: Vec<StrongKind> = Vec::new();
    let mut in_list_item = false; // Track if we're inside a list item
    let mut need_para_space = false; // Track if we need space before next paragraph in list item
    let mut depth = 0; // Track nesting depth for DoS prevention
    let iter = iter.peekable();

    for (event, range) in iter {
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
                        } else if need_para_space {
                            // Add space to join with previous paragraph in list item
                            output.push(' ');
                            end_newline = false;
                        }
                        // Typst doesn't need explicit paragraph tags for simple paragraphs
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
                        need_para_space = false; // Reset paragraph space tracker
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
                        output.push_str("#emph[");
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
                        match kind {
                            StrongKind::Underline => output.push_str("#underline["),
                            StrongKind::Bold => output.push_str("#strong["),
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
                        output.push_str(&escape_string(&dest_url));
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
                        } else {
                            // Mark that the next paragraph in this list item needs a space
                            // This ensures "First line.\n\nSecond line." becomes "First line. Second line."
                            // matching the behavior of soft breaks (single newline)
                            need_para_space = true;
                        }
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
                        output.push(']');
                        end_newline = false;
                    }
                    TagEnd::Strong => {
                        match strong_stack.pop() {
                            Some(StrongKind::Bold) | Some(StrongKind::Underline) => {
                                output.push(']');
                            }
                            None => {
                                // Malformed: more end tags than start tags
                                output.push(']');
                            }
                        }
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

/// Iterator that post-processes markdown events to handle specific edge cases:
/// 1. Coalesces adjacent `Text` events to enable intraword underscore emphasis (fix for `__` sandwich).
/// 2. Fixes `***` adjacency issues (fix for `***` sandwich).
struct MarkdownFixer<'a, I: Iterator<Item = (Event<'a>, Range<usize>)>> {
    inner: std::iter::Peekable<I>,
    source: &'a str,
    /// Buffer of events to emit before pulling from inner
    buffer: Vec<(Event<'a>, Range<usize>)>,
}

impl<'a, I> MarkdownFixer<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    fn new(inner: I, source: &'a str) -> Self {
        Self {
            inner: inner.peekable(),
            source,
            buffer: Vec::new(),
        }
    }

    /// Helper to generate events for a run of stars
    fn events_for_stars(
        star_count: usize,
        is_start: bool,
        start_idx: usize,
    ) -> Vec<(Event<'a>, Range<usize>)> {
        let mut events = Vec::new();
        let mut offset = 0;
        let mut remaining = star_count;

        // 3 stars = Strong + Emph (***)
        // 2 stars = Strong (**)
        // 1 star = Emph (*)

        if remaining >= 2 {
            let len = 2;
            let range = start_idx + offset..start_idx + offset + len;
            let event = if is_start {
                Event::Start(Tag::Strong)
            } else {
                Event::End(TagEnd::Strong)
            };
            events.push((event, range));
            remaining -= 2;
            offset += 2;
        }

        if remaining >= 1 {
            let len = 1;
            let range = start_idx + offset..start_idx + offset + len;
            let event = if is_start {
                Event::Start(Tag::Emphasis)
            } else {
                Event::End(TagEnd::Emphasis)
            };
            events.push((event, range));
        }

        // For closing tags, we need to reverse the order to close inner then outer
        // Opened: Strong, Emph -> Closes: Emph, Strong
        if !is_start {
            events.reverse();
        }

        events
    }

    /// Coalesce consecutive Text events into a single range.
    /// Returns the merged range covering all adjacent Text events.
    fn coalesce_text_range(&mut self, initial_range: Range<usize>) -> Range<usize> {
        let mut merged_range = initial_range;

        // Keep consuming Text events as long as they're adjacent
        while let Some((next_event, next_range)) = self.inner.peek() {
            if matches!(next_event, Event::Text(_)) && next_range.start == merged_range.end {
                merged_range.end = next_range.end;
                self.inner.next(); // Consume the peeked event
            } else {
                break;
            }
        }

        merged_range
    }

    /// Process a text range, potentially splitting it into multiple events with Strong markers.
    /// Uses the source slice directly based on range.
    fn process_text_from_source(&mut self, range: Range<usize>) {
        let source_slice = &self.source[range.clone()];

        // Skip processing for HTML entities (complex to handle correctly)
        if source_slice.contains('&') {
            self.buffer.push((Event::Text(source_slice.into()), range));
            return;
        }

        // Look for __ patterns in the source text, handling escapes
        let mut events: Vec<(Event<'a>, Range<usize>)> = Vec::new();
        let mut in_underline = false;
        let mut last_end = 0;
        let mut i = 0;
        let bytes = source_slice.as_bytes();

        while i < bytes.len() {
            // Skip over escaped characters (backslash followed by any char)
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2; // Skip the backslash and the escaped character
                continue;
            }

            if i + 1 < bytes.len() && bytes[i] == b'_' && bytes[i + 1] == b'_' {
                // Found __
                let before = &source_slice[last_end..i];
                if !before.is_empty() {
                    events.push((
                        Event::Text(before.into()),
                        range.start + last_end..range.start + i,
                    ));
                }

                let marker_range = range.start + i..range.start + i + 2;
                if in_underline {
                    // Close underline
                    events.push((Event::End(TagEnd::Strong), marker_range));
                    in_underline = false;
                } else {
                    // Open underline
                    events.push((Event::Start(Tag::Strong), marker_range));
                    in_underline = true;
                }

                i += 2;
                last_end = i;
            } else {
                i += 1;
            }
        }

        // Emit remaining text
        let remaining = &source_slice[last_end..];
        if !remaining.is_empty() {
            events.push((
                Event::Text(remaining.into()),
                range.start + last_end..range.end,
            ));
        }

        // If we have an unclosed underline, we need to undo the transformation
        // (just emit original text)
        if in_underline {
            #[cfg(debug_assertions)]
            eprintln!("Warning: Unclosed __ in text: {:?}", source_slice);

            self.buffer.push((Event::Text(source_slice.into()), range));
        } else if events.is_empty() {
            self.buffer.push((Event::Text(source_slice.into()), range));
        } else {
            // Events need to be reversed since we pop from buffer
            events.reverse();
            self.buffer.extend(events);
        }
    }

    fn handle_candidate(
        &mut self,
        candidate: (Event<'a>, Range<usize>),
    ) -> Option<(Event<'a>, Range<usize>)> {
        let (event, range) = candidate;

        match &event {
            Event::Text(cow_str) => {
                let s = cow_str.as_ref();
                if s.ends_with('*') {
                    // Peek next event
                    let is_strong_start = if let Some(next) = self.buffer.last() {
                        matches!(next.0, Event::Start(Tag::Strong))
                    } else {
                        matches!(self.inner.peek(), Some((Event::Start(Tag::Strong), _)))
                    };

                    if is_strong_start {
                        let star_count = s.chars().rev().take_while(|c| *c == '*').count();
                        if star_count > 0 && star_count <= 3 {
                            let text_len = s.len() - star_count;
                            let text_content = &s[..text_len];
                            // Generate star events
                            let star_events =
                                Self::events_for_stars(star_count, true, range.start + text_len);

                            // Consume next event
                            let next_event = if !self.buffer.is_empty() {
                                self.buffer.pop().unwrap()
                            } else {
                                self.inner.next().unwrap()
                            };

                            // Push reverse
                            self.buffer.push(next_event);
                            for ev in star_events.into_iter().rev() {
                                self.buffer.push(ev);
                            }

                            if !text_content.is_empty() {
                                return Some((
                                    Event::Text(text_content.to_string().into()),
                                    range.start..range.start + text_len,
                                ));
                            } else {
                                return None;
                            }
                        }
                    }
                }
            }
            Event::End(TagEnd::Strong) | Event::End(TagEnd::Emphasis) => {
                // Check if next event starts with *, which means we might need to fix closing tags
                // This happens when we have something like __Underlined__***
                // The __ produces End(Strong), and following *** should be interpreted as closing.

                // Peek next event (from buffer or inner)
                let next_is_star_text = if let Some((Event::Text(cow_str), _)) = self.buffer.last()
                {
                    cow_str.starts_with('*')
                } else if let Some((Event::Text(cow_str), _)) = self.inner.peek() {
                    cow_str.starts_with('*')
                } else {
                    false
                };

                if next_is_star_text {
                    // Retrieve the text event
                    let (text_event, text_range) = if !self.buffer.is_empty() {
                        self.buffer.pop().unwrap()
                    } else {
                        // Coalesce text from inner iterator
                        let (_ev, rng) = self.inner.next().unwrap();
                        let merged_range = self.coalesce_text_range(rng);
                        let text = self.source[merged_range.clone()].into();
                        (Event::Text(text), merged_range)
                    };

                    if let Event::Text(cow_str) = text_event {
                        let s = cow_str.as_ref();
                        let star_count = s.chars().take_while(|c| *c == '*').count();

                        if star_count > 0 && star_count <= 3 {
                            // Perform fix: Close tags using stars
                            let star_events =
                                Self::events_for_stars(star_count, false, text_range.start);
                            let text_after = &s[star_count..];

                            // We emit the `End(Strong)` event (which caused this check).

                            // We need to push remaining text first (so it comes out last)
                            if !text_after.is_empty() {
                                self.buffer.push((
                                    Event::Text(text_after.to_string().into()),
                                    text_range.start + star_count..text_range.end,
                                ));
                            }

                            // Then push star events (reversed)
                            // Self::events_for_stars returns [End(Emph), End(Strong)] (if 3 stars)
                            // We want End(Strong), then End(Emph) to be popped.
                            // So we push End(Strong) (bottom), then End(Emph) (top).
                            // This is exactly reversed order.
                            for ev in star_events.into_iter().rev() {
                                self.buffer.push(ev);
                            }

                            return Some((event, range));
                        } else {
                            // Should not happen, put back
                            self.buffer.push((Event::Text(cow_str), text_range));
                        }
                    }
                }
            }
            _ => {}
        }

        Some((event, range))
    }
}

impl<'a, I> Iterator for MarkdownFixer<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    type Item = (Event<'a>, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 1. Process buffer
            if let Some(event) = self.buffer.pop() {
                if let Some(result) = self.handle_candidate(event) {
                    return Some(result);
                } else {
                    // handle_candidate pushed to buffer and returned None
                    continue;
                }
            }

            // 2. Pull from inner
            let (event, range) = self.inner.next()?;

            // 3. Process Event
            if let Event::Text(_) = &event {
                let merged_range = self.coalesce_text_range(range);
                let source_slice = &self.source[merged_range.clone()];

                if source_slice.contains("__") {
                    self.process_text_from_source(merged_range);
                    // Buffer is populated, loop continues to process buffer
                    continue;
                } else {
                    // Fallthrough to handle_candidate for merged text
                    // We need to pass the merged event
                    if let Some(result) =
                        self.handle_candidate((Event::Text(source_slice.into()), merged_range))
                    {
                        return Some(result);
                    } else {
                        continue;
                    }
                }
            }

            // Handle other events via handle_candidate (checks for End(Strong))
            if let Some(result) = self.handle_candidate((event, range)) {
                return Some(result);
            } else {
                continue;
            }
        }
    }
}

pub fn mark_to_typst(markdown: &str) -> Result<String, ConversionError> {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);
    let fixer = MarkdownFixer::new(parser.into_offset_iter(), markdown);
    let mut typst_output = String::new();

    push_typst(&mut typst_output, markdown, fixer)?;
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

    #[test]
    fn test_escape_markup_tilde() {
        // Tilde is non-breaking space in Typst - must be escaped to prevent layout manipulation
        assert_eq!(escape_markup("Hello~World"), "Hello\\~World");
        assert_eq!(escape_markup("a~b~c"), "a\\~b\\~c");
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
            "This is #strong[bold], #emph[italic], and #strike[strikethrough] text.\n\n"
        );
    }

    #[test]
    fn test_bold_formatting() {
        assert_eq!(mark_to_typst("**bold**").unwrap(), "#strong[bold]\n\n");
        assert_eq!(
            mark_to_typst("This is **bold** text").unwrap(),
            "This is #strong[bold] text\n\n"
        );
    }

    #[test]
    fn test_italic_formatting() {
        assert_eq!(mark_to_typst("_italic_").unwrap(), "#emph[italic]\n\n");
        assert_eq!(mark_to_typst("*italic*").unwrap(), "#emph[italic]\n\n");
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
            "A paragraph with #strong[bold] and a #link(\"https://example.com\")[link].\n\nAnother paragraph with `inline code`.\n\n- A list item\n- Another item\n\n"
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
        assert_eq!(typst, "#strong[bold] #emph[italic] #strike[strike]\n\n");
    }

    #[test]
    fn test_nested_formatting() {
        let markdown = "**bold _and italic_**";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#strong[bold #emph[and italic]]\n\n");
    }

    #[test]
    fn test_list_with_formatting() {
        let markdown = "- **Bold** item\n- _Italic_ item\n- `Code` item";
        let typst = mark_to_typst(markdown).unwrap();
        // Lists end with extra newline
        assert_eq!(
            typst,
            "- #strong[Bold] item\n- #emph[Italic] item\n- `Code` item\n\n"
        );
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
    fn test_list_item_paragraph_separation_with_space() {
        // Two newlines in a list item should join text with a space
        // (matching the behavior of single newlines / soft breaks)
        let markdown = "- First line.\n\n  Second line.";
        let typst = mark_to_typst(markdown).unwrap();
        // Previously this was "- First line.Second line." (missing space)
        // Now it should be "- First line. Second line."
        assert_eq!(typst, "- First line. Second line.\n\n");
    }

    #[test]
    fn test_link_with_special_chars_in_url() {
        let markdown = "[Link](https://example.com/foo_bar)";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#link(\"https://example.com/foo_bar\")[Link]\n\n");
    }

    #[test]
    fn test_emphasis_sandwich_bug() {
        // Bug: ***__...__***asdf fails to parse outer ***
        let markdown = "***__Underlined__***suffix";
        let typst = mark_to_typst(markdown).unwrap();
        // Expect: #strong[#emph[#underline[Underlined]]]suffix
        // Or similar nesting.
        // If it fails, it will likely be: \*\*\*#underline[Underlined]\*\*\*suffix
        assert_eq!(typst, "#strong[#emph[#underline[Underlined]]]suffix\n\n");
    }

    #[test]
    fn test_link_with_anchor() {
        // URLs don't need # escaped in Typst string literals
        let markdown = "[Link](#anchor)";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#link(\"#anchor\")[Link]\n\n");
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
        // Function syntax (#emph[]) handles word boundaries naturally
        let markdown = "*Write y*our paragraphs here.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#emph[Write y]our paragraphs here.\n\n");
    }

    #[test]
    fn test_italic_followed_by_space() {
        let markdown = "*italic* text";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#emph[italic] text\n\n");
    }

    #[test]
    fn test_italic_followed_by_punctuation() {
        let markdown = "*italic*.";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#emph[italic].\n\n");
    }

    #[test]
    fn test_bold_followed_by_alphanumeric() {
        // Function syntax (#strong[]) handles word boundaries naturally
        let markdown = "**bold**text";
        let typst = mark_to_typst(markdown).unwrap();
        assert_eq!(typst, "#strong[bold]text\n\n");
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
        assert_eq!(typst, "== Heading with #strong[bold] and #emph[italic]\n\n");
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
        assert_eq!(mark_to_typst("**bold**").unwrap(), "#strong[bold]\n\n");
    }

    // Nesting Tests
    #[test]
    fn test_underline_containing_bold() {
        assert_eq!(
            mark_to_typst("__A **B** A__").unwrap(),
            "#underline[A #strong[B] A]\n\n"
        );
    }

    #[test]
    fn test_bold_containing_underline() {
        assert_eq!(
            mark_to_typst("**A __B__ A**").unwrap(),
            "#strong[A #underline[B] A]\n\n"
        );
    }

    #[test]
    fn test_deep_nesting() {
        assert_eq!(
            mark_to_typst("__A **B __C__ B** A__").unwrap(),
            "#underline[A #strong[B #underline[C] B] A]\n\n"
        );
    }

    // Adjacent Styles Tests
    #[test]
    fn test_adjacent_underline_bold() {
        assert_eq!(
            mark_to_typst("__A__**B**").unwrap(),
            "#underline[A]#strong[B]\n\n"
        );
    }

    #[test]
    fn test_adjacent_bold_underline() {
        assert_eq!(
            mark_to_typst("**A**__B__").unwrap(),
            "#strong[A]#underline[B]\n\n"
        );
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
            "#underline[underline #emph[italic]]\n\n"
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

    // Tests for intraword underscore emphasis (EmphasisFixer)
    #[test]
    fn test_intraword_underscore_emphasis() {
        // This is the user's reported issue: underscores in middle of word
        assert_eq!(
            mark_to_typst("the cow __jumped over the mo__on").unwrap(),
            "the cow #underline[jumped over the mo]on\n\n"
        );
    }

    #[test]
    fn test_intraword_underscore_start_of_word() {
        // Underscore starts mid-word
        assert_eq!(
            mark_to_typst("foo__bar__baz").unwrap(),
            "foo#underline[bar]baz\n\n"
        );
    }

    #[test]
    fn test_escaped_intraword_underscore() {
        // Escaped underscores should remain literal
        let result = mark_to_typst("foo\\__bar").unwrap();
        // Contains literal underscore, not underline
        assert!(result.contains("\\_"));
        assert!(!result.contains("#underline"));
    }
}

// Additional robustness tests
#[cfg(test)]
mod robustness_tests {
    use super::*;

    // Empty and edge case inputs

    #[test]
    fn test_only_newlines() {
        let result = mark_to_typst("\n\n\n").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_only_spaces_and_newlines() {
        let result = mark_to_typst("   \n   \n   ").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_single_character() {
        assert_eq!(mark_to_typst("a").unwrap(), "a\n\n");
    }

    #[test]
    fn test_single_special_character() {
        // Note: Single * at line start is parsed as a list marker by pulldown-cmark
        // Single # at line start is parsed as a heading marker
        // So we test with characters in context where they're literal
        assert_eq!(mark_to_typst("a # b").unwrap(), "a \\# b\n\n");
        assert_eq!(mark_to_typst("$").unwrap(), "\\$\n\n");
        // Asterisk in middle of text is escaped
        assert_eq!(mark_to_typst("a * b").unwrap(), "a \\* b\n\n");
    }

    // Unicode handling

    #[test]
    fn test_unicode_text() {
        let result = mark_to_typst("你好世界").unwrap();
        assert_eq!(result, "你好世界\n\n");
    }

    #[test]
    fn test_unicode_with_formatting() {
        let result = mark_to_typst("**你好** _世界_").unwrap();
        assert_eq!(result, "#strong[你好] #emph[世界]\n\n");
    }

    #[test]
    fn test_emoji() {
        let result = mark_to_typst("Hello 🎉 World 🚀").unwrap();
        assert_eq!(result, "Hello 🎉 World 🚀\n\n");
    }

    #[test]
    fn test_emoji_in_link() {
        let result = mark_to_typst("[Click 🎉](https://example.com)").unwrap();
        assert_eq!(result, "#link(\"https://example.com\")[Click 🎉]\n\n");
    }

    #[test]
    fn test_rtl_text() {
        // Arabic text
        let result = mark_to_typst("مرحبا بالعالم").unwrap();
        assert_eq!(result, "مرحبا بالعالم\n\n");
    }

    // Escape edge cases

    #[test]
    fn test_multiple_consecutive_slashes() {
        let result = mark_to_typst("a///b").unwrap();
        // /// should become \/\// (first // escaped, third / stays)
        assert!(result.contains("\\/\\/"));
    }

    #[test]
    fn test_escape_at_string_boundaries() {
        // Test escaping at start of string
        assert!(mark_to_typst("*start").unwrap().starts_with("\\*"));
        // Test escaping at end of string
        assert!(mark_to_typst("end*").unwrap().contains("end\\*"));
    }

    #[test]
    fn test_backslash_before_special_char() {
        // Backslash followed by special char - both should be escaped
        let result = mark_to_typst("\\*").unwrap();
        // In markdown, \* is an escaped asterisk, becomes literal *
        // Then we escape it for Typst
        assert!(result.contains("\\*"));
    }

    #[test]
    fn test_all_special_chars_together() {
        let result = mark_to_typst("*_`#[]$<>@\\").unwrap();
        assert!(result.contains("\\*"));
        assert!(result.contains("\\_"));
        assert!(result.contains("\\`"));
        assert!(result.contains("\\#"));
        assert!(result.contains("\\["));
        assert!(result.contains("\\]"));
        assert!(result.contains("\\$"));
        assert!(result.contains("\\<"));
        assert!(result.contains("\\>"));
        assert!(result.contains("\\@"));
        assert!(result.contains("\\\\"));
    }

    // Link edge cases

    #[test]
    fn test_link_with_quotes_in_url() {
        let result = mark_to_typst("[link](https://example.com?q=\"test\")").unwrap();
        assert!(result.contains("\\\"test\\\""));
    }

    #[test]
    fn test_link_with_backslash_in_url() {
        let result = mark_to_typst("[link](https://example.com\\path)").unwrap();
        assert!(result.contains("\\\\"));
    }

    #[test]
    fn test_link_with_newline_in_text() {
        // Markdown link text can span lines with soft breaks
        let result = mark_to_typst("[multi\nline](https://example.com)").unwrap();
        // Soft break becomes space in link text
        assert!(result.contains("[multi line]"));
    }

    #[test]
    fn test_empty_link_text() {
        let result = mark_to_typst("[](https://example.com)").unwrap();
        assert_eq!(result, "#link(\"https://example.com\")[]\n\n");
    }

    #[test]
    fn test_link_with_special_chars_in_text() {
        let result = mark_to_typst("[*bold* link](https://example.com)").unwrap();
        assert!(result.contains("#emph[bold]"));
    }

    // List edge cases

    #[test]
    fn test_empty_list_item() {
        let result = mark_to_typst("- \n- item").unwrap();
        // Empty list items are valid
        assert!(result.contains("- "));
    }

    #[test]
    fn test_list_with_multiple_paragraphs() {
        let markdown = "- First para\n\n  Second para in same item";
        let result = mark_to_typst(markdown).unwrap();
        assert!(result.contains("First para"));
    }

    #[test]
    fn test_very_deeply_nested_list() {
        // Create a list nested 10 levels deep (within limit)
        let mut markdown = String::new();
        for i in 0..10 {
            markdown.push_str(&"  ".repeat(i));
            markdown.push_str("- item\n");
        }
        let result = mark_to_typst(&markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_ordered_unordered_nested() {
        let markdown = "1. First\n   - Nested bullet\n   - Another bullet\n2. Second";
        let result = mark_to_typst(markdown).unwrap();
        assert!(result.contains("+ First"));
        assert!(result.contains("- Nested bullet"));
        assert!(result.contains("+ Second"));
    }

    // Heading edge cases

    #[test]
    fn test_heading_with_only_special_chars() {
        let result = mark_to_typst("# ***").unwrap();
        assert!(result.contains("= "));
    }

    #[test]
    fn test_heading_followed_by_list() {
        let result = mark_to_typst("# Heading\n\n- Item").unwrap();
        assert!(result.contains("= Heading\n\n"));
        assert!(result.contains("- Item"));
    }

    #[test]
    fn test_consecutive_headings() {
        let result = mark_to_typst("# One\n## Two\n### Three").unwrap();
        assert!(result.contains("= One"));
        assert!(result.contains("== Two"));
        assert!(result.contains("=== Three"));
    }

    // Code block handling (currently ignored but should not crash)

    #[test]
    fn test_fenced_code_block_ignored() {
        let markdown = "```rust\nfn main() {}\n```";
        let result = mark_to_typst(markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_indented_code_block_ignored() {
        let markdown = "    fn main() {}\n    println!()";
        let result = mark_to_typst(markdown);
        assert!(result.is_ok());
    }

    // Inline code edge cases

    #[test]
    fn test_inline_code_with_backticks() {
        // Using double backticks to include single backtick
        let result = mark_to_typst("`` `code` ``").unwrap();
        assert!(result.contains("`"));
    }

    #[test]
    fn test_inline_code_with_special_chars() {
        // Special chars in code should NOT be escaped
        let result = mark_to_typst("`*#$<>`").unwrap();
        assert_eq!(result, "`*#$<>`\n\n");
    }

    #[test]
    fn test_empty_inline_code() {
        // pulldown-cmark doesn't parse `` as empty inline code
        // It needs content or different backtick counts
        let result = mark_to_typst("` `").unwrap();
        assert!(result.contains("`")); // space-only code span
    }

    // Formatting edge cases

    #[test]
    fn test_adjacent_emphasis() {
        let result = mark_to_typst("*a**b*").unwrap();
        // Depends on how markdown parser handles this
        assert!(result.contains("#emph["));
    }

    #[test]
    fn test_emphasis_across_words() {
        let result = mark_to_typst("*multiple words here*").unwrap();
        assert_eq!(result, "#emph[multiple words here]\n\n");
    }

    #[test]
    fn test_strong_across_lines() {
        let result = mark_to_typst("**bold\nacross\nlines**").unwrap();
        // Soft breaks become spaces
        assert!(result.contains("bold across lines"));
    }

    #[test]
    fn test_strikethrough_with_special_chars() {
        let result = mark_to_typst("~~*text*~~").unwrap();
        // Strikethrough content: emphasis should still work
        assert!(result.contains("#strike["));
    }

    // Strong stack edge cases

    #[test]
    fn test_multiple_nested_strong() {
        // Unusual but valid: nested strongs
        let result = mark_to_typst("**a **b** a**");
        assert!(result.is_ok());
    }

    #[test]
    fn test_alternating_bold_underline() {
        let result = mark_to_typst("**bold** __under__ **bold**").unwrap();
        assert!(result.contains("#strong[bold]"));
        assert!(result.contains("#underline[under]"));
    }

    // escape_string function tests

    #[test]
    fn test_escape_string_unicode() {
        // Unicode should pass through unchanged
        assert_eq!(escape_string("你好"), "你好");
        assert_eq!(escape_string("🎉"), "🎉");
    }

    #[test]
    fn test_escape_string_all_escapes() {
        assert_eq!(escape_string("\\\"\n\r\t"), "\\\\\\\"\\n\\r\\t");
    }

    #[test]
    fn test_escape_string_nul_character() {
        assert_eq!(escape_string("\x00"), "\\u{0}");
    }

    #[test]
    fn test_escape_string_bell_character() {
        assert_eq!(escape_string("\x07"), "\\u{7}");
    }

    #[test]
    fn test_escape_string_mixed() {
        assert_eq!(
            escape_string("Hello\nWorld\t\"quoted\""),
            "Hello\\nWorld\\t\\\"quoted\\\""
        );
    }

    // escape_markup function tests

    #[test]
    fn test_escape_markup_empty() {
        assert_eq!(escape_markup(""), "");
    }

    #[test]
    fn test_escape_markup_unicode() {
        // Unicode should pass through unchanged
        assert_eq!(escape_markup("你好世界"), "你好世界");
    }

    #[test]
    fn test_escape_markup_triple_slash() {
        // /// should escape the first // and leave the third /
        assert_eq!(escape_markup("///"), "\\/\\//");
    }

    #[test]
    fn test_escape_markup_url() {
        assert_eq!(
            escape_markup("https://example.com"),
            "https:\\/\\/example.com"
        );
    }

    // Paragraph handling

    #[test]
    fn test_many_paragraphs() {
        let markdown = "P1.\n\nP2.\n\nP3.\n\nP4.\n\nP5.";
        let result = mark_to_typst(markdown).unwrap();
        assert_eq!(result.matches("P").count(), 5);
        assert!(result.contains("P1.\n\nP2."));
    }

    #[test]
    fn test_paragraph_with_only_formatting() {
        let result = mark_to_typst("**bold only**").unwrap();
        assert_eq!(result, "#strong[bold only]\n\n");
    }

    // Soft break and hard break

    #[test]
    fn test_hard_break_in_list() {
        let result = mark_to_typst("- line one  \n  line two").unwrap();
        // Hard break in list item
        assert!(result.contains("line one"));
    }

    #[test]
    fn test_multiple_hard_breaks() {
        let result = mark_to_typst("a  \nb  \nc").unwrap();
        assert_eq!(result, "a\nb\nc\n\n");
    }

    // Word boundary handling (no longer needed with function syntax)

    #[test]
    fn test_italic_before_number() {
        let result = mark_to_typst("*italic*1").unwrap();
        // Function syntax handles word boundaries naturally
        assert!(result.contains("#emph[italic]1"));
    }

    #[test]
    fn test_bold_before_underscore() {
        // In **bold**_after, the _ is literal text (not starting emphasis)
        // So it gets escaped in Typst output
        let result = mark_to_typst("**bold**_after").unwrap();
        // Underscore is escaped as literal text
        assert!(result.contains("#strong[bold]\\_after"));
    }

    #[test]
    fn test_emphasis_at_end_of_text() {
        let result = mark_to_typst("*italic*").unwrap();
        assert_eq!(result, "#emph[italic]\n\n");
    }

    // Complex real-world scenarios

    #[test]
    fn test_markdown_document() {
        let markdown = r#"# Title

This is a paragraph with **bold** and *italic* text.

## Section

- List item 1
- List item 2 with [link](https://example.com)

More text with `inline code`."#;

        let result = mark_to_typst(markdown).unwrap();
        assert!(result.contains("= Title"));
        assert!(result.contains("== Section"));
        assert!(result.contains("#strong[bold]"));
        assert!(result.contains("#emph[italic]"));
        assert!(result.contains("- List item"));
        assert!(result.contains("#link"));
        assert!(result.contains("`inline code`"));
    }

    #[test]
    fn test_typst_syntax_in_content() {
        // Content that looks like Typst syntax should be escaped
        let markdown = "Use #set for settings and $x^2$ for math.";
        let result = mark_to_typst(markdown).unwrap();
        assert!(result.contains("\\#set"));
        assert!(result.contains("\\$x^2\\$"));
    }

    #[test]
    fn test_midword_italic() {
        // Function syntax handles mid-word emphasis naturally
        let markdown = "a*sdfasd*f";
        let result = mark_to_typst(markdown).unwrap();
        assert_eq!(result, "a#emph[sdfasd]f\n\n");
    }

    #[test]
    fn test_midword_bold() {
        // Function syntax handles mid-word bold naturally
        let markdown = "word**bold**more";
        let result = mark_to_typst(markdown).unwrap();
        assert_eq!(result, "word#strong[bold]more\n\n");
    }

    #[test]
    fn test_emphasis_preceded_by_alphanumeric() {
        // Function syntax handles this naturally
        let markdown = "text*emph*";
        let result = mark_to_typst(markdown).unwrap();
        assert_eq!(result, "text#emph[emph]\n\n");
    }

    #[test]
    fn test_emphasis_after_space() {
        let markdown = "some *italic* text";
        let result = mark_to_typst(markdown).unwrap();
        assert_eq!(result, "some #emph[italic] text\n\n");
    }

    #[test]
    fn test_emphasis_after_punctuation() {
        let markdown = "(*italic*)";
        let result = mark_to_typst(markdown).unwrap();
        assert_eq!(result, "(#emph[italic])\n\n");
    }
}
