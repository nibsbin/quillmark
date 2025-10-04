#![doc = include_str!("../docs/compile.md")]

use typst::diag::Warned;
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::world::QuillWorld;
use crate::error_mapping::map_typst_errors;
use quillmark_core::{Quill, RenderError};

/// Compiles a Typst document to PDF format.
pub fn compile_to_pdf(
    quill: &Quill,
    glued_content: &str,
) -> Result<Vec<u8>, RenderError> {
    println!("Using quill: {}", quill.name);
    let world = QuillWorld::new(quill, glued_content)
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("Failed to create Typst world: {}", e)))?;
    
    let document = compile_document(&world)?;

    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("PDF generation failed: {:?}", e)))?;

    Ok(pdf)
}

/// Compiles a Typst document to SVG format (one file per page).
pub fn compile_to_svg(
    quill: &Quill,
    glued_content: &str,
) -> Result<Vec<Vec<u8>>, RenderError> {
    let world = QuillWorld::new(quill, glued_content)
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("Failed to create Typst world: {}", e)))?;
    
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
    let Warned {
        output,
        warnings: _,
    } = typst::compile::<PagedDocument>(world);
    
    match output {
        Ok(doc) => {
            // TODO: Capture and propagate warnings to RenderResult
            Ok(doc)
        }
        Err(errors) => {
            let diagnostics = map_typst_errors(&errors, world);
            Err(RenderError::CompilationFailed(diagnostics.len(), diagnostics))
        }
    }
}
