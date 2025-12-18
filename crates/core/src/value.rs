//! Value type for unified representation of TOML/YAML/JSON values.
//!
//! This module provides [`QuillValue`], a newtype wrapper around `serde_json::Value`
//! that centralizes all value conversions across the Quillmark system.

use minijinja::value::Value as MjValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::Deref;

/// Unified value type backed by `serde_json::Value`.
///
/// This type is used throughout Quillmark to represent metadata, fields, and other
/// dynamic values. It provides conversion methods for TOML, YAML, and MiniJinja.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuillValue(serde_json::Value);

impl QuillValue {
    /// Create a QuillValue from a TOML value
    pub fn from_toml(toml_val: &toml::Value) -> Result<Self, serde_json::Error> {
        let json_val = serde_json::to_value(toml_val)?;
        Ok(QuillValue(json_val))
    }

    /// Create a QuillValue from a YAML string
    pub fn from_yaml_str(yaml_str: &str) -> Result<Self, serde_saphyr::Error> {
        let json_val: serde_json::Value = serde_saphyr::from_str(yaml_str)?;
        Ok(QuillValue(json_val))
    }

    /// Convert to a MiniJinja value for templating
    pub fn to_minijinja(&self) -> Result<MjValue, String> {
        json_to_minijinja(&self.0)
    }

    /// Get a reference to the underlying JSON value
    pub fn as_json(&self) -> &serde_json::Value {
        &self.0
    }

    /// Convert into the underlying JSON value
    pub fn into_json(self) -> serde_json::Value {
        self.0
    }

    /// Create a QuillValue directly from a JSON value
    pub fn from_json(json_val: serde_json::Value) -> Self {
        QuillValue(json_val)
    }
}

impl Deref for QuillValue {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Convert a JSON value to a MiniJinja value
fn json_to_minijinja(value: &serde_json::Value) -> Result<MjValue, String> {
    use serde_json::Value as JsonValue;

    let result = match value {
        JsonValue::Null => MjValue::from(()),
        JsonValue::Bool(b) => MjValue::from(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                MjValue::from(i)
            } else if let Some(u) = n.as_u64() {
                MjValue::from(u)
            } else if let Some(f) = n.as_f64() {
                MjValue::from(f)
            } else {
                return Err("Invalid number in JSON".to_string());
            }
        }
        JsonValue::String(s) => MjValue::from(s.clone()),
        JsonValue::Array(arr) => {
            let mut vec = Vec::new();
            for item in arr {
                vec.push(json_to_minijinja(item)?);
            }
            MjValue::from(vec)
        }
        JsonValue::Object(map) => {
            let mut obj = BTreeMap::new();
            for (k, v) in map {
                obj.insert(k.clone(), json_to_minijinja(v)?);
            }
            MjValue::from_object(obj)
        }
    };

    Ok(result)
}

// Implement common delegating methods for convenience
impl QuillValue {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Get the value as a string reference
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_str()
    }

    /// Get the value as a boolean
    pub fn as_bool(&self) -> Option<bool> {
        self.0.as_bool()
    }

    /// Get the value as an i64
    pub fn as_i64(&self) -> Option<i64> {
        self.0.as_i64()
    }

    /// Get the value as a u64
    pub fn as_u64(&self) -> Option<u64> {
        self.0.as_u64()
    }

    /// Get the value as an f64
    pub fn as_f64(&self) -> Option<f64> {
        self.0.as_f64()
    }

    /// Get the value as an array reference
    pub fn as_array(&self) -> Option<&Vec<serde_json::Value>> {
        self.0.as_array()
    }

    /// Get the value as an array reference (alias for as_array, for YAML compatibility)
    pub fn as_sequence(&self) -> Option<&Vec<serde_json::Value>> {
        self.0.as_array()
    }

    /// Get the value as an object reference
    pub fn as_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.0.as_object()
    }

    /// Get the value as an object reference (alias for as_object, for YAML compatibility)
    pub fn as_mapping(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.0.as_object()
    }

    /// Get a field from an object by key
    pub fn get(&self, key: &str) -> Option<QuillValue> {
        self.0.get(key).map(|v| QuillValue(v.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_toml() {
        let toml_str = r#"
            [package]
            name = "test"
            version = "1.0.0"
        "#;
        let toml_val: toml::Value = toml::from_str(toml_str).unwrap();
        let quill_val = QuillValue::from_toml(&toml_val).unwrap();

        assert!(quill_val.as_object().is_some());
    }

    #[test]
    fn test_from_yaml_str() {
        let yaml_str = r#"
            title: Test Document
            author: John Doe
            count: 42
        "#;
        let quill_val = QuillValue::from_yaml_str(yaml_str).unwrap();

        assert_eq!(
            quill_val.get("title").as_ref().and_then(|v| v.as_str()),
            Some("Test Document")
        );
        assert_eq!(
            quill_val.get("author").as_ref().and_then(|v| v.as_str()),
            Some("John Doe")
        );
        assert_eq!(
            quill_val.get("count").as_ref().and_then(|v| v.as_i64()),
            Some(42)
        );
    }

    #[test]
    fn test_to_minijinja() {
        let json_val = serde_json::json!({
            "title": "Test",
            "count": 42,
            "active": true,
            "items": [1, 2, 3]
        });
        let quill_val = QuillValue::from_json(json_val);
        let mj_val = quill_val.to_minijinja().unwrap();

        // Verify it's convertible to MiniJinja value
        assert!(mj_val.as_object().is_some());
    }

    #[test]
    fn test_as_json() {
        let json_val = serde_json::json!({"key": "value"});
        let quill_val = QuillValue::from_json(json_val.clone());

        assert_eq!(quill_val.as_json(), &json_val);
    }

    #[test]
    fn test_into_json() {
        let json_val = serde_json::json!({"key": "value"});
        let quill_val = QuillValue::from_json(json_val.clone());

        assert_eq!(quill_val.into_json(), json_val);
    }

    #[test]
    fn test_delegating_methods() {
        let quill_val = QuillValue::from_json(serde_json::json!({
            "name": "test",
            "count": 42,
            "active": true,
            "items": [1, 2, 3]
        }));

        assert_eq!(
            quill_val.get("name").as_ref().and_then(|v| v.as_str()),
            Some("test")
        );
        assert_eq!(
            quill_val.get("count").as_ref().and_then(|v| v.as_i64()),
            Some(42)
        );
        assert_eq!(
            quill_val.get("active").as_ref().and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(quill_val
            .get("items")
            .as_ref()
            .and_then(|v| v.as_array())
            .is_some());
    }

    #[test]
    fn test_yaml_with_tags() {
        // Note: serde_saphyr handles tags differently - this tests basic parsing
        let yaml_str = r#"
            value: 42
        "#;
        let quill_val = QuillValue::from_yaml_str(yaml_str).unwrap();

        // Values should be converted to their underlying value
        assert!(quill_val.as_object().is_some());
    }

    #[test]
    fn test_null_value() {
        let quill_val = QuillValue::from_json(serde_json::Value::Null);
        assert!(quill_val.is_null());
    }

    #[test]
    fn test_yaml_custom_tags_ignored() {
        // User-defined YAML tags should be accepted and ignored
        // The value should be parsed as if the tag were not present
        let yaml_str = "memo_from: !fill 2d lt example";
        let quill_val = QuillValue::from_yaml_str(yaml_str).unwrap();

        // The tag !fill should be ignored, value parsed as string
        assert_eq!(
            quill_val.get("memo_from").as_ref().and_then(|v| v.as_str()),
            Some("2d lt example")
        );
    }
}
