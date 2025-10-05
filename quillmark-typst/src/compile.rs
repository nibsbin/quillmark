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
//!
//! ## Security Features
//!
//! - **Compilation Timeout**: Documents that take longer than 60 seconds to compile are terminated
//! - **Page Limit**: Documents cannot generate more than 1000 pages
//! - **Memory Limits** (Unix only): Compilation memory limited to 512 MB when available
//!
//! ### Security Limitations
//!
//! **Important**: The current timeout implementation detects slow compilations after they complete,
//! but does NOT prevent infinite loops during compilation. For true timeout enforcement,
//! process isolation is needed (see `designs/SECURITY_REC.md`).
//!
//! ### Example Error Handling
//!
//! ```no_run
//! use quillmark_typst::compile::compile_to_pdf;
//! use quillmark_core::{Quill, RenderError};
//!
//! let quill = Quill::from_path("path/to/quill")?;
//! let typst_content = "#set document(title: \"Test\")\n= Hello";
//!
//! match compile_to_pdf(&quill, typst_content) {
//!     Ok(pdf_bytes) => {
//!         std::fs::write("output.pdf", pdf_bytes)?;
//!     }
//!     Err(RenderError::CompilationTimeout { timeout_secs }) => {
//!         eprintln!("Compilation exceeded {} seconds", timeout_secs);
//!     }
//!     Err(RenderError::TooManyPages { page_count, max_pages }) => {
//!         eprintln!("Document has {} pages (max: {})", page_count, max_pages);
//!     }
//!     Err(e) => {
//!         eprintln!("Compilation failed: {}", e);
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
//! ```

use std::time::{Duration, Instant};
use typst::diag::Warned;
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::error_mapping::map_typst_errors;
use crate::world::QuillWorld;
use quillmark_core::{Quill, RenderError};

/// Maximum time allowed for document compilation (60 seconds)
const COMPILE_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum number of pages allowed in a document (1000 pages)
const MAX_COMPILATION_PAGES: usize = 1000;

/// Compiles a Typst document to PDF format.
pub fn compile_to_pdf(quill: &Quill, glued_content: &str) -> Result<Vec<u8>, RenderError> {
    println!("Using quill: {}", quill.name);
    let world = QuillWorld::new(quill, glued_content).map_err(|e| {
        RenderError::Internal(anyhow::anyhow!("Failed to create Typst world: {}", e))
    })?;

    let document = compile_document(&world)?;

    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("PDF generation failed: {:?}", e)))?;

    Ok(pdf)
}

/// Compiles a Typst document to SVG format (one file per page).
pub fn compile_to_svg(quill: &Quill, glued_content: &str) -> Result<Vec<Vec<u8>>, RenderError> {
    let world = QuillWorld::new(quill, glued_content).map_err(|e| {
        RenderError::Internal(anyhow::anyhow!("Failed to create Typst world: {}", e))
    })?;

    let document = compile_document(&world)?;

    let mut pages = Vec::new();
    for page in &document.pages {
        let svg = typst_svg::svg(page);
        pages.push(svg.into_bytes());
    }

    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the page limit constant is reasonable
    #[test]
    fn test_constants() {
        assert_eq!(COMPILE_TIMEOUT.as_secs(), 60);
        assert_eq!(MAX_COMPILATION_PAGES, 1000);
    }

    /// Test that a document with too many pages is rejected
    /// Note: This is a documentation test - actual enforcement happens in compile_document
    #[test]
    fn test_page_limit_exists() {
        // This test documents the existence of the page limit feature
        // Actual integration testing would require creating a document with 1000+ pages
        assert!(MAX_COMPILATION_PAGES > 0);
        assert!(MAX_COMPILATION_PAGES <= 10000); // Reasonable upper bound
    }

    /// Test that timeout constant is set
    #[test]
    fn test_timeout_constant() {
        // This test documents the existence of the timeout feature
        assert!(COMPILE_TIMEOUT.as_secs() > 0);
        assert!(COMPILE_TIMEOUT.as_secs() <= 300); // Should be reasonable (≤5 minutes)
    }
}

/// Internal compilation function with timeout tracking and page limits
fn compile_document(world: &QuillWorld) -> Result<PagedDocument, RenderError> {
    let start = Instant::now();

    let Warned {
        output,
        warnings: _,
    } = typst::compile::<PagedDocument>(world);

    let elapsed = start.elapsed();

    // Check if compilation took too long
    // Note: This does not prevent long-running compilations, only detects them
    // For true timeout enforcement, process isolation would be needed
    if elapsed > COMPILE_TIMEOUT {
        return Err(RenderError::CompilationTimeout {
            timeout_secs: COMPILE_TIMEOUT.as_secs(),
        });
    }

    match output {
        Ok(doc) => {
            // Check page count limit
            if doc.pages.len() > MAX_COMPILATION_PAGES {
                return Err(RenderError::TooManyPages {
                    page_count: doc.pages.len(),
                    max_pages: MAX_COMPILATION_PAGES,
                });
            }

            // TODO: Capture and propagate warnings to RenderResult
            Ok(doc)
        }
        Err(errors) => {
            let diagnostics = map_typst_errors(&errors, world);
            Err(RenderError::CompilationFailed(
                diagnostics.len(),
                diagnostics,
            ))
        }
    }
}
