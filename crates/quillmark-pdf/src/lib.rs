//! # quillmark-pdf: the AcroForm stamping spine
//!
//! Typst-free infra (not a backend) whose whole job is one pure operation:
//!
//! ```text
//! (base_pdf_bytes, &[FieldSpec]) -> { stamped_pdf, regions }
//! ```
//!
//! via a single incremental-update append. Both backends — Typst (geometry from
//! introspection) and `pdfform` (geometry from `form.json`) — produce a base PDF
//! plus [`FieldSpec`]s, and unify exactly at that seam. The crate owns its own
//! [`PdfError`]; each backend maps it to `quillmark_core::RenderError`.
//!
//! `crate::reader`'s docs carry the input contract the base PDF must satisfy.

mod error;
/// Byte-level reads over an existing PDF.
///
/// **Workspace-internal; not covered by this crate's semver.** `pub` only so
/// `quillmark-pdfform` can reach it. The supported surface is [`stamp`],
/// [`regions_of`], [`page_media_boxes`], [`PdfUpdate`], and the types they name.
#[doc(hidden)]
pub mod reader;
mod stamp;
mod update;
/// Byte-level writes: object emission, id allocation, string escaping, WinAnsi.
///
/// **Workspace-internal; not covered by this crate's semver.**
#[doc(hidden)]
pub mod writer;

pub use error::PdfError;
pub use stamp::{regions_of, stamp, StampOptions, CHECKBOX_ON_STATE};
pub use update::PdfUpdate;

/// The `/MediaBox` of every page of `base`, normalized to `[x0, y0, x1, y1]`
/// (lower-left, upper-right), in document order. A backend owning top-left
/// page-relative rects flips against these before building a [`FieldSpec`].
pub fn page_media_boxes(base: &[u8]) -> Result<Vec<[f32; 4]>, PdfError> {
    reader::page_media_boxes(base)
}

/// One fully-resolved form field: the backend-agnostic currency of the spine.
/// `rect` is final geometry, so the spine never reasons about page height or
/// reflow.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FieldSpec {
    /// Fully-qualified field name, written to `/T`.
    pub name: String,
    /// The quill schema field address this widget maps to. Opaque to the spine,
    /// which carries it only to key the region sidecar: `None` emits no
    /// [`RenderedRegion`](quillmark_core::RenderedRegion).
    pub schema_field: Option<String>,
    /// 0-based page index.
    pub page: usize,
    /// `[x0, y0, x1, y1]` in PDF points, bottom-left origin.
    pub rect: [f32; 4],
    pub field_type: FieldType,
    /// `None` is blank; for a checkbox, `Some` (the on-state name) is checked.
    pub value: Option<String>,
    /// Optional `/TU` tooltip / accessible name.
    pub tooltip: Option<String>,
}

impl FieldSpec {
    /// `schema_field`, `value`, and `tooltip` start `None`.
    pub fn new(name: String, page: usize, rect: [f32; 4], field_type: FieldType) -> Self {
        Self {
            name,
            schema_field: None,
            page,
            rect,
            field_type,
            value: None,
            tooltip: None,
        }
    }

    /// Set [`schema_field`](Self::schema_field); without it, no region is emitted.
    pub fn with_schema_field(mut self, schema_field: String) -> Self {
        self.schema_field = Some(schema_field);
        self
    }

    /// Set [`value`](Self::value).
    pub fn with_value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }

    /// Set [`tooltip`](Self::tooltip).
    pub fn with_tooltip(mut self, tooltip: String) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}

/// A field's definition, never a runtime value (that rides in
/// [`FieldSpec::value`]).
///
/// Deliberately exhaustive, unlike the other public enums here: `pdfform`'s
/// value resolver and content-stream flattener both dispatch over the whole set,
/// and a variant neither handles draws nothing and reports nothing, so the
/// compile error is the guardrail. The price is that a new widget type is
/// semver-major, which the AcroForm widget kinds do not make likely.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Text { multiline: bool },
    /// A checkbox with the engine's fixed on-state ([`CHECKBOX_ON_STATE`]).
    Checkbox,
    /// A dropdown over `options`, bare display strings.
    Choice { options: Vec<String> },
    /// An unsigned signature field.
    Signature,
}
