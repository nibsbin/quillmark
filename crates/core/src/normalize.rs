//! # Input Normalization
//!
//! This module provides input normalization for markdown content before parsing.
//! Normalization ensures that invisible control characters and other artifacts
//! that can interfere with markdown parsing are handled consistently.
//!
//! ## Overview
//!
//! Input text may contain invisible Unicode characters (especially from copy-paste)
//! that interfere with markdown parsing. This module provides functions to:
//!
//! - Strip Unicode bidirectional formatting characters that break delimiter recognition
//! - Fix HTML comment fences to preserve trailing text
//! - Apply all normalizations in the correct order
//!
//! Double chevrons (`<<` and `>>`) are passed through unchanged without conversion.
//!
//! ## Functions
//!
//! - [`strip_bidi_formatting`] - Remove Unicode bidi control characters
//! - [`normalize_markdown`] - Apply all markdown-specific normalizations
//! - [`normalize_fields`] - Normalize document fields (bidi stripping)
//!
//! ## Why Normalize?
//!
//! Unicode bidirectional formatting characters (LRO, RLO, LRE, RLE, etc.) are invisible
//! control characters used for bidirectional text layout. When placed adjacent to markdown
//! delimiters like `**`, they can prevent parsers from recognizing the delimiters:
//!
//! ```text
//! **bold** or <U+202D>**(1234**
//!             ^^^^^^^^ invisible LRO here prevents second ** from being recognized as bold
//! ```
//!
//! These characters commonly appear when copying text from:
//! - Web pages with mixed LTR/RTL content
//! - PDF documents
//! - Word processors
//! - Some clipboard managers
//!
//! ## Examples
//!
//! ```
//! use quillmark_core::normalize::strip_bidi_formatting;
//!
//! // Input with invisible U+202D (LRO) before second **
//! let input = "**asdf** or \u{202D}**(1234**";
//! let cleaned = strip_bidi_formatting(input);
//! assert_eq!(cleaned, "**asdf** or **(1234**");
//! ```

use crate::parse::BODY_FIELD;
use crate::value::QuillValue;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

/// Errors that can occur during normalization
#[derive(Debug, thiserror::Error)]
pub enum NormalizationError {
    /// JSON nesting depth exceeded maximum allowed
    #[error("JSON nesting too deep: {depth} levels (max: {max} levels)")]
    NestingTooDeep {
        /// Actual depth
        depth: usize,
        /// Maximum allowed depth
        max: usize,
    },
}

/// Check if a character is a Unicode bidirectional formatting character
#[inline]
fn is_bidi_char(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' // ARABIC LETTER MARK (ALM)
        | '\u{200E}' // LEFT-TO-RIGHT MARK (LRM)
        | '\u{200F}' // RIGHT-TO-LEFT MARK (RLM)
        | '\u{202A}' // LEFT-TO-RIGHT EMBEDDING (LRE)
        | '\u{202B}' // RIGHT-TO-LEFT EMBEDDING (RLE)
        | '\u{202C}' // POP DIRECTIONAL FORMATTING (PDF)
        | '\u{202D}' // LEFT-TO-RIGHT OVERRIDE (LRO)
        | '\u{202E}' // RIGHT-TO-LEFT OVERRIDE (RLO)
        | '\u{2066}' // LEFT-TO-RIGHT ISOLATE (LRI)
        | '\u{2067}' // RIGHT-TO-LEFT ISOLATE (RLI)
        | '\u{2068}' // FIRST STRONG ISOLATE (FSI)
        | '\u{2069}' // POP DIRECTIONAL ISOLATE (PDI)
    )
}

/// Strips Unicode bidirectional formatting characters that can interfere with markdown parsing.
///
/// These invisible control characters are used for bidirectional text layout but can
/// break markdown delimiter recognition when placed adjacent to `**`, `*`, `_`, etc.
///
/// # Characters Stripped
///
/// - U+061C (ARABIC LETTER MARK, ALM)
/// - U+200E (LEFT-TO-RIGHT MARK, LRM)
/// - U+200F (RIGHT-TO-LEFT MARK, RLM)
/// - U+202A (LEFT-TO-RIGHT EMBEDDING, LRE)
/// - U+202B (RIGHT-TO-LEFT EMBEDDING, RLE)
/// - U+202C (POP DIRECTIONAL FORMATTING, PDF)
/// - U+202D (LEFT-TO-RIGHT OVERRIDE, LRO)
/// - U+202E (RIGHT-TO-LEFT OVERRIDE, RLO)
/// - U+2066 (LEFT-TO-RIGHT ISOLATE, LRI)
/// - U+2067 (RIGHT-TO-LEFT ISOLATE, RLI)
/// - U+2068 (FIRST STRONG ISOLATE, FSI)
/// - U+2069 (POP DIRECTIONAL ISOLATE, PDI)
///
/// # Examples
///
/// ```
/// use quillmark_core::normalize::strip_bidi_formatting;
///
/// // Normal text is unchanged
/// assert_eq!(strip_bidi_formatting("hello"), "hello");
///
/// // LRO character is stripped
/// assert_eq!(strip_bidi_formatting("he\u{202D}llo"), "hello");
///
/// // All bidi characters are stripped
/// let input = "\u{200E}\u{200F}\u{202A}\u{202B}\u{202C}\u{202D}\u{202E}";
/// assert_eq!(strip_bidi_formatting(input), "");
/// ```
pub fn strip_bidi_formatting(s: &str) -> String {
    // Early return optimization: avoid allocation if no bidi characters present
    if !s.chars().any(is_bidi_char) {
        return s.to_string();
    }

    s.chars().filter(|c| !is_bidi_char(*c)).collect()
}

