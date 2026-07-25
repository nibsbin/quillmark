//! AcroForm readers shared by the backend's acceptance tests, which reparse
//! rendered output with lopdf and assert on it.

// Each integration test binary compiles this module and uses part of it.
#![allow(dead_code)]

use lopdf::Document as PdfDoc;

/// Decode a PDF text string: UTF-16BE when it carries a BOM (pdf-writer picks
/// this for values with characters outside the literal-safe set, e.g. a
/// newline in a multiline field), else treat the bytes as Latin-1/ASCII.
pub fn decode_pdf_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// The `/AcroForm` widget named `name`, found by scanning `/Fields` on `/T`.
pub fn widget<'a>(doc: &'a PdfDoc, af: &lopdf::Dictionary, name: &str) -> &'a lopdf::Dictionary {
    for f in af.get(b"Fields").unwrap().as_array().unwrap() {
        let w = doc
            .get_object(f.as_reference().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        if w.get(b"T").unwrap().as_str().unwrap() == name.as_bytes() {
            return w;
        }
    }
    panic!("no field named {name}");
}
