use crate::convert::{escape_string, mark_to_typst};
use quillmark_core::templating::filter_api::{Error, ErrorKind, Kwargs, State, Value};
use serde_json as json;
use std::collections::BTreeMap;
use time::format_description::well_known::Iso8601;
use time::{Date, OffsetDateTime}; // <-- add Date

// ---------- small helpers ----------

fn apply_default(mut v: Value, kwargs: &Kwargs) -> Result<Value, Error> {
    if v.is_undefined() {
        if let Some(def) = kwargs.get("default")? {
            v = def;
        }
    }
    Ok(v)
}

fn inject_json(bytes: &str) -> String {
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

    let arr = jv.as_array().ok_or_else(|| {
        err(
            ErrorKind::InvalidOperation,
            format!("Value is not an array of strings: got {}", jv),
        )
    })?;

    let mut items = Vec::with_capacity(arr.len());
    for el in arr {
        let s = el.as_str().ok_or_else(|| {
            err(
                ErrorKind::InvalidOperation,
                format!("Element is not a string: got {}", el),
            )
        })?;
        items.push(s.to_owned());
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
    if value.is_undefined() {
        if let Some(def) = kwargs.get("default")? {
            value = def;
        }
    }

    // 2) if still undefined, use today's date (UTC) as "YYYY-MM-DD"
    let s = if value.is_undefined() {
        OffsetDateTime::now_utc().date().to_string()
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

    let markup = mark_to_typst(&content);
    Ok(Value::from_safe_string(format!(
        "eval(\"{}\", mode: \"markup\")",
        escape_string(&markup)
    )))
}

pub fn asset_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    // Get the filename from the value
    let filename = value.to_string();

    // Validate filename (no path separators allowed for security)
    if filename.contains('/') || filename.contains('\\') {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!(
                "Asset filename cannot contain path separators: '{}'",
                filename
            ),
        ));
    }

    // Build the prefixed path
    let asset_path = format!("assets/DYNAMIC_ASSET__{}", filename);

    // Return as a Typst string literal
    Ok(Value::from_safe_string(format!("\"{}\"", asset_path)))
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
}

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_inject_json_no_injection(s in "\\PC*") {
            // Test the inject_json helper with various inputs
            let result = inject_json(&s);

            // Should always start with json(bytes("
            assert!(result.starts_with("json(bytes(\""));
            assert!(result.ends_with("\"))"));

            // Extract just the inner content (between quotes)
            if result.len() > "json(bytes(\"\"))".len() {
                let inner_start = "json(bytes(\"".len();
                let inner_end = result.len() - "\"))".len();
                let inner = &result[inner_start..inner_end];

                // Check for unescaped quotes in the inner content
                // (the closing quote is part of the wrapper, not the content)
                let chars: Vec<char> = inner.chars().collect();
                for i in 0..chars.len() {
                    if chars[i] == '"' {
                        // Quote must be preceded by backslash
                        assert!(i > 0 && chars[i-1] == '\\',
                            "Unescaped quote in inject_json inner content at position {}: {}", i, inner);
                    }
                }
            }
        }

        #[test]
        fn fuzz_inject_json_escaping_consistency(s in "\\PC{0,100}") {
            // Test that inject_json uses proper escaping
            let result = inject_json(&s);

            // Key property: should not contain unescaped quotes that could break out
            // Extract the inner content
            if let Some(start_pos) = result.find("json(bytes(\"") {
                let content_start = start_pos + "json(bytes(\"".len();
                if let Some(end_offset) = result[content_start..].rfind("\"))") {
                    let content_end = content_start + end_offset;
                    let escaped_content = &result[content_start..content_end];

                    // Check for unescaped quotes
                    let chars: Vec<char> = escaped_content.chars().collect();
                    for i in 0..chars.len() {
                        if chars[i] == '"' {
                            assert!(i > 0 && chars[i-1] == '\\',
                                "Unescaped quote at position {} in: {}", i, escaped_content);
                        }
                    }
                }
            }
        }

        #[test]
        fn fuzz_inject_json_dangerous_patterns(s in "[\\\\\"'`$#]{0,50}") {
            // Test with characters that might cause injection
            let result = inject_json(&s);

            // Should not contain patterns that could break out of string context
            let dangerous_patterns = ["\"); ", "\")); "];
            for pattern in &dangerous_patterns {
                assert!(!result.contains(pattern),
                    "Dangerous pattern '{}' found in: {}", pattern, result);
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn fuzz_inject_json_size_limits(size in 0usize..1000) {
            // Test with various input sizes
            let input = "a".repeat(size);
            let result = inject_json(&input);

            // Output should be proportional to input
            assert!(result.len() >= input.len());
            // For control characters or special chars, output can be much longer (up to 10x)
            // For empty string, wrapper is "json(bytes(\"\"))" which is 16 chars
            if size == 0 {
                assert_eq!(result, "json(bytes(\"\"))");
            } else {
                // Normal chars don't expand much, but allow generous headroom
                assert!(result.len() < input.len() * 20 || result.len() < 1000);
            }
        }

        #[test]
        fn fuzz_inject_json_unicode(s in "\\PC{0,100}") {
            // Test with unicode characters
            let result = inject_json(&s);

            // Should handle unicode without panic
            assert!(result.starts_with("json(bytes(\""));
        }
    }
}
