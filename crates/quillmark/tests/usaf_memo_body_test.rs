//! Heading survival through `usaf_memo`'s paragraph rebuild.
//!
//! The memo's `render-body` buffers a heading and prepends it to the next
//! element as `strong[…]`: the AFH run-in style, deliberate. The buffer is one
//! variable, so three shapes used to lose their heading with no trace in the
//! render — a heading with nothing after it (the buffer died with the loop), a
//! heading whose next element began a *different* list item (its text was
//! delivered into that item), and a heading following a heading (the assignment
//! overwrote the earlier one).
//!
//! The oracle is `regions()`, not output size: a region's `span` is the content
//! range its glyphs came from, so text that never reached the page has no span
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

/// A heading with nothing after it to run into. Both shapes ended the buffer
/// holding content no later iteration consumed; the drop was total, taking the
/// list item's bullet with it.
#[test]
fn a_trailing_heading_reaches_the_page() {
    // "one" then "Trailing", the second on its own line.
    assert_eq!(spans("one\n\n# Trailing"), [[0, 3], [4, 12]]);
    assert_eq!(spans("- one\n- # Absorbed"), [[0, 3], [4, 12]]);
    // The degenerate case: a body that is nothing but a heading.
    assert_eq!(spans("# Only"), [[0, 4]]);
}

/// Two headings in a row. The buffer was assigned, not appended, so the first
/// heading was destroyed by the second; only the second's run-in survived.
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
/// see this one — the text inked either way — but its place could not be more
/// wrong: the heading's text was prepended to the *next* item's own content, so
/// the two shared a line.
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

/// The seeded document still renders: the repair reuses the standalone-emit
/// branch a heading before a table already took, so the common path is untouched.
#[test]
fn the_seeded_memo_still_renders() {
    let engine = Quillmark::new();
    let quill = quillmark::quill_from_path(quills_path("usaf_memo")).expect("usaf_memo loads");
    let parsed = quill.seed_document();
    engine
        .render(&quill, &parsed, &RenderOptions::default())
        .expect("the seeded memo renders");
}

/// The memo declares the one construct it genuinely does not typeset. An
/// AFH 33-337 memo has no dividers, and `render-body` typesets none at any
/// depth — silently, until the body says so on the pre-render walk.
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
