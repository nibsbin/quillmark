//! Spec requirements from `prose/references/markdown-spec.md` that have no
//! owner among the `document/tests/` unit modules.
//!
//! `Document::parse` is a pass-through to `assemble::decompose_with_warnings`,
//! the same path those modules drive, so a probe here proves nothing extra
//! about the grammar. What this file carries is what they do not: the §8 input
//! caps (sole coverage in the workspace) and the `normalize_document` pass.

use quillmark_core::normalize::normalize_document;
use quillmark_core::Document;

// YAML `#` comment lines inside a block are accepted as ordinary YAML.
#[test]
fn yaml_comment_banners_inside_block_are_accepted() {
    let md = "~~~card-yaml\n$quill: t\n$kind: main\n# Essential\ntitle: T\n~~~\n\nBody.";
    let doc = Document::parse(md).unwrap().document;
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "T"
    );
}

// Card body normalization reaches nested cards.
#[test]
fn normalize_reaches_card_body() {
    let md = "~~~card-yaml\n$quill: t\n$kind: main\n~~~\n\n~~~card-yaml\n$kind: x\n~~~\n\n<!-- c -->trailing\u{202D}text";
    let doc = Document::parse(md).unwrap().document;
    let doc = normalize_document(doc).unwrap();
    let body = doc.cards()[0].body_markdown();
    // Bidi-strip normalization reaches the nested card body at import
    // (`trailing\u{202D}text` → `trailingtext`). The HTML comment is not
    // representable in the content and is dropped by the projection.
    assert!(
        body.contains("trailingtext"),
        "card body missing bidi-strip, got: {:?}",
        body
    );
}

// CRLF normalization reaches card bodies.
#[test]
fn card_body_crlf_line_endings_are_normalized() {
    let md = "~~~card-yaml\n$quill: t\n$kind: main\n~~~\n\n~~~card-yaml\n$kind: x\n~~~\n\nCard line one.\r\nCard line two.\r\n";
    let doc = Document::parse(md).unwrap().document;
    let doc = normalize_document(doc).unwrap();
    let body = doc.cards()[0].body_markdown();
    assert!(
        !body.contains('\r'),
        "card body must not contain bare \\r after normalization, got: {:?}",
        body
    );
}

// Unclosed fenced code block at end-of-document emits a warning.
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
    // And the shielded card block must NOT have registered.
    assert!(
        out.document.cards().is_empty(),
        "shielded card block must not have been parsed"
    );
}

// Per-block field-count cap.
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

// Card count cap counts cards only.
#[test]
fn card_count_cap_is_per_card() {
    let mut s = String::from("~~~card-yaml\n$quill: t\n$kind: main\n~~~\n");
    for _ in 0..1001 {
        s.push_str("\n~~~card-yaml\n$kind: x\n~~~\n\nB.\n");
    }
    let err = Document::parse(&s).unwrap_err().to_string();
    assert!(err.contains("Input too large"), "got: {}", err);
}
