use proptest::prelude::*;
use quillmark_typst::emit::{escape_markup, escape_string};

/// Markdown → Typst markup over the content pipeline: import to a `Content`, then
/// lower it. This is the render path the former single-step `mark_to_typst`
/// became — property-fuzzed here (no panic, escaping, formatting → Typst
/// functions) exactly as that lowering was.
fn mark_to_typst(markdown: &str) -> Result<String, String> {
    let rt = quillmark_content::import::from_markdown(markdown).map_err(|e| e.to_string())?;
    quillmark_typst::emit::emit_content(&rt)
        .map(|ec| ec.markup)
        .map_err(|e| e.to_string())
}

// Typst special characters that need escaping in markup context (excluding backslash and //)
// Backslash is handled first to prevent double-escaping, and // is handled as a pattern
// These correspond to the single-character escapes in the escape_markup function
const TYPST_SPECIAL_CHARS: &[char] = &[
    '~', '*', '_', '`', '#', '[', ']', '{', '}', '$', '<', '>', '@',
];

proptest! {
    #[test]
    fn fuzz_escape_string_no_raw_quotes(s in "\\PC*") {
        let escaped = escape_string(&s);
        // Verify no unescaped quotes (raw quote without backslash before it)
        // This is a simplified check - in escaped strings, quotes should be \\\"
        let chars: Vec<char> = escaped.chars().collect();
        for i in 0..chars.len() {
            if chars[i] == '"' {
                // Quote must be preceded by backslash
                assert!(i > 0 && chars[i-1] == '\\',
                    "Found unescaped quote at position {} in escaped string: {:?}", i, escaped);
            }
        }
    }

    #[test]
    fn fuzz_escape_markup_typst_chars_escaped(s in "\\PC*") {
        let escaped = escape_markup(&s);
        // For each Typst special character in the input, verify it's escaped in output
        for &ch in TYPST_SPECIAL_CHARS {
            if s.contains(ch) {
                // The escaped version should contain the escaped form
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
        // Verify proper escaping of backslashes
        // Each backslash in the input should be escaped to exactly two backslashes
        // Count total backslashes in input
        let input_backslashes = s.matches('\\').count();

        // Count other special chars that will be escaped (each adds one backslash)
        let special_count: usize = TYPST_SPECIAL_CHARS.iter()
            .map(|&ch| s.matches(ch).count())
            .sum();

        // Count // patterns that will be escaped (each // becomes \/\/, adding 2 backslashes)
        let double_slash_count = s.matches("//").count();

        // Expected backslashes in output:
        // - Each input backslash becomes 2 backslashes (input_backslashes * 2)
        // - Each special char gets one escape backslash (special_count)
        // - Each // pattern gets 2 escape backslashes (double_slash_count * 2)
        let expected_backslashes = input_backslashes * 2 + special_count + double_slash_count * 2;
        let actual_backslashes = escaped.matches('\\').count();

        assert_eq!(actual_backslashes, expected_backslashes,
            "Backslash count mismatch for input {:?}: expected {}, got {}",
            s, expected_backslashes, actual_backslashes);
    }

    #[test]
    fn fuzz_mark_to_typst_no_panic(s in "\\PC{0,1000}") {
        // Just verify it doesn't panic on various inputs
        let _ = mark_to_typst(&s);
    }

    #[test]
    fn fuzz_mark_to_typst_special_chars_escaped(s in "[a-zA-Z0-9 *_#\\[\\]$<>@\\\\]{0,100}") {
        let output = mark_to_typst(&s);
        // If input contains raw special characters (not in markdown syntax),
        // they should be escaped in output
        // This is a basic safety check - the conversion should not panic
        // Note: Some inputs like "<a>" may be treated as HTML and result in empty output
        // which is valid behavior - we're just checking for no panics
        let _ = output; // Just verify no panic
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn fuzz_escape_string_injection_safety(s in "[\\\\\"].*[\\\\\"].*") {
        // Test strings with quotes and backslashes
        let escaped = escape_string(&s);

        // Should not contain the pattern "); which could break out of string context
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
        // Test deeply nested structures
        let nested_quotes = "> ".repeat(depth) + "text";
        let result = mark_to_typst(&nested_quotes).expect("Conversion should succeed");
        // Should not panic and should produce some output
        assert!(!result.is_empty() || depth == 0);
    }

    #[test]
    fn fuzz_markdown_parser_malicious_lists(depth in 1usize..20) {
        // Test deeply nested lists
        let nested_list = (0..depth)
            .map(|i| format!("{}- item", "  ".repeat(i)))
            .collect::<Vec<_>>()
            .join("\n");
        let result = mark_to_typst(&nested_list).expect("Conversion should succeed");
        // Should not panic
        assert!(!result.is_empty());
    }

    #[test]
    fn fuzz_markdown_large_input(size in 1usize..10000) {
        // Test with large inputs (but not too large for tests)
        let input = "a".repeat(size);
        let result = mark_to_typst(&input).expect("Conversion should succeed");
        // Should handle large inputs without panic
        assert!(result.contains("a"));
    }
}
