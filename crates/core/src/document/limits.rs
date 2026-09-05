//! Nesting-depth budget for document parsing.
//!
//! [`MAX_YAML_DEPTH`] governs the maximum container nesting accepted during
//! parsing, preventing denial-of-service via deeply nested input. Sibling
//! size/count limits (input bytes, YAML bytes, card count, field count) live
//! in [`crate::error`].

/// Maximum nesting depth, counted in **container levels** (64).
///
/// The unit is container levels, not nodes: a value may nest up to this many
/// arrays/objects deep, and the scalar leaf at the bottom is *not* charged a
/// level. So `{"a":{"a":…{"a":1}}}` with exactly 64 objects is accepted, and
/// 65 objects is rejected: whether the deepest container is empty, holds a
/// scalar, or holds another container. Every ingestion boundary enforces this
/// identical shape: the YAML parser via [`serde_saphyr::Budget`], the payload
/// paths via [`crate::value::json_depth_exceeds`], and the bindings' own
/// converters (e.g. Python `py_to_json_at`).
///
/// The value states what `serde_saphyr::Budget::default()` enforces, which
/// [`yaml_parse_options`] adopts wholesale; the two are held in agreement by
/// `yaml_depth_boundary_matches_max_yaml_depth`.
///
/// Prevents stack overflow from deeply nested input.
pub const MAX_YAML_DEPTH: usize = 64;

/// serde-saphyr parse options for every YAML entry point.
///
/// The default [`serde_saphyr::Budget`] carries the depth limit
/// [`MAX_YAML_DEPTH`] names, alongside the size and count caps.
pub(crate) fn yaml_parse_options() -> serde_saphyr::Options {
    serde_saphyr::Options::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// YAML nested `levels` containers deep, the innermost holding a scalar.
    fn nested_yaml(levels: usize) -> String {
        let mut yaml = String::new();
        for i in 0..levels - 1 {
            yaml.push_str(&"  ".repeat(i));
            yaml.push_str("nest:\n");
        }
        yaml.push_str(&"  ".repeat(levels - 1));
        yaml.push_str("leaf: 1\n");
        yaml
    }

    fn parses(levels: usize) -> bool {
        serde_saphyr::from_str_with_options::<serde_json::Value>(
            &nested_yaml(levels),
            yaml_parse_options(),
        )
        .is_ok()
    }

    #[test]
    fn yaml_depth_boundary_matches_max_yaml_depth() {
        assert!(
            parses(MAX_YAML_DEPTH),
            "{MAX_YAML_DEPTH} container levels must be accepted"
        );
        assert!(
            !parses(MAX_YAML_DEPTH + 1),
            "{} container levels must be rejected",
            MAX_YAML_DEPTH + 1
        );
    }
}
