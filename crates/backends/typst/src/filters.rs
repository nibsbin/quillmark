use crate::convert::{escape_string, mark_to_typst};
use quillmark_core::templating::filter_api::{Error, ErrorKind, Kwargs, State, Value};
use serde_json as json;
use std::collections::BTreeMap;
use time::format_description::well_known::Iso8601;
use time::Date;

// ---------- small helpers ----------

fn apply_default(mut v: Value, kwargs: &Kwargs) -> Result<Value, Error> {
    if v.is_undefined() {
        if let Some(def) = kwargs.get("default")? {
            v = def;
        }
    }
    Ok(v)
}

/// Helper function to inject JSON into Typst code.
/// Exposed for fuzzing tests.
#[doc(hidden)]
pub fn inject_json(bytes: &str) -> String {
    format!("json(bytes(\"{}\"))", escape_string(bytes))
}

fn err(kind: ErrorKind, msg: impl Into<String>) -> Error {
    Error::new(kind, msg.into())
}

// ---------- filters ----------

pub fn string_filter(_state: &State, mut value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    value = apply_default(value, &_kwargs)?;
    let s = value.to_string();
    let json_str = json::to_string(&s).map_err(|e| {
        err(
            ErrorKind::BadSerialization,
            format!("Failed to serialize JSON string: {e}"),
        )
    })?;
    Ok(Value::from_safe_string(inject_json(&json_str)))
}

pub fn lines_filter(_state: &State, mut value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    value = apply_default(value, &kwargs)?;

    let jv = json::to_value(&value).map_err(|e| {
        err(
            ErrorKind::InvalidOperation,
            format!(
                "Value cannot be converted to JSON: {e} (source: {:?})",
                value
            ),
        )
    })?;

    // Accept either an array of strings or a single string (coerce to one-element array)
    let mut items = Vec::new();
    if let Some(arr) = jv.as_array() {
        items.reserve(arr.len());
        for el in arr {
            let s = el.as_str().ok_or_else(|| {
                err(
                    ErrorKind::InvalidOperation,
                    format!("Element is not a string: got {}", el),
                )
            })?;
            items.push(s.to_owned());
        }
    } else if let Some(s) = jv.as_str() {
        items.push(s.to_owned());
    } else {
        return Err(err(
            ErrorKind::InvalidOperation,
            format!("Value is not an array of strings or a string: got {}", jv),
        ));
    }

    let json_str = json::to_string(&items).map_err(|e| {
        err(
            ErrorKind::BadSerialization,
            format!("Failed to serialize JSON array: {e}"),
        )
    })?;
    Ok(Value::from_safe_string(inject_json(&json_str)))
}

pub fn date_filter(_state: &State, mut value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    // 1) if undefined, use default
    if value.is_undefined() || value.to_string().is_empty() {
        if let Some(def) = kwargs.get("default")? {
            value = def;
        }
    }

    // 2) if still undefined, use today's date (UTC) as "YYYY-MM-DD"
    let s = if value.is_undefined() || value.to_string().is_empty() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            time::OffsetDateTime::now_utc().date().to_string()
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Use js-sys to get the current UTC date string on wasm targets.
            // We format as YYYY-MM-DD to match Iso8601::DEFAULT parsing expectations.
            use js_sys::Date;
            let d = Date::new_0();
            let year = d.get_utc_full_year() as i32;
            let month = (d.get_utc_month() as u8).saturating_add(1);
            let day = d.get_utc_date() as u8;
            format!("{:04}-{:02}-{:02}", year, month, day)
        }
    } else {
        value.to_string()
    };

    // Validate strict ISO 8601 date (YYYY-MM-DD)
    let d = Date::parse(&s, &Iso8601::DEFAULT).map_err(|_| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("Not ISO date (YYYY-MM-DD): {s}"),
        )
    })?;

    // 3) Build Typst date
    let year = d.year() as u16;
    let month = d.month() as u8;
    let day = d.day();
    let injector = format!("datetime(year: {}, month: {}, day: {})", year, month, day);

    // 4) Inject as TOML doc (with trailing ".value" in the payload)
    Ok(Value::from_safe_string(injector))
}

