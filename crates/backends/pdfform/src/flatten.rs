//! Draw field values as PDF content stream operators instead of AcroForm
//! widgets, so they are visible in non-interactive rasterizers (pdfium,
//! Ghostscript) that never synthesize `/NeedAppearances` appearances.
//!
//! Drawing directly commits to a byte encoding: text is transcoded to WinAnsi
//! and shown with a `WinAnsiEncoding` Helvetica, clipped to its field box. The
//! stream is appended last to the page `/Contents` and positions text with
//! absolute `Td`, so it assumes the identity CTM of page default user space —
//! true of any background with balanced `q`/`Q` and no dangling `cm`. Backs the
//! SVG/PNG raster outputs only; the AcroForm PDF deliverable is stamped.

use quillmark_pdf::{
    reader::{
        extract_outer_dict, find_dict_value, find_object_bytes, splice_dict_value, UpdatedObject,
    },
    writer::{alloc_id, dict_object, pdf_escape, winansi_encode},
    FieldSpec, FieldType, PdfError, PdfUpdate, CHECKBOX_ON_STATE,
};

use crate::typography;

const CODE_PARSE: &str = "pdf::flatten_parse";

/// Flatten `fields` onto `base` by drawing values as content stream operators.
/// Backs raster output only, so it stamps no `/Info /Producer`.
pub fn flatten(base: Vec<u8>, fields: &[FieldSpec]) -> Result<Vec<u8>, PdfError> {
    if fields.is_empty() {
        return Ok(base);
    }

    let pdf = base;
    let mut up = PdfUpdate::begin(&pdf, None)?;

    let page_ids = up.resolve_pages(&pdf, fields)?;
    let page_count = page_ids.len();

    // Helvetica and ZapfDingbats are among the 14 standard PDF fonts every
    // conforming reader provides, so neither is embedded.
    let helv_id = alloc_id(&mut up.next_id)?;
    let zadb_id = alloc_id(&mut up.next_id)?;
    up.objects.push(type1_font_object(
        helv_id,
        typography::TEXT_FONT,
        Some("WinAnsiEncoding"),
    ));
    up.objects
        .push(type1_font_object(zadb_id, typography::CHECK_FONT, None));

    let mut fields_by_page: Vec<Vec<&FieldSpec>> = vec![Vec::new(); page_count];
    for spec in fields {
        fields_by_page[spec.page].push(spec);
    }

    for (page_idx, page_fields) in fields_by_page.iter().enumerate() {
        let drawable: Vec<&FieldSpec> = page_fields
            .iter()
            .copied()
            .filter(|s| has_drawable_value(s))
            .collect();
        if drawable.is_empty() {
            continue;
        }

        let stream_id = alloc_id(&mut up.next_id)?;
        up.objects.push(content_stream_object(
            stream_id,
            &build_content_stream(&drawable),
        ));

        let page_obj_id = page_ids[page_idx];
        let (s, e) = find_object_bytes(&pdf, page_obj_id)
            .ok_or_else(|| PdfError::new(CODE_PARSE, format!("page object {page_obj_id} not found")))?;
        let pg_dict = extract_outer_dict(&pdf[s..e])
            .ok_or_else(|| PdfError::new(CODE_PARSE, "page dict not parseable"))?;

        let new_pg = rewrite_page_for_flatten(pg_dict, helv_id, zadb_id, stream_id)?;
        up.objects.push(dict_object(page_obj_id, &new_pg));
    }

    up.finish(pdf)
}

fn has_drawable_value(spec: &FieldSpec) -> bool {
    match &spec.field_type {
        FieldType::Signature => false,
        FieldType::Checkbox => spec.value.as_deref() == Some(CHECKBOX_ON_STATE),
        _ => spec.value.is_some(),
    }
}

