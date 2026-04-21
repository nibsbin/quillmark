//! Independent spec conformance probes for prose/designs/MARKDOWN.md.
//!
//! These tests exercise concrete spec requirements that are most likely to
//! diverge between the parser and the written standard.

use quillmark_core::normalize::{normalize_document, normalize_fields};
use quillmark_core::{ParsedDocument, QuillValue};

// §4 F2 — Leading blank: a `---` line directly under non-blank text is not a fence.
#[test]
fn f2_fence_directly_under_paragraph_is_not_a_fence() {
    let md = "---\nQUILL: t\n---\n\nParagraph text.\n---\n\nAfter.";
    let doc = ParsedDocument::from_markdown(md).unwrap();
    let body = doc.body().unwrap();
    assert!(
        body.contains("Paragraph text.") && body.contains("---") && body.contains("After."),
        "stray `---` under paragraph must be left to CommonMark, body was: {:?}",
        body
    );
}

// §4 F1 — first fence must carry QUILL. A first fence with some other key must not
// be accepted silently.
#[test]
fn f1_first_fence_without_quill_is_rejected() {
    let md = "---\ntitle: X\n---\n\nBody.";
    let err = ParsedDocument::from_markdown(md).unwrap_err().to_string();
    assert!(err.contains("Missing required QUILL field"), "got: {}", err);
}

// §4 F1 — YAML `#` comment lines at the top of a fence are skipped when
// locating the sentinel. A banner comment above `QUILL:` must not trip F1.
#[test]
fn f1_yaml_comment_banners_above_sentinel_are_accepted() {
    let md = "---\n# Essential\n#===========\nQUILL: t\ntitle: T\n---\n\nBody.";
    let doc = ParsedDocument::from_markdown(md).unwrap();
    assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "T");
}