/// Fixes HTML comment closing fences to prevent content loss.
///
/// According to CommonMark, HTML block type 2 (comments) ends with the line containing `-->`.
/// This means any text on the same line after `-->` is included in the HTML block and would
/// be discarded by markdown parsers that ignore HTML blocks.
///
/// This function inserts a newline after `-->` when followed by non-whitespace content,
/// ensuring the trailing text is parsed as regular markdown.
///
/// # Examples
///
/// ```
/// use quillmark_core::normalize::fix_html_comment_fences;
///
/// // Text on same line as --> is moved to next line
/// assert_eq!(
///     fix_html_comment_fences("<!-- comment -->Some text"),
///     "<!-- comment -->\nSome text"
/// );
///
/// // Already on separate line - no change
/// assert_eq!(
///     fix_html_comment_fences("<!-- comment -->\nSome text"),
///     "<!-- comment -->\nSome text"
/// );
///
/// // Only whitespace after --> - no change needed
/// assert_eq!(
///     fix_html_comment_fences("<!-- comment -->   \nSome text"),
///     "<!-- comment -->   \nSome text"
/// );
///
/// // Multi-line comments with trailing text
/// assert_eq!(
///     fix_html_comment_fences("<!--\nmultiline\n-->Trailing text"),
///     "<!--\nmultiline\n-->\nTrailing text"
/// );
/// ```
pub fn fix_html_comment_fences(s: &str) -> String {
    // Early return if no HTML comment closing fence present
    if !s.contains("-->") {
        return s.to_string();
    }

    // Context-aware processing: only fix `-->` if we are inside a comment started by `<!--`
    let mut result = String::with_capacity(s.len() + 16);
    let mut current_pos = 0;

    // Find first opener
    while let Some(open_idx) = s[current_pos..].find("<!--") {
        let abs_open = current_pos + open_idx;

        // Find matching closer AFTER the opener
        if let Some(close_idx) = s[abs_open..].find("-->") {
            let abs_close = abs_open + close_idx;
            let mut after_fence = abs_close + 3;

            // Handle `<!--- ... --->` style fences by treating the extra
            // hyphen as part of the comment content, not leaked trailing text.
            // 4 == "<!--".len(); check whether opener is `<!---` (extra hyphen).
            let opener_has_extra_hyphen = s
                .get(abs_open + 4..)
                .is_some_and(|rest| rest.starts_with('-'));
            if opener_has_extra_hyphen
                && s.get(after_fence..)
                    .is_some_and(|rest| rest.starts_with('-'))
            {
                after_fence += 1;
            }

            // Append everything up to and including the closing fence
            result.push_str(&s[current_pos..after_fence]);

            // Check what comes after the fence
            let after_content = &s[after_fence..];

            // Determine if we need to insert a newline
            let needs_newline = if after_content.is_empty() {
                false
            } else if after_content.starts_with('\n') || after_content.starts_with("\r\n") {
                false
            } else {
                // Check if there's only whitespace until end of line
                let next_newline = after_content.find('\n');
                let until_newline = match next_newline {
                    Some(pos) => &after_content[..pos],
                    None => after_content,
                };
                !until_newline.trim().is_empty()
            };

            if needs_newline {
                result.push('\n');
            }

            // Move position to after the fence (we'll process the rest in next iteration)
            current_pos = after_fence;
        } else {
            // Unclosed comment at end of string - just append the rest and break
            // The opener was found but no closer exists.
            result.push_str(&s[current_pos..]);
            current_pos = s.len();
            break;
        }
    }

    // Append remaining content (text after last closed comment, or text if no comments found)
    if current_pos < s.len() {
        result.push_str(&s[current_pos..]);
    }

    result
}

