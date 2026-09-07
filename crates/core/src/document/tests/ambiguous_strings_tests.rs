//! Strings that a YAML 1.1 parser would silently coerce to booleans, integers,
//! or dates, or misread as anchors/aliases/tags. Each round-trips as a string.
use crate::document::Document;

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

fn parse_fixture() -> Document {
    let src = ambiguous_strings_fixture();
    Document::parse(&src)
        .unwrap_or_else(|e| panic!("ambiguous_strings.md failed to parse: {}", e))
        .document
}

fn assert_string_field(doc: &Document, key: &str, expected: &str, when: &str) {
    let value = doc
        .main()
        .payload()
        .get(key)
        .unwrap_or_else(|| panic!("field '{}' not found in payload ({})", key, when));

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

#[test]
fn ambiguous_strings_round_trip() {
    // `on`, `off`, `yes`, `no`, `true`, `false` are YAML 1.1 booleans.
    let word_booleans: &[(&str, &str)] = &[
        ("on_word", "on"),
        ("off_word", "off"),
        ("yes_word", "yes"),
        ("no_word", "no"),
        ("true_word", "true"),
        ("false_word", "false"),
    ];

    let null_like: &[(&str, &str)] = &[("null_word", "null"), ("tilde", "~")];

    // `01234` (octal-like), `1e10` (scientific notation), `0x1F` (hex-like).
    // A YAML 1.1 parser would silently coerce these to integers or floats.
    let numeric_like: &[(&str, &str)] = &[
        ("leading_zeros", "01234"),
        ("exponential", "1e10"),
        ("hex_like", "0x1F"),
    ];

    let iso_date: &[(&str, &str)] = &[("iso_date", "2024-01-15")];

    let special_characters: &[(&str, &str)] = &[
        ("empty_string", ""),
        ("single_space", " "),
        ("embedded_newline", "line1\nline2"),
        ("embedded_quote", "he said \"hi\""),
        ("embedded_backslash", "a\\b"),
    ];

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

    let doc = parse_fixture();
    let emitted = doc.to_markdown();
    let doc2 = Document::parse(&emitted)
        .unwrap_or_else(|e| panic!("re-parse after emit failed: {}\nEmitted:\n{}", e, emitted))
        .document;

    for category in all_categories {
        for (key, expected) in *category {
            assert_string_field(&doc, key, expected, "first parse");
            assert_string_field(&doc2, key, expected, "after round-trip");
        }
    }
}
