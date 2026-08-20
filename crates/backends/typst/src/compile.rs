//! Compiles a plate plus injected JSON data into a Typst paged document, then
//! renders selected pages to output bytes (PDF, SVG, or PNG).

use typst::diag::Warned;
use typst::utils::Scalar;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;
use typst_render::RenderOptions;
use typst_svg::SvgOptions;

use crate::error_mapping::map_typst_errors;
use crate::overlay;
use crate::world::QuillWorld;
use quillmark_core::{Artifact, Diagnostic, OutputFormat, RenderError, RenderResult, Severity};
use quillmark_pdf::{stamp, StampOptions};

pub(crate) fn render_options(pixel_per_pt: f32) -> RenderOptions {
    RenderOptions {
        pixel_per_pt: Scalar::new(pixel_per_pt as f64),
        ..Default::default()
    }
}

/// `comemo`'s cache is process-global and unbounded without eviction; 10 matches
/// typst-cli's watch-loop policy. The age clock is process-global too, so
/// interleaved sessions evict each other's entries early — lost reuse, never a
/// wrong render.
const COMEMO_EVICT_MAX_AGE: usize = 10;

/// On failure the warnings of the failed compile are dropped along with it.
pub(crate) fn compile_document(
    world: &QuillWorld,
) -> Result<(PagedDocument, Vec<Diagnostic>), RenderError> {
    let Warned { output, warnings } = typst::compile::<PagedDocument>(world);
    comemo::evict(COMEMO_EVICT_MAX_AGE);

    match output {
        Ok(doc) => Ok((doc, map_typst_errors(&warnings, world))),
        Err(errors) => Err(RenderError::new(map_typst_errors(&errors, world))),
    }
}

/// 2x at 72pt/inch.
const DEFAULT_PPI: f32 = 144.0;

/// `field_placements` are stamped as AcroForm widgets by the PDF path only;
/// `producer` overrides the PDF `/Info` `/Producer` string.
pub(crate) fn render_document_pages(
    document: &PagedDocument,
    pages: Option<&[usize]>,
    format: OutputFormat,
    ppi: Option<f32>,
    field_placements: &[overlay::FieldPlacement],
    producer: Option<&str>,
) -> Result<RenderResult, RenderError> {
    if format == OutputFormat::Pdf && pages.is_some() {
        return Err(RenderError::from_diag(
            Diagnostic::new(
                Severity::Error,
                "PDF does not support page selection; pass null/None to render the full document, or use PNG/SVG".to_string(),
            )
            .with_code("typst::pdf_page_selection_not_supported".to_string()),
        ));
    }

    let page_count = document.pages().len();
    let selected_indices: Vec<usize> = match pages {
        Some(slice) => {
            let out_of_bounds: Vec<usize> =
                slice.iter().copied().filter(|&i| i >= page_count).collect();
            if !out_of_bounds.is_empty() {
                return Err(RenderError::from_diag(
                    Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Page index out of bounds (page_count={}); offending indices: {:?}. Check `LiveSession.pageCount` before requesting pages.",
                            page_count, out_of_bounds
                        ),
                    )
                    .with_code("typst::page_index_out_of_bounds".to_string()),
                ));
            }
            slice.to_vec()
        }
        None => (0..page_count).collect(),
    };

    match format {
        OutputFormat::Svg => {
            let artifacts = selected_indices
                .into_iter()
                .map(|idx| {
                    Artifact::new(
                        typst_svg::svg(&document.pages()[idx], &SvgOptions::default())
                            .into_bytes(),
                        OutputFormat::Svg,
                    )
                })
                .collect();
            Ok(RenderResult::new(artifacts, OutputFormat::Svg))
        }
        OutputFormat::Png => {
            let scale = ppi.unwrap_or(DEFAULT_PPI) / 72.0;
            let opts = render_options(scale);
            let mut artifacts = Vec::with_capacity(selected_indices.len());
            for idx in selected_indices {
                let pixmap = typst_render::render(&document.pages()[idx], &opts);
                let png_data = pixmap.encode_png().map_err(|e| {
                    RenderError::from_diag(
                        Diagnostic::new(Severity::Error, format!("PNG encoding failed: {}", e))
                            .with_code("typst::png_encoding".to_string()),
                    )
                })?;
                artifacts.push(Artifact::new(png_data, OutputFormat::Png));
            }
            Ok(RenderResult::new(artifacts, OutputFormat::Png))
        }
        OutputFormat::Pdf => {
            let pdf = typst_pdf::pdf(document, &PdfOptions::default()).map_err(|e| {
                RenderError::from_diag(
                    Diagnostic::new(Severity::Error, format!("PDF generation failed: {:?}", e))
                        .with_code("typst::pdf_generation".to_string()),
                )
            })?;
            let field_specs = overlay::build_field_specs(document, field_placements)?;
            let producer = producer
                .map(str::to_string)
                .unwrap_or_else(overlay::default_producer);
            let opts = StampOptions::default().with_producer(producer);
            let stamped = stamp(pdf, &field_specs, &opts)?;
            Ok(RenderResult::new(
                vec![Artifact::new(stamped, OutputFormat::Pdf)],
                OutputFormat::Pdf,
            ))
        }
        // Forced by `#[non_exhaustive]`; `TypstSession::render` already rejects
        // formats outside `SUPPORTED_FORMATS`.
        other => Err(quillmark_core::unsupported_format(
            other,
            "typst",
            crate::SUPPORTED_FORMATS,
        )),
    }
}