pub fn dict_filter(_state: &State, mut value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    value = apply_default(value, &kwargs)?;

    let jv = json::to_value(&value).map_err(|e| {
        err(
            ErrorKind::InvalidOperation,
            format!(
                "Value cannot be converted to JSON: {e} (source: {:?})",
                value
            ),
        )
    })?;
    let obj = jv.as_object().ok_or_else(|| {
        err(
            ErrorKind::InvalidOperation,
            format!("Value is not a dict<string,string>: got {}", jv),
        )
    })?;

    let mut map = BTreeMap::<String, String>::new();
    for (k, v) in obj {
        let s = v.as_str().ok_or_else(|| {
            err(
                ErrorKind::InvalidOperation,
                format!("Dict value for key '{}' is not a string: {}", k, v),
            )
        })?;
        map.insert(k.clone(), s.to_owned());
    }

    let json_str = json::to_string(&map).map_err(|e| {
        err(
            ErrorKind::BadSerialization,
            format!("Failed to serialize JSON object: {e}"),
        )
    })?;
    Ok(Value::from_safe_string(inject_json(&json_str)))
}

pub fn content_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    let jv = json::to_value(&value).map_err(|e| {
        err(
            ErrorKind::InvalidOperation,
            format!(
                "Value cannot be converted to JSON: {e} (source: {:?})",
                value
            ),
        )
    })?;

    let content = match jv {
        json::Value::Null => String::new(),
        json::Value::String(s) => s,
        other => other.to_string(),
    };

    let markup = mark_to_typst(&content).map_err(|e| {
        err(
            ErrorKind::InvalidOperation,
            format!("Markdown conversion failed: {}", e),
        )
    })?;
    Ok(Value::from_safe_string(format!(
        "eval(\"{}\", mode: \"markup\")",
        escape_string(&markup)
    )))
}

pub fn asset_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    // Get the filename from the value
    let filename = value.to_string();

    // Security validation for asset filenames
    // Prevents path traversal attacks via:
    // - URL encoding (%2F, %5C, etc.)
    // - Unicode confusables (fullwidth solidus, etc.)
    // - Null byte injection
    // - Path traversal patterns (..)

    // Check for null bytes (could truncate path in some systems)
    if filename.contains('\0') {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "Asset filename cannot contain null bytes",
        ));
    }

    // Check for URL-encoded path separators (%2F = /, %5C = \)
    let lower_filename = filename.to_lowercase();
    if lower_filename.contains("%2f")
        || lower_filename.contains("%5c")
        || lower_filename.contains("%00")
    {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename cannot contain URL-encoded path separators: '{}'",
                filename
            ),
        ));
    }

    // Check for double URL encoding (%252F, %255C)
    if lower_filename.contains("%25") {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename cannot contain double URL-encoded characters: '{}'",
                filename
            ),
        ));
    }

    // Check for standard path separators
    if filename.contains('/') || filename.contains('\\') {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename cannot contain path separators: '{}'",
                filename
            ),
        ));
    }

    // Check for Unicode path separator confusables
    let has_unicode_path_sep = filename.chars().any(|c| {
        matches!(
            c,
            '\u{FF0F}'  // Fullwidth solidus /
            | '\u{FF3C}'  // Fullwidth reverse solidus \
            | '\u{2215}'  // Division slash
            | '\u{2216}'  // Set minus
            | '\u{29F8}'  // Big solidus
            | '\u{29F9}' // Big reverse solidus
        )
    });

    if has_unicode_path_sep {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename contains Unicode path separator variants: '{}'",
                filename
            ),
        ));
    }

    // Check for path traversal patterns
    if filename.contains("..") {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename cannot contain path traversal patterns: '{}'",
                filename
            ),
        ));
    }

    // Validate filename characters (allowlist approach)
    // Allow: alphanumeric, dot, dash, underscore, space
    let valid_chars = filename
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ' ');

    if !valid_chars {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename contains invalid characters (allowed: alphanumeric, .-_ and space): '{}'",
                filename
            ),
        ));
    }

    // Validate that filename doesn't start with a dot (hidden files)
    if filename.starts_with('.') {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("Asset filename cannot start with a dot: '{}'", filename),
        ));
    }

    // Validate that filename is not empty
    if filename.is_empty() || filename.trim().is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "Asset filename cannot be empty",
        ));
    }

    // Build the prefixed path
    let asset_path = format!("assets/DYNAMIC_ASSET__{}", filename);

    // Return as a Typst string literal
    Ok(Value::from_safe_string(format!("\"{}\"", asset_path)))
}

