//! Pre-scan of a metadata fence's YAML content to recover features that
//! serde_saphyr discards.
//!
//! Two features are recovered here:
//!
//! 1. **Top-level comments.** YAML comments are dropped by the YAML parser.
//!    To round-trip them as [`FrontmatterItem::Comment`], we extract them
//!    before parsing.
//!
//! 2. **`!fill` tags.** Custom YAML tags are accepted and dropped by
//!    serde_saphyr; the value survives but the tag annotation is lost. We
//!    detect `!fill` on top-level scalar fields, strip the tag from the
//!    cleaned YAML (so serde_saphyr sees a plain scalar), and record a
//!    `fill: true` marker on the resulting `Field` item.
//!
//! Anything else is left to the YAML parser. Nested comments inside block
//! mappings/sequences are silently dropped; we emit a single
//! `parse::comments_in_nested_yaml_dropped` warning per document when we
//! encounter the first one. Other custom tags (`!include`, `!env`, …) are
//! stripped with a `parse::unsupported_yaml_tag` warning.

use crate::Diagnostic;
use crate::Severity;

/// One ordered hint extracted from the fence body.
///
/// `Comment` stands alone; `Field` captures only the `fill` flag because the
/// value is produced by serde_saphyr parsing the cleaned text. The matching
/// YAML key is the lookup key into the parsed map.
#[derive(Debug, Clone, PartialEq)]
pub enum PreItem {
    Field { key: String, fill: bool },
    Comment(String),
}

/// Output of [`prescan_fence_content`].
#[derive(Debug, Clone, Default)]
pub struct PreScan {
    /// YAML text with `!fill` tags stripped and top-level comment lines
    /// removed. Suitable for feeding into serde_saphyr.
    pub cleaned_yaml: String,
    /// Ordered items discovered in source order — fields (with fill flags)
    /// and top-level comments.
    pub items: Vec<PreItem>,
    /// Warnings produced during the scan.
    pub warnings: Vec<Diagnostic>,
    /// Unsupported-fill-target errors. The parser turns these into
    /// `ParseError::InvalidStructure` rejections per tasking 02.
    pub fill_target_errors: Vec<String>,
}

/// Scan the body of a YAML metadata fence.
///
/// `content` is the text between the opening and closing `---` markers
/// (exclusive), with leading/trailing whitespace preserved.
pub fn prescan_fence_content(content: &str) -> PreScan {
    let mut out = PreScan::default();
    let mut saw_nested_comment = false;

    // We operate on the raw text to preserve positions. `lines()` strips
    // line endings; we rebuild with `\n` which is what serde_saphyr expects.
    let lines: Vec<&str> = content.split('\n').collect();
    let mut cleaned_lines: Vec<String> = Vec::with_capacity(lines.len());

    for raw_line in &lines {
        let line = *raw_line;

        // Preserve the original line for any pass-through cases.
        let trimmed = line.trim_start_matches([' ', '\t']);

        // Top-level lines have no leading whitespace. That means the
        // dedented portion equals the original line. (Serde_saphyr reads
        // YAML relative to the leftmost column; indentation inside the
        // fence is carried verbatim.)
        let is_top_level = line.len() == trimmed.len();

        // Case 1: own-line comment.
        if trimmed.starts_with('#') {
            if is_top_level {
                // Preserve the comment text (without the leading `#` and
                // one optional space).
                let mut text = &trimmed[1..];
                if text.starts_with(' ') {
                    text = &text[1..];
                }
                out.items.push(PreItem::Comment(text.to_string()));
                // Don't emit any line into the cleaned YAML — serde_saphyr
                // ignores comments but omitting the line avoids any
                // ambiguity.
                continue;
            } else {
                // Nested comment (inside a block mapping/sequence). Drop
                // silently and warn once per document.
                if !saw_nested_comment {
                    saw_nested_comment = true;
                    out.warnings.push(
                        Diagnostic::new(
                            Severity::Warning,
                            "YAML comments inside nested values are dropped during parse; only top-level frontmatter comments round-trip".to_string(),
                        )
                        .with_code("parse::comments_in_nested_yaml_dropped".to_string()),
                    );
                }
                // Keep the line out of the cleaned YAML so parsing isn't
                // confused. (Comments are structurally transparent to YAML,
                // so we can just drop them.)
                continue;
            }
        }

        // Case 2: top-level field line with possible `!fill` tag and/or
        // trailing comment.
        if is_top_level {
            if let Some((key, after_colon)) = split_key(line) {
                let (value_part, trailing_comment) = split_trailing_comment(&after_colon);

                let (fill, value_without_tag, had_non_fill_tag, fill_target_err) =
                    inspect_fill_and_tags(&value_part, &key);

                if had_non_fill_tag {
                    out.warnings.push(
                        Diagnostic::new(
                            Severity::Warning,
                            format!(
                                "YAML tag on key `{}` is not supported; the tag has been dropped and the value kept",
                                key
                            ),
                        )
                        .with_code("parse::unsupported_yaml_tag".to_string()),
                    );
                }
                if let Some(err) = fill_target_err {
                    out.fill_target_errors.push(err);
                }

                out.items.push(PreItem::Field {
                    key: key.clone(),
                    fill,
                });

                // Rebuild the line without the `!fill` tag (and without
                // the trailing comment, since that goes on its own
                // line now).
                let cleaned = format!("{}:{}", key, value_without_tag);
                cleaned_lines.push(cleaned);

                if let Some(c) = trailing_comment {
                    let mut text = c.trim_start_matches('#');
                    if text.starts_with(' ') {
                        text = &text[1..];
                    }
                    out.items.push(PreItem::Comment(text.to_string()));
                }

                continue;
            }
        }

        // Everything else: pass through verbatim.
        cleaned_lines.push(line.to_string());
    }

    out.cleaned_yaml = cleaned_lines.join("\n");
    out
}

