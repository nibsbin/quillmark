//! # Quillmark Helper Package Generator
//!
//! This module generates the virtual `@local/quillmark-helper:0.1.0` package
//! that provides document data and helper functions to Typst plates.
//!
//! ## Package Contents
//!
//! The generated package exports:
//! - `data` - A dictionary containing all document fields as JSON
//! - `content(string)` - Evaluates pre-converted Typst markup strings
//! - `parse-date(string)` - Parses ISO 8601 date strings to Typst datetime
//!
//! ## Usage in Plates
//!
//! ```typst
//! #import "@local/quillmark-helper:0.1.0": data, content, parse-date
//!
//! #data.title
//! #content(data.BODY)
//! #parse-date(data.date)
//! ```

use crate::convert::escape_string;

/// Helper package version
pub const HELPER_VERSION: &str = "0.1.0";

/// Helper package namespace
pub const HELPER_NAMESPACE: &str = "local";

/// Helper package name
pub const HELPER_NAME: &str = "quillmark-helper";

/// Generate the `lib.typ` content for the quillmark-helper package.
///
/// The generated file contains:
/// - Embedded JSON data as `#let data = json(bytes("..."))`
/// - `#let content(s) = eval(s, mode: "markup")` helper
/// - `#let parse-date(s) = { ... }` helper for ISO 8601 dates
pub fn generate_lib_typ(json_data: &str) -> String {
    let escaped_json = escape_string(json_data);

    format!(
        r#"// Auto-generated quillmark-helper package
// Version: {version}

/// Document data as a dictionary
#let data = json(bytes("{escaped_json}"))

/// Evaluate a pre-converted Typst markup string as content
#let content(s) = eval(s, mode: "markup")

/// Parse an ISO 8601 date string (YYYY-MM-DD) to a Typst datetime
/// Handles both pure dates (2024-01-15) and datetime strings (2024-01-15T10:30:00)
#let parse-date(s) = {{
  if s == none {{ return none }}
  let date-str = str(s)
  // Handle datetime strings by extracting just the date part
  if date-str.contains("T") {{
    date-str = date-str.split("T").at(0)
  }}
  let parts = date-str.split("-")
  if parts.len() < 3 {{ return none }}
  let year = int(parts.at(0))
  let month = int(parts.at(1))
  // Take only the first 2 characters in case there's extra content
  let day-str = parts.at(2)
  if day-str.len() > 2 {{ day-str = day-str.slice(0, 2) }}
  let day = int(day-str)
  datetime(year: year, month: month, day: day)
}}
"#,
        version = HELPER_VERSION,
        escaped_json = escaped_json
    )
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

    #[test]
    fn test_generate_lib_typ_basic() {
        let json = r#"{"title": "Test", "BODY": "Hello"}"#;
        let lib = generate_lib_typ(json);

        // Should contain the version comment
        assert!(lib.contains("Version: 0.1.0"));

        // Should contain the data binding
        assert!(lib.contains("#let data = json(bytes("));

        // Should contain the content helper
        assert!(lib.contains("#let content(s) = eval(s, mode: \"markup\")"));

        // Should contain the parse-date helper
        assert!(lib.contains("#let parse-date(s)"));
    }

    #[test]
    fn test_generate_lib_typ_escapes_json() {
        // JSON with special characters that need escaping
        let json = r#"{"title": "Test \"quoted\""}"#;
        let lib = generate_lib_typ(json);

        // The quotes in JSON should be escaped for Typst string literal
        assert!(lib.contains("\\\""));
    }

    #[test]
    fn test_generate_lib_typ_handles_newlines() {
        let json = "{\n\"title\": \"Test\"\n}";
        let lib = generate_lib_typ(json);

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
}
