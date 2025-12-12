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
//! - Orchestrate guillemet preprocessing (`<<text>>` → `«text»`)
//! - Apply all normalizations in the correct order
//!
//! ## Functions
//!
//! - [`strip_bidi_formatting`] - Remove Unicode bidi control characters
//! - [`normalize_markdown`] - Apply all markdown-specific normalizations
//! - [`normalize_fields`] - Normalize document fields (bidi + guillemets)
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

use crate::guillemet::{preprocess_markdown_guillemets, strip_chevrons};
use crate::parse::BODY_FIELD;
use crate::value::QuillValue;
use std::collections::HashMap;

/// Maximum nesting depth for JSON value normalization to prevent stack overflow
const MAX_NESTING_DEPTH: usize = 100;

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
        '\u{200E}' // LEFT-TO-RIGHT MARK (LRM)
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

/// Normalizes markdown content by applying all preprocessing steps.
///
/// This function applies normalizations in the correct order:
/// 1. Strip Unicode bidirectional formatting characters
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
/// ```
pub fn normalize_markdown(markdown: &str) -> String {
    strip_bidi_formatting(markdown)
}

/// Normalizes a string value by stripping bidi characters and optionally processing guillemets.
///
/// - For body content: applies `preprocess_markdown_guillemets` (converts `<<text>>` to `«text»`)
/// - For other fields: applies `strip_chevrons` (removes chevrons entirely)
fn normalize_string(s: &str, is_body: bool) -> String {
    // First strip bidi formatting characters
    let cleaned = strip_bidi_formatting(s);

    // Then apply guillemet preprocessing
    if is_body {
        preprocess_markdown_guillemets(&cleaned)
    } else {
        strip_chevrons(&cleaned)
    }
}

/// Recursively normalize a JSON value with depth tracking.
///
/// Returns an error if nesting exceeds MAX_NESTING_DEPTH to prevent stack overflow.
fn normalize_json_value_inner(
    value: serde_json::Value,
    is_body: bool,
    depth: usize,
) -> Result<serde_json::Value, NormalizationError> {
    if depth > MAX_NESTING_DEPTH {
        return Err(NormalizationError::NestingTooDeep {
            depth,
            max: MAX_NESTING_DEPTH,
        });
    }

    match value {
        serde_json::Value::String(s) => {
            Ok(serde_json::Value::String(normalize_string(&s, is_body)))
        }
        serde_json::Value::Array(arr) => {
            let normalized: Result<Vec<_>, _> = arr
                .into_iter()
                .map(|v| normalize_json_value_inner(v, false, depth + 1))
                .collect();
            Ok(serde_json::Value::Array(normalized?))
        }
        serde_json::Value::Object(map) => {
            let processed: Result<serde_json::Map<String, serde_json::Value>, _> = map
                .into_iter()
                .map(|(k, v)| {
                    let is_body = k == BODY_FIELD;
                    normalize_json_value_inner(v, is_body, depth + 1).map(|nv| (k, nv))
                })
                .collect();
            Ok(serde_json::Value::Object(processed?))
        }
        // Pass through other types unchanged (numbers, booleans, null)
        other => Ok(other),
    }
}

/// Recursively normalize a JSON value.
///
/// This is a convenience wrapper that starts depth tracking at 0.
/// Logs a warning and returns the original value if depth is exceeded.
fn normalize_json_value(value: serde_json::Value, is_body: bool) -> serde_json::Value {
    match normalize_json_value_inner(value.clone(), is_body, 0) {
        Ok(normalized) => normalized,
        Err(e) => {
            // Log warning but don't fail - return original value
            eprintln!("Warning: {}", e);
            value
        }
    }
}