/// Split a line into `(key, rest_after_colon)`. Returns `None` if the line
/// does not start with a bare YAML key.
fn split_key(line: &str) -> Option<(String, String)> {
    // Must start at column 0.
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    // Identifier-like keys only. YAML allows more, but Quillmark's schema
    // restricts field names to `[a-zA-Z_][a-zA-Z0-9_]*` (and reserved
    // uppercase sentinels). Anything more exotic falls through to the
    // unmodified path and will be parsed (or rejected) by serde_saphyr.
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b':' {
        return None;
    }
    let key = line[..i].to_string();
    let rest = line[i + 1..].to_string();
    Some((key, rest))
}

/// Split a value string into `(value, trailing_comment)`.
///
/// Trailing comments begin with ` #` or `\t#` outside of any quoted string.
/// This is a simple scanner: it respects `"..."` and `'...'` quoting.
fn split_trailing_comment(value: &str) -> (String, Option<String>) {
    let bytes = value.as_bytes();
    let mut i = 0;
    let mut prev_was_ws = true; // allow `key:#` edge case to NOT be a comment
    let mut in_dq = false;
    let mut in_sq = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_dq {
            if b == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_dq = false;
            }
        } else if in_sq {
            if b == b'\'' {
                in_sq = false;
            }
        } else {
            if b == b'"' {
                in_dq = true;
            } else if b == b'\'' {
                in_sq = true;
            } else if b == b'#' && prev_was_ws {
                let v = value[..i].trim_end().to_string();
                let c = value[i..].to_string();
                return (v, Some(c));
            }
        }
        prev_was_ws = matches!(b, b' ' | b'\t');
        i += 1;
    }
    (value.to_string(), None)
}

