//! Typst document compilation to output formats.
//!
//! This module provides functions for compiling Typst documents to PDF and SVG formats.
//! It handles the Typst compilation process and manages the `QuillWorld` environment.
//!
//! # Functions
//!
//! - [`compile_to_pdf`] - Compile Typst to PDF format
//! - [`compile_to_svg`] - Compile Typst to SVG format (one file per page)
//!
//! # Compilation Process
//!
//! 1. Creates a `QuillWorld` with the quill's assets and packages
//! 2. Compiles the Typst document using the Typst compiler
//! 3. Converts the compiled document to the target format
//! 4. Returns the output bytes
//!
//! # Example
//!
//! ```no_run
//! use quillmark_typst::compile::compile_to_pdf;
//! use quillmark_core::Quill;
//!
//! let quill = Quill::from_path("path/to/quill").unwrap();
//! let typst_content = r#"
//!     #set document(title: "My Document")
//!     = Hello World
//! "#;
//!
//! let pdf_bytes = compile_to_pdf(&quill, typst_content).unwrap();
//! std::fs::write("output.pdf", pdf_bytes).unwrap();
//! ```

use typst::diag::{SourceDiagnostic, Warned};
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::world::QuillWorld;
use quillmark_core::Quill;

/// Compiles a Typst document to PDF format.
///
/// This function takes a quill template and Typst source code, creates a compilation
/// environment, and produces a PDF file as bytes.
///
/// # Arguments
///
/// * `quill` - The quill template providing assets, packages, and fonts
/// * `glued_content` - The complete Typst source code to compile
///
/// # Returns
///
/// Returns `Ok(Vec<u8>)` containing the PDF file bytes on success, or an error
/// if compilation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The Typst source has syntax errors
/// - Required assets or packages are missing
/// - PDF generation fails
///
/// # Examples
///
/// ```no_run
/// use quillmark_typst::compile::compile_to_pdf;
/// use quillmark_core::Quill;
///
/// let quill = Quill::from_path("path/to/quill")?;
/// let typst_content = "#set document(title: \"Test\")\n= Hello";
///
/// let pdf_bytes = compile_to_pdf(&quill, typst_content)?;
/// std::fs::write("output.pdf", pdf_bytes)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn compile_to_pdf(
    quill: &Quill,
    glued_content: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("Using quill: {}", quill.name);
    let world = QuillWorld::new(quill, glued_content)?;
    let document = compile_document(&world)?;

    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| format!("PDF generation failed: {:?}", e))?;

    Ok(pdf)
}

/// Compiles a Typst document to SVG format.
///
/// This function takes a quill template and Typst source code, creates a compilation
/// environment, and produces SVG files (one per page) as bytes.
///
/// # Arguments
///
/// * `quill` - The quill template providing assets, packages, and fonts
/// * `glued_content` - The complete Typst source code to compile
///
/// # Returns
///
/// Returns `Ok(Vec<Vec<u8>>)` containing a vector of SVG file bytes (one per page)
/// on success, or an error if compilation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The Typst source has syntax errors
/// - Required assets or packages are missing
/// - SVG generation fails
///
/// # Examples
///
/// ```no_run
/// use quillmark_typst::compile::compile_to_svg;
/// use quillmark_core::Quill;
///
/// let quill = Quill::from_path("path/to/quill")?;
/// let typst_content = r#"
///     = Page 1
///     Content on first page.
///     
///     #pagebreak()
///     
///     = Page 2
///     Content on second page.
/// "#;
///
/// let svg_pages = compile_to_svg(&quill, typst_content)?;
/// for (i, svg_bytes) in svg_pages.iter().enumerate() {
///     std::fs::write(format!("page_{}.svg", i + 1), svg_bytes)?;
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Note
///
/// Each page is rendered as a separate SVG document for maximum compatibility.
pub fn compile_to_svg(
    quill: &Quill,
    glued_content: &str,
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let world = QuillWorld::new(quill, glued_content)?;
    let document = compile_document(&world)?;

    let mut pages = Vec::new();
    for page in &document.pages {
        let svg = typst_svg::svg(page);
        pages.push(svg.into_bytes());
    }

    Ok(pages)
}

/// Internal compilation function
fn compile_document(world: &QuillWorld) -> Result<PagedDocument, Box<dyn std::error::Error>> {
    let Warned {
        output,
        warnings: _,
    } = typst::compile::<PagedDocument>(world);
    output.map_err(|errors| format_compilation_errors(&errors, world).into())
}

/// Format compilation errors with better visibility
fn format_compilation_errors(errors: &[SourceDiagnostic], world: &QuillWorld) -> String {
    if errors.is_empty() {
        return "Compilation failed with unknown errors".to_string();
    }

    let mut formatted = format!("Compilation failed with {} error(s):", errors.len());

    for (i, error) in errors.iter().enumerate() {
        formatted.push_str(&format!("\n\nError #{}: {}", i + 1, error.message));

        // Try to get line information from the span
        if let Some(line_info) = get_line_info_from_span(error.span, world) {
            formatted.push_str(&format!("\n  Location: {}", line_info));
        } else {
            formatted.push_str(&format!("\n  Span: {:?}", error.span));
        }

        formatted.push_str(&format!("\n  Severity: {:?}", error.severity));

        // Add hints if available
        if !error.hints.is_empty() {
            formatted.push_str("\n  Hints:");
            for hint in &error.hints {
                formatted.push_str(&format!("\n    - {}", hint));
            }
        }

        // Add trace if available
        if !error.trace.is_empty() {
            formatted.push_str("\n  Trace:");
            for trace_entry in &error.trace {
                formatted.push_str(&format!("\n    - {:?}", trace_entry));
            }
        }
    }

    formatted
}

/// Extract line information from a span
fn get_line_info_from_span(span: typst::syntax::Span, world: &QuillWorld) -> Option<String> {
    use typst::World;

    // Try to find the source that contains this span
    let source_id = world.main();
    if let Ok(source) = world.source(source_id) {
        if let Some(range) = source.range(span) {
            let text = source.text();
            let start_line = text[..range.start].matches('\n').count() + 1;
            let start_col =
                range.start - text[..range.start].rfind('\n').map_or(0, |pos| pos + 1) + 1;

            // Get the actual line content
            let lines: Vec<&str> = text.lines().collect();
            let line_content = lines.get(start_line - 1).unwrap_or(&"<line not found>");

            return Some(format!(
                "line {}, column {} in file '{}'\n    {}",
                start_line,
                start_col,
                source.id().vpath().as_rootless_path().display(),
                line_content
            ));
        }
    }

    None
}
