//! Write a fresh `/AcroForm` (and an `/Info` `/Producer` stamp) onto a base PDF
//! via one incremental-update append.
//!
//! Technique A: style the real AcroForm fields and set `/NeedAppearances`, never
//! baking `/AP` appearance streams. Appearance synthesis is the viewer's job, so
//! flat rasterizers render the fields blank and values reach non-interactive
//! output only via the [`RenderedRegion`] sidecar.
//!
//! The background owns all visual chrome, so a widget is a transparent input
//! over it: no borders, no fills, and black as the only text color. What a
//! widget does choose is the type its value is *set in*, since that has to
//! match the background it sits on rather than decorate it: each field names
//! its own face, size, and justification, and every face so named is registered
//! in the form `/DR`. The form is built fresh from the spec; a foreign AcroForm
//! is never reconciled.

use pdf_writer::types::{AnnotationFlags, FieldFlags, FieldType as PwFieldType, Quadding, SigFlags};
use pdf_writer::writers::{Field, Form};
use pdf_writer::{Chunk, Finish, Name, Rect, Ref, Str, TextStr};

use quillmark_core::RenderedRegion;

use crate::error::PdfError;
use crate::reader::{
    err, extract_outer_dict, find_dict_value, find_object_bytes, parse_indirect_ref,
    splice_dict_value, UpdatedObject,
};
use crate::update::PdfUpdate;
use crate::writer::{alloc_id, dict_object};
use crate::{FieldSpec, FieldType, FormFont, TextAlign};

const CODE_PARSE: &str = "pdf::stamp_parse";

/// The fixed checkbox on-state export name. A checkbox [`FieldSpec`] carries
/// this as its `value` when checked, `None` when not.
pub const CHECKBOX_ON_STATE: &str = "Yes";

/// House-style default appearance: Helvetica, `0 Tf` (auto-size), black fill.
/// `Helv` is registered in the form `/DR` `/Font`.
///
/// The form-level `/DA` fallback only; a widget carries its own, built by
/// [`field_appearance`].
const DEFAULT_APPEARANCE: &[u8] = b"/Helv 0 Tf 0 g";

/// One widget's `/DA`: its face and size over the house black fill. `f32`'s
/// `Display` drops the trailing `.0`, so a whole-point size writes `12`, and an
/// absent size writes the `0 Tf` that defers to the viewer's auto-size.
///
/// A size that is not a positive finite number falls back to that same `0 Tf`:
/// `font_size` is a public field, and `NaN`/`inf` would reach the `/DA` as a
/// token no PDF number grammar admits, while a negative one is `0 Tf` said
/// obscurely. The Typst helper rejects all three at the call site; this keeps a
/// direct spine consumer from writing a file no viewer can parse.
fn field_appearance(spec: &FieldSpec) -> Vec<u8> {
    let size = spec
        .font_size
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(0.0);
    let mut da = b"/".to_vec();
    da.extend_from_slice(spec.font.resource_name());
    da.extend_from_slice(format!(" {size} Tf 0 g").as_bytes());
    da
}

/// Every face named by a `/DA` this stamp writes: the widgets' own, plus the
/// Helvetica of the form-level [`DEFAULT_APPEARANCE`].
fn fonts_used(fields: &[FieldSpec]) -> Vec<FormFont> {
    let mut fonts: Vec<FormFont> = fields.iter().map(|f| f.font).collect();
    fonts.push(FormFont::Helvetica);
    fonts.sort_by_key(|f| f.resource_name());
    fonts.dedup();
    fonts
}

/// Options for [`stamp`](crate::stamp).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct StampOptions {
    /// `None` leaves the base PDF's `/Producer` untouched. The spine never
    /// defaults this from its own crate version.
    pub producer: Option<String>,
}

impl StampOptions {
    /// Set [`producer`](Self::producer).
    pub fn with_producer(mut self, producer: String) -> Self {
        self.producer = Some(producer);
        self
    }
}

