//! Regression content for YAML-ambiguous string values.
//!
//! Every string in this module is "dangerous" to a YAML parser that lacks
//! type-fidelity guarantees: bare `on`, `01234`, `2024-01-15`, etc. would be
//! silently coerced to booleans, integers, or dates by a YAML 1.1 parser, or
//! misread as anchors/aliases/tags by any YAML parser.
//!
//! The canonical emitter (§9) double-quotes every string scalar with
//! JSON-style escaping, which is what buys the round-trip guarantee tested
//! here.
//!
//! `ambiguous_strings.md` declares every ambiguous field in one fixture, so
//! all categories below share a single parse → emit → re-parse cycle.

use crate::document::Document;

// ── Fixture path ──────────────────────────────────────────────────────────────

fn ambiguous_strings_fixture() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .join("..")
        .join("fixtures")
        .join("resources")
        .join("ambiguous_strings.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "Cannot read ambiguous_strings.md at {}: {}",
            path.display(),
            e
        )
    })
}

/// Parse the fixture and return the document.
fn parse_fixture() -> Document {
    let src = ambiguous_strings_fixture();
    Document::parse(&src)
        .unwrap_or_else(|e| panic!("ambiguous_strings.md failed to parse: {}", e))
        .document
}

/// Assert that a payload field is `QuillValue::String` with exactly the
/// expected bytes. `when` names which parse (first parse vs. post-round-trip)
/// this check belongs to, for failure messages.
fn assert_string_field(doc: &Document, key: &str, expected: &str, when: &str) {
    let value = doc
        .main()
        .payload()
        .get(key)
        .unwrap_or_else(|| panic!("field '{}' not found in payload ({})", key, when));

    // Must be a string, not a bool / number / null.
    assert!(
        value.as_str().is_some(),
        "field '{}' ({}): expected QuillValue::String, got {:?}",
        key,
        when,
        value
    );
    assert_eq!(
        value.as_str().unwrap(),
        expected,
        "field '{}' ({}): string value mismatch",
        key,
        when
    );
}

/// Every ambiguous-string field declared in the fixture, grouped by the YAML
/// 1.1 hazard it exercises, checked against one shared parse → emit →
/// re-parse cycle.
#[test]
fn ambiguous_strings_round_trip() {
    // `on`, `off`, `yes`, `no`, `true`, `false` are YAML 1.1 booleans.
    // Quillmark always emits them double-quoted so they re-parse as strings.
    let word_booleans: &[(&str, &str)] = &[
        ("on_word", "on"),
        ("off_word", "off"),
        ("yes_word", "yes"),
        ("no_word", "no"),
        ("true_word", "true"),
        ("false_word", "false"),
    ];

    // `null` and `~` parse as YAML null in many parsers.
    let null_like: &[(&str, &str)] = &[("null_word", "null"), ("tilde", "~")];

    // `01234` (octal-like), `1e10` (scientific notation), `0x1F` (hex-like).
    // A YAML 1.1 parser would silently coerce these to integers or floats.
    let numeric_like: &[(&str, &str)] = &[
        ("leading_zeros", "01234"),
        ("exponential", "1e10"),
        ("hex_like", "0x1F"),
    ];

    // ISO 8601 date strings look like YAML dates in YAML 1.1.
    let iso_date: &[(&str, &str)] = &[("iso_date", "2024-01-15")];

    // Empty string, single space, embedded newline, embedded quote, backslash.
    let special_characters: &[(&str, &str)] = &[
        ("empty_string", ""),
        ("single_space", " "),
        ("embedded_newline", "line1\nline2"),
        ("embedded_quote", "he said \"hi\""),
        ("embedded_backslash", "a\\b"),
    ];

    // Strings that look like YAML structural tokens: map entries, sequence
    // markers, comments, anchors, aliases, tags.
    let yaml_syntax: &[(&str, &str)] = &[
        ("looks_like_map", "key: value"),
        ("looks_like_seq", "- item"),
        ("hash_comment", "#comment"),
        ("yaml_anchor", "&anchor"),
        ("yaml_alias", "*alias"),
        ("yaml_tag", "!tag"),
    ];

    let all_categories: &[&[(&str, &str)]] = &[
        word_booleans,
        null_like,
        numeric_like,
        iso_date,
        special_characters,
        yaml_syntax,
    ];

    // One parse, one emit, one re-parse for the whole fixture: every
    // category above reads from these two documents instead of re-parsing.
    let doc = parse_fixture();
    let emitted = doc.to_markdown();
    let doc2 = Document::parse(&emitted)
        .unwrap_or_else(|e| panic!("re-parse after emit failed: {}\nEmitted:\n{}", e, emitted))
        .document;

    for category in all_categories {
        for (key, expected) in *category {
            // First parse: string type + value.
            assert_string_field(&doc, key, expected, "first parse");
            // Re-parsed after emit: still a byte-identical string.
            assert_string_field(&doc2, key, expected, "after round-trip");
        }
    }
}
