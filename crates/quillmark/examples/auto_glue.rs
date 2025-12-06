use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

fn main() {
    // Create a temporary directory for the plate
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("auto-example");

    // Create a minimal plate without a glue file (will use auto glue output)
    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "auto-example"
backend = "typst"
description = "Example plate that outputs JSON using auto glue"
"#,
    )
    .expect("Failed to write Quill.toml");

    // Create a markdown document with frontmatter
    let markdown = r#"---
title: Auto Glue Example
author: Quillmark Team
version: 1.0
tags:
  - auto
  - glue
  - example
metadata:
  status: draft
  priority: high
---

# Introduction

This example demonstrates the auto glue functionality.

When a plate doesn't specify a glue file, the context is automatically
output as JSON instead of being processed through a template.
"#;

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Create the engine and load the plate
    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");

    println!("Quill name: {}", plate.name);
    println!(
        "Glue file: {:?}",
        plate.metadata.get("glue_file").and_then(|v| v.as_str())
    );
    println!(
        "Glue template empty: {}",
        plate.glue.clone().unwrap_or_default().is_empty()
    );
    println!();

    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    // Create workflow and process the glue
    let workflow = engine
        .workflow("auto-example")
        .expect("Failed to load workflow");

    let json_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    println!("JSON Output:");
    println!("{}", json_output);
}
