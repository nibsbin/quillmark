//! Laid-out text no loaded font covers. Typst's shaper draws such a
//! character as a `.notdef` box and reports nothing.

use std::collections::BTreeSet;

use quillmark_core::error::{Diagnostic, Severity};
use typst::layout::{Frame, FrameItem};
use typst::text::is_default_ignorable;
use typst_layout::PagedDocument;

const CODE: &str = "typst::missing_glyph";

/// The message names at most this many characters; the rest are counted.
const NAMED: usize = 8;

/// One `typst::missing_glyph` naming, in code-point order, every character
/// the laid-out `document` draws as `.notdef`.
pub(crate) fn missing(document: &PagedDocument) -> Option<Diagnostic> {
    let mut seen = BTreeSet::new();
    for page in document.pages() {
        collect(&page.frame, &mut seen);
    }
    let chars: Vec<char> = seen.into_iter().collect();
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
                "no loaded font has a glyph for {}; the page draws an empty box in its place",
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

fn collect(frame: &Frame, chars: &mut BTreeSet<char>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(group) => collect(&group.frame, chars),
            FrameItem::Text(text) => chars.extend(
                text.glyphs
                    .iter()
                    .filter(|g| g.id == 0)
                    // A glyph's range is `u16` and saturates past 64 KiB into
                    // its item, where it can split a character.
                    .flat_map(|g| text.text.get(g.range()).unwrap_or_default().chars())
                    .filter(|&c| !c.is_whitespace() && !c.is_control() && !is_default_ignorable(c)),
            ),
            _ => {}
        }
    }
}