/// Normalizes document fields by applying all preprocessing steps.
///
/// This function orchestrates input normalization for document fields:
/// 1. Strips Unicode bidirectional formatting characters from all string values
/// 2. For the body field: converts `<<text>>` to `«text»` (guillemets)
/// 3. For other fields: strips chevrons entirely (`<<text>>` → `text`)
///
/// # Processing Order
///
/// The normalization order is important:
/// 1. **Bidi stripping** - Must happen first so markdown delimiters are recognized
/// 2. **Guillemet preprocessing** - Converts user syntax to internal markers
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
/// fields.insert("body".to_string(), QuillValue::from_json(serde_json::json!("**bold** \u{202D}**more**")));
///
/// let result = normalize_fields(fields);
///
/// // Title has chevrons stripped
/// assert_eq!(result.get("title").unwrap().as_str().unwrap(), "hello");
///
/// // Body has bidi chars stripped (guillemet would apply if there were any <<>>)
/// assert_eq!(result.get("body").unwrap().as_str().unwrap(), "**bold** **more**");
/// ```
pub fn normalize_fields(fields: HashMap<String, QuillValue>) -> HashMap<String, QuillValue> {
    fields
        .into_iter()
        .map(|(key, value)| {
            let json = value.into_json();
            let processed = normalize_json_value(json, key == BODY_FIELD);
            (key, QuillValue::from_json(processed))
        })
        .collect()
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
        let all_bidi = "\u{200E}\u{200F}\u{202A}\u{202B}\u{202C}\u{202D}\u{202E}\u{2066}\u{2067}\u{2068}\u{2069}";
        assert_eq!(strip_bidi_formatting(all_bidi), "");
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

    // Tests for normalize_fields

    #[test]
    fn test_normalize_fields_body_bidi() {
        let mut fields = HashMap::new();
        fields.insert(
            "body".to_string(),
            QuillValue::from_json(serde_json::json!("**bold** \u{202D}**more**")),
        );

        let result = normalize_fields(fields);
        assert_eq!(
            result.get("body").unwrap().as_str().unwrap(),
            "**bold** **more**"
        );
    }

    #[test]
    fn test_normalize_fields_body_guillemets() {
        let mut fields = HashMap::new();
        fields.insert(
            "body".to_string(),
            QuillValue::from_json(serde_json::json!("<<raw>>")),
        );

        let result = normalize_fields(fields);
        assert_eq!(result.get("body").unwrap().as_str().unwrap(), "«raw»");
    }

    #[test]
    fn test_normalize_fields_body_both() {
        let mut fields = HashMap::new();
        fields.insert(
            "body".to_string(),
            QuillValue::from_json(serde_json::json!("<<raw>> \u{202D}**bold**")),
        );

        let result = normalize_fields(fields);
        // Bidi stripped first, then guillemets converted
        assert_eq!(
            result.get("body").unwrap().as_str().unwrap(),
            "«raw» **bold**"
        );
    }

    #[test]
    fn test_normalize_fields_other_field_chevrons_stripped() {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("<<hello>>")),
        );

        let result = normalize_fields(fields);
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "hello");
    }

    #[test]
    fn test_normalize_fields_other_field_bidi_stripped() {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(serde_json::json!("he\u{202D}llo")),
        );

        let result = normalize_fields(fields);
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "hello");
    }

    #[test]
    fn test_normalize_fields_nested_values() {
        let mut fields = HashMap::new();
        fields.insert(
            "items".to_string(),
            QuillValue::from_json(serde_json::json!(["<<a>>", "\u{202D}b"])),
        );

        let result = normalize_fields(fields);
        let items = result.get("items").unwrap().as_array().unwrap();
        assert_eq!(items[0].as_str().unwrap(), "a");
        assert_eq!(items[1].as_str().unwrap(), "b");
    }

    #[test]
    fn test_normalize_fields_object_values() {
        let mut fields = HashMap::new();
        fields.insert(
            "meta".to_string(),
            QuillValue::from_json(serde_json::json!({
                "title": "<<hello>>",
                "body": "<<content>>"
            })),
        );

        let result = normalize_fields(fields);
        let meta = result.get("meta").unwrap();
        let meta_obj = meta.as_object().unwrap();
        // Nested "body" key should be recognized
        assert_eq!(meta_obj.get("title").unwrap().as_str().unwrap(), "hello");
        assert_eq!(meta_obj.get("body").unwrap().as_str().unwrap(), "«content»");
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

    // Tests for depth limiting

    #[test]
    fn test_normalize_json_value_inner_depth_exceeded() {
        // Create a deeply nested JSON structure that exceeds MAX_NESTING_DEPTH
        let mut value = serde_json::json!("leaf");
        for _ in 0..=super::MAX_NESTING_DEPTH {
            value = serde_json::json!([value]);
        }

        // The inner function should return an error
        let result = super::normalize_json_value_inner(value, false, 0);
        assert!(result.is_err());

        if let Err(NormalizationError::NestingTooDeep { depth, max }) = result {
            assert!(depth > max);
            assert_eq!(max, super::MAX_NESTING_DEPTH);
        } else {
            panic!("Expected NestingTooDeep error");
        }
    }

    #[test]
    fn test_normalize_json_value_inner_within_limit() {
        // Create a nested structure just within the limit
        let mut value = serde_json::json!("leaf");
        for _ in 0..50 {
            value = serde_json::json!([value]);
        }

        // This should succeed
        let result = super::normalize_json_value_inner(value, false, 0);
        assert!(result.is_ok());
    }
}
