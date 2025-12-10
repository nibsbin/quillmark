//! # Common Test Utilities
//!
//! Shared test helpers and utilities for integration tests.
//!
//! ## Purpose
//!
//! This module provides common functionality used across multiple test files:
//! - **`demo()` function** - Centralized example plumbing for rendering demos
//!
//! ## Usage
//!
//! The `demo()` helper simplifies the common pattern of:
//! 1. Loading a quill from a path
//! 2. Using the quill's example markdown
//! 3. Processing through the plate template (returning the print)
//! 4. Rendering to final output
//! 5. Writing outputs to example directory
//!
//! This reduces code duplication in tests and examples.
//!
//! ## Design Pattern
//!
//! Test utilities should be minimal and focused. Complex test setup indicates
//! that the public API may need simplification rather than more test helpers.

use quillmark_fixtures::{example_output_dir, quills_path, write_example_output};
use std::error::Error;

/// Demo helper that centralizes example plumbing.
///
/// It loads the quill and uses its markdown template, then processes and renders it.
pub fn demo(
    quill_dir: &str,
    print_output: &str,
    render_output: &str,
    use_resource_path: bool,
) -> Result<(), Box<dyn Error>> {
    // quill path (folder)
    let quill_path = if use_resource_path {
        quillmark_fixtures::resource_path(quill_dir)
    } else {
        quills_path(quill_dir)
    };
    // Default engine flow used by examples: Typst backend, Quill from path, Workflow
    let quill = quillmark::Quill::from_path(quill_path.clone()).expect("Failed to load quill");

    // Load the markdown template from the quill
    let markdown = quill
        .example
        .as_ref()
        .ok_or("Quill does not have a markdown template")?
        .clone();

    // Parse the markdown once
    let parsed = quillmark::ParsedDocument::from_markdown(&markdown)?;

    let engine = quillmark::Quillmark::new();
    let workflow = engine.workflow(&quill).expect("Failed to load workflow");

    // render plate (returns the print)
    let print = workflow.render_plate(&parsed)?;

    // write outputs
    let print_bytes = print.into_bytes();
    write_example_output(print_output, &print_bytes)?;

    println!(
        "Print outputted to: {}",
        example_output_dir().join(print_output).display()
    );

    // render output
    let rendered = workflow.render(&parsed, Some(quillmark_core::OutputFormat::Pdf))?;
    let output_bytes = rendered.artifacts[0].bytes.clone();

    write_example_output(render_output, &output_bytes)?;

    println!("------------------------------");
    println!(
        "Access print output: {}",
        example_output_dir().join(print_output).display()
    );
    println!(
        "Access render output: {}",
        example_output_dir().join(render_output).display()
    );

    Ok(())
}
