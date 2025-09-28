//! filters.rs (external crate)
//! Implement filters against the core crate's stable filter ABI (no minijinja deps here).

use serde_json::{self, Value as Json};
use toml::{self, value::Datetime as TomlDatetime, Value as TomlValue};
use chrono::Utc;

use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};

use crate::convert::{escape_string, mark_to_typst};

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

fn none_value() -> Value {
    Value::from("none")
}

/* (Removed unused light-heuristics helpers; lines_filter now always returns strings.) */

/* ---------------- filter implementations (MiniJinja ABI via filter_api) ---------------- */

pub fn string_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    // Default: value from kwargs["default"], else empty string
    let mut jv = v_to_json(&value);

    // If input is null, prefer the provided default if any.
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { jv = Json::String(String::new()); }
    } else {
        // If input is an empty string, also allow the default to take effect.
        if let Some(s) = jv.as_str() {
            if s.is_empty() {
                if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
            }
        }
    }

    // If the value is the sentinel string "none", inject the Typst `none` literal
    // (unquoted) so templates can opt-out of rendering.
    if let Some(s) = jv.as_str() {
        if s == "none" {
            return Ok(none_value());
        }
    }

    let s = escape_string(&json_to_string_lossy(&jv));
    let injector = format!("\"{}\"", &s);
    Ok(Value::from(injector))
}

pub fn lines_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    // Select input or default; if null and no default, use [].
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        jv = if let Some(defj) = kwargs_default(&kwargs)? {
            defj
        } else {
            Json::Array(Vec::new())
        };
    }

    // Must be an array
    let arr = match jv.as_array() {
        Some(a) => a,
        None => return Err(Error::new(
            ErrorKind::InvalidOperation,
            "Value (or default) is not an array",
        )),
    };

    // Convert each element to string
    let out: Vec<Json> = arr.iter().map(|e| Json::String(json_to_string_lossy(e))).collect();

    // Serialize directly to compact JSON
    let serialized = serde_json::to_string(&Json::Array(out)).map_err(|e| {
        Error::new(
            ErrorKind::BadSerialization,
            format!("Failed to serialize array to JSON: {e}"),
        )
    })?;

    // Escape once for embedding into Typst source
    Ok(Value::from(format!(
        "json(bytes(\"{}\"))",
        escape_string(&serialized)
    )))
}

pub fn datetime_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    // Default: "01/01/2024"
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else {
            // Default to today's date in YYYY-MM-DD form
            let today = Utc::now().date_naive();
            jv = Json::String(today.to_string());
        }
    }
    let raw = json_to_string_lossy(&jv).trim().to_string();

    // Strict: only accept valid TOML datetimes. Do not attempt to guess
    // or normalize other formats. Return a clear error if parsing fails.
    let dt = raw.parse::<TomlDatetime>().map_err(|_| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("Value is not a valid TOML datetime (ISO date or RFC3339 expected): got {:?}", raw),
        )
    })?;

    let mut table = toml::map::Map::new();
    table.insert("date".to_string(), TomlValue::Datetime(dt));
    let doc = TomlValue::Table(table);

    let serialized = toml::to_string(&doc).map_err(|e| {
        Error::new(
            ErrorKind::BadSerialization,
            format!("Failed to serialize TOML date: {e}"),
        )
    })?;

    let injector = format!("toml(bytes(\"{}\")).date", escape_string(&serialized));
    Ok(Value::from(injector))
}

pub fn dict_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    // Choose value or default; fall back to {}
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? {
            jv = defj;
        } else {
            jv = Json::Object(serde_json::Map::new());
        }
    }

    // Require an object
    if jv.as_object().is_none() {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "Value is not an object (dictionary)",
        ));
    }

    // Serialize directly to compact JSON
    let serialized = serde_json::to_string(&jv).map_err(|e| {
        Error::new(
            ErrorKind::BadSerialization,
            format!("Failed to serialize object to JSON: {e}"),
        )
    })?;

    // Escape for embedding into Typst source
    Ok(Value::from(format!(
        "json(bytes(\"{}\"))",
        escape_string(&serialized)
    )))
}

pub fn body_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
    // Default: empty string (renders nothing)
    let content = match v_to_json(&value) {
        Json::Null => String::new(),
        Json::String(s) => s,
        other => other.to_string(),
    };
    let markup = mark_to_typst(&content);
    let injector = format!("eval(\"{}\", mode: \"markup\")", escape_string(&markup));
    Ok(Value::from(injector))
}
