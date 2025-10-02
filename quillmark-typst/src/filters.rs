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

pub fn body_filter(_state: &State, value: Value, _kwargs: Kwargs) -> Result<Value, Error> {
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
