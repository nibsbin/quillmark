//! `usaf_memo`'s CUI block as an enum variant of `classification`.
//!
//! The migration moved four flat fields under `classification.variants.CUI` and
//! dropped the `default: ""` that made `cui_controlled_by` and `cui_poc`
//! unaskable. Both halves of the contract are asserted here: the plate still
//! receives every CUI field unconditionally (`plate.typ` reads `data.cui_*`
//! with no guard), and the obligation the statute states — DoDM 5200.48 —
//! now fires as a diagnostic instead of living in `description:` prose.

use quillmark_fixtures::quills_path;

fn memo(classification: &str, extra: &str) -> String {
    format!(
        "~~~card-yaml
$quill: usaf_memo@0.2.0
$kind: main
letterhead_title: DEPARTMENT OF THE AIR FORCE
letterhead_caption: [123D EXAMPLE WING]
memo_for: [SOME/CC]
subject: A memo
classification: {classification}
{extra}signature_block: [A. AUTHOR, Captain, USAF, Flight Commander]
~~~

Body prose.
"
    )
}

fn quill() -> quillmark::Quill {
    quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo should load")
}

fn must_fill_paths(quill: &quillmark::Quill, md: &str) -> Vec<String> {
    let doc = quillmark::Document::parse(md).expect("parse").document;
    let mut paths: Vec<String> = quill
        .validate(&doc)
        .iter()
        .filter(|d| d.code.as_deref() == Some("validation::must_fill"))
        .filter_map(|d| d.path.clone())
        .filter(|p| p.starts_with("main.cui_"))
        .collect();
    paths.sort();
    paths
}

/// The floor stays total: a variant field is declared, blank-filled, and
/// present in the plate projection whatever the discriminant reads. `plate.typ`
/// reads `data.cui_controlled_by` unguarded, so anything less fails the compile.
#[test]
fn every_cui_field_reaches_the_plate_whatever_the_classification() {
    let quill = quill();
    for classification in ["\"\"", "UNCLASSIFIED", "CUI", "SECRET"] {
        let doc = quillmark::Document::parse(&memo(classification, ""))
            .expect("parse")
            .document;
        let plate = quill.compile_data(&doc).expect("blank-filled render is total");
        for field in [
            "cui_controlled_by",
            "cui_poc",
            "cui_category",
            "cui_limited_dissemination",
        ] {
            assert_eq!(
                plate[field], "",
                "{field} must reach the plate under classification {classification}"
            );
        }
    }
}

/// The statutory obligation, as a diagnostic. Unclassified asks for nothing;
/// flipping the one cell to CUI asks for exactly the two fields DoDM 5200.48
/// requires, and leaves the two optional ones alone.
#[test]
fn cui_obliges_controlled_by_and_poc_only_under_cui() {
    let quill = quill();

    assert!(
        must_fill_paths(&quill, &memo("UNCLASSIFIED", "")).is_empty(),
        "an unclassified memo owes no CUI answer"
    );
    assert_eq!(
        must_fill_paths(&quill, &memo("CUI", "")),
        ["main.cui_controlled_by", "main.cui_poc"],
        "CUI obliges the two fields the statute names, and only those"
    );
    assert!(
        must_fill_paths(
            &quill,
            &memo("CUI", "cui_controlled_by: SAF/AA\ncui_poc: Capt J. Smith, DSN 555-1234\n")
        )
        .is_empty(),
        "authoring both discharges the obligation"
    );
}

/// Stranded data warns and survives. An editor flipping the discriminant back
/// to UNCLASSIFIED must not lose the author's answers, and must not be handed
/// an invalid document either.
#[test]
fn a_cui_answer_on_an_unclassified_memo_warns_without_blocking() {
    let quill = quill();
    let md = memo("UNCLASSIFIED", "cui_poc: Capt J. Smith, DSN 555-1234\n");
    let doc = quillmark::Document::parse(&md).expect("parse").document;

    let diags = quill.validate(&doc);
    let stranded: Vec<_> = diags
        .iter()
        .filter(|d| d.code.as_deref() == Some("validation::out_of_variant"))
        .collect();
    assert_eq!(stranded.len(), 1, "got: {diags:?}");
    assert_eq!(stranded[0].path.as_deref(), Some("main.cui_poc"));
    assert_eq!(stranded[0].severity, quillmark::Severity::Warning);

    assert!(
        diags
            .iter()
            .all(|d| d.severity != quillmark::Severity::Error),
        "stranded data is never a blocker: {diags:?}"
    );
    let plate = quill.compile_data(&doc).expect("still renders");
    assert_eq!(
        plate["cui_poc"], "Capt J. Smith, DSN 555-1234",
        "the value is carried, not dropped"
    );
}
