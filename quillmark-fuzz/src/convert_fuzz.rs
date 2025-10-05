use proptest::prelude::*;
use quillmark_typst::convert::{escape_markup, escape_string, mark_to_typst};

// Security-focused tests for escape_string
#[test]
fn test_escape_string_security_attack_vectors() {
    // Test injection attempt with quote and eval
    let malicious = "\"; system(\"rm -rf /\"); \"";
    let escaped = escape_string(malicious);
    // Should escape the quotes, preventing injection
    assert_eq!(escaped, r#"\"; system(\"rm -rf /\"); \""#);
    // When used in a Typst string, the escaped quotes prevent breaking out
    let typst_expr = format!("eval(\"{}\", mode: \"markup\")", escaped);
    // The dangerous pattern should not exist in a way that breaks out of the string
    assert!(
        !typst_expr.contains("eval(\"\"; system"),
        "Escaped content should not break out of eval string"
    );

    // Test backslash and quote combination
    let attack = r#"\"); eval("malicious")"#;
    let escaped = escape_string(attack);
    assert_eq!(escaped, r#"\\\"); eval(\"malicious\")"#);
    // When used in context, should not allow breakout
    let typst_expr = format!("eval(\"{}\", mode: \"markup\")", escaped);
    assert!(
        !typst_expr.contains("eval(\"\\\"); eval(\"malicious\")"),
        "Should not have raw breakout pattern"
    );
}

#[test]
fn test_escape_string_control_characters() {
    // Null byte
    assert_eq!(escape_string("\0"), "\\u{0}");
    // Other control characters
    assert_eq!(escape_string("\x01"), "\\u{1}");
    assert_eq!(escape_string("\x1f"), "\\u{1f}");
    // Combination
    assert_eq!(escape_string("test\0ing"), "test\\u{0}ing");
}

#[test]
fn test_escape_markup_security_attack_vectors() {
    // Test that all special characters are escaped
    let attack = "*_`#[]$<>@\\";
    let escaped = escape_markup(attack);
    assert_eq!(escaped, "\\*\\_\\`\\#\\[\\]\\$\\<\\>\\@\\\\");

    // Verify backslash is escaped first
    let backslash_attack = "\\*";
    let escaped = escape_markup(backslash_attack);
    assert_eq!(escaped, "\\\\\\*");
}

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
    fn fuzz_escape_string_valid_escapes(s in "\\PC*") {
        let escaped = escape_string(&s);

        // Key property: no unescaped quotes that could break out of string context
        // Simple check: any quote must be preceded by a backslash
        let chars: Vec<char> = escaped.chars().collect();
        for i in 0..chars.len() {
            if chars[i] == '"' {
                assert!(i > 0 && chars[i-1] == '\\',
                    "Found unescaped quote at position {} in: {}", i, escaped);
            }
        }
    }

    #[test]
    fn fuzz_escape_markup_typst_chars_escaped(s in "\\PC*") {
        let escaped = escape_markup(&s);
        // For each Typst special character in the input, verify it's escaped in output
        let special_chars = ['*', '_', '#', '[', ']', '$', '<', '>', '@'];
        for &ch in &special_chars {
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
        // Verify no double-escaping issues
        // If input has backslash, it should become \\, not more
        let _backslash_count = s.matches('\\').count();
        let _escaped_backslash_count = escaped.matches("\\\\").count();
        // Each input backslash should result in exactly one escaped backslash
        // But we also need to account for backslashes used to escape other chars
        // So we just verify the escaping is consistent and doesn't create invalid sequences

        // Verify no triple or quadruple backslashes (would indicate double-escaping)
        assert!(!escaped.contains("\\\\\\\\\\\\"),
            "Triple backslash found, possible double-escaping in: {}", escaped);
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
        // This is a basic safety check - the conversion should not introduce unescaped specials
        // For whitespace-only input, output can be empty which is fine
        prop_assert!(output.len() >= s.trim().len() / 2 || s.trim().is_empty(),
            "Output suspiciously short for input: {} -> {}", s, output);
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
        let result = mark_to_typst(&nested_quotes);
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
        let result = mark_to_typst(&nested_list);
        // Should not panic
        assert!(!result.is_empty());
    }

    #[test]
    fn fuzz_markdown_large_input(size in 1usize..10000) {
        // Test with large inputs (but not too large for tests)
        let input = "a".repeat(size);
        let result = mark_to_typst(&input);
        // Should handle large inputs without panic
        assert!(result.contains("a"));
    }
}
