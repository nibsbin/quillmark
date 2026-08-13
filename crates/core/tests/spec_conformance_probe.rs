//! Spec requirements from `prose/references/markdown-spec.md` that have no
//! owner among the `document/tests/` unit modules.
//!
//! What those modules do not carry: the §8 input caps (sole coverage in the
//! workspace) and the `normalize_document` pass.

use quillmark_core::normalize::normalize_document;
use quillmark_core::Document;

#[test]
fn normalize_reaches_card_body() {
    let md = "~~~card-yaml\n$quill: t\n$kind: main\n~~~\n\n~~~card-yaml\n$kind: x\n~~~\n\n<!-- c -->trailing\u{202D}text";
    let doc = Document::parse(md).unwrap().document;
    let doc = normalize_document(doc).unwrap();
    let body = doc.cards()[0].body_markdown();
    assert!(
        body.contains("trailingtext"),
        "card body missing bidi-strip, got: {:?}",
        body
    );
}

#[test]
fn unclosed_code_block_emits_warning() {
    let md = "~~~card-yaml\n$quill: t\n$kind: main\n~~~\n\n```\ncode line\n\n~~~card-yaml\n$kind: x\n~~~\n\ntrailing body";
    let out = Document::parse(md).unwrap();
    assert!(
        out.warnings
            .iter()
            .any(|w| w.code.as_deref() == Some("parse::unclosed_code_block")),
        "expected unclosed-code-block warning, got: {:?}",
        out.warnings
            .iter()
            .map(|w| (w.code.clone(), w.message.clone()))
            .collect::<Vec<_>>()
    );
    assert!(
        out.document.cards().is_empty(),
        "shielded card block must not have been parsed"
    );
}

#[test]
fn per_block_field_count_cap() {
    let mut s = String::from("~~~card-yaml\n$quill: t\n$kind: main\n");
    for i in 0..1001 {
        s.push_str(&format!("f{}: v\n", i));
    }
    s.push_str("~~~\n\nBody.");
    let err = Document::parse(&s).unwrap_err().to_string();
    assert!(err.contains("Input too large"), "got: {}", err);
}

#[test]
fn card_count_cap_is_per_card() {
    let mut s = String::from("~~~card-yaml\n$quill: t\n$kind: main\n~~~\n");
    for _ in 0..1001 {
        s.push_str("\n~~~card-yaml\n$kind: x\n~~~\n\nB.\n");
    }
    let err = Document::parse(&s).unwrap_err().to_string();
    assert!(err.contains("Input too large"), "got: {}", err);
}

/// `docs/integration/operations.md` tells an integrator to read the caps rather
/// than copy the numbers, so what this pins is the paths, not the values. Written
/// out-of-crate, as a consumer writes them.
#[test]
fn spec_caps_are_reachable_at_their_documented_paths() {
    use quillmark_core::document::limits::MAX_YAML_DEPTH;
    use quillmark_core::error::{
        MAX_CARD_COUNT, MAX_FIELD_COUNT, MAX_INPUT_SIZE, MAX_YAML_SIZE,
    };

    for cap in [
        MAX_INPUT_SIZE,
        MAX_YAML_SIZE,
        MAX_CARD_COUNT,
        MAX_FIELD_COUNT,
        MAX_YAML_DEPTH,
    ] {
        assert!(cap > 0);
    }
}
