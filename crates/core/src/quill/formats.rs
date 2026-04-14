use std::sync::LazyLock;
use time::format_description::{self, FormatItem};

pub(crate) static DATE_FORMAT: LazyLock<Vec<FormatItem<'static>>> = LazyLock::new(|| {
    format_description::parse("[year]-[month]-[day]").expect("valid date format description")
});
