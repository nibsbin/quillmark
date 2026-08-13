//! The form-field adapter: a thin introspection→[`FieldSpec`] bridge onto the
//! shared `quillmark-pdf` stamping spine. Typst→PDF coordinate ownership lives
//! here so the spine never imports `typst_layout`.

use quillmark_core::{Diagnostic, RenderError, Severity};
use quillmark_pdf::{FieldSpec, FieldType, CHECKBOX_ON_STATE};
use typst_layout::PagedDocument;

mod extract;
mod span_scan;

pub(crate) use extract::extract;
pub(crate) use span_scan::{
    field_at, locate, position_at, scalar_windows, scan_content_regions, FieldWindow,
};

/// Mirrors the spine's [`FieldType`] but carries the *resolved* Typst value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FieldKind {
    Text {
        multiline: bool,
        value: Option<String>,
    },
    Checkbox { checked: bool },
    Choice {
        options: Vec<String>,
        value: Option<String>,
    },
    Signature,
}

/// One form field's geometry in Typst (top-left origin) points, plus its
/// kind/value payload.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FieldPlacement {
    pub name: String,
    /// The `field:` argument; `None` when the plate omits it, and the widget
    /// then exposes no region.
    pub schema_field: Option<String>,
    pub page: usize,
    pub rect_typst_pt: [f32; 4],
    pub kind: FieldKind,
}

pub(crate) fn err(code: &'static str, msg: impl Into<String>) -> RenderError {
    RenderError::from_diag(Diagnostic::new(Severity::Error, msg.into()).with_code(code.into()))
}

/// Owned by the backend, never defaulted from the leaf spine's version.
pub(crate) fn default_producer() -> String {
    format!("Quillmark {}", env!("CARGO_PKG_VERSION"))
}

/// Flips each rect from Typst's top-left origin to the PDF bottom-left origin
/// the spine consumes. The value coercion mirrors `quillmark-pdfform`'s
/// resolver, duplicated because this crate must not depend on it: the two
/// backends meet only at the `&[FieldSpec]` seam.
pub(crate) fn build_field_specs(
    doc: &PagedDocument,
    placements: &[FieldPlacement],
) -> Result<Vec<FieldSpec>, RenderError> {
    let page_heights: Vec<f32> = doc
        .pages()
        .iter()
        .map(|p| p.frame.size().y.to_pt() as f32)
        .collect();

    placements
        .iter()
        .map(|p| {
            let page_h = *page_heights.get(p.page).ok_or_else(|| {
                err(
                    "typst::form_field_page_out_of_range",
                    format!(
                        "form-field {:?} targets page {} but the document has {} page(s)",
                        p.name,
                        p.page,
                        page_heights.len()
                    ),
                )
            })?;
            let [x0, y0, x1, y1] = p.rect_typst_pt;
            let (field_type, value) = match &p.kind {
                FieldKind::Text { multiline, value } => (
                    FieldType::Text {
                        multiline: *multiline,
                    },
                    value.clone(),
                ),
                FieldKind::Checkbox { checked } => (
                    FieldType::Checkbox,
                    checked.then(|| CHECKBOX_ON_STATE.to_string()),
                ),
                FieldKind::Choice { options, value } => {
                    // Mirrors pdfform's `coerce_choice`.
                    let bound = value
                        .as_ref()
                        .filter(|v| options.iter().any(|o| o == *v))
                        .cloned();
                    (
                        FieldType::Choice {
                            options: options.clone(),
                        },
                        bound,
                    )
                }
                FieldKind::Signature => (FieldType::Signature, None),
            };
            // Typst top-left → PDF bottom-left.
            let mut spec = FieldSpec::new(
                p.name.clone(),
                p.page,
                [x0, page_h - y1, x1, page_h - y0],
                field_type,
            );
            spec.schema_field = p.schema_field.clone();
            spec.value = value;
            Ok(spec)
        })
        .collect()
}
