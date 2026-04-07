//! # Typst Compilation
//!
//! This module compiles Typst documents to output formats (PDF, SVG, and PNG).
//!
//! ## Functions
//!
//! - [`compile_to_pdf()`] - Compile Typst to PDF format
//! - [`compile_to_svg()`] - Compile Typst to SVG format (one file per page)
//! - [`compile_to_png()`] - Compile Typst to PNG format (one image per page) at a given PPI
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
//! let pdf_bytes = compile_to_pdf(&quill, typst_content, "{}", &Default::default())?;
//! std::fs::write("output.pdf", pdf_bytes)?;
//! # Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
//! ```
//!
//! ## Process
//!
//! 1. Creates a `QuillWorld` with the quill's assets and packages
//! 2. Compiles the Typst document using the Typst compiler
//! 3. Converts to target format (PDF, SVG, or PNG)
//! 4. Returns output bytes
//!
//! The output bytes can be written to a file or returned directly to the caller.

use typst::diag::Warned;
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::error_mapping::map_typst_errors;
use crate::helper::ContentFields;
use crate::world::QuillWorld;
use quillmark_core::{
    Artifact, Diagnostic, OutputFormat, Quill, RenderError, RenderResult, Severity,
};

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

/// Compile Typst source into a paged document with injected JSON data.
pub fn compile_to_document(
    quill: &Quill,
    plated_content: &str,
    json_data: &str,
    content_fields: &ContentFields,
) -> Result<PagedDocument, RenderError> {
    let world =
        QuillWorld::new_with_data(quill, plated_content, json_data, content_fields).map_err(|e| {
        RenderError::EngineCreation {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!("Failed to create Typst compilation environment: {}", e),
                )
                .with_code("typst::world_creation".to_string())
                .with_source(e),
            ),
        }
    })?;

    compile_document(&world)
}

