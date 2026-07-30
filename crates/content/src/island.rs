//! Island types — the closed dispatch authority.
//!
//! [`Island::island_type`](crate::model::Island::island_type) stays an **open**
//! string on the wire: an unknown type (one this build lacks, arriving via
//! storage) round-trips opaque. [`KnownIslandType`] is that string's *parse* into
//! the closed set, and every site that dispatches on a type matches
//! [`KnownIslandType::parse`]. The `Some(k)` arm is exhaustive, so **adding a
//! variant is a compile error at every dispatch site**; a known type wired into
//! only some sites cannot reach the `None` path, which is the open set's alone.
//!
//! Behavior a type must supply lives on the enum, one method per seam (loss
//! class, cell marks, shape normalize/validate). The two projections that can't
//! live in this crate — markdown emit ([`crate::export`]) and Typst emit (the
//! typst backend) — match on the enum where they are, so the guarantee crosses
//! the crate boundary. The `table` codec stays in [`crate::serial`]; this module
//! dispatches into it.

use crate::model::{Invariant, Island, Loss, Mark};
use serde_json::Value;

/// The island types this build understands. The wire discriminator is the open
/// string [`Island::island_type`](crate::model::Island::island_type); this is its
/// closed parse. Adding a variant forces every dispatch arm — here and in the two
/// emitters — to be supplied before the workspace compiles.
///
/// **Deliberately not `#[non_exhaustive]`**, unlike every other public enum in
/// this crate. The attribute forces a `_` arm on downstream matchers, and two of
/// the dispatch sites this enum exists to police — the typst backend's markdown
/// and Typst emitters — are in another crate, where that `_` would swallow
/// exactly the compile error described above.
///
/// The cost is that **adding a variant is a semver-major change**, since a
/// downstream exhaustive match stops compiling. That is the price of the
/// guarantee, paid on purpose: an island type wired into some emitters and not
/// others projects the island away silently, which is worse than a major bump.
/// Routing an internal exhaustive enum through a `#[non_exhaustive]` public
/// re-export would buy the semver back and lose the guarantee — not the trade
/// this module wants.
///
/// The trade runs the other way for [`MarkKind`](crate::model::MarkKind) and
/// [`LineKind`](crate::model::LineKind), which do carry the attribute. What an
/// unhandled arm costs there is decoration: the text still renders, the mark
/// loses its delimiters, the line lowers as a paragraph. An unhandled island
/// type costs the island — content leaves the projection entirely. A silent
/// gap in a total function is worth a major bump; a silent gap in a
/// degradation ladder that already has an `Unknown` rung is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownIslandType {
    /// `{header, rows, aligns}` with inline `{text, marks}` cells. Mark-carrying,
    /// shape-validated (one column count, `\n`-free cells).
    Table,
    /// `{url, alt}`. No cell model, no shape invariants.
    Image,
}

impl KnownIslandType {
    /// Every known type. The one enumeration point, so a reader that needs the
    /// closed set whole (the WASM guards' known-name table, say) asks rather than
    /// re-spelling it.
    ///
    /// A slice, not an array: an array's length is part of its type, so a third
    /// island type would break every caller that names this constant's type on
    /// top of the exhaustive-match break the enum already promises.
    pub const ALL: &'static [KnownIslandType] = &[KnownIslandType::Table, KnownIslandType::Image];

    /// The wire discriminator; `parse(k.as_str()) == Some(k)` for every variant.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Image => "image",
        }
    }

    /// Parse a wire discriminator into the closed set. `None` is the open-set
    /// escape hatch — a genuinely-unknown type, round-tripped opaque — never a
    /// known type wired into only some sites.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "table" => Some(Self::Table),
            "image" => Some(Self::Image),
            _ => None,
        }
    }

    /// The best markdown-projection loss class this type achieves — the ceiling
    /// the importer stamps at mint. A per-island [`Loss`] may sit *below* it (a
    /// table cell dropping an inline image's url) but never above.
    pub fn default_loss(self) -> Loss {
        match self {
            Self::Table => Loss::Lossless,
            Self::Image => Loss::Lossless,
        }
    }

    /// This type's `(text, marks)` cells — the set that participates in mark
    /// normalization and cell-mark validation. Empty for a type with no cell
    /// model, so neither normalize nor validate can silently skip a new type
    /// (which would void the canonical-bytes guarantee for its cells).
    pub fn cell_marks(self, props: &Value) -> Vec<(String, Vec<Mark>)> {
        match self {
            Self::Table => crate::serial::table_cells(props),
            Self::Image => Vec::new(),
        }
    }

    /// Repair this type's props to canonical shape in place (a no-op for a type
    /// with no shape invariants) — the normalize-side dispatch.
    pub fn normalize_props(self, props: &mut Value) {
        match self {
            Self::Table => crate::serial::normalize_table_props(props),
            Self::Image => {}
        }
    }

    /// This type's shape violation, if any (`None` for a well-formed or shape-free
    /// island) — the validate-side twin of [`normalize_props`](Self::normalize_props),
    /// which guarantees this returns `None`.
    pub fn shape_error(self, props: &Value) -> Option<Invariant> {
        match self {
            Self::Table => crate::serial::table_shape_error(props),
            Self::Image => None,
        }
    }
}

// The three `Island`-taking wrappers below are where the open set's `None` arm
// is answered — once each, so `model.rs` calls a total function and no site
// re-decides what an unknown type does.

/// [`KnownIslandType::normalize_props`] for a known type; for an unknown one, a
/// no-op that leaves the opaque props verbatim.
pub(crate) fn normalize_island_structure(island: &mut Island) {
    if let Some(k) = KnownIslandType::parse(&island.island_type) {
        k.normalize_props(&mut island.props);
    }
}

/// [`KnownIslandType::cell_marks`] for a known type; empty for an unknown one.
pub(crate) fn island_cell_marks(island: &Island) -> Vec<(String, Vec<Mark>)> {
    match KnownIslandType::parse(&island.island_type) {
        Some(k) => k.cell_marks(&island.props),
        None => Vec::new(),
    }
}

/// [`KnownIslandType::shape_error`] for a known type; `None` for an unknown one.
/// [`normalize_island_structure`] guarantees `None` for the known ones too.
pub(crate) fn island_shape_error(island: &Island) -> Option<Invariant> {
    KnownIslandType::parse(&island.island_type).and_then(|k| k.shape_error(&island.props))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_types_round_trip() {
        for k in [KnownIslandType::Table, KnownIslandType::Image] {
            assert_eq!(KnownIslandType::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn unknown_type_parses_to_none() {
        assert_eq!(KnownIslandType::parse("figure"), None);
        assert_eq!(KnownIslandType::parse(""), None);
    }
}
