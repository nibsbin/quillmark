use std::collections::HashMap;
use tera::{Value, Filter};
use serde_yaml;
use crate::convert::escape_markup;

/// Sanitize string by escaping quotes and backslashes
fn sanitize_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// String filter - converts values to escaped Typst strings
pub struct StringFilter;

impl Filter for StringFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let string_val = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => {
                if let Some(def_val) = args.get("default") {
                    match def_val {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "".to_string(),
                        other => format!("{}", other),
                    }
                } else {
                    "".to_string()
                }
            }
            _ => format!("{}", value),
        };

        let escaped = escape_markup(&string_val);
        Ok(Value::String(escaped))
    }
}

/// Array filter - converts arrays to Typst tuple format
pub struct ArrayFilter;

impl Filter for ArrayFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let items = match value {
            Value::Array(arr) => {
                let escaped_items: Vec<String> = arr.iter()
                    .map(|item| {
                        let item_str = match item {
                            Value::String(s) => s.clone(),
                            _ => format!("{}", item),
                        };
                        escape_markup(&item_str)
                    })
                    .collect();
                format!("({})", escaped_items.join(", "))
            }
            Value::String(s) => {
                // Split string by lines or commas and treat as array
                let lines: Vec<String> = s.lines()
                    .map(|line| escape_markup(line.trim()))
                    .collect();
                format!("({})", lines.join(", "))
            }
            Value::Null => {
                if let Some(def_val) = args.get("default") {
                    match def_val {
                        Value::Array(arr) => {
                            let escaped_items: Vec<String> = arr.iter()
                                .map(|item| {
                                    let item_str = match item {
                                        Value::String(s) => s.clone(),
                                        _ => format!("{}", item),
                                    };
                                    escape_markup(&item_str)
                                })
                                .collect();
                            format!("({})", escaped_items.join(", "))
                        }
                        Value::String(s) => {
                            let lines: Vec<String> = s.lines()
                                .map(|line| escape_markup(line.trim()))
                                .collect();
                            format!("({})", lines.join(", "))
                        }
                        other => {
                            let escaped = escape_markup(&format!("{}", other));
                            format!("({})", escaped)
                        }
                    }
                } else {
                    let escaped = escape_markup(&format!("{}", value));
                    format!("({})", escaped)
                }
            }
            _ => {
                let escaped = escape_markup(&format!("{}", value));
                format!("({})", escaped)
            }
        };
        
        Ok(Value::String(items))
    }
}

/// Integer filter - verifies data is an int and formats without quotes
pub struct IntFilter;

impl Filter for IntFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        match value {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::String(i.to_string()))
                } else if let Some(f) = n.as_f64() {
                    // Check if it's a whole number
                    if f.fract() == 0.0 {
                        Ok(Value::String((f as i64).to_string()))
                    } else {
                        Err(tera::Error::msg("Value is not an integer"))
                    }
                } else {
                    Err(tera::Error::msg("Value is not a valid number"))
                }
            }
            Value::String(s) => {
                match s.parse::<i64>() {
                    Ok(i) => Ok(Value::String(i.to_string())),
                    Err(_) => Err(tera::Error::msg("String cannot be parsed as integer")),
                }
            }
            Value::Null => {
                if let Some(def_val) = args.get("default") {
                    match def_val {
                        Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                Ok(Value::String(i.to_string()))
                            } else if let Some(f) = n.as_f64() {
                                if f.fract() == 0.0 {
                                    Ok(Value::String((f as i64).to_string()))
                                } else {
                                    Err(tera::Error::msg("Default value is not an integer"))
                                }
                            } else {
                                Err(tera::Error::msg("Default value is not a valid number"))
                            }
                        }
                        Value::String(s) => {
                            match s.parse::<i64>() {
                                Ok(i) => Ok(Value::String(i.to_string())),
                                Err(_) => Err(tera::Error::msg("Default string cannot be parsed as integer")),
                            }
                        }
                        _ => Err(tera::Error::msg("Default value is not an integer")),
                    }
                } else {
                    Err(tera::Error::msg("Value is not an integer"))
                }
            }
            _ => Err(tera::Error::msg("Value is not an integer")),
        }
    }
}