fn build_content_stream(fields: &[&FieldSpec]) -> Vec<u8> {
    let mut out = Vec::new();
    for spec in fields {
        let [x0, y0, x1, y1] = spec.rect;
        let w = x1 - x0;
        let h = y1 - y0;
        match &spec.field_type {
            FieldType::Signature => {}
            FieldType::Checkbox => {
                let size = typography::check_size(h);
                // ZapfDingbats check glyphs are roughly square; centre in the box.
                let x_pos = x0 + (w - size * typography::CHECK_GLYPH_WIDTH_FACTOR) * 0.5;
                let y_pos = y0 + (h - size) * 0.5;
                write_zadb_char(&mut out, x_pos, y_pos, size);
            }
            FieldType::Text { .. } => {
                if let Some(value) = &spec.value {
                    let size = typography::value_size(h);
                    let x_pos = x0 + typography::TEXT_INSET;
                    let y_top = y1 - size - typography::TEXT_TOP_INSET;
                    let lines: Vec<&str> = value.lines().collect();
                    write_text_block(&mut out, &lines, x_pos, y_top, size, spec.rect);
                }
            }
            FieldType::Choice { .. } => {
                if let Some(value) = &spec.value {
                    let size = typography::value_size(h);
                    let x_pos = x0 + typography::TEXT_INSET;
                    let y_pos = y0 + (h - size) * 0.5;
                    write_text_block(&mut out, &[value.as_str()], x_pos, y_pos, size, spec.rect);
                }
            }
        }
    }
    out
}

/// Draw `lines` in `/Helv`, clipped to `clip` so an over-long value cannot paint
/// over neighbouring content. Bytes are transcoded to WinAnsi to match the
/// font's `/Encoding /WinAnsiEncoding`.
fn write_text_block(out: &mut Vec<u8>, lines: &[&str], x: f32, y: f32, size: f32, clip: [f32; 4]) {
    if lines.is_empty() {
        return;
    }
    let line_h = size * typography::LINE_SPACING;
    let [cx0, cy0, cx1, cy1] = clip;
    out.extend_from_slice(b"q\n");
    // `x y w h re W n`
    push_f32(out, cx0);
    out.push(b' ');
    push_f32(out, cy0);
    out.push(b' ');
    push_f32(out, cx1 - cx0);
    out.push(b' ');
    push_f32(out, cy1 - cy0);
    out.extend_from_slice(b" re W n\n");
    out.extend_from_slice(b"BT\n/Helv ");
    push_f32(out, size);
    out.extend_from_slice(b" Tf\n");
    push_f32(out, x);
    out.push(b' ');
    push_f32(out, y);
    out.extend_from_slice(b" Td\n");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b"0 ");
            push_f32(out, -line_h);
            out.extend_from_slice(b" Td\n");
        }
        out.push(b'(');
        pdf_escape(out, &winansi_encode(line));
        out.extend_from_slice(b") Tj\n");
    }
    out.extend_from_slice(b"ET\nQ\n");
}

/// Draw ZapfDingbats glyph 0x34 (`'4'`), the filled check mark — the same glyph
/// the AcroForm stamp path declares via `/MK /CA (4)`.
fn write_zadb_char(out: &mut Vec<u8>, x: f32, y: f32, size: f32) {
    out.extend_from_slice(b"q\nBT\n/ZaDb ");
    push_f32(out, size);
    out.extend_from_slice(b" Tf\n");
    push_f32(out, x);
    out.push(b' ');
    push_f32(out, y);
    out.extend_from_slice(b" Td\n(4) Tj\nET\nQ\n");
}

fn rewrite_page_for_flatten(
    pg_dict: &[u8],
    helv_id: u32,
    zadb_id: u32,
    stream_id: u32,
) -> Result<Vec<u8>, PdfError> {
    let with_stream = add_content_stream(pg_dict, stream_id)?;
    let with_helv = add_font_resource(&with_stream, "Helv", helv_id)?;
    add_font_resource(&with_helv, "ZaDb", zadb_id)
}

fn add_content_stream(pg_dict: &[u8], stream_id: u32) -> Result<Vec<u8>, PdfError> {
    let ref_str = format!("{stream_id} 0 R");
    match find_dict_value(pg_dict, "Contents") {
        None => {
            let mut out = pg_dict.to_vec();
            out.extend_from_slice(format!(" /Contents [{ref_str}]").as_bytes());
            Ok(out)
        }
        Some(existing) => {
            let trimmed = existing.trim_ascii();
            let new_val = if trimmed.starts_with(b"[") {
                let end = trimmed
                    .iter()
                    .rposition(|&b| b == b']')
                    .ok_or_else(|| PdfError::new(CODE_PARSE, "/Contents array missing ]"))?;
                let inner = String::from_utf8_lossy(&trimmed[1..end]);
                format!("[{} {ref_str}]", inner.trim())
            } else {
                format!("[{} {ref_str}]", String::from_utf8_lossy(trimmed).trim())
            };
            Ok(splice_dict_value(
                pg_dict,
                b"/Contents",
                existing,
                new_val.as_bytes(),
            ))
        }
    }
}

