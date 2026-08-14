//! The variant axis: which enum-discriminated fields are in play.
//!
//! A field hoisted out of an enum's `variants:` carries a
//! [`VariantOf`](super::VariantOf) back-reference and exists only where its
//! discriminant resolves to that value. Each projection cuts a different ladder
//! to resolve it, so the predicate takes the resolution as a closure.

use indexmap::IndexMap;

use super::{CardSchema, FieldSchema};
use crate::value::QuillValue;

/// Whether `field` is in play when each discriminant resolves through
/// `resolve`. The blank is no member, so it activates nothing.
pub(crate) fn in_play(field: &FieldSchema, resolve: impl Fn(&str) -> String) -> bool {
    match &field.variant_of {
        None => true,
        Some(variant) => resolve(&variant.field) == variant.value,
    }
}

/// The discriminant value a *document* holds: authored non-null › `default:` ›
/// blank. The render ladder, minus `example:`, which never enters it.
pub(crate) fn document_value(
    card: &CardSchema,
    name: &str,
    authored: Option<&QuillValue>,
) -> String {
    // Null ≡ absent, so only those two fall through to the schema rung: an
    // authored blank is an answer, and it activates nothing.
    if let Some(value) = authored.filter(|v| !v.as_json().is_null()) {
        return value.as_str().unwrap_or_default().to_string();
    }
    card.fields
        .get(name)
        .and_then(|field| field.default.as_ref())
        .and_then(|d| d.as_str())
        .unwrap_or_default()
        .to_string()
}

/// The discriminant value the **blueprint** shows: its cell's value axis,
/// `default:` › `example:` › the blank. The form's own answer decides which
/// variant the form shows.
pub(crate) fn blueprint_value(card: &CardSchema, name: &str) -> String {
    let Some(field) = card.fields.get(name) else {
        return String::new();
    };
    field
        .default
        .as_ref()
        .or(field.example.as_ref())
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// The discriminant value a **seed** renders: `example:` (the one source seeding
/// commits) › `default:` › blank.
pub(crate) fn seed_value(card: &CardSchema, name: &str) -> String {
    let Some(field) = card.fields.get(name) else {
        return String::new();
    };
    field
        .example
        .as_ref()
        .or(field.default.as_ref())
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Discriminant → value → the fields hoisted out of it, in hoisted order.
/// Rebuilt from the flat field map, so `variant_of` stays the one carrier.
pub(crate) fn index(card: &CardSchema) -> IndexMap<&str, IndexMap<&str, Vec<&str>>> {
    let mut out: IndexMap<&str, IndexMap<&str, Vec<&str>>> = IndexMap::new();
    for (name, field) in &card.fields {
        let Some(variant) = &field.variant_of else {
            continue;
        };
        out.entry(variant.field.as_str())
            .or_default()
            .entry(variant.value.as_str())
            .or_default()
            .push(name.as_str());
    }
    out
}

#[cfg(test)]
mod tests;