/// Boolean filter - verifies and formats boolean values
pub struct BoolFilter;

impl Filter for BoolFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let bool_val = match value {
            Value::Bool(b) => *b,
            Value::String(s) => {
                let lower = s.to_lowercase();
                match lower.as_str() {
                    "true" | "yes" | "1" => true,
                    "false" | "no" | "0" => false,
                    _ => return Err(tera::Error::msg("String cannot be parsed as boolean")),
                }
            }
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i != 0
                } else {
                    return Err(tera::Error::msg("Number cannot be converted to boolean"));
                }
            }
            Value::Null => {
                if let Some(def_val) = args.get("default") {
                    match def_val {
                        Value::Bool(b) => *b,
                        Value::String(s) => {
                            let lower = s.to_lowercase();
                            match lower.as_str() {
                                "true" | "yes" | "1" => true,
                                "false" | "no" | "0" => false,
                                _ => return Err(tera::Error::msg("Default string cannot be parsed as boolean")),
                            }
                        }
                        Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                i != 0
                            } else {
                                return Err(tera::Error::msg("Default number cannot be converted to boolean"));
                            }
                        }
                        _ => return Err(tera::Error::msg("Default value is not a boolean")),
                    }
                } else {
                    return Err(tera::Error::msg("Value is not a boolean"));
                }
            }
            _ => return Err(tera::Error::msg("Value is not a boolean")),
        };

        Ok(Value::String(bool_val.to_string()))
    }
}

/// DateTime filter - formats date/time values
pub struct DateTimeFilter;

impl Filter for DateTimeFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let date_str = match value {
            Value::String(s) => s.clone(),
            Value::Null => {
                if let Some(def_val) = args.get("default") {
                    match def_val {
                        Value::String(s) => s.clone(),
                        other => format!("{}", other),
                    }
                } else {
                    format!("{}", value)
                }
            }
            _ => format!("{}", value),
        };

        // For now, just pass through the date string - backends can implement more sophisticated formatting
        Ok(Value::String(escape_markup(&date_str)))
    }
}

/// Dict filter - serializes to TOML and formats for Typst
pub struct DictFilter;

impl Filter for DictFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        match value {
            Value::Object(obj) => {
                // Convert to a YAML value first, then to TOML
                let yaml_value: serde_yaml::Value = serde_json::from_value(serde_json::Value::Object(obj.clone()))
                    .map_err(|e| tera::Error::msg(format!("Failed to convert object: {}", e)))?;
                
                let toml_string = toml::to_string(&yaml_value)
                    .map_err(|e| tera::Error::msg(format!("Failed to serialize to TOML: {}", e)))?;
                
                let sanitized = sanitize_string(&toml_string);
                let result = format!("toml(bytes(\"{}\"))", sanitized);
                Ok(Value::String(result))
            }
            Value::Null => {
                if let Some(def_val) = args.get("default") {
                    if let Value::Object(obj) = def_val {
                        let yaml_value: serde_yaml::Value = serde_json::from_value(serde_json::Value::Object(obj.clone()))
                            .map_err(|e| tera::Error::msg(format!("Failed to convert default object: {}", e)))?;

                        let toml_string = toml::to_string(&yaml_value)
                            .map_err(|e| tera::Error::msg(format!("Failed to serialize default to TOML: {}", e)))?;

                        let sanitized = sanitize_string(&toml_string);
                        let result = format!("toml(bytes(\"{}\"))", sanitized);
                        Ok(Value::String(result))
                    } else {
                        Err(tera::Error::msg("Default value is not a dictionary/object"))
                    }
                } else {
                    Err(tera::Error::msg("Value is not a dictionary/object"))
                }
            }
            _ => Err(tera::Error::msg("Value is not a dictionary/object")),
        }
    }
}

/// Body filter - handles markdown content conversion
pub struct BodyFilter;

