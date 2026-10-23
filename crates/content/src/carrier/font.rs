//! `qm-font`: a run's font family and size step spelled as the attributes of
//! an inline pair around it.

use super::{Attrs, Element};
use crate::model::{is_font_family_name, FontStep, MarkKind};
use std::collections::BTreeMap;

/// The size attribute's spelling of the surrounding text's own size, which
/// imports as no size mark.
const NORMAL: &str = "normal";

/// What one open tag sets: a family, and a size where `Some(None)` is
/// `size="normal"`, which sets the surrounding size over a run another pair
/// sizes.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Font {
    pub(crate) family: Option<String>,
    pub(crate) size: Option<Option<FontStep>>,
}

impl Font {
    /// The attributes `attrs` spell, and every attribute name they hold that
    /// spells nothing: one outside `family` and `size`, a family
    /// [`is_font_family_name`] refuses, a size outside the steps and `normal`,
    /// and each the grammar refused.
    pub(crate) fn read(attrs: &Attrs) -> (Font, Vec<String>) {
        let mut font = Font::default();
        let mut refused = attrs.refused.clone();
        for (name, value) in &attrs.values {
            match name.as_str() {
                "family" if is_font_family_name(value) => font.family = Some(value.clone()),
                "size" if value == NORMAL => font.size = Some(None),
                "size" if FontStep::parse(value).is_some() => font.size = Some(FontStep::parse(value)),
                _ => refused.push(name.clone()),
            }
        }
        (font, refused)
    }

    /// Whether the tag sets anything: one that does not drops as a `<u>`
    /// carrying an attribute does.
    pub(crate) fn sets(&self) -> bool {
        self.family.is_some() || self.size.is_some()
    }

    /// The marks it sets over a run, `None` in a slot it holds at the
    /// surrounding size, each beside the keyed tag it paints.
    pub(crate) fn marks(self) -> impl Iterator<Item = (&'static str, Option<MarkKind>)> {
        let family = self.family.map(|name| ("font_family", Some(MarkKind::FontFamily { name })));
        let size = self.size.map(|step| ("font_size", step.map(|step| MarkKind::FontSize { step })));
        family.into_iter().chain(size)
    }
}

/// The pair spelling a run set in `family` at `step`, attributes sorted, or
/// `None` where it sets neither.
pub(crate) fn pair(family: Option<&str>, step: Option<FontStep>) -> Option<Element> {
    let attrs: BTreeMap<String, String> = [
        family.map(|f| ("family".to_string(), f.to_string())),
        step.map(|s| ("size".to_string(), s.as_str().to_string())),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!attrs.is_empty()).then(|| Element::new(super::FONT, attrs).expect("font attribute names are in the grammar"))
}