/// Stamp `fields` onto `base` as a fresh AcroForm via one incremental update,
/// optionally stamping `/Info` `/Producer`. Geometry is not produced here; it is
/// a session-level query (see [`regions_of`]).
///
/// `base` must satisfy the reader's input contract (traditional-xref,
/// unencrypted, inline-annots, flat-tree), and each `rect` must already be final
/// bottom-left PDF-point geometry.
pub fn stamp(
    base: Vec<u8>,
    fields: &[FieldSpec],
    opts: &StampOptions,
) -> Result<Vec<u8>, PdfError> {
    // Return the base as-is rather than append an empty revision.
    if opts.producer.is_none() && fields.is_empty() {
        return Ok(base);
    }

    let pdf = base;
    let mut up = PdfUpdate::begin(&pdf, opts.producer.as_deref())?;

    if !fields.is_empty() {
        let page_ids = up.resolve_pages(&pdf, fields)?;
        let page_count = page_ids.len();

        let fonts = fonts_used(fields);
        let font_ids: Vec<u32> = fonts
            .iter()
            .map(|_| alloc_id(&mut up.next_id))
            .collect::<Result<_, _>>()?;
        let widget_ids: Vec<u32> = fields
            .iter()
            .map(|_| alloc_id(&mut up.next_id))
            .collect::<Result<_, _>>()?;
        let acroform_id = alloc_id(&mut up.next_id)?;

        // Grouped by page for the page-side `/Annots`.
        let mut widgets_by_page: Vec<Vec<u32>> = vec![Vec::new(); page_count];
        for (spec, &wid) in fields.iter().zip(&widget_ids) {
            widgets_by_page[spec.page].push(wid);
            let page_ref = Ref::new(page_ids[spec.page] as i32);
            up.objects.push(UpdatedObject {
                id: wid,
                bytes: write_widget_object(spec, Ref::new(wid as i32), page_ref),
            });
        }

        for (font, &fid) in fonts.iter().zip(&font_ids) {
            let mut fchunk = Chunk::new();
            fchunk
                .type1_font(Ref::new(fid as i32))
                .base_font(Name(font.base_font()));
            up.objects.push(UpdatedObject {
                id: fid,
                bytes: fchunk.as_bytes().to_vec(),
            });
        }

        let has_signature = fields
            .iter()
            .any(|f| matches!(f.field_type, FieldType::Signature));
        let mut achunk = Chunk::new();
        {
            let mut form: Form<'_> = achunk
                .indirect(Ref::new(acroform_id as i32))
                .start::<Form>();
            form.fields(widget_ids.iter().map(|&id| Ref::new(id as i32)));
            if has_signature {
                form.sig_flags(SigFlags::SIGNATURES_EXIST);
            }
            form.pair(Name(b"NeedAppearances"), true);
            form.default_appearance(Str(DEFAULT_APPEARANCE));
            // /DR << /Font << /Helv <font> .. >> >>
            {
                let mut dr = form.insert(Name(b"DR")).dict();
                let mut font_dict = dr.insert(Name(b"Font")).dict();
                for (font, &fid) in fonts.iter().zip(&font_ids) {
                    font_dict.pair(Name(font.resource_name()), Ref::new(fid as i32));
                }
            }
            form.finish();
        }
        up.objects.push(UpdatedObject {
            id: acroform_id,
            bytes: achunk.as_bytes().to_vec(),
        });

        // A widget is fillable only if reachable both ways: the catalog's
        // `/AcroForm /Fields` (added here) and the page's `/Annots` (below).
        let (cs, ce) = find_object_bytes(&pdf, up.catalog_id)
            .ok_or_else(|| err(CODE_PARSE, "catalog not found"))?;
        let cat_dict = extract_outer_dict(&pdf[cs..ce])
            .ok_or_else(|| err(CODE_PARSE, "catalog dict not parseable"))?;
        let mut cat_inner = cat_dict.to_vec();
        cat_inner.extend_from_slice(format!(" /AcroForm {acroform_id} 0 R").as_bytes());
        up.objects.push(dict_object(up.catalog_id, &cat_inner));

        for (page_idx, widget_refs) in widgets_by_page.iter().enumerate() {
            if widget_refs.is_empty() {
                continue;
            }
            let page_obj_id = page_ids[page_idx];
            let (s, e) = find_object_bytes(&pdf, page_obj_id)
                .ok_or_else(|| err(CODE_PARSE, format!("page node {page_obj_id} not found")))?;
            let pg_dict = extract_outer_dict(&pdf[s..e])
                .ok_or_else(|| err(CODE_PARSE, format!("page node {page_obj_id} not parseable")))?;
            up.objects.push(dict_object(
                page_obj_id,
                &rewrite_page_with_annots(pg_dict, widget_refs)?,
            ));
        }
    }

    up.finish(pdf)
}

/// One [`RenderedRegion`] per field carrying a schema address, keyed on that
/// address. A widget with no schema field is a backend-only artifact and emits
/// nothing. Shared by `stamp` and the no-stamp render paths, so region geometry
/// always matches the widget.
pub fn regions_of(fields: &[FieldSpec]) -> Vec<RenderedRegion> {
    fields
        .iter()
        .filter_map(|f| {
            Some(RenderedRegion::new(
                f.schema_field.clone()?,
                f.page,
                f.rect,
            ))
        })
        .collect()
}

