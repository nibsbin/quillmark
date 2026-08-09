//! The AcroForm stamp spine's byte-level reads: arbitrary and corrupted PDF
//! bytes yield `Err`, never a panic.
//!
//! `quillmark-pdf` parses PDF by hand — object lookup by scanning, dictionary
//! splicing, xref and trailer reads — over `&[u8]` the caller supplies. Safe
//! Rust bounds the damage to a panic, and a panic is still the worst outcome
//! the workspace has: the CLI and the Python extension die on it, and the WASM
//! module is left poisoned, since nothing anywhere catches unwind.
//!
//! The oracle is deliberately weak: **no panic, and a refusal is an
//! acceptable answer.** The reader's input contract (traditional-xref,
//! unencrypted, inline-annots, flat-tree) means most well-formed PDFs are
//! refused too, so "returns `Ok`" is not a property any of this can assert.
//!
//! Two input populations, because they fail differently. Arbitrary bytes almost
//! never reach past the trailer scan; a real form with one byte changed gets
//! deep into object parsing carrying a length, offset, or delimiter that lies.

use std::sync::LazyLock;

use proptest::prelude::*;
use quillmark_pdf::{page_media_boxes, stamp, FieldSpec, FieldType, PdfUpdate, StampOptions};

/// `sample_form`'s `form.pdf`: a real AcroForm the spine accepts, so a mutant of
/// it exercises the parse paths a random buffer never reaches.
///
/// Read once for the module, not once per case: every case below mutates its
/// own copy, and thousands of cases run against it.
static BASE_PDF: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let path = quillmark_fixtures::quills_path("sample_form").join("form.pdf");
    std::fs::read(&path).expect("the sample_form fixture ships a form.pdf")
});

fn base_pdf() -> Vec<u8> {
    BASE_PDF.clone()
}

/// One field of each `FieldType`, so `stamp` walks every widget writer.
fn every_field_kind() -> Vec<FieldSpec> {
    vec![
        FieldSpec::new("t".into(), 0, [10.0, 10.0, 90.0, 30.0], FieldType::Text {
            multiline: false,
        }),
        FieldSpec::new("c".into(), 0, [10.0, 40.0, 30.0, 60.0], FieldType::Checkbox),
        FieldSpec::new("s".into(), 0, [10.0, 70.0, 90.0, 90.0], FieldType::Signature),
        FieldSpec::new("h".into(), 0, [10.0, 100.0, 90.0, 120.0], FieldType::Choice {
            options: vec!["a".into(), "b".into()],
        }),
    ]
}

/// Drive every byte-taking entry point once. Each returns a `Result`; the call
/// completing at all is the property.
fn exercise(pdf: &[u8]) {
    let _ = page_media_boxes(pdf);
    let _ = PdfUpdate::begin(pdf, None);
    let _ = PdfUpdate::begin(pdf, Some("quillmark-fuzz"));
    let _ = stamp(pdf.to_vec(), &[], &StampOptions::default());
    let _ = stamp(
        pdf.to_vec(),
        &every_field_kind(),
        &StampOptions::default().with_producer("quillmark-fuzz".into()),
    );
}

proptest! {
    // Above proptest's default 256: a case is one parse over at most a few
    // kilobytes with no oracle to evaluate and no I/O, so mutants are cheap
    // enough that the wider net costs less than the coverage is worth.
    #![proptest_config(ProptestConfig::with_cases(1024))]

    /// Arbitrary bytes. Nothing checks for a `%PDF-` header — `PdfUpdate::begin`
    /// scans backwards for `startxref` — so a buffer that opens like a PDF takes
    /// the same path as one that does not, and this covers both.
    #[test]
    fn fuzz_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        exercise(&bytes);
    }

    /// A real form truncated at an arbitrary point: every length and offset the
    /// file declares now points past the end.
    ///
    /// The range is the fixture's own length. A fixed upper bound would sample
    /// mostly past it and re-run the intact file.
    #[test]
    fn fuzz_truncated_form(cut in any::<prop::sample::Index>()) {
        let pdf = base_pdf();
        let cut = cut.index(pdf.len() + 1);
        exercise(&pdf[..cut]);
    }

    /// A real form with one byte overwritten. Hits the cases a truncation
    /// cannot: a corrupted xref offset, a `/Length` that overshoots, an
    /// unbalanced dictionary delimiter, a broken object header.
    #[test]
    fn fuzz_single_byte_corruption(at in any::<prop::sample::Index>(), to in any::<u8>()) {
        let mut pdf = base_pdf();
        let i = at.index(pdf.len());
        pdf[i] = to;
        exercise(&pdf);
    }

    /// A real form with a run of bytes overwritten, which can take out a whole
    /// keyword (`trailer`, `startxref`, `endobj`) rather than one character.
    #[test]
    fn fuzz_spliced_run(
        at in any::<prop::sample::Index>(),
        run in proptest::collection::vec(any::<u8>(), 1..64),
    ) {
        let mut pdf = base_pdf();
        let start = at.index(pdf.len());
        let end = (start + run.len()).min(pdf.len());
        pdf[start..end].copy_from_slice(&run[..end - start]);
        exercise(&pdf);
    }

    /// Field geometry the caller controls. `quillmark-pdf`'s unit tests pin the
    /// individual refusals (non-finite rect, corner ordering, missing page);
    /// what this adds is the combinations, and a page index drawn past the
    /// fixture's single page, since `stamp` resolves pages by index into a
    /// `Vec` and must refuse rather than index.
    #[test]
    fn fuzz_field_geometry(
        page in 0usize..8,
        x0 in prop::num::f32::ANY,
        y0 in prop::num::f32::ANY,
        x1 in prop::num::f32::ANY,
        y1 in prop::num::f32::ANY,
        name in "\\PC{0,32}",
    ) {
        let field = FieldSpec::new(name, page, [x0, y0, x1, y1], FieldType::Checkbox);
        let _ = stamp(base_pdf(), &[field], &StampOptions::default());
    }
}
