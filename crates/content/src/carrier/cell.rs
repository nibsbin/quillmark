//! `qm-cell`: a table cell's alignment and span spelled as the attributes of
//! the inline pair around the cell's whole content.

use super::Element;
use crate::island::{cell_alignment, cell_span, is_weight, CELL_KEYS, CELL_SPANS};
use serde_json::Value;
use std::collections::BTreeMap;

/// The cell value attribute `name` spells with `value`, or `None` for a name
/// outside [`CELL_KEYS`] and [`CELL_SPANS`] or a value outside its set. A span
/// is spelled in decimal digits with no leading zero.
pub(crate) fn key(name: &str, value: &str) -> Option<Value> {
    if CELL_SPANS.contains(&name) {
        let digits = !value.is_empty() && !value.starts_with('0') && value.bytes().all(|b| b.is_ascii_digit());
        return digits
            .then(|| value.parse::<u64>().ok())
            .flatten()
            .filter(|n| *n >= 2 && is_weight(*n))
            .map(Value::from);
    }
    let (_, set) = CELL_KEYS.iter().find(|(key, _)| *key == name)?;
    set.contains(&value).then(|| value.into())
}

/// The pair spelling `cell`'s alignment and span, or `None` when it holds
/// none in its set.
pub(crate) fn pair(cell: &Value) -> Option<Element> {
    let (align, valign) = cell_alignment(cell);
    let (colspan, rowspan) = cell_span(cell);
    let aligned = CELL_KEYS.iter().zip([align, valign]).filter_map(|((k, _), v)| Some((k.to_string(), v?.to_string())));
    let spanned = CELL_SPANS
        .iter()
        .zip([colspan, rowspan])
        .filter(|(_, n)| *n > 1)
        .map(|(k, n)| (k.to_string(), n.to_string()));
    let attrs: BTreeMap<String, String> = aligned.chain(spanned).collect();
    (!attrs.is_empty()).then(|| Element::new(super::CELL, attrs).expect("cell attribute names are in the grammar"))
}
