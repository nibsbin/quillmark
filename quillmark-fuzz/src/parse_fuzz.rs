use proptest::prelude::*;
use quillmark_core::ParsedDocument;

proptest! {
    #[test]
    fn fuzz_decompose_no_panic(s in "\\PC{0,1000}") {
        // Test that decompose doesn't panic on arbitrary input
        let _ = ParsedDocument::from_markdown(&s);
        // We don't care about the result, just that it doesn't panic
    }

    #[test]
    fn fuzz_decompose_with_dashes(s in "---[\\s\\S]*---[\\s\\S]*") {
        // Test inputs that might look like frontmatter
        let result = ParsedDocument::from_markdown(&s);
        // Should either succeed or return an error, but not panic
        match result {
            Ok(doc) => {
                // If it parsed, we should be able to access the document safely
                let _ = doc.body();
                let _ = doc.fields();
            }
            Err(_) => {
                // Error is fine - malformed YAML or other issues
            }
        }
    }

    #[test]
    fn fuzz_decompose_valid_frontmatter(
        title in "[a-zA-Z0-9 ]{1,50}",
        author in "[a-zA-Z ]{1,30}",
        content in "\\PC{0,200}"
    ) {
        // Test with valid-looking frontmatter
        let markdown = format!(
            "---\ntitle: {}\nauthor: {}\n---\n\n{}",
            title, author, content
        );

        let result = ParsedDocument::from_markdown(&markdown);
        // Should parse successfully for valid YAML
        if !title.contains(':') && !author.contains(':') {
            assert!(result.is_ok(), "Should parse valid frontmatter");
        }
    }

    #[test]
    fn fuzz_decompose_tag_directives(tag_name in "[a-z]{1,20}") {
        // Test tag directive parsing
        let markdown = format!(
            "---\nglobal: value\n---\n\n---\n!{}\nfield: data\n---\n\nContent",
            tag_name
        );

        let result = ParsedDocument::from_markdown(&markdown);
        // Should handle tag directives without panic
        if let Ok(doc) = result {
            // Tag might create a collection
            let _ = doc.get_field(&tag_name);
        }
    }

    #[test]
    fn fuzz_decompose_malformed_yaml(s in "[^a-zA-Z0-9\\s]{1,50}") {
        // Test with potentially malformed YAML
        let markdown = format!("---\n{}\n---\n\nContent", s);
        let _ = ParsedDocument::from_markdown(&markdown);
        // Should handle errors gracefully
    }

    #[test]
    fn fuzz_decompose_large_frontmatter(size in 1usize..100) {
        // Test with large frontmatter blocks
        let fields: Vec<String> = (0..size)
            .map(|i| format!("field{}: value{}", i, i))
            .collect();
        let frontmatter = fields.join("\n");
        let markdown = format!("---\n{}\n---\n\nContent", frontmatter);

        let result = ParsedDocument::from_markdown(&markdown);
        if let Ok(doc) = result {
            // Should be able to access all fields
            assert!(doc.fields().len() <= size + 1); // +1 for body field
        }
    }

    #[test]
    fn fuzz_decompose_nested_structures(depth in 1usize..5) {
        // Test with nested YAML structures
        let mut yaml = String::from("root:\n");
        for i in 0..depth {
            let indent = "  ".repeat(i + 1);
            yaml.push_str(&format!("{}level{}:\n", indent, i));
        }
        yaml.push_str(&format!("{}value: data", "  ".repeat(depth + 1)));

        let markdown = format!("---\n{}\n---\n\nContent", yaml);
        let _ = ParsedDocument::from_markdown(&markdown);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn fuzz_decompose_special_characters(s in "[\\\\\"'`$#*_\\[\\]<>@\\n\\r\\t]{0,100}") {
        // Test with special characters in content
        let markdown = format!("---\ntitle: Test\n---\n\n{}", s);
        let result = ParsedDocument::from_markdown(&markdown);

        if let Ok(doc) = result {
            // Should be able to retrieve body with special chars
            let body = doc.body();
            assert!(body.is_some());
        }
    }

    #[test]
    fn fuzz_decompose_unicode(s in "\\PC{0,100}") {
        // Test with Unicode content
        let markdown = format!("---\ntitle: Test\n---\n\n{}", s);
        let result = ParsedDocument::from_markdown(&markdown);

        if let Ok(doc) = result {
            let _ = doc.body();
        }
    }

    #[test]
    fn fuzz_decompose_multiple_sections(count in 1usize..10) {
        // Test with multiple tagged sections
        let mut markdown = String::from("---\nglobal: value\n---\n\n");

        for i in 0..count {
            markdown.push_str(&format!(
                "---\n!section{}\ndata: value{}\n---\n\nContent {}\n\n",
                i, i, i
            ));
        }

        let result = ParsedDocument::from_markdown(&markdown);
        if let Ok(doc) = result {
            // Should handle multiple sections
            let _ = doc.fields();
        }
    }
}
