use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

fn main() {
    // Create a temporary directory for the quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("auto-example");

    // Create a minimal quill without a plate file (will use auto plate output)
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "auto-example"
backend = "typst"
description = "Example quill that outputs JSON using auto plate"
"#,
    )
    .expect("Failed to write Quill.toml");

    // Create a markdown document with frontmatter
    let markdown = r#"---
title: Auto Plate Example
author: Quillmark Team
version: 1.0
tags:
  - auto
  - plate
  - example
metadata:
  status: draft
  priority: high
---

# Introduction

This example demonstrates the auto plate functionality.

When a quill doesn't specify a plate file, the context is automatically
output as JSON instead of being processed through a template.
"#;

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Create the engine and load the quill
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    println!("Quill name: {}", quill.name);
    // Verify plate is empty (will trigger auto plate)
    println!("Plate field: {:?}", quill.plate.clone());
    println!("Backend supports auto plate: Yes (Typst backend)\n");

    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Create workflow and process the plate
    let workflow = engine
        .workflow("auto-example")
        .expect("Failed to load workflow");

    // Process plate output (will be JSON)
    let json_output = workflow
        .process_plate(&parsed)
        .expect("Failed to process plate");

    println!("Generated Auto Plate (JSON):");
    println!("{}", json_output);
}
