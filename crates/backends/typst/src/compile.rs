//! # Typst Compilation
//!
//! This module compiles Typst documents to output formats (PDF and SVG).
//!
//! ## Functions
//!
//! - [`compile_to_pdf()`] - Compile Typst to PDF format
//! - [`compile_to_svg()`] - Compile Typst to SVG format (one file per page)
//!
//! ## Quick Example
//!
//! ```no_run
//! use quillmark_typst::compile::compile_to_pdf;
//! use quillmark_core::Quill;
//!
//! let quill = Quill::from_path("path/to/quill")?;
//! let typst_content = "#set document(title: \"Test\")\n= Hello";
//!
//! let pdf_bytes = compile_to_pdf(&quill, typst_content)?;
//! std::fs::write("output.pdf", pdf_bytes)?;
//! # Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
//! ```
//!
//! ## Process
//!
//! 1. Creates a `QuillWorld` with the quill's assets and packages
//! 2. Compiles the Typst document using the Typst compiler
//! 3. Converts to target format (PDF or SVG)
//! 4. Returns output bytes
//!
//! The output bytes can be written to a file or returned directly to the caller.

use typst::diag::Warned;
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::error_mapping::map_typst_errors;
use crate::world::QuillWorld;
use quillmark_core::{Diagnostic, Quill, RenderError, Severity};

/// Compiles a Typst document to PDF format.
pub fn compile_to_pdf(quill: &Quill, plated_content: &str) -> Result<Vec<u8>, RenderError> {
    let world =
        QuillWorld::new(quill, plated_content).map_err(|e| RenderError::EngineCreation {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!("Failed to create Typst compilation environment: {}", e),
                )
                .with_code("typst::world_creation".to_string())
                .with_source(e),
            ),
        })?;

    let document = compile_document(&world)?;

    let pdf = typst_pdf::pdf(&document, &PdfOptions::default()).map_err(|e| {
        RenderError::CompilationFailed {
            diags: vec![Diagnostic::new(
                Severity::Error,
                format!("PDF generation failed: {:?}", e),
            )
            .with_code("typst::pdf_generation".to_string())],
        }
    })?;

    Ok(pdf)
}

/// Compiles a Typst document to SVG format (one file per page).
pub fn compile_to_svg(quill: &Quill, plated_content: &str) -> Result<Vec<Vec<u8>>, RenderError> {
    let world =
        QuillWorld::new(quill, plated_content).map_err(|e| RenderError::EngineCreation {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!("Failed to create Typst compilation environment: {}", e),
                )
                .with_code("typst::world_creation".to_string())
                .with_source(e),
            ),
        })?;

    let document = compile_document(&world)?;

    let mut pages = Vec::new();
    for page in &document.pages {
        let svg = typst_svg::svg(page);
        pages.push(svg.into_bytes());
    }

    Ok(pages)
}

/// Internal compilation function
fn compile_document(world: &QuillWorld) -> Result<PagedDocument, RenderError> {
    let Warned { output, warnings } = typst::compile::<PagedDocument>(world);

    for warning in warnings {
        eprintln!("Warning: {}", warning.message);
    }

    match output {
        Ok(doc) => Ok(doc),
        Err(errors) => {
            let diagnostics = map_typst_errors(&errors, world);
            Err(RenderError::CompilationFailed { diags: diagnostics })
        }
    }
}
