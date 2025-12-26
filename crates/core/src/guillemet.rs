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

/// Maximum length for guillemet content (single line, 64 KiB)
pub const MAX_GUILLEMET_LENGTH: usize = 64 * 1024;

/// Finds the position of `>>` that matches an opening `<<`, returns offset from search start
fn find_matching_guillemet_end(chars: &[char]) -> Option<usize> {
    (0..chars.len().saturating_sub(1)).find(|&i| chars[i] == '>' && chars[i + 1] == '>')
}

/// Counts consecutive occurrences of a character from the start of the slice.
fn count_consecutive(chars: &[char], target: char) -> usize {
    chars.iter().take_while(|&&c| c == target).count()
}

/// Counts leading spaces (not tabs) at the start of the slice.
fn count_leading_spaces(chars: &[char]) -> usize {
    chars.iter().take_while(|&&c| c == ' ').count()
}

/// Internal implementation for guillemet preprocessing with optional markdown awareness.
///
/// When `skip_code_blocks` is true, guillemet conversion is skipped inside:
/// - Fenced code blocks (` ``` ` or `~~~`)
/// - Indented code blocks (4+ spaces)
/// - Inline code spans (backticks)
fn preprocess_guillemets_impl(text: &str, skip_code_blocks: bool) -> String {
    // Early exit: if no closing >> exists, no conversion is possible
    // This prevents O(n²) complexity on pathological input with many unmatched <<
    if !text.contains(">>") {
        return text.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    // Markdown-specific state (only used when skip_code_blocks is true)
    let mut fence_state: Option<(char, usize)> = None;
    let mut inline_code_backticks: Option<usize> = None;
    let mut at_line_start = true;

    while i < chars.len() {
        let ch = chars[i];

        // Markdown code block handling (only when skip_code_blocks is enabled)
        if skip_code_blocks {
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
            if fence_state.is_none() && inline_code_backticks.is_none() && (ch == '`' || ch == '~')
            {
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
        }

        // Process << when found (common to both modes)
        if i + 1 < chars.len() && ch == '<' && chars[i + 1] == '<' {
            // Find matching >>
            if let Some(end_offset) = find_matching_guillemet_end(&chars[i + 2..]) {
                let content_end = i + 2 + end_offset;
                let content: String = chars[i + 2..content_end].iter().collect();

                // Check constraints: same line and size limit
                if !content.contains('\n') && content.len() <= MAX_GUILLEMET_LENGTH {
                    // Trim leading/trailing whitespace
                    result.push('«');
                    result.push_str(content.trim());
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
#[inline]
pub fn preprocess_guillemets(text: &str) -> String {
    preprocess_guillemets_impl(text, false)
}

/// Preprocesses markdown to convert guillemets: `<<text>>` → `«text»`
///
/// This is a markdown-aware conversion that skips guillemet conversion inside:
/// - Fenced code blocks (` ``` ` or `~~~`)
/// - Indented code blocks (4+ spaces)
/// - Inline code spans (backticks)
///
/// # Examples
///
/// ```
/// use quillmark_core::guillemet::preprocess_markdown_guillemets;
///
/// let result = preprocess_markdown_guillemets("<<hello>>");
/// assert_eq!(result, "«hello»");
///
/// // Code spans are not converted
/// let result = preprocess_markdown_guillemets("`<<code>>`");
/// assert_eq!(result, "`<<code>>`");
/// ```
#[inline]
pub fn preprocess_markdown_guillemets(markdown: &str) -> String {
    preprocess_guillemets_impl(markdown, true)
}

/// Strips chevrons from text, extracting inner content: `<<text>>` → `text`
///
/// This is used for YAML field values where we want to interpolate the content
/// without any surrounding markers (neither chevrons nor guillemets).
///
/// Constraints:
/// - Content must be on a single line (no newlines between `<<` and `>>`)
/// - Content must not exceed [`MAX_GUILLEMET_LENGTH`] bytes
///
/// # Examples
///
/// ```
/// use quillmark_core::guillemet::strip_chevrons;
///
/// assert_eq!(strip_chevrons("<<hello>>"), "hello");
/// assert_eq!(strip_chevrons("<< spaced >>"), "spaced");
/// assert_eq!(strip_chevrons("no chevrons"), "no chevrons");
/// assert_eq!(strip_chevrons("<<one>> and <<two>>"), "one and two");
/// ```
pub fn strip_chevrons(text: &str) -> String {
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
                    // Output just the trimmed content (no chevrons, no guillemets)
                    result.push_str(content.trim());
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

use crate::parse::BODY_FIELD;
use crate::value::QuillValue;
use std::collections::HashMap;

/// Preprocess guillemets in a map of QuillValue fields.
///
/// This function processes the body field to convert `<<text>>` to `«text»`
/// with markdown-awareness (skipping code blocks). Other YAML field values
/// have chevrons stripped to extract just the inner content (`<<text>>` → `text`).
///
/// # Examples
///
/// ```
/// use quillmark_core::guillemet::preprocess_fields_guillemets;
/// use quillmark_core::QuillValue;
/// use std::collections::HashMap;
///
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), QuillValue::from_json(serde_json::json!("<<hello>>")));
/// fields.insert("BODY".to_string(), QuillValue::from_json(serde_json::json!("<<world>>")));
///
/// let result = preprocess_fields_guillemets(fields);
/// // YAML fields have chevrons stripped
/// assert_eq!(result.get("title").unwrap().as_str().unwrap(), "hello");
/// // Body field converts to guillemets
/// assert_eq!(result.get("BODY").unwrap().as_str().unwrap(), "«world»");
/// ```
pub fn preprocess_fields_guillemets(
    fields: HashMap<String, QuillValue>,
) -> HashMap<String, QuillValue> {
    fields
        .into_iter()
        .map(|(key, value)| {
            let json = value.into_json();
            let processed = preprocess_json_value(json, key == BODY_FIELD);
            (key, QuillValue::from_json(processed))
        })
        .collect()
}

/// Recursively preprocess guillemets in a JSON value.
/// The `is_body` flag indicates whether this is the body field or nested within it.
/// Body content undergoes chevron-to-guillemet conversion (`<<text>>` → `«text»`).
/// YAML field values have chevrons stripped, leaving just the inner content (`<<text>>` → `text`).
fn preprocess_json_value(value: serde_json::Value, is_body: bool) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            let processed = if is_body {
                // Body field: convert <<text>> to «text»
                preprocess_markdown_guillemets(&s)
            } else {
                // YAML fields: strip chevrons, keep inner content
                strip_chevrons(&s)
            };
            serde_json::Value::String(processed)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.into_iter()
                .map(|v| preprocess_json_value(v, false))
                .collect(),
        ),
        serde_json::Value::Object(map) => {
            let processed_map: serde_json::Map<String, serde_json::Value> = map
                .into_iter()
                .map(|(k, v)| {
                    let is_body = k == BODY_FIELD;
                    (k, preprocess_json_value(v, is_body))
                })
                .collect();
            serde_json::Value::Object(processed_map)
        }
        // Pass through other types unchanged (numbers, booleans, null)
        other => other,
    }
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
        let result = preprocess_markdown_guillemets("<<text>>");
        assert_eq!(result, "«text»");
    }

    #[test]
    fn test_markdown_not_in_code_span() {
        let result = preprocess_markdown_guillemets("`<<code>>`");
        assert_eq!(result, "`<<code>>`");
    }

    #[test]
    fn test_markdown_not_in_multi_backtick_code_span() {
        let result = preprocess_markdown_guillemets("`` <<text>> ``");
        assert!(result.contains("<<text>>"));
    }

    #[test]
    fn test_markdown_not_in_fenced_code_block() {
        let result = preprocess_markdown_guillemets("```\n<<text>>\n```");
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
    }

    #[test]
    fn test_markdown_not_in_tilde_fence() {
        let result = preprocess_markdown_guillemets("~~~\n<<text>>\n~~~");
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
    }

    #[test]
    fn test_markdown_not_in_indented_code_block() {
        let result = preprocess_markdown_guillemets("    <<not converted>>");
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
    }

    #[test]
    fn test_markdown_multiple_same_line() {
        let result = preprocess_markdown_guillemets("<<one>> and <<two>>");
        assert_eq!(result, "«one» and «two»");
    }

    #[test]
    fn test_markdown_respects_buffer_limit() {
        // Create content larger than MAX_GUILLEMET_LENGTH
        let large_content = "a".repeat(MAX_GUILLEMET_LENGTH + 1);
        let markdown = format!("<<{}>>", large_content);
        let result = preprocess_markdown_guillemets(&markdown);
        // Should not convert due to buffer limit
        assert!(!result.contains('«'));
        assert!(!result.contains('»'));
    }

    #[test]
    fn test_markdown_mixed_context() {
        // Some inside code, some outside
        let result = preprocess_markdown_guillemets("<<converted>> and `<<not converted>>`");
        assert!(result.contains("«converted»"));
        assert!(result.contains("`<<not converted>>`"));
    }

    // Additional robustness tests

    #[test]
    fn test_simple_unicode_content() {
        // Multi-byte UTF-8 characters should work correctly
        assert_eq!(preprocess_guillemets("<<你好>>"), "«你好»");
        assert_eq!(preprocess_guillemets("<<émoji 🎉>>"), "«émoji 🎉»");
    }

    #[test]
    fn test_simple_only_whitespace_content() {
        // Content that is only whitespace gets trimmed to empty
        assert_eq!(preprocess_guillemets("<<   >>"), "«»");
        assert_eq!(preprocess_guillemets("<<\t\t>>"), "«»");
    }

    #[test]
    fn test_simple_triple_chevrons() {
        // <<< should match the first << with >>
        assert_eq!(preprocess_guillemets("<<<text>>>"), "«<text»>");
    }

    #[test]
    fn test_simple_adjacent_guillemets() {
        // Two guillemets directly adjacent
        assert_eq!(preprocess_guillemets("<<a>><<b>>"), "«a»«b»");
    }

    #[test]
    fn test_simple_single_chevron_inside() {
        // Single < or > inside should be preserved
        assert_eq!(preprocess_guillemets("<<a < b>>"), "«a < b»");
        assert_eq!(preprocess_guillemets("<<a > b>>"), "«a > b»");
    }

    #[test]
    fn test_simple_carriage_return_not_converted() {
        // \r without \n should still work (content.contains('\n') checks \n only)
        // But \r\n is a newline on Windows, so this tests \r alone
        assert_eq!(preprocess_guillemets("<<text\rhere>>"), "«text\rhere»");
    }

    #[test]
    fn test_markdown_unclosed_code_span() {
        // Unclosed backtick - should still not convert what appears to be in code
        let result = preprocess_markdown_guillemets("`<<code>>");
        // The backtick opens a code span that never closes, so everything after is in code
        assert!(!result.contains('«'));
    }

    #[test]
    fn test_markdown_fence_with_info_string() {
        // Fenced code block with language info string
        let result = preprocess_markdown_guillemets("```rust\n<<text>>\n```");
        assert!(!result.contains('«'));
    }

    #[test]
    fn test_markdown_nested_fences_different_chars() {
        // ~~~ inside ``` should not close the fence
        let result = preprocess_markdown_guillemets("```\n~~~\n<<text>>\n~~~\n```");
        assert!(!result.contains('«'));
    }

    #[test]
    fn test_markdown_longer_closing_fence() {
        // Closing fence can be longer than opening
        let result = preprocess_markdown_guillemets("```\n<<text>>\n`````");
        assert!(!result.contains('«'));
    }

    #[test]
    fn test_markdown_indented_block_after_content() {
        // Indented code only triggers at line start
        let result = preprocess_markdown_guillemets("text    <<convert>>");
        assert!(result.contains("«convert»"));
    }

    #[test]
    fn test_markdown_tabs_not_indented_code() {
        // Tabs don't count as indented code block (only spaces)
        let result = preprocess_markdown_guillemets("\t<<should convert>>");
        assert!(result.contains("«should convert»"));
    }

    #[test]
    fn test_markdown_three_spaces_not_code() {
        // 3 spaces is not an indented code block (needs 4+)
        let result = preprocess_markdown_guillemets("   <<should convert>>");
        assert!(result.contains("«should convert»"));
    }

    #[test]
    fn test_markdown_content_preserved() {
        // Verify content is correctly preserved in output
        let result = preprocess_markdown_guillemets("<<hello>>");
        assert_eq!(result, "«hello»");
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_markdown_unicode_content() {
        // Verify unicode content works
        let result = preprocess_markdown_guillemets("<<你好>>");
        assert_eq!(result, "«你好»");
        assert!(result.contains("你好"));
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(preprocess_guillemets(""), "");
        let result = preprocess_markdown_guillemets("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_only_chevrons() {
        // Just << or >> alone
        assert_eq!(preprocess_guillemets("<<"), "<<");
        assert_eq!(preprocess_guillemets(">>"), ">>");
        assert_eq!(preprocess_guillemets("<"), "<");
        assert_eq!(preprocess_guillemets(">"), ">");
    }

    #[test]
    fn test_early_exit_no_closing_chevrons() {
        // If there's no >> at all, the early exit optimization should kick in
        // and return the input unchanged immediately (prevents O(n²) scanning)
        let input_without_closing = "<<<<<<<<<<<<<<<<<<<<text<<<<<<<<<<";
        assert_eq!(
            preprocess_guillemets(input_without_closing),
            input_without_closing
        );

        let input_with_only_opening = "Some <<text and <<more <<here";
        assert_eq!(
            preprocess_guillemets(input_with_only_opening),
            input_with_only_opening
        );
    }
}