// §4.2 — near-miss detection also ignores `#` comment banners, so a fence
// whose first real key is a near-miss still warns rather than silently
// delegating.
#[test]
fn near_miss_sentinel_sees_past_comment_banners() {
    let md = "---\nQUILL: t\n---\n\nB.\n\n---\n# banner\nCard: oops\nname: X\n---\n\nTrailing.";
    let out = ParsedDocument::from_markdown_with_warnings(md).unwrap();
    assert!(
        out.warnings.iter().any(
            |w| w.message.contains("Near-miss metadata sentinel") && w.message.contains("Card")
        ),
        "expected near-miss warning even with leading comment, got: {:?}",
        out.warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
}

// §4.2 — Near-miss sentinel warning.
#[test]
fn near_miss_sentinel_emits_warning_and_delegates() {
    let md = "---\nQUILL: t\n---\n\nBody.\n\n---\nCard: oops\nname: X\n---\n\nTrailing.";
    let out = ParsedDocument::from_markdown_with_warnings(md).unwrap();
    assert!(
        out.warnings.iter().any(
            |w| w.message.contains("Near-miss metadata sentinel") && w.message.contains("Card")
        ),
        "expected near-miss warning, got: {:?}",
        out.warnings
            .iter()
            .map(|w| w.message.clone())
            .collect::<Vec<_>>()
    );
    // And the `Card:` fence must NOT have registered as a card.
    let cards = out.document.get_field("CARDS").unwrap().as_array().unwrap();
    assert!(
        cards.is_empty(),
        "near-miss CARD must be delegated, not registered"
    );
    // Body must contain the delegated content.
    assert!(out.document.body().unwrap().contains("Card: oops"));
}

// §3 — Trailing whitespace on the fence marker must be accepted.
#[test]
fn fence_marker_with_trailing_whitespace_is_accepted() {
    let md = "---  \nQUILL: t\ntitle: T\n---\t\n\nBody.";
    let doc = ParsedDocument::from_markdown(md).unwrap();
    assert_eq!(doc.get_field("title").unwrap().as_str().unwrap(), "T");
}

// §3 — `---` inside a fenced code block must be ignored.
#[test]
fn fences_inside_code_blocks_are_ignored() {
    let md = "---\nQUILL: t\n---\n\n```\n---\nCARD: x\n---\n```\n\nBody.";
    let doc = ParsedDocument::from_markdown(md).unwrap();
    let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
    assert!(cards.is_empty(), "fences inside code blocks must not parse");
}

// §3 — Reserved keys BODY/CARDS cannot be user-defined.
#[test]
fn reserved_keys_in_frontmatter_are_rejected() {
    for reserved in ["BODY", "CARDS"] {
        let md = format!("---\nQUILL: t\n{}: nope\n---\n\nBody.", reserved);
        let err = ParsedDocument::from_markdown(&md).unwrap_err().to_string();
        assert!(
            err.contains(&format!("Reserved field name '{}'", reserved)),
            "reserved key {} must error, got: {}",
            reserved,
            err
        );
    }
}

// §5 — CARDS is always present, even when empty.
#[test]
fn cards_is_always_present_even_when_empty() {
    let doc = ParsedDocument::from_markdown("---\nQUILL: t\n---\n\nBody.").unwrap();
    let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
    assert!(cards.is_empty());
}

// §5 — CARD value pattern.
#[test]
fn card_name_pattern_enforced() {
    let md = "---\nQUILL: t\n---\n\nB.\n\n---\nCARD: ITEMS\n---\n\nX.";
    let err = ParsedDocument::from_markdown(md).unwrap_err().to_string();
    assert!(err.contains("Invalid card field name"), "got: {}", err);
}

// §7 — Body bidi stripped.
#[test]
fn normalize_body_strips_bidi() {
    let mut f = std::collections::HashMap::new();
    f.insert(
        "BODY".to_string(),
        QuillValue::from_json(serde_json::json!("hi\u{202D}there")),
    );
    let out = normalize_fields(f);
    assert_eq!(out.get("BODY").unwrap().as_str().unwrap(), "hithere");
}

// §7 — YAML scalar bidi NOT stripped.
#[test]
fn normalize_yaml_scalar_keeps_bidi() {
    let mut f = std::collections::HashMap::new();
    f.insert(
        "title".to_string(),
        QuillValue::from_json(serde_json::json!("hi\u{202D}there")),
    );
    let out = normalize_fields(f);
    assert_eq!(
        out.get("title").unwrap().as_str().unwrap(),
        "hi\u{202D}there"
    );
}

// §7 — Card body normalization reaches nested cards.
#[test]
fn normalize_reaches_card_body() {
    let md = "---\nQUILL: t\n---\n\n---\nCARD: x\n---\n\n<!-- c -->trailing\u{202D}text";
    let doc = ParsedDocument::from_markdown(md).unwrap();
    let doc = normalize_document(doc).unwrap();
    let cards = doc.get_field("CARDS").unwrap().as_array().unwrap();
    let body = cards[0].get("BODY").unwrap().as_str().unwrap();
    assert!(
        body.contains("<!-- c -->\ntrailingtext"),
        "card body missing repair/bidi-strip, got: {:?}",
        body
    );
}

// §8 — Per-fence field-count cap.
#[test]
fn per_fence_field_count_cap() {
    let mut s = String::from("---\nQUILL: t\n");
    for i in 0..1001 {
        s.push_str(&format!("f{}: v\n", i));
    }
    s.push_str("---\n\nBody.");
    let err = ParsedDocument::from_markdown(&s).unwrap_err().to_string();
    assert!(err.contains("Input too large"), "got: {}", err);
}

// §8 — Card count cap counts cards only.
#[test]
fn card_count_cap_is_per_card() {
    let mut s = String::from("---\nQUILL: t\n---\n");
    for _ in 0..1001 {
        s.push_str("\n---\nCARD: x\n---\n\nB.\n");
    }
    let err = ParsedDocument::from_markdown(&s).unwrap_err().to_string();
    assert!(err.contains("Input too large"), "got: {}", err);
}
