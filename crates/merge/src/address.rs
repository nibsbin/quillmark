//! Mapping targets: the schema-address grammar without the element step.

use indexmap::IndexMap;
use quillmark_core::{FieldSchema, FieldType};

/// The body target, spelled as the plate grammar spells it.
pub(crate) const BODY: &str = "$body";

/// Check a target against a card's declared fields: `$body`, a field, a
/// typed dictionary's property, a variant's cell or its `value` discriminant,
/// at whatever depth the schema nests. No element step: a repeated shape
/// reaches a document through `split:` or a card kind, never a table.
pub(crate) fn resolve(fields: &IndexMap<String, FieldSchema>, address: &str) -> Result<(), String> {
    if address == BODY {
        return Ok(());
    }
    let segments: Vec<&str> = address.split('.').collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err("empty segment".to_string());
    }
    let mut cursor = fields
        .get(segments[0])
        .ok_or_else(|| format!("no field '{}'", segments[0]))?;
    for (depth, seg) in segments.iter().enumerate().skip(1) {
        let so_far = segments[..depth].join(".");
        cursor = match &cursor.r#type {
            FieldType::Object => cursor
                .properties
                .as_ref()
                .and_then(|p| p.get(*seg))
                .map(|b| &**b)
                .ok_or_else(|| format!("'{so_far}' declares no property '{seg}'"))?,
            FieldType::Enum if cursor.is_variant_bearing() => {
                if *seg == "value" {
                    if depth + 1 != segments.len() {
                        return Err(format!("'{so_far}.value' is the discriminant, a scalar"));
                    }
                    return Ok(());
                }
                cursor
                    .variant_field(seg)
                    .ok_or_else(|| format!("no variant of '{so_far}' declares a cell '{seg}'"))?
            }
            other => {
                return Err(format!(
                    "'{so_far}' is {}, which has no property '{seg}'",
                    other.as_str()
                ))
            }
        };
    }
    Ok(())
}