/// Inject `/<name> <font_id> 0 R` into the page's `/Resources /Font` dict,
/// creating intermediate dicts as needed. An indirect `/Resources` or `/Font`
/// is an error rather than a skip: a `Tf` name resolves only through the page's
/// `/Font` subdictionary, so an uninjected name draws nothing.
fn add_font_resource(pg_dict: &[u8], name: &str, font_id: u32) -> Result<Vec<u8>, PdfError> {
    let helv_entry = format!("/{name} {font_id} 0 R");

    match find_dict_value(pg_dict, "Resources") {
        None => {
            let mut out = pg_dict.to_vec();
            out.extend_from_slice(format!(" /Resources << /Font << {helv_entry} >> >>").as_bytes());
            Ok(out)
        }
        Some(res_val) => {
            if !res_val.trim_ascii().starts_with(b"<<") {
                // Indirect /Resources ref: cannot inject the named font, and the
                // emitted `/Helv`/`/ZaDb` Tf operators would not resolve.
                return Err(PdfError::new(
                    CODE_PARSE,
                    "page /Resources is an indirect reference; flatten requires inline resources",
                ));
            }
            let res_inner = extract_outer_dict(res_val)
                .ok_or_else(|| PdfError::new(CODE_PARSE, "page /Resources dict not parseable"))?;

            let new_res_inner: Vec<u8> = match find_dict_value(res_inner, "Font") {
                None => {
                    let mut out = res_inner.to_vec();
                    out.extend_from_slice(format!(" /Font << {helv_entry} >>").as_bytes());
                    out
                }
                Some(font_val) => {
                    if !font_val.trim_ascii().starts_with(b"<<") {
                        return Err(PdfError::new(
                            CODE_PARSE,
                            "page /Resources /Font is an indirect reference; flatten requires \
                             an inline /Font dict",
                        ));
                    }
                    let font_inner = extract_outer_dict(font_val).ok_or_else(|| {
                        PdfError::new(CODE_PARSE, "page /Resources /Font dict not parseable")
                    })?;
                    let mut new_font_val = b"<< ".to_vec();
                    new_font_val.extend_from_slice(font_inner);
                    new_font_val.extend_from_slice(format!(" {helv_entry} >>").as_bytes());

                    splice_dict_value(res_inner, b"/Font", font_val, &new_font_val)
                }
            };

            let mut new_res_val = b"<< ".to_vec();
            new_res_val.extend_from_slice(&new_res_inner);
            new_res_val.extend_from_slice(b" >>");

            Ok(splice_dict_value(
                pg_dict,
                b"/Resources",
                res_val,
                &new_res_val,
            ))
        }
    }
}

/// Build a base-14 Type1 font object. Symbol fonts pass `encoding: None` to keep
/// their built-in encoding; text fonts name `WinAnsiEncoding`.
fn type1_font_object(id: u32, base_font: &str, encoding: Option<&str>) -> UpdatedObject {
    let enc = match encoding {
        Some(name) => format!(" /Encoding /{name}"),
        None => String::new(),
    };
    UpdatedObject::new(
        id,
        format!(
            "{id} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /{base_font}{enc} >>\nendobj\n"
        )
        .into_bytes(),
    )
}

fn content_stream_object(id: u32, content: &[u8]) -> UpdatedObject {
    let mut bytes = format!("{id} 0 obj\n<< /Length {} >>\nstream\n", content.len()).into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    UpdatedObject::new(id, bytes)
}