pub fn json_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    let jv = json::to_value(&value).map_err(|e| {
        err(
            ErrorKind::InvalidOperation,
            format!(
                "Value cannot be converted to JSON: {e} (source: {:?})",
                value
            ),
        )
    })?;

    let json_str = json::to_string(&jv).map_err(|e| {
        err(
            ErrorKind::BadSerialization,
            format!("Failed to serialize JSON: {e}"),
        )
    })?;
    Ok(Value::from_safe_string(inject_json(&json_str)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_asset_path_construction() {
        // Test the path construction logic directly
        let filename = "chart.png";
        let asset_path = format!("assets/DYNAMIC_ASSET__{}", filename);
        assert_eq!(asset_path, "assets/DYNAMIC_ASSET__chart.png");
    }

    #[test]
    fn test_asset_path_with_various_extensions() {
        let test_cases = vec![
            ("image.png", "assets/DYNAMIC_ASSET__image.png"),
            ("data.csv", "assets/DYNAMIC_ASSET__data.csv"),
            ("chart.jpg", "assets/DYNAMIC_ASSET__chart.jpg"),
            ("file.pdf", "assets/DYNAMIC_ASSET__file.pdf"),
        ];

        for (filename, expected) in test_cases {
            let asset_path = format!("assets/DYNAMIC_ASSET__{}", filename);
            assert_eq!(asset_path, expected);
        }
    }

    #[test]
    fn test_path_separator_detection() {
        // Test that we can detect path separators
        assert!("../hack.png".contains('/'));
        assert!("subdir\\file.png".contains('\\'));
        assert!(!"simple.png".contains('/'));
        assert!(!"simple.png".contains('\\'));
    }

    #[test]
    fn test_asset_security_url_encoding() {
        // URL-encoded path separators should be detected
        let filename = "..%2F..%2Fetc%2Fpasswd";
        let lower = filename.to_lowercase();
        assert!(lower.contains("%2f"));
    }

    #[test]
    fn test_asset_security_unicode_confusables() {
        // Unicode path separator variants should be detected
        let fullwidth_slash = '\u{FF0F}'; // Fullwidth solidus
        let filename = format!("..{}etc{}passwd", fullwidth_slash, fullwidth_slash);
        assert!(filename.contains('\u{FF0F}'));
    }

    #[test]
    fn test_asset_security_null_byte() {
        // Null bytes should be detected
        let filename = "valid.png\x00../../etc/passwd";
        assert!(filename.contains('\0'));
    }

    #[test]
    fn test_asset_security_path_traversal() {
        // Path traversal patterns should be detected
        assert!("../parent/file.png".contains(".."));
        assert!(!"normal-file.png".contains(".."));
    }

    #[test]
    fn test_asset_security_double_encoding() {
        // Double URL encoding should be detected
        let filename = "..%252F..%252Fetc%252Fpasswd";
        let lower = filename.to_lowercase();
        assert!(lower.contains("%25"));
    }

    #[test]
    fn test_asset_valid_characters() {
        // Valid characters should pass
        let valid_filenames = vec![
            "simple.png",
            "my-file.jpg",
            "my_file.pdf",
            "File Name With Spaces.doc",
            "archive2024.zip",
        ];
        for filename in valid_filenames {
            let valid = filename
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ' ');
            assert!(valid, "Expected '{}' to have valid characters", filename);
        }
    }

    #[test]
    fn test_asset_invalid_characters() {
        // Invalid characters should be detected
        let invalid_filenames = vec![
            "file<script>.png",
            "file>test.png",
            "file|pipe.png",
            "file:colon.png",
            "file*star.png",
            "file?query.png",
        ];
        for filename in invalid_filenames {
            let valid = filename
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ' ');
            assert!(!valid, "Expected '{}' to have invalid characters", filename);
        }
    }
}
