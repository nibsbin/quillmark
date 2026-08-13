use proptest::prelude::*;
use quillmark_typst::emit::{escape_markup, escape_string};

/// The render path: import markdown to a `Content`, then lower it to markup.
fn mark_to_typst(markdown: &str) -> Result<String, String> {
    let rt = quillmark_content::import::from_markdown(markdown).map_err(|e| e.to_string())?;
    quillmark_typst::emit::emit_content(&rt)
        .map(|ec| ec.markup)
        .map_err(|e| e.to_string())
}

// The single-character markup escapes. Backslash and `//` are excluded: the
// former is escaped first to prevent double-escaping, the latter as a pattern.
const TYPST_SPECIAL_CHARS: &[char] = &[
    '~', '*', '_', '`', '#', '[', ']', '{', '}', '$', '<', '>', '@',
];

proptest! {
    #[test]
    fn fuzz_escape_string_no_raw_quotes(s in "\\PC*") {
        let escaped = escape_string(&s);
        let chars: Vec<char> = escaped.chars().collect();
        for i in 0..chars.len() {
            if chars[i] == '"' {
                assert!(i > 0 && chars[i-1] == '\\',
                    "Found unescaped quote at position {} in escaped string: {:?}", i, escaped);
            }
        }
    }

    #[test]
    fn fuzz_escape_markup_typst_chars_escaped(s in "\\PC*") {
        let escaped = escape_markup(&s);
        for &ch in TYPST_SPECIAL_CHARS {
            if s.contains(ch) {
                let escaped_form = format!("\\{}", ch);
                assert!(escaped.contains(&escaped_form),
                    "Character '{}' in input '{}' not properly escaped in output '{}'",
                    ch, s, escaped);
            }
        }
    }

    #[test]
    fn fuzz_escape_markup_backslash_first(s in "\\PC*") {
        let escaped = escape_markup(&s);
        let input_backslashes = s.matches('\\').count();

        let special_count: usize = TYPST_SPECIAL_CHARS.iter()
            .map(|&ch| s.matches(ch).count())
            .sum();

        // Each `//` becomes `\/\/`, so two backslashes apiece.
        let double_slash_count = s.matches("//").count();

        let expected_backslashes = input_backslashes * 2 + special_count + double_slash_count * 2;
        let actual_backslashes = escaped.matches('\\').count();

        assert_eq!(actual_backslashes, expected_backslashes,
            "Backslash count mismatch for input {:?}: expected {}, got {}",
            s, expected_backslashes, actual_backslashes);
    }

    #[test]
    fn fuzz_mark_to_typst_no_panic(s in "\\PC{0,1000}") {
        let _ = mark_to_typst(&s);
    }

}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn fuzz_escape_string_injection_safety(s in "[\\\\\"].*[\\\\\"].*") {
        let escaped = escape_string(&s);

        // Patterns that would break out of the string context.
        let dangerous_patterns = [
            "\"); ",
            "\")); ",
            "\\\"); ",
        ];

        for pattern in &dangerous_patterns {
            assert!(!escaped.contains(pattern),
                "Dangerous pattern '{}' found in escaped output: {}", pattern, escaped);
        }
    }

    #[test]
    fn fuzz_markdown_parser_malicious_nesting(depth in 1usize..20) {
        let nested_quotes = "> ".repeat(depth) + "text";
        let result = mark_to_typst(&nested_quotes).expect("Conversion should succeed");
        assert!(!result.is_empty() || depth == 0);
    }

    #[test]
    fn fuzz_markdown_parser_malicious_lists(depth in 1usize..20) {
        let nested_list = (0..depth)
            .map(|i| format!("{}- item", "  ".repeat(i)))
            .collect::<Vec<_>>()
            .join("\n");
        let result = mark_to_typst(&nested_list).expect("Conversion should succeed");
        assert!(!result.is_empty());
    }

    #[test]
    fn fuzz_markdown_large_input(size in 1usize..10000) {
        let input = "a".repeat(size);
        let result = mark_to_typst(&input).expect("Conversion should succeed");
        assert!(result.contains("a"));
    }
}
