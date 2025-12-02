//! # Guillemet Preprocessing
//!
//! This module provides preprocessing for converting double chevrons (`<<text>>`)
//! into French guillemets (`«text»`).
//!
//! ## Overview
//!
//! Guillemets are used in Quillmark as a lightweight syntax for marking raw/verbatim
//! content that should be passed through to the backend without markdown processing.
//!
//! ## Functions
//!
//! - [`preprocess_guillemets`] - Converts `<<text>>` to `«text»` in simple text
//! - [`preprocess_markdown_guillemets`] - Same conversion but skips code blocks/spans
//!
//! ## Examples
//!
//! ```
//! use quillmark_core::guillemet::preprocess_guillemets;
//!
//! let text = "Use <<raw content>> here";
//! let result = preprocess_guillemets(text);
//! assert_eq!(result, "Use «raw content» here");
//! ```

use std::ops::Range;

/// Maximum length for guillemet content (single line, 64 KiB)
pub const MAX_GUILLEMET_LENGTH: usize = 64 * 1024;

/// Finds the position of `>>` that matches an opening `<<`, returns offset from search start
fn find_matching_guillemet_end(chars: &[char]) -> Option<usize> {
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i] == '>' && chars[i + 1] == '>' {
            return Some(i);
        }
    }
    None
}

/// Trims whitespace from guillemet content.
fn trim_guillemet_content(content: &str) -> String {
    content.trim().to_string()
}

/// Counts consecutive occurrences of a character from the start of the slice.
fn count_consecutive(chars: &[char], target: char) -> usize {
    chars.iter().take_while(|&&c| c == target).count()
}

/// Counts leading spaces (not tabs) at the start of the slice.
fn count_leading_spaces(chars: &[char]) -> usize {
    chars.iter().take_while(|&&c| c == ' ').count()
}

/// Preprocesses text to convert guillemets: `<<text>>` → `«text»`
///
/// This is a simple conversion that does NOT skip code blocks or code spans.
/// Use this for YAML field values or other non-markdown contexts.
///
/// Constraints:
/// - Content must be on a single line (no newlines between `<<` and `>>`)
/// - Content must not exceed [`MAX_GUILLEMET_LENGTH`] bytes
///
/// # Examples
///
/// ```
/// use quillmark_core::guillemet::preprocess_guillemets;
///
/// assert_eq!(preprocess_guillemets("<<hello>>"), "«hello»");
/// assert_eq!(preprocess_guillemets("<< spaced >>"), "«spaced»");
/// assert_eq!(preprocess_guillemets("no chevrons"), "no chevrons");
/// ```
pub fn preprocess_guillemets(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // Process << when found
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
                    result.push_str(&clean);
                    result.push('»');
                    i = content_end + 2; // Skip past >>
                    continue;
                }
            }
        }

        // Regular character - just copy it
        result.push(ch);
        i += 1;
    }

    result
}

/// Preprocesses markdown to convert guillemets: `<<text>>` → `«text»`
///
/// This is a markdown-aware conversion that skips guillemet conversion inside:
/// - Fenced code blocks (` ``` ` or `~~~`)
/// - Indented code blocks (4+ spaces)
/// - Inline code spans (backticks)
///
/// Returns both the preprocessed source and the byte ranges for guillemet
/// content within the preprocessed source. This lets consumers perform
/// context-aware processing for events that occur inside guillemets.
///
/// # Examples
///
/// ```
/// use quillmark_core::guillemet::preprocess_markdown_guillemets;
///
/// let (result, ranges) = preprocess_markdown_guillemets("<<hello>>");
/// assert_eq!(result, "«hello»");
/// assert_eq!(ranges.len(), 1);
///
/// // Code spans are not converted
/// let (result, _) = preprocess_markdown_guillemets("`<<code>>`");
/// assert_eq!(result, "`<<code>>`");
/// ```
pub fn preprocess_markdown_guillemets(markdown: &str) -> (String, Vec<Range<usize>>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for preprocess_guillemets (simple)
    #[test]
    fn test_simple_guillemet() {
        assert_eq!(preprocess_guillemets("<<text>>"), "«text»");
    }

    #[test]
    fn test_simple_guillemet_with_spaces() {
        assert_eq!(preprocess_guillemets("<< spaced >>"), "«spaced»");
    }

    #[test]
    fn test_simple_no_conversion() {
        assert_eq!(preprocess_guillemets("no chevrons"), "no chevrons");
    }

    #[test]
    fn test_simple_unmatched_open() {
        assert_eq!(preprocess_guillemets("<<unmatched"), "<<unmatched");
    }

    #[test]
    fn test_simple_unmatched_close() {
        assert_eq!(preprocess_guillemets("unmatched>>"), "unmatched>>");
    }

    #[test]
    fn test_simple_multiple() {
        assert_eq!(
            preprocess_guillemets("<<one>> and <<two>>"),
            "«one» and «two»"
        );
    }

    #[test]
    fn test_simple_multiline_not_converted() {
        // Newlines between chevrons should prevent conversion
        assert_eq!(preprocess_guillemets("<<text\nhere>>"), "<<text\nhere>>");
    }

    #[test]
    fn test_simple_empty_content() {
        assert_eq!(preprocess_guillemets("<<>>"), "«»");
    }

    #[test]
    fn test_simple_nested_chevrons() {
        // Nearest-match logic: first << matches first >>
        assert_eq!(
            preprocess_guillemets("<<outer <<inner>> text>>"),
            "«outer <<inner» text>>"
        );
    }

    // Tests for preprocess_markdown_guillemets (markdown-aware)
    #[test]
    fn test_markdown_basic() {
        let (result, ranges) = preprocess_markdown_guillemets("<<text>>");
        assert_eq!(result, "«text»");
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_markdown_not_in_code_span() {
        let (result, ranges) = preprocess_markdown_guillemets("`<<code>>`");
        assert_eq!(result, "`<<code>>`");
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_markdown_not_in_multi_backtick_code_span() {
        let (result, ranges) = preprocess_markdown_guillemets("`` <<text>> ``");
        assert!(result.contains("<<text>>"));
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_markdown_not_in_fenced_code_block() {
        let (result, ranges) = preprocess_markdown_guillemets("```\n<<text>>\n```");
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_markdown_not_in_tilde_fence() {
        let (result, ranges) = preprocess_markdown_guillemets("~~~\n<<text>>\n~~~");
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_markdown_not_in_indented_code_block() {
        let (result, ranges) = preprocess_markdown_guillemets("    <<not converted>>");
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_markdown_multiple_same_line() {
        let (result, ranges) = preprocess_markdown_guillemets("<<one>> and <<two>>");
        assert_eq!(result, "«one» and «two»");
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_markdown_respects_buffer_limit() {
        // Create content larger than MAX_GUILLEMET_LENGTH
        let large_content = "a".repeat(MAX_GUILLEMET_LENGTH + 1);
        let markdown = format!("<<{}>>", large_content);
        let (result, ranges) = preprocess_markdown_guillemets(&markdown);
        // Should not convert due to buffer limit
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_markdown_mixed_context() {
        // Some inside code, some outside
        let (result, ranges) =
            preprocess_markdown_guillemets("<<converted>> and `<<not converted>>`");
        assert!(result.contains("«converted»"));
        assert!(result.contains("`<<not converted>>`"));
        assert_eq!(ranges.len(), 1);
    }
}
