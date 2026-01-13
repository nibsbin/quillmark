use crate::convert::escape_string;

/// Helper function to inject JSON into Typst code.
/// Exposed for fuzzing tests.
#[doc(hidden)]
pub fn inject_json(bytes: &str) -> String {
    format!("json(bytes(\"{}\"))", escape_string(bytes))
}