/// Normalizes markdown content by applying all preprocessing steps.
///
/// This function applies normalizations in the correct order:
/// 1. Strip Unicode bidirectional formatting characters
/// 2. Fix HTML comment closing fences (ensure text after `-->` is preserved)
///
/// Note: Guillemet preprocessing (`<<text>>` → `«text»`) is handled separately
/// in [`normalize_fields`] because it needs to be applied after schema defaults
/// and coercion.
///
/// # Examples
///
/// ```
/// use quillmark_core::normalize::normalize_markdown;
///
/// // Bidi characters are stripped
/// let input = "**bold** \u{202D}**more**";
/// let normalized = normalize_markdown(input);
/// assert_eq!(normalized, "**bold** **more**");
///
/// // HTML comment trailing text is preserved
/// let with_comment = "<!-- comment -->Some text";
/// let normalized = normalize_markdown(with_comment);
/// assert_eq!(normalized, "<!-- comment -->\nSome text");
/// ```
pub fn normalize_markdown(markdown: &str) -> String {
    let cleaned = strip_bidi_formatting(markdown);
    fix_html_comment_fences(&cleaned)
}

/// Normalize a single card object: apply `normalize_markdown` to the `BODY` key only.
/// All other fields in the card pass through verbatim.
fn normalize_card_object(
    map: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    map.into_iter()
        .map(|(k, v)| {
            if k == BODY_FIELD {
                let normalized = match v {
                    serde_json::Value::String(s) => {
                        serde_json::Value::String(normalize_markdown(&s))
                    }
                    other => other,
                };
                (k, normalized)
            } else {
                // All other card fields pass through verbatim
                (k, v)
            }
        })
        .collect()
}

/// Normalize the `CARDS` array: for each element that is an object, normalize its
/// `BODY` field via `normalize_markdown`; all other card fields pass through verbatim.
/// Non-object elements (malformed) pass through unchanged.
fn normalize_cards_array(arr: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    arr.into_iter()
        .map(|elem| match elem {
            serde_json::Value::Object(map) => serde_json::Value::Object(normalize_card_object(map)),
            other => other,
        })
        .collect()
}


/// Normalizes document fields per the Quillmark §7 spec.
///
/// Only **body regions** receive normalization (bidi stripping + HTML comment fence
/// repair). All other field values — including nested objects and arrays — pass
/// through verbatim so that YAML scalar values are never silently mutated.
///
/// Specifically:
/// - The top-level `BODY` field is fully normalized via [`normalize_markdown`].
/// - Each object inside the top-level `CARDS` array has its own `BODY` field
///   normalized via [`normalize_markdown`]; all other fields in those objects
///   pass through unchanged.
/// - Every other top-level field (strings, numbers, booleans, nested maps,
///   arrays of scalars, etc.) passes through verbatim.
///
/// Field names at the top level are NFC-normalized (see [`normalize_field_name`]).
/// Keys inside nested objects are **not** NFC-normalized.
///
/// Double chevrons (`<<` and `>>`) are passed through unchanged in all fields.
///
/// # Examples
///
/// ```
/// use quillmark_core::normalize::normalize_fields;
/// use quillmark_core::QuillValue;
/// use std::collections::HashMap;
///
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), QuillValue::from_json(serde_json::json!("<<hello>>")));
/// fields.insert("BODY".to_string(), QuillValue::from_json(serde_json::json!("**bold** \u{202D}**more**")));
///
/// let result = normalize_fields(fields);
///
/// // Title passes through verbatim (no bidi stripping on YAML fields)
/// assert_eq!(result.get("title").unwrap().as_str().unwrap(), "<<hello>>");
///
/// // Body has bidi chars stripped and HTML comment fences repaired
/// assert_eq!(result.get("BODY").unwrap().as_str().unwrap(), "**bold** **more**");
/// ```
pub fn normalize_fields(fields: HashMap<String, QuillValue>) -> HashMap<String, QuillValue> {
    const CARDS_FIELD: &str = "CARDS";

    fields
        .into_iter()
        .map(|(key, value)| {
            // Normalize field name to NFC form for consistent key comparison.
            // This ensures café (composed) and café (decomposed) are treated as the same key.
            // NFC normalization is applied to top-level keys only.
            let normalized_key = normalize_field_name(&key);

            let processed = if normalized_key == BODY_FIELD {
                // Top-level BODY: full markdown normalization (bidi + HTML fence repair).
                let json = value.into_json();
                let normalized = match json {
                    serde_json::Value::String(s) => {
                        serde_json::Value::String(normalize_markdown(&s))
                    }
                    other => other,
                };
                QuillValue::from_json(normalized)
            } else if normalized_key == CARDS_FIELD {
                // CARDS array: normalize only the BODY field inside each card object.
                let json = value.into_json();
                let normalized = match json {
                    serde_json::Value::Array(arr) => {
                        serde_json::Value::Array(normalize_cards_array(arr))
                    }
                    other => other,
                };
                QuillValue::from_json(normalized)
            } else {
                // All other top-level fields pass through verbatim.
                value
            };

            (normalized_key, processed)
        })
        .collect()
}