impl Filter for BodyFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let content = match value {
            Value::String(s) => s.clone(),
            _ => format!("{}", value),
        };
        
        // Convert markdown to Typst markup using the existing conversion function
        let typst_markup = crate::convert::mark_to_typst(&content);
        Ok(Value::String(typst_markup))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_string_filter() {
        let filter = StringFilter;
        let value = Value::String("Hello *world*".to_string());
        let result = filter.filter(&value, &HashMap::new()).unwrap();
        
        if let Value::String(s) = result {
            assert!(s.contains("Hello \\*world\\*"));
        } else {
            panic!("Expected string result");
        }
    }
    
    #[test]
    fn test_array_filter() {
        let filter = ArrayFilter;
        let value = Value::Array(vec![
            Value::String("item1".to_string()),
            Value::String("item2".to_string()),
        ]);
        let result = filter.filter(&value, &HashMap::new()).unwrap();
        
        if let Value::String(s) = result {
            assert!(s.starts_with("("));
            assert!(s.ends_with(")"));
            assert!(s.contains("item1"));
            assert!(s.contains("item2"));
        } else {
            panic!("Expected string result");
        }
    }
    
    #[test]
    fn test_int_filter() {
        let filter = IntFilter;
        let value = Value::Number(serde_json::Number::from(42));
        let result = filter.filter(&value, &HashMap::new()).unwrap();
        
        if let Value::String(s) = result {
            assert_eq!(s, "42");
        } else {
            panic!("Expected string result");
        }
    }
    
    #[test]
    fn test_bool_filter() {
        let filter = BoolFilter;
        let value = Value::Bool(true);
        let result = filter.filter(&value, &HashMap::new()).unwrap();
        
        if let Value::String(s) = result {
            assert_eq!(s, "true");
        } else {
            panic!("Expected string result");
        }
    }

    #[test]
    fn test_array_filter_null_with_default() {
        let filter = ArrayFilter;
        let value = Value::Null;
        let mut args: HashMap<String, Value> = HashMap::new();
        args.insert("default".to_string(), Value::Array(vec![Value::String("a".into()), Value::String("b".into())]));

        let result = filter.filter(&value, &args).unwrap();
        if let Value::String(s) = result {
            assert!(s.contains("a"));
            assert!(s.contains("b"));
        } else { panic!("Expected string result"); }
    }

    #[test]
    fn test_int_filter_null_with_default() {
        let filter = IntFilter;
        let value = Value::Null;
        let mut args: HashMap<String, Value> = HashMap::new();
        args.insert("default".to_string(), Value::Number(serde_json::Number::from(7)));

        let result = filter.filter(&value, &args).unwrap();
        if let Value::String(s) = result { assert_eq!(s, "7"); } else { panic!("Expected string result"); }
    }

    #[test]
    fn test_bool_filter_null_with_default() {
        let filter = BoolFilter;
        let value = Value::Null;
        let mut args: HashMap<String, Value> = HashMap::new();
        args.insert("default".to_string(), Value::Bool(false));

        let result = filter.filter(&value, &args).unwrap();
        if let Value::String(s) = result { assert_eq!(s, "false"); } else { panic!("Expected string result"); }
    }

    #[test]
    fn test_datetime_filter_null_with_default() {
        let filter = DateTimeFilter;
        let value = Value::Null;
        let mut args: HashMap<String, Value> = HashMap::new();
        args.insert("default".to_string(), Value::String("2020-01-01".into()));

        let result = filter.filter(&value, &args).unwrap();
        if let Value::String(s) = result { assert!(s.contains("2020-01-01")); } else { panic!("Expected string result"); }
    }

    #[test]
    fn test_dict_filter_null_with_default() {
        let filter = DictFilter;
        let value = Value::Null;
        let mut args: HashMap<String, Value> = HashMap::new();
        let mut map = serde_json::Map::new();
        map.insert("k".to_string(), Value::String("v".into()));
        args.insert("default".to_string(), Value::Object(map));

        let result = filter.filter(&value, &args).unwrap();
        if let Value::String(s) = result { assert!(s.contains("k")); } else { panic!("Expected string result"); }
    }
}