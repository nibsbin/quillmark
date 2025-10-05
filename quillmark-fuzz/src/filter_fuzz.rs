use proptest::prelude::*;
use quillmark_typst::fuzz_utils::inject_json;

proptest! {
    #[test]
    fn fuzz_inject_json_no_injection(s in "\\PC*") {
        // Test the inject_json helper with various inputs
        let result = inject_json(&s);

        // Should always start with json(bytes("
        assert!(result.starts_with("json(bytes(\""));
        assert!(result.ends_with("\"))"));

        // Extract just the inner content (between quotes)
        if result.len() > "json(bytes(\"\"))".len() {
            let inner_start = "json(bytes(\"".len();
            let inner_end = result.len() - "\"))".len();
            let inner = &result[inner_start..inner_end];

            // Check for unescaped quotes in the inner content
            // (the closing quote is part of the wrapper, not the content)
            let chars: Vec<char> = inner.chars().collect();
            for i in 0..chars.len() {
                if chars[i] == '"' {
                    // Quote must be preceded by backslash
                    assert!(i > 0 && chars[i-1] == '\\',
                        "Unescaped quote in inject_json inner content at position {}: {}", i, inner);
                }
            }
        }
    }

    #[test]
    fn fuzz_inject_json_escaping_consistency(s in "\\PC{0,100}") {
        // Test that inject_json uses proper escaping
        let result = inject_json(&s);

        // Key property: should not contain unescaped quotes that could break out
        // Extract the inner content
        if let Some(start_pos) = result.find("json(bytes(\"") {
            let content_start = start_pos + "json(bytes(\"".len();
            if let Some(end_offset) = result[content_start..].rfind("\"))") {
                let content_end = content_start + end_offset;
                let escaped_content = &result[content_start..content_end];

                // Check for unescaped quotes
                let chars: Vec<char> = escaped_content.chars().collect();
                for i in 0..chars.len() {
                    if chars[i] == '"' {
                        assert!(i > 0 && chars[i-1] == '\\',
                            "Unescaped quote at position {} in: {}", i, escaped_content);
                    }
                }
            }
        }
    }

    #[test]
    fn fuzz_inject_json_dangerous_patterns(s in "[\\\\\"'`$#]{0,50}") {
        // Test with characters that might cause injection
        let result = inject_json(&s);

        // Should not contain patterns that could break out of string context
        let dangerous_patterns = ["\"); ", "\")); "];
        for pattern in &dangerous_patterns {
            assert!(!result.contains(pattern),
                "Dangerous pattern '{}' found in: {}", pattern, result);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn fuzz_inject_json_size_limits(size in 0usize..1000) {
        // Test with various input sizes
        let input = "a".repeat(size);
        let result = inject_json(&input);

        // Output should be proportional to input
        assert!(result.len() >= input.len());
        // For control characters or special chars, output can be much longer (up to 10x)
        // For empty string, wrapper is "json(bytes(\"\"))" which is 16 chars
        if size == 0 {
            assert_eq!(result, "json(bytes(\"\"))");
        } else {
            // Normal chars don't expand much, but allow generous headroom
            assert!(result.len() < input.len() * 20 || result.len() < 1000);
        }
    }

    #[test]
    fn fuzz_inject_json_unicode(s in "\\PC{0,100}") {
        // Test with unicode characters
        let result = inject_json(&s);

        // Should handle unicode without panic
        assert!(result.starts_with("json(bytes(\""));
    }
}
