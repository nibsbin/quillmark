//! Font and size policy for values the flatten path draws itself.
//!
//! An AcroForm widget uses `/Helv 0 Tf` and lets the viewer auto-size, but the
//! flatten path must commit to a concrete font and size. Keeping that decision
//! here is what makes preview and flattening agree exactly.

/// Base-14 Helvetica, registered as `/Helv`, for text and choice values.
pub(crate) const TEXT_FONT: &str = "Helvetica";

/// Base-14 ZapfDingbats, registered as `/ZaDb`, for the checkbox check glyph.
pub(crate) const CHECK_FONT: &str = "ZapfDingbats";

pub(crate) const MIN_SIZE: f32 = 4.0;
pub(crate) const MAX_SIZE: f32 = 12.0;

/// Inset, in points, of value text from the field box's left edge.
pub(crate) const TEXT_INSET: f32 = 2.0;

/// Inset, in points, between the box's top edge and the first baseline.
pub(crate) const TEXT_TOP_INSET: f32 = 1.0;

/// Line height = point size × this.
pub(crate) const LINE_SPACING: f32 = 1.2;

/// Emulates the AcroForm `0 Tf` auto-size a synthesizing viewer would pick.
pub(crate) fn value_size(h: f32) -> f32 {
    (h * 0.65).clamp(MIN_SIZE, MAX_SIZE)
}

/// Larger than [`value_size`]: the check glyph reads a touch small.
pub(crate) fn check_size(h: f32) -> f32 {
    (h * 0.75).clamp(MIN_SIZE, MAX_SIZE)
}

/// Approximate advance width of the ZapfDingbats check glyph (`'4'`) as a
/// fraction of its point size, used to horizontally centre it in the box.
pub(crate) const CHECK_GLYPH_WIDTH_FACTOR: f32 = 0.6;