/// Append `v` as a compact `%.2f` float, stripping trailing zeros and dot.
fn push_f32(out: &mut Vec<u8>, v: f32) {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    out.extend_from_slice(s.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::Document as PdfDoc;
    use quillmark_pdf::CHECKBOX_ON_STATE;

    /// Single-page US-Letter background, no AcroForm and no annots.
    const BASE: &[u8] =
        include_bytes!("../../../fixtures/resources/quills/sample_form/0.1.0/form.pdf");

    fn text_field(name: &str, value: &str) -> FieldSpec {
        FieldSpec::new(
            name.to_string(),
            0,
            [72.0, 700.0, 300.0, 720.0],
            FieldType::Text { multiline: false },
        )
        .with_schema_field(name.to_string())
        .with_value(value.to_string())
    }

    fn checkbox_field(name: &str, checked: bool) -> FieldSpec {
        let mut spec = FieldSpec::new(
            name.to_string(),
            0,
            [72.0, 660.0, 90.0, 678.0],
            FieldType::Checkbox,
        )
        .with_schema_field(name.to_string());
        spec.value = checked.then(|| CHECKBOX_ON_STATE.to_string());
        spec
    }

    fn flatten_ok(fields: &[FieldSpec]) -> Vec<u8> {
        flatten(BASE.to_vec(), fields).expect("flatten succeeds")
    }

    fn contains_window(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn flatten_produces_no_acroform() {
        let pdf = flatten_ok(&[text_field("FullName", "Ada Lovelace")]);

        let doc = PdfDoc::load_mem(&pdf).expect("lopdf reparse: structurally valid");
        let cat = doc.catalog().expect("catalog");
        assert!(
            cat.get(b"AcroForm").is_err(),
            "flat PDF must not contain /AcroForm"
        );
    }

    #[test]
    fn flatten_text_font_declares_winansi_encoding() {
        let pdf = flatten_ok(&[text_field("FullName", "Ada Lovelace")]);
        assert!(
            String::from_utf8_lossy(&pdf).contains("/Encoding /WinAnsiEncoding"),
            "text font must declare WinAnsiEncoding"
        );
    }

    #[test]
    fn flatten_clips_to_field_box() {
        let pdf = flatten_ok(&[text_field("FullName", "Ada Lovelace")]);
        let text = String::from_utf8_lossy(&pdf);
        assert!(
            text.contains(" re W n"),
            "text must clip to the field box (re W n)"
        );
    }

    #[test]
    fn build_content_stream_transcodes_non_ascii_to_winansi() {
        let value = "Caf\u{e9} \u{2014} Se\u{f1}or \u{2019}A\u{2019}";
        let spec = text_field("FullName", value);
        let stream = build_content_stream(&[&spec]);

        // WinAnsi: é→0xE9, —→0x97, ñ→0xF1, ’→0x92.
        let want: &[u8] = &[
            b'C', b'a', b'f', 0xE9, b' ', 0x97, b' ', b'S', b'e', 0xF1, b'o', b'r', b' ', 0x92,
            b'A', 0x92,
        ];
        assert!(
            contains_window(&stream, want),
            "content stream must carry the WinAnsi-encoded value bytes"
        );
        assert!(
            !contains_window(&stream, &[b'f', 0xC3, 0xA9, b' ']),
            "value must not be drawn as raw UTF-8"
        );

        let pdf = flatten_ok(&[spec]);
        assert!(
            contains_window(&pdf, want),
            "flat PDF must carry the WinAnsi-encoded value bytes"
        );
    }

    #[test]
    fn flatten_checked_checkbox_emits_zapfdingbats_glyph() {
        let spec = checkbox_field("Agree", true);
        let stream = build_content_stream(&[&spec]);
        let text = String::from_utf8_lossy(&stream);

        assert!(
            text.contains("/ZaDb"),
            "checked checkbox must select the ZapfDingbats font (/ZaDb)"
        );
        assert!(
            contains_window(&stream, b"(4) Tj"),
            "checked checkbox must draw the check glyph"
        );

        let pdf = flatten_ok(&[spec]);
        assert!(
            String::from_utf8_lossy(&pdf).contains("/ZapfDingbats"),
            "flat PDF must declare the ZapfDingbats font"
        );

        assert!(has_drawable_value(&checkbox_field("Agree", true)));
        assert!(!has_drawable_value(&checkbox_field("Agree", false)));
        let unchecked = flatten_ok(&[checkbox_field("Agree", false)]);
        assert!(
            !contains_window(&unchecked, b"(4) Tj"),
            "unchecked checkbox must not draw the check glyph"
        );
    }
}
