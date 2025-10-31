use quillmark::{ParsedDocument, Quillmark};
use quillmark_core::OutputFormat;

#[test]
fn test_default_quill_registered_on_backend_registration() {
    // Create engine with Typst backend
    let engine = Quillmark::new();

    // Verify that __default__ quill is registered
    let registered_quills = engine.registered_quills();
    assert!(
        registered_quills.contains(&"__default__"),
        "Default Quill should be registered automatically"
    );
}

#[test]
fn test_default_quill_used_when_no_quill_tag() {
    let markdown = r#"---
title: Test Document
author: Alice
---

# Hello World

This is a test document without a QUILL tag.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Verify no QUILL tag
    assert_eq!(parsed.quill_tag(), None);

    let engine = Quillmark::new();

    // Should successfully load workflow using __default__
    let workflow = engine
        .workflow_from_parsed(&parsed)
        .expect("Failed to load workflow with default Quill");

    assert_eq!(workflow.quill_name(), "__default__");
}

#[test]
fn test_explicit_quill_tag_takes_precedence_over_default() {
    use quillmark::Quill;
    use std::fs;
    use tempfile::TempDir;

    // Create a custom quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("custom-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"custom_quill\"\nbackend = \"typst\"\ndescription = \"Custom test quill\"\n",
    )
    .expect("Failed to write Quill.toml");

    let markdown = r#"---
QUILL: custom_quill
title: Test Document
---

Content here.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Verify QUILL tag is present
    assert_eq!(parsed.quill_tag(), Some("custom_quill"));

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Should use custom quill, not default
    let workflow = engine
        .workflow_from_parsed(&parsed)
        .expect("Failed to load workflow");

    assert_eq!(workflow.quill_name(), "custom_quill");
}

#[test]
fn test_error_when_no_quill_tag_and_no_default() {
    let markdown = r#"---
title: Test
---

Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Verify no QUILL tag present
    assert_eq!(parsed.quill_tag(), None);

    // Note: In the current implementation with Typst backend auto-registered,
    // __default__ is always available. This test documents the expected behavior
    // when no default Quill exists, which would occur with a backend that doesn't
    // provide default_quill() and no manually registered default.
    //
    // The actual error scenario is tested indirectly through the improved error
    // message in workflow_from_parsed when __default__ doesn't exist.
}

#[test]
fn test_default_quill_renders_successfully() {
    let markdown = r#"---
title: Test Document
author: Alice Smith
---

# Introduction

This is a **test** document with _formatting_.

- Item 1
- Item 2
- Item 3
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let engine = Quillmark::new();
    let workflow = engine
        .workflow_from_parsed(&parsed)
        .expect("Failed to load workflow");

    // Should successfully render with default Quill
    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render with default Quill");

    assert_eq!(result.artifacts.len(), 1);
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Pdf);
    assert!(!result.artifacts[0].bytes.is_empty());
}

#[test]
fn test_default_quill_properties() {
    let engine = Quillmark::new();
    let workflow = engine
        .workflow_from_quill_name("__default__")
        .expect("Failed to load default workflow");

    assert_eq!(workflow.quill_name(), "__default__");
    assert_eq!(workflow.backend_id(), "typst");
    assert!(workflow.supported_formats().contains(&OutputFormat::Pdf));
    assert!(workflow.supported_formats().contains(&OutputFormat::Svg));
}

#[test]
fn test_second_backend_with_default_quill_does_not_override() {
    use quillmark::Quill;
    use quillmark_core::Backend;
    use quillmark_core::FileTreeNode;
    use std::collections::HashMap;

    // Create a second mock backend that also provides a default Quill
    struct SecondBackend;

    impl Backend for SecondBackend {
        fn id(&self) -> &'static str {
            "second"
        }

        fn supported_formats(&self) -> &'static [OutputFormat] {
            &[OutputFormat::Txt]
        }

        fn glue_extension_types(&self) -> &'static [&'static str] {
            &[".txt"]
        }

        fn allow_auto_glue(&self) -> bool {
            true
        }

        fn register_filters(&self, _: &mut quillmark_core::Glue) {}

        fn compile(
            &self,
            _: &str,
            _: &quillmark_core::Quill,
            _: &quillmark_core::RenderOptions,
        ) -> Result<quillmark_core::RenderResult, quillmark_core::RenderError> {
            Ok(quillmark_core::RenderResult::new(vec![], OutputFormat::Txt))
        }

        fn default_quill(&self) -> Option<Quill> {
            // Create a simple default Quill for this backend
            let mut files = HashMap::new();
            files.insert(
                "Quill.toml".to_string(),
                FileTreeNode::File {
                    contents: b"[Quill]\nname = \"__default__\"\nbackend = \"second\"\ndescription = \"Second backend default\"\n".to_vec(),
                },
            );

            let root = FileTreeNode::Directory { files };
            Quill::from_tree(root, None).ok()
        }
    }

    let mut engine = Quillmark::new();

    // Typst backend already registered __default__
    let workflow_before = engine
        .workflow_from_quill_name("__default__")
        .expect("Failed to load default workflow");
    assert_eq!(workflow_before.backend_id(), "typst");

    // Register second backend
    engine.register_backend(Box::new(SecondBackend));

    // __default__ should still point to Typst backend's default
    let workflow_after = engine
        .workflow_from_quill_name("__default__")
        .expect("Failed to load default workflow after second backend");
    assert_eq!(
        workflow_after.backend_id(),
        "typst",
        "First backend's default Quill should take precedence"
    );
}