/// Left is the PDF's own `/Q` default, so it is left unwritten: a field that
/// never touches this dial stamps exactly as it did before the dial existed.
fn write_quadding(field: &mut Field<'_>, align: TextAlign) {
    let q = match align {
        TextAlign::Left => return,
        TextAlign::Center => Quadding::Center,
        TextAlign::Right => Quadding::Right,
    };
    field.vartext_quadding(q);
}

/// Serialize one field as a merged field+widget indirect object.
fn write_widget_object(spec: &FieldSpec, wid: Ref, page_ref: Ref) -> Vec<u8> {
    let mut chunk = Chunk::new();
    {
        let mut field = chunk.form_field(wid);
        field.partial_name(TextStr(&spec.name));
        if let Some(tt) = spec.tooltip.as_deref() {
            field.alternate_name(TextStr(tt));
        }

        // Captured to also set the annotation `/AS` below.
        let mut checkbox_on: Option<bool> = None;
        let da = field_appearance(spec);

        match &spec.field_type {
            FieldType::Text { multiline } => {
                field.field_type(PwFieldType::Text);
                field.vartext_default_appearance(Str(&da));
                write_quadding(&mut field, spec.align);
                if *multiline {
                    field.field_flags(FieldFlags::MULTILINE);
                }
                if let Some(v) = spec.value.as_deref() {
                    field.text_value(TextStr(v));
                }
            }
            FieldType::Checkbox => {
                field.field_type(PwFieldType::Button);
                let on = spec.value.is_some();
                checkbox_on = Some(on);
                field.pair(
                    Name(b"V"),
                    if on {
                        Name(CHECKBOX_ON_STATE.as_bytes())
                    } else {
                        Name(b"Off")
                    },
                );
                // /MK /CA (4): the ZapfDingbats check glyph the viewer
                // synthesizes under NeedAppearances.
                {
                    let mut mk = field.insert(Name(b"MK")).dict();
                    mk.pair(Name(b"CA"), Str(b"4"));
                }
            }
            FieldType::Choice { options } => {
                field.field_type(PwFieldType::Choice);
                field.field_flags(FieldFlags::COMBO);
                field.vartext_default_appearance(Str(&da));
                write_quadding(&mut field, spec.align);
                {
                    let mut opts = field.choice_options();
                    for o in options {
                        opts.option(TextStr(o));
                    }
                }
                if let Some(v) = spec.value.as_deref() {
                    field.choice_value(Some(TextStr(v)));
                }
            }
            FieldType::Signature => {
                field.field_type(PwFieldType::Signature);
            }
        }

        // `into_annotation` writes `/Type /Annot` + `/Subtype /Widget` exactly
        // once; do not call `.subtype()` again or it duplicates the key.
        let mut ann = field.into_annotation();
        ann.rect(Rect::new(
            spec.rect[0],
            spec.rect[1],
            spec.rect[2],
            spec.rect[3],
        ))
        .page(page_ref)
        .flags(AnnotationFlags::PRINT);
        if let Some(on) = checkbox_on {
            ann.appearance_state(if on {
                Name(CHECKBOX_ON_STATE.as_bytes())
            } else {
                Name(b"Off")
            });
        }
        ann.finish();
    }
    chunk.as_bytes().to_vec()
}

/// Three cases for the existing `/Annots`: absent (write a fresh array);
/// inline array (splice widget refs before `]`); indirect reference (hard
/// error, the input contract requires inline annots).
fn rewrite_page_with_annots(pg_dict: &[u8], widget_refs: &[u32]) -> Result<Vec<u8>, PdfError> {
    let widgets_str = widget_refs
        .iter()
        .map(|r| format!("{r} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");

    match find_dict_value(pg_dict, "Annots") {
        None => {
            let mut out = pg_dict.to_vec();
            out.extend_from_slice(format!(" /Annots [{widgets_str}]").as_bytes());
            Ok(out)
        }
        Some(existing) => {
            let trimmed = existing.trim_ascii();
            if trimmed.starts_with(b"[") {
                let end = trimmed
                    .iter()
                    .rposition(|&b| b == b']')
                    .ok_or_else(|| err(CODE_PARSE, "/Annots array missing ]"))?;
                let inner = &trimmed[1..end];
                let merged = format!(
                    "[{} {}]",
                    String::from_utf8_lossy(inner).trim(),
                    widgets_str
                );
                Ok(splice_dict_value(
                    pg_dict,
                    b"/Annots",
                    existing,
                    merged.as_bytes(),
                ))
            } else if parse_indirect_ref(existing).is_some() {
                Err(err(
                    "pdf::indirect_annots",
                    "/Annots is an indirect reference; only inline arrays are supported",
                ))
            } else {
                Err(err(CODE_PARSE, "/Annots is neither array nor indirect ref"))
            }
        }
    }
}
