//! # Quillmark Helper Package Generator
//!
//! This module generates the virtual `@local/quillmark-helper:0.1.0` package
//! that provides document data and helper functions to Typst plates.
//!
//! ## Package Contents
//!
//! The generated package exports:
//! - `data` - A dictionary containing all document fields, with markdown fields
//!   automatically converted to Typst content objects
//! - `parse-date(string)` - Parses ISO 8601 date strings to Typst datetime
//!
//! ## Usage in Plates
//!
//! ```typst
//! #import "@local/quillmark-helper:0.1.0": data, parse-date
//!
//! #data.title
//! #data.BODY
//! #parse-date(data.date)
//! ```

use crate::convert::escape_string;
use std::collections::HashMap;

/// Helper function to inject JSON into Typst code.
/// Exposed for fuzzing tests.
#[doc(hidden)]
pub fn inject_json(bytes: &str) -> String {
    format!("json(bytes(\"{}\"))", escape_string(bytes))
}

/// Helper package version
pub const HELPER_VERSION: &str = "0.1.0";

/// Helper package namespace
pub const HELPER_NAMESPACE: &str = "local";

/// Helper package name
pub const HELPER_NAME: &str = "quillmark-helper";

/// Template for the `lib.typ` file, loaded at compile time
const LIB_TYP_TEMPLATE: &str = include_str!("lib.typ.template");

/// Describes which fields contain pre-converted Typst markup that should be
/// automatically evaluated into content objects by the helper package.
#[derive(Debug, Clone, Default)]
pub struct ContentFields {
    /// Top-level field names with `contentMediaType: "text/markdown"`.
    pub top_level: Vec<String>,
    /// Per-card-type field names with `contentMediaType: "text/markdown"`.
    /// Keys are card type names (e.g. `"experience_section"`), values are
    /// the field names within that card type.
    pub card_types: HashMap<String, Vec<String>>,
}

impl ContentFields {
    /// Format top-level field names as a Typst array literal.
    fn to_typst_top_level(&self) -> String {
        format_typst_string_array(&self.top_level)
    }

    /// Format card-type field mapping as a Typst dictionary literal.
    ///
    /// Produces e.g. `(experience_section: ("BODY",), quotes: ("BODY",))`
    /// or `(:)` when empty.
    fn to_typst_card_fields(&self) -> String {
        if self.card_types.is_empty() {
            return "(:)".to_string();
        }
        let entries: Vec<String> = self
            .card_types
            .iter()
            .map(|(card_type, fields)| {
                format!("{}: {}", card_type, format_typst_string_array(fields))
            })
            .collect();
        format!("({})", entries.join(", "))
    }
}

/// Format a list of strings as a Typst array literal: `("a", "b")` or `()`.
fn format_typst_string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "()".to_string();
    }
    let inner: Vec<String> = items.iter().map(|s| format!("\"{}\"", s)).collect();
    // Trailing comma for single-element arrays to distinguish from parenthesised expr
    if inner.len() == 1 {
        format!("({},)", inner[0])
    } else {
        format!("({})", inner.join(", "))
    }
}

/// Generate the `lib.typ` content for the quillmark-helper package.
///
/// The generated file contains:
/// - Embedded JSON data with markdown fields auto-evaluated into content
/// - `#let parse-date(s) = { ... }` helper for ISO 8601 dates
pub fn generate_lib_typ(json_data: &str, content_fields: &ContentFields) -> String {
    let escaped_json = escape_string(json_data);

    LIB_TYP_TEMPLATE
        .replace("{version}", HELPER_VERSION)
        .replace("{escaped_json}", &escaped_json)
        .replace("{top_content_fields}", &content_fields.to_typst_top_level())
        .replace("{card_content_fields}", &content_fields.to_typst_card_fields())
}

/// Generate the `typst.toml` content for the quillmark-helper package.
pub fn generate_typst_toml() -> String {
    format!(
        r#"[package]
name = "{name}"
version = "{version}"
namespace = "{namespace}"
entrypoint = "lib.typ"
"#,
        name = HELPER_NAME,
        version = HELPER_VERSION,
        namespace = HELPER_NAMESPACE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_content_fields() -> ContentFields {
        ContentFields {
            top_level: vec!["BODY".to_string()],
            card_types: HashMap::new(),
        }
    }

    #[test]
    fn test_generate_lib_typ_basic() {
        let json = r#"{"title": "Test", "BODY": "Hello"}"#;
        let lib = generate_lib_typ(json, &default_content_fields());

        // Should contain the version comment
        assert!(lib.contains("Version: 0.1.0"));

        // Should contain the raw JSON data
        assert!(lib.contains("json(bytes("));

        // Should NOT contain eval-markup (auto-eval replaces it)
        assert!(!lib.contains("eval-markup"));

        // Should contain the parse-date helper
        assert!(lib.contains("#let parse-date(s)"));

        // Should contain content field spec
        assert!(lib.contains("\"BODY\""));
    }

    #[test]
    fn test_generate_lib_typ_escapes_json() {
        // JSON with special characters that need escaping
        let json = r#"{"title": "Test \"quoted\""}"#;
        let lib = generate_lib_typ(json, &ContentFields::default());

        // The quotes in JSON should be escaped for Typst string literal
        assert!(lib.contains("\\\""));
    }

    #[test]
    fn test_generate_lib_typ_handles_newlines() {
        let json = "{\n\"title\": \"Test\"\n}";
        let lib = generate_lib_typ(json, &ContentFields::default());

        // Newlines should be escaped
        assert!(lib.contains("\\n"));
    }

    #[test]
    fn test_generate_typst_toml() {
        let toml = generate_typst_toml();

        assert!(toml.contains("name = \"quillmark-helper\""));
        assert!(toml.contains("version = \"0.1.0\""));
        assert!(toml.contains("namespace = \"local\""));
        assert!(toml.contains("entrypoint = \"lib.typ\""));
    }

    #[test]
    fn test_helper_constants() {
        assert_eq!(HELPER_VERSION, "0.1.0");
        assert_eq!(HELPER_NAMESPACE, "local");
        assert_eq!(HELPER_NAME, "quillmark-helper");
    }

    #[test]
    fn test_format_typst_string_array() {
        assert_eq!(format_typst_string_array(&[]), "()");
        assert_eq!(
            format_typst_string_array(&["BODY".to_string()]),
            "(\"BODY\",)"
        );
        assert_eq!(
            format_typst_string_array(&["BODY".to_string(), "summary".to_string()]),
            "(\"BODY\", \"summary\")"
        );
    }

    #[test]
    fn test_content_fields_to_typst() {
        let cf = ContentFields {
            top_level: vec!["BODY".to_string()],
            card_types: {
                let mut m = HashMap::new();
                m.insert("quotes".to_string(), vec!["BODY".to_string()]);
                m
            },
        };
        assert_eq!(cf.to_typst_top_level(), "(\"BODY\",)");
        assert_eq!(cf.to_typst_card_fields(), "(quotes: (\"BODY\",))");
    }

    #[test]
    fn test_content_fields_empty() {
        let cf = ContentFields::default();
        assert_eq!(cf.to_typst_top_level(), "()");
        assert_eq!(cf.to_typst_card_fields(), "(:)");
    }
}
