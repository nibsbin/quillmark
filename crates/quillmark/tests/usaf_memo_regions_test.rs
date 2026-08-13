//! Region coverage on the flagship `usaf_memo` quill, whose package rebuilds
//! body paragraphs through a state buffer (AFH 33-337 auto-numbering): span
//! tracking rides the rebuilt glyphs' own origins, so bodies stay addressable
//! with no recovery step in the plate.

#![cfg(feature = "typst")]

use std::collections::HashSet;

use quillmark::{OutputFormat, Quillmark, RenderOptions};
use quillmark_fixtures::quills_path;

#[test]
fn usaf_memo_regions_cover_body_signature_and_cards() {
    let engine = Quillmark::new();
    let quill =
        quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo should load");

    // One card per declared kind, so the indorsement addresses are present.
    let parsed = quill.seed_document();

    let mut session = engine
        .open(&quill, &parsed)
        .expect("usaf_memo should open a session");

    let regions = session.regions();
    let fields: HashSet<&str> = regions.iter().map(|r| r.field.as_str()).collect();

    for expected in [
        "$body",
        "signature_block",
        "$cards.indorsement.0.signature_block",
        // Card scalars render through their generated `display` closure, so
        // they region per instance even though the plate hands them to a
        // vendored package through the shared loop variable.
        "$cards.indorsement.0.from",
        "$cards.indorsement.0.for",
    ] {
        assert!(
            fields.contains(expected),
            "expected a region keyed {expected:?}; got {fields:?}"
        );
    }
    assert!(
        !fields.contains("$cards.indorsement.0.$body"),
        "an empty card body draws nothing and surfaces no region: {fields:?}"
    );

    // Plate space keys an array element `<field>.<i>`; a binding hands out the
    // bracketed `DocPath` index instead.
    assert!(
        fields.contains("references.0"),
        "each `references` element regions on its own address: {fields:?}"
    );
    let kinds: Vec<Option<&str>> = parsed.cards().iter().map(|c| c.kind()).collect();
    let translated: HashSet<String> =
        quillmark_core::regions_to_doc_path(regions.clone(), &kinds)
            .into_iter()
            .map(|r| r.field)
            .collect();
    assert!(
        translated.contains("main.references[0]"),
        "the element address crosses as a bracketed DocPath index: {translated:?}"
    );

    let body = regions
        .iter()
        .find(|r| r.field == "$body")
        .expect("$body region present");
    let cx = (body.rect[0] + body.rect[2]) / 2.0;
    let cy = (body.rect[1] + body.rect[3]) / 2.0;
    assert_eq!(
        session.field_at(body.page, cx, cy).as_deref(),
        Some("$body"),
        "a click inside the rebuilt body routes to $body"
    );

    let from = regions
        .iter()
        .find(|r| r.field == "$cards.indorsement.0.from")
        .expect("the indorsement's FROM cell regions");
    let cx = (from.rect[0] + from.rect[2]) / 2.0;
    let cy = (from.rect[1] + from.rect[3]) / 2.0;
    assert_eq!(
        session.field_at(from.page, cx, cy).as_deref(),
        Some("$cards.indorsement.0.from"),
        "a click on the indorsement's FROM line routes to that card's own field"
    );

    let mut edited = parsed.clone();
    quillmark::TypedWriter::new(quill.config(), &mut edited)
        .card(0)
        .expect("the indorsement card")
        .revise_body("The indorsement **body**, rebuilt by render-body.")
        .expect("set the card body");
    session.update(&edited).expect("apply edited card body");
    let fields: HashSet<String> = session.regions().into_iter().map(|r| r.field).collect();
    assert!(
        fields.contains("$cards.indorsement.0.$body"),
        "a non-empty card body regions through the rebuild: {fields:?}"
    );

    let with_regions = engine
        .render(
            &quill,
            &parsed,
            &RenderOptions::default().with_output_format(OutputFormat::Pdf).with_regions(true),
        )
        .expect("usaf_memo should render to PDF");
    assert_eq!(
        with_regions.regions, regions,
        "one-shot sidecar matches the session query"
    );

    let without_regions = engine
        .render(
            &quill,
            &parsed,
            &RenderOptions::default().with_output_format(OutputFormat::Pdf),
        )
        .expect("usaf_memo should render to PDF");
    assert!(
        without_regions.regions.is_empty(),
        "the sidecar is opt-in; exports carry no regions by default"
    );
}

/// The laundered shape: the vendored package, not the plate, inks the date
/// deep inside `utils.typ`'s `display-date`. The value-object's `display`
/// closure is born in the generated helper, so its glyphs carry a helper span
/// that resolves to the recorded window wherever the package calls it.
#[test]
fn usaf_memo_date_region_rides_the_vendored_display() {
    let engine = Quillmark::new();
    let quill =
        quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo should load");
    let parsed = quill.seed_document();
    let mut session = engine.open(&quill, &parsed).expect("open a session");

    // The seed leaves the date blank: a native `today()` fallback inks no field
    // region, so commit a real date first.
    let mut edited = parsed.clone();
    quillmark::TypedWriter::new(quill.config(), &mut edited)
        .set("date", "2026-01-02")
        .expect("set a real date");
    session.update(&edited).expect("apply a real memo date");

    let regions = session.regions();
    let date = regions
        .iter()
        .find(|r| r.field == "date")
        .unwrap_or_else(|| panic!("a real memo date must surface a `date` region: {regions:?}"));
    assert!(
        date.rect[2] > date.rect[0] && date.rect[3] > date.rect[1],
        "the date region has positive area: {:?}",
        date.rect
    );
    let cx = (date.rect[0] + date.rect[2]) / 2.0;
    let cy = (date.rect[1] + date.rect[3]) / 2.0;
    assert_eq!(
        session.field_at(date.page, cx, cy).as_deref(),
        Some("date"),
        "a click on the vendored-placed memo date routes to its schema path"
    );
}
