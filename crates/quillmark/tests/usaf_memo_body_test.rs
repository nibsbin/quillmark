//! Heading survival through `usaf_memo`'s paragraph rebuild.
//!
//! The memo's `render-body` buffers a heading and prepends it to the next
//! element as `strong[…]`: the AFH run-in style, deliberate. The invariant the
//! buffer must hold is that every heading reaches the page exactly once, in its
//! own place. Three shapes press on it — a heading with nothing after it, a
//! heading whose next element opens a *different* list item, and a heading
//! following a heading — because each is a heading the next iteration does not
//! consume.
//!
//! The oracle is `regions()`, not output size: a region's `span` is the content
//! range its glyphs came from, so text that does not reach the page has no span
//! covering it. Byte-length only says "something changed".

#![cfg(feature = "typst")]

use quillmark::{Document, Quillmark, RenderOptions};
use quillmark_fixtures::quills_path;

/// The `$body` regions for a memo whose body is `body`, as (content span, top
/// edge) pairs sorted by content order.
fn body_regions(body: &str) -> Vec<([usize; 2], f32)> {
    let markdown = format!("~~~card-yaml\n$quill: usaf_memo\n$kind: main\n~~~\n\n{body}\n");
    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo loads");
    let parsed = Document::parse(&markdown).expect("parses").document;
    let session = engine.open(&quill, &parsed).expect("opens");
    let mut regions: Vec<_> = session
        .regions()
        .iter()
        .filter(|r| r.field == "$body")
        .filter_map(|r| r.span.map(|s| (s, r.rect[1])))
        .collect();
    regions.sort_by_key(|(span, _)| *span);
    regions
}

fn spans(body: &str) -> Vec<[usize; 2]> {
    body_regions(body).into_iter().map(|(s, _)| s).collect()
}

/// A heading with nothing after it to run into: the buffer holds content no
/// later iteration consumes, so it must be drained after the loop rather than
/// dying with it. Undrained, the loss is total, the list item's bullet included.
#[test]
fn a_trailing_heading_reaches_the_page() {
    // "one" then "Trailing", the second on its own line.
    assert_eq!(spans("one\n\n# Trailing"), [[0, 3], [4, 12]]);
    assert_eq!(spans("- one\n- # Absorbed"), [[0, 3], [4, 12]]);
    // The degenerate case: a body that is nothing but a heading.
    assert_eq!(spans("# Only"), [[0, 4]]);
}

/// Two headings in a row. The buffer holds one heading, so the second must
/// flush the first rather than overwrite it.
#[test]
fn consecutive_headings_both_reach_the_page() {
    assert_eq!(spans("# First\n\n# Second\n\ntext"), [[0, 5], [6, 12], [13, 17]]);
}

/// Whether every `$body` region sits on one line. A run-in *is* the heading and
/// the text it joined sharing a line, so this is the property to measure;
/// coverage cannot see it (the text inks either way) and exact spans cannot
/// either (a run-in surfaces as one merged region or two adjacent ones
/// depending on the shape). The tolerance separates a bold/regular baseline
/// difference (hundredths of a point) from a paragraph gap (tens).
fn on_one_line(body: &str) -> bool {
    let tops: Vec<f32> = body_regions(body).into_iter().map(|(_, y)| y).collect();
    let (lo, hi) = tops.iter().fold((f32::MAX, f32::MIN), |(l, h), y| (l.min(*y), h.max(*y)));
    hi - lo < 2.0
}

/// A heading whose next element opens a *different* list item. Coverage cannot
/// see this one: the text inks whether or not it lands in the right item. Place
/// is the tell — prepended to the next item's own content, a heading shares that
/// item's line.
#[test]
fn a_heading_does_not_cross_into_the_next_item() {
    let regions = body_regions("- # Absorbed\n- second");
    assert_eq!(
        regions.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
        [[0, 8], [9, 15]],
        "both texts ink"
    );
    assert!(
        !on_one_line("- # Absorbed\n- second"),
        "the heading holds its own line rather than joining the next item: {regions:?}"
    );
}

/// The run-in itself is the point of the rebuild and is unchanged: a heading
/// joins the next block *of its own item*, at top level and inside a list item
/// alike.
#[test]
fn the_run_in_style_still_runs_in() {
    assert!(on_one_line("- # Absorbed\n  item text"));
    assert!(on_one_line("# Absorbed\n\nitem text"));
}

/// The common path is untouched: standalone emission is the branch a heading
/// before a table already takes, and the seeded document renders through it.
#[test]
fn the_seeded_memo_still_renders() {
    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo loads");
    let parsed = quill.seed_document();
    engine
        .render(&quill, &parsed, &RenderOptions::default())
        .expect("the seeded memo renders");
}

/// The memo declares the one construct it does not typeset. An AFH 33-337 memo
/// has no dividers, and `render-body` draws none at any depth, so a rule leaves
/// the page unmarked and the walk is the only place that says so.
#[test]
fn a_rule_in_the_memo_body_warns() {
    let quill = quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo loads");
    let markdown = "~~~card-yaml\n$quill: usaf_memo\n$kind: main\n~~~\n\none\n\n***\n\ntwo\n";
    let warnings: Vec<_> = quill
        .parse(markdown)
        .expect("parses and conforms")
        .warnings
        .iter()
        .filter(|d| d.code.as_deref() == Some(quillmark_core::quill::UNSUPPORTED_CONSTRUCT))
        .map(|d| (d.path.clone(), d.args["construct"].as_str().unwrap().to_string()))
        .collect();
    assert_eq!(
        warnings,
        [(Some("main.body".to_string()), "rule".to_string())]
    );
    // Everything else the memo renders, so a body without a rule is silent.
    let clean = "~~~card-yaml\n$quill: usaf_memo\n$kind: main\n~~~\n\n# H\n\n- a\n- b\n";
    assert!(quill
        .parse(clean)
        .expect("parses")
        .warnings
        .iter()
        .all(|d| d.code.as_deref() != Some(quillmark_core::quill::UNSUPPORTED_CONSTRUCT)));
}
