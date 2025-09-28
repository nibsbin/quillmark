//! filters.rs (external crate)
//! Implement filters against the core crate's stable filter ABI (no minijinja deps here).

use serde_json::{self, Value as Json};

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

/* ---------- light heuristics (no extra deps) ---------- */

fn is_date_like(s: &str) -> bool {
    // Accept "YYYY-MM-DD"
    fn is_yyyy_mm_dd(x: &str) -> bool {
        if x.len() == 10 {
            let b = x.as_bytes();
            b[4] == b'-' && b[7] == b'-'
                && b[..4].iter().all(|c| c.is_ascii_digit())
                && b[5..7].iter().all(|c| c.is_ascii_digit())
                && b[8..].iter().all(|c| c.is_ascii_digit())
        } else { false }
    }
    // Accept "MM/DD/YYYY" or "M/D/YYYY"
    fn is_mm_dd_yyyy(x: &str) -> bool {
        let parts: Vec<_> = x.split('/').collect();
        if parts.len() != 3 { return false; }
        let (m, d, y) = (parts[0], parts[1], parts[2]);
        !m.is_empty() && !d.is_empty() && y.len() == 4
            && m.chars().all(|c| c.is_ascii_digit())
            && d.chars().all(|c| c.is_ascii_digit())
            && y.chars().all(|c| c.is_ascii_digit())
    }
    // Accept "YYYY/MM/DD"
    fn is_yyyy_mm_dd_slash(x: &str) -> bool {
        let parts: Vec<_> = x.split('/').collect();
        if parts.len() != 3 { return false; }
        let (y, m, d) = (parts[0], parts[1], parts[2]);
        y.len() == 4
            && y.chars().all(|c| c.is_ascii_digit())
            && m.chars().all(|c| c.is_ascii_digit())
            && d.chars().all(|c| c.is_ascii_digit())
    }
    is_yyyy_mm_dd(s) || is_mm_dd_yyyy(s) || is_yyyy_mm_dd_slash(s)
}

fn empty_of_same_kind(j: &Json) -> Json {
    match j {
        Json::String(s) if is_date_like(s) => Json::String("01/01/2024".to_string()),
        Json::String(_) => Json::String(String::new()),
        Json::Number(n) => {
            if n.is_i64() { Json::from(0) }
            else if n.is_u64() { Json::from(0u64) }
            else { Json::from(0.0) }
        }
        Json::Bool(_) => Json::Bool(false),
        Json::Object(_) => Json::Object(serde_json::Map::new()),
        Json::Array(_) => Json::Array(Vec::new()),
        Json::Null => Json::Null,
    }
}

/* ---------------- filter implementations (MiniJinja ABI via filter_api) ---------------- */

pub fn string_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    // Default: empty string
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { jv = Json::String(String::new()); }
    }
    let s = escape_string(&json_to_string_lossy(&jv));
    let injector = format!("\"{}\"", &s);
    Ok(Value::from(injector))
}

pub fn array_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
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

    // Determine representative element kind (first non-null), then prepend empty-of-same-kind.
    let rep_kind = arr.iter().find(|e| !e.is_null());
    let mut out = arr.clone();

    let empty_item = if let Some(rep) = rep_kind {
        empty_of_same_kind(rep)
    } else {
        // Unknown kind → provide a sensible empty string placeholder
        Json::String(String::new())
    };

    // Only prepend if not already the first element
    let should_prepend = match (out.first(), &empty_item) {
        (Some(Json::String(a)), Json::String(b)) => a != b,
        (Some(Json::Number(a)), Json::Number(b)) => a != b,
        (Some(Json::Bool(a)),   Json::Bool(b))   => a != b,
        (Some(Json::Object(a)), Json::Object(b)) => a != b,
        (Some(Json::Array(a)),  Json::Array(b))  => a != b,
        (Some(Json::Null),      Json::Null)      => false,
        (Some(_),               _)               => true,
        (None,                  _)               => true,
    };
    if should_prepend {
        out.insert(0, empty_item);
    }

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

pub fn int_filter(_state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    // Default: 0
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { jv = Json::from(0); }
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
    // Default: false
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { jv = Json::Bool(false); }
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
    // Default: "01/01/2024"
    let mut jv = v_to_json(&value);
    if jv.is_null() {
        if let Some(defj) = kwargs_default(&kwargs)? { jv = defj; }
        else { jv = Json::String("01/01/2024".to_string()); }
    }
    Ok(Value::from(escape_string(&json_to_string_lossy(&jv))))
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
