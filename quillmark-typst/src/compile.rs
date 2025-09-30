use typst::diag::{Warned, SourceDiagnostic};
use typst::layout::PagedDocument;
use typst_pdf::PdfOptions;

use quillmark_core::Quill;
use crate::world::QuillWorld;

/// Compile a quill template with Typst content to PDF
pub fn compile_to_pdf(quill: &Quill, glued_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("Using quill: {}", quill.name);
    let world = QuillWorld::new(quill, glued_content)?;
    let document = compile_document(&world)?;
    
    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| format!("PDF generation failed: {:?}", e))?;

    Ok(pdf)
}

/// Compile a quill template with Typst content to SVG pages
pub fn compile_to_svg(quill: &Quill, glued_content: &str) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
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
    let Warned { output, warnings: _ } = typst::compile::<PagedDocument>(world);
    output.map_err(|errors| {
        format_compilation_errors(&errors, world).into()
    })
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
            let start_col = range.start - text[..range.start].rfind('\n').map_or(0, |pos| pos + 1) + 1;
            
            // Get the actual line content
            let lines: Vec<&str> = text.lines().collect();
            let line_content = lines.get(start_line - 1).unwrap_or(&"<line not found>");
            
            return Some(format!("line {}, column {} in file '{}'\n    {}", 
                start_line, start_col, source.id().vpath().as_rootless_path().display(), line_content));
        }
    }
    
    None
}