/// Normalize field name to Unicode NFC (Canonical Decomposition, followed by Canonical Composition)
///
/// This ensures that equivalent Unicode strings (e.g., "café" composed vs decomposed)
/// are treated as identical field names, preventing subtle bugs where visually
/// identical keys are treated as different.
///
/// # Examples
///
/// ```
/// use quillmark_core::normalize::normalize_field_name;
///
/// // Composed form (single code point for é)
/// let composed = "café";
/// // Decomposed form (e + combining acute accent)
/// let decomposed = "cafe\u{0301}";
///
/// // Both normalize to the same NFC form
/// assert_eq!(normalize_field_name(composed), normalize_field_name(decomposed));
/// ```
pub fn normalize_field_name(name: &str) -> String {
    name.nfc().collect()
}

/// Normalizes a parsed document by applying all field-level normalizations.
///
/// This is the **primary entry point** for normalizing documents after parsing.
/// It ensures consistent processing regardless of how the document was created.
///
/// # Normalization Steps
///
/// This function applies all normalizations in the correct order:
/// 1. **Unicode NFC normalization** - Field names are normalized to NFC form
/// 2. **Bidi stripping** - Invisible bidirectional control characters are removed
/// 3. **HTML comment fence fixing** - Trailing text after `-->` is preserved (body only)
///
/// Double chevrons (`<<` and `>>`) are passed through unchanged without conversion.
///
/// # When to Use
///
/// Call this function after parsing and before rendering:
///
/// ```no_run
/// use quillmark_core::{ParsedDocument, normalize::normalize_document};
///
/// let markdown = "---\ntitle: Example\n---\n\nBody with <<placeholder>>";
/// let doc = ParsedDocument::from_markdown(markdown).unwrap();
/// let normalized = normalize_document(doc);
/// // Use normalized document for rendering...
/// ```
///
/// # Direct API Usage
///
/// If you're constructing a `ParsedDocument` directly via [`crate::parse::ParsedDocument::new`]
/// rather than parsing from markdown, you **MUST** call this function to ensure
/// consistent normalization:
///
/// ```
/// use quillmark_core::{ParsedDocument, QuillValue, normalize::normalize_document};
/// use quillmark_core::version::QuillReference;
/// use std::collections::HashMap;
///
/// // Direct construction (e.g., from API or database)
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), QuillValue::from_json(serde_json::json!("Test")));
/// fields.insert("BODY".to_string(), QuillValue::from_json(serde_json::json!("<<content>>")));
///
/// let quill_ref = QuillReference::latest("my_quill".to_string());
/// let doc = ParsedDocument::new(fields, quill_ref);
/// let normalized = normalize_document(doc).expect("Failed to normalize document");
///
/// // Body has chevrons preserved
/// assert_eq!(normalized.body().unwrap(), "<<content>>");
/// ```
///
/// # Idempotency
///
/// This function is idempotent - calling it multiple times produces the same result.
/// However, for performance reasons, avoid unnecessary repeated calls.
pub fn normalize_document(
    doc: crate::parse::ParsedDocument,
) -> Result<crate::parse::ParsedDocument, crate::error::ParseError> {
    let normalized_fields = normalize_fields(doc.fields().clone());
    Ok(crate::parse::ParsedDocument::new(
        normalized_fields,
        doc.quill_reference().clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for strip_bidi_formatting

    #[test]
    fn test_strip_bidi_no_change() {
        assert_eq!(strip_bidi_formatting("hello world"), "hello world");
        assert_eq!(strip_bidi_formatting(""), "");
        assert_eq!(strip_bidi_formatting("**bold** text"), "**bold** text");
    }

    #[test]
    fn test_strip_bidi_lro() {
        // U+202D (LEFT-TO-RIGHT OVERRIDE)
        assert_eq!(strip_bidi_formatting("he\u{202D}llo"), "hello");
        assert_eq!(
            strip_bidi_formatting("**asdf** or \u{202D}**(1234**"),
            "**asdf** or **(1234**"
        );
    }

    #[test]
    fn test_strip_bidi_rlo() {
        // U+202E (RIGHT-TO-LEFT OVERRIDE)
        assert_eq!(strip_bidi_formatting("he\u{202E}llo"), "hello");
    }

    #[test]
    fn test_strip_bidi_marks() {
        // U+200E (LRM) and U+200F (RLM)
        assert_eq!(strip_bidi_formatting("a\u{200E}b\u{200F}c"), "abc");
    }

    #[test]
    fn test_strip_bidi_embeddings() {
        // U+202A (LRE), U+202B (RLE), U+202C (PDF)
        assert_eq!(
            strip_bidi_formatting("\u{202A}text\u{202B}more\u{202C}"),
            "textmore"
        );
    }

    #[test]
    fn test_strip_bidi_isolates() {
        // U+2066 (LRI), U+2067 (RLI), U+2068 (FSI), U+2069 (PDI)
        assert_eq!(
            strip_bidi_formatting("\u{2066}a\u{2067}b\u{2068}c\u{2069}"),
            "abc"
        );
    }

    #[test]
    fn test_strip_bidi_all_chars() {
        let all_bidi = "\u{061C}\u{200E}\u{200F}\u{202A}\u{202B}\u{202C}\u{202D}\u{202E}\u{2066}\u{2067}\u{2068}\u{2069}";
        assert_eq!(strip_bidi_formatting(all_bidi), "");
    }

    #[test]
    fn test_strip_bidi_arabic_letter_mark() {
        // U+061C ARABIC LETTER MARK (ALM) should be stripped
        assert_eq!(strip_bidi_formatting("hello\u{061C}world"), "helloworld");
        assert_eq!(strip_bidi_formatting("\u{061C}**bold**"), "**bold**");
    }

    #[test]
    fn test_strip_bidi_unicode_preserved() {
        // Non-bidi unicode should be preserved
        assert_eq!(strip_bidi_formatting("你好世界"), "你好世界");
        assert_eq!(strip_bidi_formatting("مرحبا"), "مرحبا");
        assert_eq!(strip_bidi_formatting("🎉"), "🎉");
    }

    // Tests for normalize_markdown

    #[test]
    fn test_normalize_markdown_basic() {
        assert_eq!(normalize_markdown("hello"), "hello");
        assert_eq!(
            normalize_markdown("**bold** \u{202D}**more**"),
            "**bold** **more**"
        );
    }

    #[test]
    fn test_normalize_markdown_html_comment() {
        assert_eq!(
            normalize_markdown("<!-- comment -->Some text"),
            "<!-- comment -->\nSome text"
        );
    }

    // Tests for fix_html_comment_fences

    #[test]
    fn test_fix_html_comment_no_comment() {
        assert_eq!(fix_html_comment_fences("hello world"), "hello world");
        assert_eq!(fix_html_comment_fences("**bold** text"), "**bold** text");
        assert_eq!(fix_html_comment_fences(""), "");
    }

    #[test]
    fn test_fix_html_comment_single_line_trailing_text() {
        // Text on same line as --> should be moved to next line
        assert_eq!(
            fix_html_comment_fences("<!-- comment -->Same line text"),
            "<!-- comment -->\nSame line text"
        );
    }

    #[test]
    fn test_fix_html_comment_already_newline() {
        // Already has newline after --> - no change
        assert_eq!(
            fix_html_comment_fences("<!-- comment -->\nNext line text"),
            "<!-- comment -->\nNext line text"
        );
    }

    #[test]
    fn test_fix_html_comment_only_whitespace_after() {
        // Only whitespace after --> until newline - no change needed
        assert_eq!(
            fix_html_comment_fences("<!-- comment -->   \nSome text"),
            "<!-- comment -->   \nSome text"
        );
    }

    #[test]
    fn test_fix_html_comment_multiline_trailing_text() {
        // Multi-line comment with text on closing line
        assert_eq!(
            fix_html_comment_fences("<!--\nmultiline\ncomment\n-->Trailing text"),
            "<!--\nmultiline\ncomment\n-->\nTrailing text"
        );
    }

    #[test]
    fn test_fix_html_comment_multiline_proper() {
        // Multi-line comment with proper newline after -->
        assert_eq!(
            fix_html_comment_fences("<!--\nmultiline\n-->\n\nParagraph text"),
            "<!--\nmultiline\n-->\n\nParagraph text"
        );
    }

    #[test]
    fn test_fix_html_comment_multiple_comments() {
        // Multiple comments in the same document
        assert_eq!(
            fix_html_comment_fences("<!-- first -->Text\n\n<!-- second -->More text"),
            "<!-- first -->\nText\n\n<!-- second -->\nMore text"
        );
    }

    #[test]
    fn test_fix_html_comment_end_of_string() {
        // Comment at end of string - no trailing content
        assert_eq!(
            fix_html_comment_fences("Some text before <!-- comment -->"),
            "Some text before <!-- comment -->"
        );
    }

    #[test]
    fn test_fix_html_comment_only_comment() {
        // Just a comment with nothing after
        assert_eq!(
            fix_html_comment_fences("<!-- comment -->"),
            "<!-- comment -->"
        );
    }

    #[test]
    fn test_fix_html_comment_arrow_not_comment() {
        // --> that's not part of a comment (standalone)
        // Should NOT be touched by the context-aware fixer
        assert_eq!(fix_html_comment_fences("-->some text"), "-->some text");
    }

    #[test]
    fn test_fix_html_comment_nested_opener() {
        // Nested openers are just text inside the comment
        // <!-- <!-- -->Trailing
        // The first <!-- opens, the first --> closes.
        assert_eq!(
            fix_html_comment_fences("<!-- <!-- -->Trailing"),
            "<!-- <!-- -->\nTrailing"
        );
    }

    #[test]
    fn test_fix_html_comment_unmatched_closer() {
        // Closer without opener
        assert_eq!(
            fix_html_comment_fences("text --> more text"),
            "text --> more text"
        );
    }

    #[test]
    fn test_fix_html_comment_multiple_valid_invalid() {
        // Mixed valid and invalid comments
        // <!-- valid -->FixMe
        // text --> Ignore
        // <!-- valid2 -->FixMe2
        let input = "<!-- valid -->FixMe\ntext --> Ignore\n<!-- valid2 -->FixMe2";
        let expected = "<!-- valid -->\nFixMe\ntext --> Ignore\n<!-- valid2 -->\nFixMe2";
        assert_eq!(fix_html_comment_fences(input), expected);
    }

    #[test]
    fn test_fix_html_comment_crlf() {
        // CRLF line endings
        assert_eq!(
            fix_html_comment_fences("<!-- comment -->\r\nSome text"),
            "<!-- comment -->\r\nSome text"
        );
    }

    #[test]
    fn test_fix_html_comment_triple_hyphen_single_line() {
        assert_eq!(
            fix_html_comment_fences("<!--- comment --->Trailing text"),
            "<!--- comment --->\nTrailing text"
        );
    }

    #[test]
    fn test_fix_html_comment_triple_hyphen_multiline() {
        assert_eq!(
            fix_html_comment_fences("<!---\ncomment\n--->Trailing text"),
            "<!---\ncomment\n--->\nTrailing text"
        );
    }

    // Tests for normalize_fields

    #[test]
    fn test_normalize_fields_body_bidi() {
        let mut fields = HashMap::new();
        fields.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("**bold** \u{202D}**more**")),
        );

        let result = normalize_fields(fields);
        assert_eq!(
            result.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "**bold** **more**"
        );
    }

    #[test]
    fn test_normalize_fields_body_chevrons_preserved() {
        let mut fields = HashMap::new();
        fields.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("<<raw>>")),
        );

        let result = normalize_fields(fields);
        // Chevrons are passed through unchanged
        assert_eq!(result.get(BODY_FIELD).unwrap().as_str().unwrap(), "<<raw>>");
    }

    #[test]
    fn test_normalize_fields_body_chevrons_and_bidi() {
        let mut fields = HashMap::new();
        fields.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("<<raw>> \u{202D}**bold**")),
        );

        let result = normalize_fields(fields);
        // Bidi stripped, chevrons preserved
        assert_eq!(
            result.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "<<raw>> **bold**"
        );
    }

    #[test]
    fn test_normalize_fields_other_field_chevrons_preserved() {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("<<hello>>")),
        );

        let result = normalize_fields(fields);
        // Chevrons are passed through unchanged
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "<<hello>>");
    }

    #[test]
    fn test_normalize_fields_other_field_bidi_preserved() {
        // Per spec §7: bidi stripping is NOT applied to YAML field values.
        // Only body regions are normalized.
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("a\u{202D}b")),
        );

        let result = normalize_fields(fields);
        // Bidi character must be PRESERVED in non-body fields
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "a\u{202D}b");
    }

    #[test]
    fn test_normalize_fields_nested_values_verbatim() {
        // Nested arrays inside YAML fields pass through verbatim (no bidi stripping).
        let mut fields = HashMap::new();
        fields.insert(
            "items".to_string(),
            QuillValue::from_json(serde_json::json!(["<<a>>", "\u{202D}b"])),
        );

        let result = normalize_fields(fields);
        let items = result.get("items").unwrap().as_array().unwrap();
        // All values pass through unchanged — no bidi stripping on YAML fields
        assert_eq!(items[0].as_str().unwrap(), "<<a>>");
        assert_eq!(items[1].as_str().unwrap(), "\u{202D}b");
    }

    #[test]
    fn test_normalize_fields_object_values_verbatim() {
        // Nested objects inside YAML fields pass through verbatim.
        // Even if a nested key happens to be named BODY, it is NOT a body region.
        let mut fields = HashMap::new();
        fields.insert(
            "meta".to_string(),
            QuillValue::from_json(serde_json::json!({
                "title": "a\u{202D}b",
                BODY_FIELD: "c\u{202D}d"
            })),
        );

        let result = normalize_fields(fields);
        let meta = result.get("meta").unwrap();
        let meta_obj = meta.as_object().unwrap();
        // Both fields pass through verbatim — nested objects are not body regions
        assert_eq!(
            meta_obj.get("title").unwrap().as_str().unwrap(),
            "a\u{202D}b"
        );
        assert_eq!(
            meta_obj.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "c\u{202D}d"
        );
    }

    #[test]
    fn test_normalize_fields_non_string_unchanged() {
        let mut fields = HashMap::new();
        fields.insert(
            "count".to_string(),
            QuillValue::from_json(serde_json::json!(42)),
        );
        fields.insert(
            "enabled".to_string(),
            QuillValue::from_json(serde_json::json!(true)),
        );

        let result = normalize_fields(fields);
        assert_eq!(result.get("count").unwrap().as_i64().unwrap(), 42);
        assert!(result.get("enabled").unwrap().as_bool().unwrap());
    }

    // ── §7 spec contract tests ──────────────────────────────────────────────────
    // Cover all 8 cases required by MARKDOWN_GAPS.md §3.

    /// Case 1: Bidi character in top-level BODY → stripped.
    #[test]
    fn test_spec_case1_body_bidi_stripped() {
        let mut fields = HashMap::new();
        fields.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("hello\u{202D}world")),
        );
        let result = normalize_fields(fields);
        assert_eq!(
            result.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "helloworld"
        );
    }

    /// Case 2: Bidi character in a top-level YAML string field → PRESERVED.
    #[test]
    fn test_spec_case2_yaml_field_bidi_preserved() {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("a\u{202D}b")),
        );
        let result = normalize_fields(fields);
        // Must NOT be stripped — YAML field values pass through verbatim.
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "a\u{202D}b");
    }

    /// Case 3: Bidi character inside CARDS[0].BODY → stripped.
    #[test]
    fn test_spec_case3_card_body_bidi_stripped() {
        let card = serde_json::json!({
            "CARD": "profile",
            BODY_FIELD: "card\u{202D}body",
            "name": "Alice"
        });
        let mut fields = HashMap::new();
        fields.insert(
            "CARDS".to_string(),
            QuillValue::from_json(serde_json::json!([card])),
        );
        let result = normalize_fields(fields);
        let cards = result.get("CARDS").unwrap().as_array().unwrap();
        let body = cards[0].get(BODY_FIELD).unwrap().as_str().unwrap();
        assert_eq!(body, "cardbody");
    }

    /// Case 4: Bidi character inside CARDS[0].someField (not BODY) → PRESERVED.
    #[test]
    fn test_spec_case4_card_other_field_bidi_preserved() {
        let card = serde_json::json!({
            "CARD": "profile",
            BODY_FIELD: "clean body",
            "name": "Ali\u{202D}ce"
        });
        let mut fields = HashMap::new();
        fields.insert(
            "CARDS".to_string(),
            QuillValue::from_json(serde_json::json!([card])),
        );
        let result = normalize_fields(fields);
        let cards = result.get("CARDS").unwrap().as_array().unwrap();
        let name = cards[0].get("name").unwrap().as_str().unwrap();
        // Non-BODY card fields pass through verbatim.
        assert_eq!(name, "Ali\u{202D}ce");
    }

    /// Case 5: HTML comment fence repair applied inside CARDS[0].BODY.
    #[test]
    fn test_spec_case5_card_body_html_comment_repair() {
        let card = serde_json::json!({
            "CARD": "note",
            BODY_FIELD: "<!-- comment -->Trailing text"
        });
        let mut fields = HashMap::new();
        fields.insert(
            "CARDS".to_string(),
            QuillValue::from_json(serde_json::json!([card])),
        );
        let result = normalize_fields(fields);
        let cards = result.get("CARDS").unwrap().as_array().unwrap();
        let body = cards[0].get(BODY_FIELD).unwrap().as_str().unwrap();
        assert_eq!(body, "<!-- comment -->\nTrailing text");
    }

    /// Case 6: HTML comment fence repair applied on top-level BODY.
    #[test]
    fn test_spec_case6_toplevel_body_html_comment_repair() {
        let mut fields = HashMap::new();
        fields.insert(
            BODY_FIELD.to_string(),
            QuillValue::from_json(serde_json::json!("<!-- note -->Content here")),
        );
        let result = normalize_fields(fields);
        assert_eq!(
            result.get(BODY_FIELD).unwrap().as_str().unwrap(),
            "<!-- note -->\nContent here"
        );
    }

    /// Case 7: Non-string fields (numbers, bools, nested arrays of numbers) pass through untouched.
    #[test]
    fn test_spec_case7_non_string_fields_untouched() {
        let mut fields = HashMap::new();
        fields.insert(
            "count".to_string(),
            QuillValue::from_json(serde_json::json!(42)),
        );
        fields.insert(
            "active".to_string(),
            QuillValue::from_json(serde_json::json!(false)),
        );
        fields.insert(
            "scores".to_string(),
            QuillValue::from_json(serde_json::json!([1, 2, 3])),
        );
        let result = normalize_fields(fields);
        assert_eq!(result.get("count").unwrap().as_i64().unwrap(), 42);
        assert_eq!(result.get("active").unwrap().as_bool().unwrap(), false);
        let scores = result.get("scores").unwrap().as_array().unwrap();
        assert_eq!(scores.len(), 3);
        assert_eq!(scores[0].as_i64().unwrap(), 1);
    }

    /// Case 8: Nested objects / arrays of strings inside YAML fields: strings NOT modified.
    #[test]
    fn test_spec_case8_nested_strings_not_modified() {
        let bidi = "\u{202D}";
        let mut fields = HashMap::new();
        // Nested object inside a YAML field
        fields.insert(
            "address".to_string(),
            QuillValue::from_json(serde_json::json!({
                "city": format!("New{bidi}York"),
                "zip": "10001"
            })),
        );
        // Array of strings
        fields.insert(
            "tags".to_string(),
            QuillValue::from_json(serde_json::json!([format!("rust{bidi}lang"), "markdown"])),
        );
        let result = normalize_fields(fields);

        // Nested object strings pass through verbatim
        let addr = result.get("address").unwrap().as_object().unwrap();
        assert_eq!(
            addr.get("city").unwrap().as_str().unwrap(),
            "New\u{202D}York"
        );

        // Array of strings pass through verbatim
        let tags = result.get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags[0].as_str().unwrap(), "rust\u{202D}lang");
        assert_eq!(tags[1].as_str().unwrap(), "markdown");
    }

    // Tests for normalize_document

    #[test]
    fn test_normalize_document_basic() {
        use crate::parse::ParsedDocument;

        let mut fields = std::collections::HashMap::new();
        fields.insert(
            "title".to_string(),
            crate::value::QuillValue::from_json(serde_json::json!("<<placeholder>>")),
        );
        fields.insert(
            BODY_FIELD.to_string(),
            crate::value::QuillValue::from_json(serde_json::json!("<<content>> \u{202D}**bold**")),
        );

        let doc = ParsedDocument::new(
            fields,
            crate::version::QuillReference::latest("test".to_string()),
        );
        let normalized = super::normalize_document(doc).unwrap();

        // Title has chevrons preserved (only bidi stripped)
        assert_eq!(
            normalized.get_field("title").unwrap().as_str().unwrap(),
            "<<placeholder>>"
        );

        // Body has bidi stripped, chevrons preserved
        assert_eq!(normalized.body().unwrap(), "<<content>> **bold**");
    }

    #[test]
    fn test_normalize_document_preserves_quill_tag() {
        use crate::parse::ParsedDocument;
        use crate::version::QuillReference;
        use std::str::FromStr;

        let fields = std::collections::HashMap::new();
        let quill_ref = QuillReference::from_str("custom_quill").unwrap();
        let doc = ParsedDocument::new(fields, quill_ref);
        let normalized = super::normalize_document(doc).unwrap();

        assert_eq!(normalized.quill_reference().name, "custom_quill");
    }

    #[test]
    fn test_normalize_document_idempotent() {
        use crate::parse::ParsedDocument;

        let mut fields = std::collections::HashMap::new();
        fields.insert(
            BODY_FIELD.to_string(),
            crate::value::QuillValue::from_json(serde_json::json!("<<content>>")),
        );

        let doc = ParsedDocument::new(
            fields,
            crate::version::QuillReference::latest("test".to_string()),
        );
        let normalized_once = super::normalize_document(doc).unwrap();
        let normalized_twice = super::normalize_document(normalized_once.clone()).unwrap();

        // Calling normalize_document twice should produce the same result
        assert_eq!(
            normalized_once.body().unwrap(),
            normalized_twice.body().unwrap()
        );
    }
}
