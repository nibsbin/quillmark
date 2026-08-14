//! The three cross-field constraints `usaf_memo` used to state only in prose.
//!
//! Issue #1202's evidence, pinned against the flagship fixture rather than a
//! synthetic schema: a `classification: CUI` memo with no controlling office,
//! and a `SEE DISTRIBUTION` memo with no distribution list, each validated
//! clean while breaking the quill's own stated rule. They now warn, and — the
//! other half of the contract — they still render.

use quillmark::Quillmark;
use quillmark_fixtures::quills_path;

/// A memo with every unconditional obligation discharged, so the only
/// diagnostics left are the conditional ones under test.
fn memo(classification: &str, memo_for: &str, extra: &str) -> String {
    format!(
        "~~~card-yaml\n\
         $quill: usaf_memo@0.3.0\n\
         $kind: main\n\
         letterhead_title: DEPARTMENT OF THE AIR FORCE\n\
         letterhead_caption: [123D EXAMPLE WING]\n\
         memo_for: [{memo_for}]\n\
         subject: A memo under test\n\
         classification: {classification}\n\
         signature_block: [A. AUTHOR, Captain, USAF, Flight Commander]\n\
         {extra}~~~\n\
         \n\
         Body prose.\n"
    )
}

/// Every `validation::must_fill` path, sorted.
fn obligations(md: &str) -> Vec<String> {
    let quill = quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo should load");
    let doc = quillmark::Document::parse(md).expect("parse").document;
    let mut paths: Vec<String> = quill
        .validate(&doc)
        .into_iter()
        .filter(|d| d.code.as_deref() == Some("validation::must_fill"))
        .filter_map(|d| d.path)
        .collect();
    paths.sort();
    paths
}

#[test]
fn an_unclassified_memo_owes_no_cui_block() {
    assert!(
        obligations(&memo("UNCLASSIFIED", "SOME/CC", "")).is_empty(),
        "the CUI rules must stay dormant outside CUI"
    );
}

/// The issue's headline: this document validated clean.
#[test]
fn a_cui_memo_without_its_cui_block_now_warns() {
    assert_eq!(
        obligations(&memo("CUI", "SOME/CC", "")),
        ["main.cui_controlled_by", "main.cui_poc"],
        "DoDM 5200.48 requires both, and both are now checked"
    );
}

#[test]
fn authoring_the_cui_block_discharges_both() {
    assert!(obligations(&memo(
        "CUI",
        "SOME/CC",
        "cui_controlled_by: SAF/AA\ncui_poc: Capt J. Smith, DSN 555-1234\n"
    ))
    .is_empty());
}

/// The value-dependent pairing: `SEE DISTRIBUTION` in `memo_for` obliges the
/// list it points at.
#[test]
fn see_distribution_obliges_the_distribution_list() {
    assert_eq!(
        obligations(&memo("UNCLASSIFIED", "SEE DISTRIBUTION", "")),
        ["main.distribution"]
    );
    assert!(
        obligations(&memo(
            "UNCLASSIFIED",
            "SEE DISTRIBUTION",
            "distribution: [ORG1/SYMBOL, ORG2/SYMBOL]\n"
        ))
        .is_empty(),
        "an authored list discharges it"
    );
}

/// Obligation is a warning, never a gate: the memo that breaks every rule still
/// produces a PDF.
#[cfg(feature = "typst")]
#[test]
fn a_memo_breaking_every_rule_still_renders() {
    use quillmark::{OutputFormat, RenderOptions};

    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo should load");
    let md = memo("CUI", "SEE DISTRIBUTION", "");
    let doc = quillmark::Document::parse(&md).expect("parse").document;

    assert_eq!(
        obligations(&md),
        [
            "main.cui_controlled_by",
            "main.cui_poc",
            "main.distribution"
        ],
        "all three rules fire at once"
    );

    let rendered = engine
        .render(
            &quill,
            &doc,
            &RenderOptions::default().with_output_format(OutputFormat::Pdf),
        )
        .expect("an outstanding obligation must never gate render");
    assert!(!rendered.artifacts[0].bytes.is_empty());
}