/// Compiles a Typst document to PDF format with JSON data injection.
///
/// This function creates a `@local/quillmark-helper:0.1.0` package containing
/// the JSON data, which can be imported by the plate file.
pub fn compile_to_pdf(
    quill: &Quill,
    plated_content: &str,
    json_data: &str,
    content_fields: &ContentFields,
) -> Result<Vec<u8>, RenderError> {
    let document = compile_to_document(quill, plated_content, json_data, content_fields)?;

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

/// Compiles a Typst document to SVG format with JSON data injection.
///
/// This function creates a `@local/quillmark-helper:0.1.0` package containing
/// the JSON data, which can be imported by the plate file.
pub fn compile_to_svg(
    quill: &Quill,
    plated_content: &str,
    json_data: &str,
    content_fields: &ContentFields,
) -> Result<Vec<Vec<u8>>, RenderError> {
    let document = compile_to_document(quill, plated_content, json_data, content_fields)?;

    let mut pages = Vec::new();
    for page in &document.pages {
        let svg = typst_svg::svg(page);
        pages.push(svg.into_bytes());
    }

    Ok(pages)
}

/// Default pixels per inch for PNG rendering (2x at 72pt/inch).
const DEFAULT_PPI: f32 = 144.0;

/// Compiles a Typst document to PNG format with JSON data injection.
///
/// Returns one PNG image (as bytes) per page.
///
/// # Arguments
///
/// * `quill` - The quill template containing assets and configuration
/// * `plated_content` - The plate file content (Typst source)
/// * `json_data` - JSON string containing the document data
/// * `ppi` - Pixels per inch. Defaults to 144.0 when `None`.
pub fn compile_to_png(
    quill: &Quill,
    plated_content: &str,
    json_data: &str,
    content_fields: &ContentFields,
    ppi: Option<f32>,
) -> Result<Vec<Vec<u8>>, RenderError> {
    let document = compile_to_document(quill, plated_content, json_data, content_fields)?;

    let ppi = ppi.unwrap_or(DEFAULT_PPI);

    let mut pages = Vec::new();
    for page in &document.pages {
        let pixmap = typst_render::render(page, ppi / 72.0);
        let png_data = pixmap
            .encode_png()
            .map_err(|e| RenderError::CompilationFailed {
                diags: vec![Diagnostic::new(
                    Severity::Error,
                    format!("PNG encoding failed: {}", e),
                )
                .with_code("typst::png_encoding".to_string())],
            })?;
        pages.push(png_data);
    }

    Ok(pages)
}

/// Render selected pages from an already-compiled Typst document.
pub fn render_document_pages(
    document: &PagedDocument,
    pages: Option<&[usize]>,
    format: OutputFormat,
    ppi: Option<f32>,
) -> Result<RenderResult, RenderError> {
    // PDF does not support selective page rendering
    if format == OutputFormat::Pdf && pages.is_some() {
        return Err(RenderError::FormatNotSupported {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    "PDF does not support page selection; pass null/None to render the full document, or use PNG/SVG".to_string(),
                )
                .with_code("typst::pdf_page_selection_not_supported".to_string()),
            ),
        });
    }

    let page_count = document.pages.len();
    let requested_indices: Vec<usize> = match pages {
        Some(slice) => slice.to_vec(),
        None => (0..page_count).collect(),
    };

    // Partition into valid and out-of-bounds indices, preserving requested order
    let mut warnings = Vec::new();
    let valid_indices: Vec<usize> = requested_indices
        .into_iter()
        .filter(|&idx| {
            if idx >= page_count {
                warnings.push(
                    Diagnostic::new(
                        Severity::Warning,
                        format!(
                            "Page index {} out of bounds (page_count={}), skipped",
                            idx, page_count
                        ),
                    )
                    .with_code("typst::page_index_out_of_bounds".to_string()),
                );
                false
            } else {
                true
            }
        })
        .collect();

    match format {
        OutputFormat::Svg => {
            let artifacts = valid_indices
                .into_iter()
                .map(|idx| Artifact {
                    bytes: typst_svg::svg(&document.pages[idx]).into_bytes(),
                    output_format: OutputFormat::Svg,
                })
                .collect();
            let mut result = RenderResult::new(artifacts, OutputFormat::Svg);
            result.warnings = warnings;
            Ok(result)
        }
        OutputFormat::Png => {
            let scale = ppi.unwrap_or(DEFAULT_PPI) / 72.0;
            let mut artifacts = Vec::with_capacity(valid_indices.len());
            for idx in valid_indices {
                let pixmap = typst_render::render(&document.pages[idx], scale);
                let png_data = pixmap
                    .encode_png()
                    .map_err(|e| RenderError::CompilationFailed {
                        diags: vec![Diagnostic::new(
                            Severity::Error,
                            format!("PNG encoding failed: {}", e),
                        )
                        .with_code("typst::png_encoding".to_string())],
                    })?;
                artifacts.push(Artifact {
                    bytes: png_data,
                    output_format: OutputFormat::Png,
                });
            }
            let mut result = RenderResult::new(artifacts, OutputFormat::Png);
            result.warnings = warnings;
            Ok(result)
        }
        OutputFormat::Pdf => {
            let pdf = typst_pdf::pdf(document, &PdfOptions::default()).map_err(|e| {
                RenderError::CompilationFailed {
                    diags: vec![Diagnostic::new(
                        Severity::Error,
                        format!("PDF generation failed: {:?}", e),
                    )
                    .with_code("typst::pdf_generation".to_string())],
                }
            })?;
            Ok(RenderResult::new(
                vec![Artifact {
                    bytes: pdf,
                    output_format: OutputFormat::Pdf,
                }],
                OutputFormat::Pdf,
            ))
        }
        OutputFormat::Txt => Err(RenderError::FormatNotSupported {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    "TXT output is not supported for Typst".into(),
                )
                .with_code("typst::format_not_supported".to_string()),
            ),
        }),
    }
}
