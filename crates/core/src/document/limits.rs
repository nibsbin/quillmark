//! Nesting-depth budget for document parsing.
//!
//! [`MAX_YAML_DEPTH`] governs the maximum container nesting accepted on the
//! payload paths, preventing denial-of-service via deeply nested input.
//! Sibling size/count limits (input bytes, YAML bytes, card count, field
//! count) live in [`crate::error`].

/// Maximum nesting depth, counted in **container levels** (64).
///
/// The unit is container levels, not nodes: a value may nest up to this many
/// arrays/objects deep, and the scalar leaf at the bottom is *not* charged a
/// level. So `{"a":{"a":…{"a":1}}}` with exactly 64 objects is accepted, and
/// 65 objects is rejected: whether the deepest container is empty, holds a
/// scalar, or holds another container. The payload paths enforce this shape
/// via [`crate::value::json_depth_exceeds`] and the bindings' own converters
/// (e.g. Python `py_to_json_at`).
///
/// The YAML parser carries its own depth limit, `serde_saphyr::Budget`'s
/// default, which this value matches.
///
/// Prevents stack overflow from deeply nested input.
pub const MAX_YAML_DEPTH: usize = 64;

/// serde-saphyr parse options for every YAML entry point.
///
/// The default [`serde_saphyr::Budget`] carries the depth, size, and count
/// caps the parser enforces.
pub(crate) fn yaml_parse_options() -> serde_saphyr::Options {
    serde_saphyr::Options::default()
}
