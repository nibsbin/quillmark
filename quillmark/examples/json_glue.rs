use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

fn main() {
    // Create a temporary directory for the quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("json-example");

    // Create a minimal quill without a glue file (will use JSON output)
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "json-example"
backend = "typst"
description = "Example quill that outputs JSON"
"#,
    )
    .expect("Failed to write Quill.toml");

    // Create a markdown document with frontmatter
    let markdown = r#"---
title: JSON Glue Example
author: Quillmark Team
version: 1.0
tags:
  - json
  - glue
  - example
metadata:
  status: draft
  priority: high
---

# Introduction

This example demonstrates the JSON glue functionality.

When a quill doesn't specify a glue file, the context is automatically
output as JSON instead of being processed through a template.
"#;

    // Parse the markdown
    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Create the engine and load the quill
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    println!("Quill name: {}", quill.name);
    println!("Glue file: {:?}", quill.glue_file);
    println!("Glue template empty: {}", quill.glue_template.is_empty());
    println!();

    engine.register_quill(quill);

    // Create workflow and process the glue
    let workflow = engine
        .workflow_from_quill_name("json-example")
        .expect("Failed to load workflow");

    let json_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    println!("JSON Output:");
    println!("{}", json_output);
}