/// Inspect the value portion of a field line for `!fill` and other tags.
///
/// Returns `(fill, value_without_tag, had_other_tag, fill_target_err)`.
///
/// - `fill`: `true` when the value starts with `!fill`.
/// - `value_without_tag`: the same text with the `!fill` tag stripped;
///   leading whitespace is preserved so YAML parsing still sees a clean
///   scalar.
/// - `had_other_tag`: `true` when a non-`!fill` `!tag` was found at the
///   start of the value. The tag is *not* stripped (serde_saphyr tolerates
///   and drops unknown tags), so callers get a warning only.
/// - `fill_target_err`: populated when `!fill` is applied to a block
///   mapping or sequence (non-scalar). Per tasking 02 that is rejected.
fn inspect_fill_and_tags(value: &str, key: &str) -> (bool, String, bool, Option<String>) {
    let trimmed = value.trim_start();
    let leading_ws_len = value.len() - trimmed.len();

    // Exactly empty / null (e.g. `key:` with nothing) — not a fill target.
    if trimmed.is_empty() {
        return (false, value.to_string(), false, None);
    }

    // `!fill` alone on the line (bare tag, no value) → null placeholder.
    if trimmed == "!fill" {
        // Replace the tag with nothing; leave the leading whitespace so the
        // line shape is preserved (serde_saphyr treats `key: ` as null).
        let reconstructed = value[..leading_ws_len].to_string();
        return (true, reconstructed, false, None);
    }

    // `!fill <value>` → strip tag, record fill=true.
    if let Some(rest) = trimmed.strip_prefix("!fill") {
        // Must be followed by whitespace or end-of-value to count; otherwise
        // it's `!fillwhatever` which is a non-`!fill` tag.
        if rest.starts_with(' ') || rest.starts_with('\t') || rest.is_empty() {
            let rest_trim = rest.trim_start();
            // Flow-mapping / flow-sequence explicitly rejected; `!fill` only
            // applies to plain scalars (or null).
            let starts_block = rest_trim.starts_with('[') || rest_trim.starts_with('{');
            let err = if starts_block {
                Some(format!(
                    "`!fill` on key `{}` targets a non-scalar value; only scalars (string, int, float, bool, null) may be tagged `!fill`",
                    key
                ))
            } else {
                None
            };
            // Reconstruct: one space + the rest (trimmed) so the cleaned
            // text reads `key: rest`.
            let reconstructed = if rest_trim.is_empty() {
                value[..leading_ws_len].to_string()
            } else {
                format!(" {}", rest_trim)
            };
            return (true, reconstructed, false, err);
        }
    }

    // Any other `!tag` prefix is a non-fill custom tag. Leave the value
    // alone; serde_saphyr will strip the tag.
    if trimmed.starts_with('!') {
        return (false, value.to_string(), true, None);
    }

    (false, value.to_string(), false, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_own_line_comments() {
        let input = "# top\ntitle: foo\n# mid\nauthor: bar\n";
        let out = prescan_fence_content(input);
        assert_eq!(
            out.items,
            vec![
                PreItem::Comment("top".to_string()),
                PreItem::Field {
                    key: "title".to_string(),
                    fill: false,
                },
                PreItem::Comment("mid".to_string()),
                PreItem::Field {
                    key: "author".to_string(),
                    fill: false,
                },
            ]
        );
    }

    #[test]
    fn splits_trailing_comments() {
        let input = "title: foo # inline\n";
        let out = prescan_fence_content(input);
        assert_eq!(
            out.items,
            vec![
                PreItem::Field {
                    key: "title".to_string(),
                    fill: false,
                },
                PreItem::Comment("inline".to_string()),
            ]
        );
        assert!(out.cleaned_yaml.contains("title: foo"));
        assert!(!out.cleaned_yaml.contains("inline"));
    }

    #[test]
    fn detects_fill_on_scalar() {
        let input = "dept: !fill Department\n";
        let out = prescan_fence_content(input);
        assert_eq!(
            out.items,
            vec![PreItem::Field {
                key: "dept".to_string(),
                fill: true,
            }]
        );
        assert!(out.cleaned_yaml.contains("dept: Department"));
        assert!(!out.cleaned_yaml.contains("!fill"));
    }

    #[test]
    fn detects_bare_fill() {
        let input = "dept: !fill\n";
        let out = prescan_fence_content(input);
        assert_eq!(
            out.items,
            vec![PreItem::Field {
                key: "dept".to_string(),
                fill: true,
            }]
        );
        assert!(!out.cleaned_yaml.contains("!fill"));
    }

    #[test]
    fn unknown_tag_warns() {
        let input = "x: !custom value\n";
        let out = prescan_fence_content(input);
        assert!(
            out.warnings
                .iter()
                .any(|w| w.code.as_deref() == Some("parse::unsupported_yaml_tag")),
            "expected unsupported_yaml_tag warning"
        );
    }

    #[test]
    fn nested_comment_warns_once() {
        let input = "arr:\n  - a # inline\n  # own-line\n  - b\n";
        let out = prescan_fence_content(input);
        let nested = out
            .warnings
            .iter()
            .filter(|w| w.code.as_deref() == Some("parse::comments_in_nested_yaml_dropped"))
            .count();
        assert_eq!(nested, 1, "expected exactly one nested-comment warning");
    }

    #[test]
    fn fill_on_flow_sequence_errors() {
        let input = "x: !fill [1, 2]\n";
        let out = prescan_fence_content(input);
        assert!(
            !out.fill_target_errors.is_empty(),
            "expected unsupported_fill_target error"
        );
    }
}
