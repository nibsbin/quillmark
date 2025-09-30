use std::fs;
use tempfile::TempDir;

use quillmark::{OutputFormat, Quill, Workflow};
use quillmark_typst::TypstBackend;

#[test]
fn test_end_to_end_rendering() {
    // Create temporary directory structure for test quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    // Create quill directory structure
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::create_dir_all(quill_path.join("assets")).expect("Failed to create assets dir");
    fs::create_dir_all(quill_path.join("packages")).expect("Failed to create packages dir");

    // Create Quill.toml
    let quill_toml = r#"[Quill]
name = "test-quill"
backend = "typst"
glue = "glue.typ"
description = "Test quill for integration testing"
author = "QuillMark Test"
"#;
    fs::write(quill_path.join("Quill.toml"), quill_toml).expect("Failed to write Quill.toml");

    // Create glue.typ template
    let glue_template = r#"
= {{ title | String(default="Untitled") }}

_By {{ author | String(default="Unknown") }}_

{{ body | Body }}
"#;
    fs::write(quill_path.join("glue.typ"), glue_template).expect("Failed to write glue.typ");

    // Test markdown content with frontmatter
    let markdown = r#"---
title: Test Document
author: Test Author
date: 2024-01-01
---

# Introduction

This is a **test document** with *italic* text and some features:

- First item
- Second item with `code`
- Third item

## Conclusion

This concludes the test document.
"#;

    // Create engine and render
    let backend = Box::new(TypstBackend::default());
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    let engine = Workflow::new(backend, quill).expect("Failed to create engine");

    // Test PDF rendering
    let pdf_result = engine
        .render(markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render PDF");

    assert!(!pdf_result.artifacts.is_empty());
    assert_eq!(pdf_result.artifacts[0].output_format, OutputFormat::Pdf);
    assert!(!pdf_result.artifacts[0].bytes.is_empty());

    // Test SVG rendering
    let svg_result = engine
        .render(markdown, Some(OutputFormat::Svg))
        .expect("Failed to render SVG");

    assert!(!svg_result.artifacts.is_empty());
    assert_eq!(svg_result.artifacts[0].output_format, OutputFormat::Svg);
    assert!(!svg_result.artifacts[0].bytes.is_empty());

    println!(
        "Integration test passed! Generated {} PDF bytes and {} SVG bytes",
        pdf_result.artifacts[0].bytes.len(),
        svg_result.artifacts[0].bytes.len()
    );
}

#[test]
fn test_engine_properties() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "Test template").expect("Failed to write glue.typ");

    let backend = Box::new(TypstBackend::default());
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    let engine = Workflow::new(backend, quill).expect("Failed to create engine");

    assert_eq!(engine.backend_id(), "typst");
    assert_eq!(engine.quill_name(), "test-quill");
    assert!(engine.supported_formats().contains(&OutputFormat::Pdf));
    assert!(engine.supported_formats().contains(&OutputFormat::Svg));
}

#[test]
fn test_unsupported_format() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "Test template").expect("Failed to write glue.typ");

    let backend = Box::new(TypstBackend::default());
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    let engine = Workflow::new(backend, quill).expect("Failed to create engine");

    let result = engine.render("# Test", Some(OutputFormat::Txt));

    match result {
        Err(quillmark::RenderError::FormatNotSupported { backend, format }) => {
            assert_eq!(backend, "typst");
            assert_eq!(format, OutputFormat::Txt);
        }
        _ => panic!("Expected FormatNotSupported error"),
    }
}

#[test]
fn test_typst_packages_parsing_from_bubble() {
    use quillmark_fixtures::resource_path;
    
    let bubble_path = resource_path("bubble");
    let quill = Quill::from_path(&bubble_path).expect("Failed to load bubble quill");
    
    // Check that typst packages are parsed from Quill.toml
    let packages = quill.typst_packages();
    assert!(!packages.is_empty(), "Expected packages to be specified in bubble quill");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0], "@preview/bubble:0.2.2");
    
    println!("Successfully parsed typst packages from bubble quill: {:?}", packages);
}
