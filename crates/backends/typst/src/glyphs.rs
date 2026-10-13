//! Input text no loaded font covers. Typst's shaper draws such a character
//! as a `.notdef` box, or as nothing when no font was selected before it, and
//! reports neither.

use std::collections::BTreeSet;

use quillmark_core::error::{Diagnostic, Severity};
use typst::text::{is_default_ignorable, FontBook};

const CODE: &str = "typst::missing_glyph";

/// The message names at most this many characters; the rest are counted.
const NAMED: usize = 8;

/// One `typst::missing_glyph` naming, in code-point order, every character of
/// the plate JSON `data`'s strings that no font in `book` covers.
pub(crate) fn missing(book: &FontBook, data: &serde_json::Value) -> Option<Diagnostic> {
    let mut seen = BTreeSet::new();
    collect(data, &mut seen);
    let covered = |c: char| {
        (0..)
            .map_while(|i| book.info(i))
            .any(|info| info.coverage.contains(u32::from(c)))
    };
    let chars: Vec<char> = seen.into_iter().filter(|&c| !covered(c)).collect();
    if chars.is_empty() {
        return None;
    }
    let mut named: Vec<String> = chars
        .iter()
        .take(NAMED)
        .map(|c| format!("`{c}` (U+{:04X})", u32::from(*c)))
        .collect();
    if chars.len() > NAMED {
        named.push(format!("{} more", chars.len() - NAMED));
    }
    Some(
        Diagnostic::new(
            Severity::Warning,
            format!(
                "no loaded font has a glyph for {}; the page draws an empty box or nothing in its place",
                named.join(", ")
            ),
        )
        .with_code(CODE.to_string())
        .with_hint(
            "bundle a font that covers these characters under the quill's `assets/fonts/`"
                .to_string(),
        ),
    )
}

fn collect(value: &serde_json::Value, chars: &mut BTreeSet<char>) {
    match value {
        serde_json::Value::String(s) => chars.extend(
            s.chars()
                .filter(|&c| !c.is_whitespace() && !c.is_control() && !is_default_ignorable(c)),
        ),
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect(v, chars)),
        serde_json::Value::Object(map) => map.values().for_each(|v| collect(v, chars)),
        _ => {}
    }
}
