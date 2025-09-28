//! filters.rs (external crate)
//! Implement filters against the core crate's stable filter ABI (no minijinja deps here).

use serde_json::{self, Value as Json};
use serde_yaml;

use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};

use crate::convert::{escape_markup, mark_to_typst};

/// Sanitize string by escaping quotes and backslashes
fn sanitize_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/* ---------- helpers ---------- */

fn v_to_json(v: &Value) -> Json {
    serde_json::to_value(v).unwrap_or(Json::Null)
}

fn json_to_string_lossy(j: &Json) -> String {
    match j {
        Json::String(s) => s.clone(),
        Json::Number(n) => n.to_string(),
        Json::Bool(b)   => b.to_string(),
        Json::Null      => String::new(),
        other           => other.to_string(),
    }
}

fn kwargs_default(kwargs: &Kwargs) -> Result<Option<Json>, Error> {
    let opt: Option<Value> = kwargs.get("default")?;
    Ok(opt.as_ref().map(v_to_json))
}

/* ---------------- filter implementations (MiniJinja ABI via filter_api) ---------------- */

pub fn string_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
    }
    Ok(Value::from(escape_markup(&json_to_string_lossy(&jv))))
}

pub fn array_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    let jv = v_to_json(&value);
    let out = match jv {
        Json::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|it| {
                let s = if let Json::String(s) = it { s.clone() } else { it.to_string() };
                escape_markup(&s)
            }).collect();
            format!("({})", items.join(", "))
        }
        Json::String(s) => {
            let items: Vec<String> = s.lines().map(|l| escape_markup(l.trim())).collect();
            format!("({})", items.join(", "))
        }
        Json::Null => {
            if let Some(defj) = kwargs_default(&kwargs)? {
                match defj {
                    Json::Array(arr) => {
                        let items: Vec<String> = arr.iter().map(|it| {
                            let s = if let Json::String(s) = it { s.clone() } else { it.to_string() };
                            escape_markup(&s)
                        }).collect();
                        format!("({})", items.join(", "))
                    }
                    Json::String(s) => {
                        let items: Vec<String> = s.lines().map(|l| escape_markup(l.trim())).collect();
                        format!("({})", items.join(", "))
                    }
                    other => format!("({})", escape_markup(&other.to_string())),
                }
            } else {
                format!("({})", escape_markup(&jv.to_string()))
            }
        }
        other => format!("({})", escape_markup(&other.to_string())),
    };
    Ok(Value::from(out))
}

pub fn int_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { return Err(Error::new(ErrorKind::InvalidOperation, "Value is not an integer")); }
    }
    let to_ok_i64 = |n: &serde_json::Number| -> Result<i64, Error> {
        if let Some(i) = n.as_i64() { Ok(i) }
        else if let Some(f) = n.as_f64() {
            if f.fract() == 0.0 { Ok(f as i64) }
            else { Err(Error::new(ErrorKind::InvalidOperation, "Value is not an integer")) }
        } else {
            Err(Error::new(ErrorKind::InvalidOperation, "Value is not a valid number"))
        }
    };
    let s = match jv {
        Json::Number(ref n) => to_ok_i64(n)?.to_string(),
        Json::String(ref s) => s.parse::<i64>()
            .map_err(|_| Error::new(ErrorKind::InvalidOperation, "String cannot be parsed as integer"))?
            .to_string(),
        _ => return Err(Error::new(ErrorKind::InvalidOperation, "Value is not an integer")),
    };
    Ok(Value::from(s))
}

pub fn bool_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { return Err(Error::new(ErrorKind::InvalidOperation, "Value is not a boolean")); }
    }
    let b = match jv {
        Json::Bool(b) => b,
        Json::String(s) => match s.to_lowercase().as_str() {
            "true" | "yes" | "1" => true,
            "false" | "no" | "0" => false,
            _ => return Err(Error::new(ErrorKind::InvalidOperation, "String cannot be parsed as boolean")),
        },
        Json::Number(n) => n.as_i64()
            .map(|i| i != 0)
            .ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "Number cannot be converted to boolean"))?,
        _ => return Err(Error::new(ErrorKind::InvalidOperation, "Value is not a boolean")),
    };
    Ok(Value::from(b.to_string()))
}

pub fn datetime_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
    }
    Ok(Value::from(escape_markup(&json_to_string_lossy(&jv))))
}

pub fn dict_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { return Err(Error::new(ErrorKind::InvalidOperation, "Value is not a dictionary/object")); }
    }
    let obj = match jv {
        Json::Object(ref m) => Json::Object(m.clone()),
        _ => return Err(Error::new(ErrorKind::InvalidOperation, "Value is not a dictionary/object")),
    };
    let yaml_value: serde_yaml::Value = serde_json::from_value(obj.clone())
        .map_err(|e| Error::new(ErrorKind::BadSerialization, format!("Failed to convert object: {e}")))?;
    let toml_string = toml::to_string(&yaml_value)
        .map_err(|e| Error::new(ErrorKind::BadSerialization, format!("Failed to serialize to TOML: {e}")))?;
    Ok(Value::from(format!("toml(bytes(\"{}\"))", sanitize_string(&toml_string))))
}

pub fn body_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    let content = match v_to_json(&value) {
        Json::String(s) => s,
        other => other.to_string(),
    };
    Ok(Value::from(mark_to_typst(&content)))
}