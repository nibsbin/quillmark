//! Tests for [`Quill::conform`] and [`Quill::parse`]: the resting-form
//! invariant, lane convergence, and the four exception states.

use serde_json::json;

use crate::document::StoredDocument;
use crate::quill::quill_from_yaml;
use crate::{Document, Quill, QuillValue, SeedOverlay};

const QUILL: &str = r#"
quill:
  name: conform_test
  version: "1.0"
  backend: typst
  description: Conform test
main:
  fields:
    subject:
      type: richtext
      inline: true
    note:
      type: plaintext
    qty:
      type: integer
    tags:
      type: array
      items:
        type: richtext
    meta:
      type: object
      properties:
        label:
          type: plaintext
        blurb:
          type: richtext
card_kinds:
  entry:
    fields:
      body:
        type: richtext
      caption:
        type: plaintext
"#;

fn quill() -> Quill {
    quill_from_yaml(QUILL)
}

/// Storage bytes: the hash a consumer keys a cache on.
fn bytes(doc: &Document) -> String {
    serde_json::to_string(&StoredDocument::from(doc.clone())).expect("storage DTO serializes")
}

fn parse_bound(quill: &Quill, md: &str) -> (Document, Vec<crate::Diagnostic>) {
    let parsed = quill.parse(md).expect("the bound door parses");
    (parsed.document, parsed.warnings)
}

const MD: &str = "\
~~~card-yaml
$quill: conform_test@1.0.0
subject: Q3 **results**
note: a *literal* line
qty: 3
tags:
  - one **bold**
  - two
meta:
  label: keep *this*
  blurb: and **this**
~~~

Main body.

~~~card-yaml
$kind: entry
body: card **body**
caption: raw *text*
~~~

Entry body.
";

/// The invariant, stated as an equality: whatever the lane, a content field
/// rests in the same bytes. The typed writer is the reference; the bound door
/// has to land there.
#[test]
fn parse_then_conform_equals_typed_write() {
    let quill = quill();
    let (conformed, warnings) = parse_bound(&quill, MD);
    assert!(warnings.is_empty(), "clean document: {warnings:?}");

    // The same values, committed through the typed writer onto a parsed-then-
    // stripped twin: build from the same markdown through the transport door and
    // re-commit every content field.
    let mut written = Document::parse(MD).expect("transport parse").document;
    {
        let mut w = quill.writer(&mut written);
        w.set("subject", "Q3 **results**").unwrap();
        w.set("note", "a *literal* line").unwrap();
        w.set("tags", json!(["one **bold**", "two"])).unwrap();
        w.set(
            "meta",
            json!({ "label": "keep *this*", "blurb": "and **this**" }),
        )
        .unwrap();
        let mut card = w.card(0).unwrap();
        card.set("body", "card **body**").unwrap();
        card.set("caption", "raw *text*").unwrap();
    }

    assert_eq!(conformed, written, "the two lanes must rest equal");
    assert_eq!(bytes(&conformed), bytes(&written), "and byte-equal");
}

/// Per-codec rest: `richtext` as the canonical content object, `plaintext` as
/// the literal string, at every depth.
#[test]
fn rest_is_per_codec_at_every_depth() {
    let quill = quill();
    let (doc, _) = parse_bound(&quill, MD);
    let payload = doc.main().payload();

    assert!(payload.get("subject").unwrap().as_json().is_object());
    assert_eq!(
        payload.get("note").unwrap().as_json(),
        &json!("a *literal* line"),
        "plaintext rests as the literal string, escapes and all"
    );
    // A non-content field keeps its authored shorthand: conform is not its
    // canonicalizer, the typed write is.
    assert_eq!(payload.get("qty").unwrap().as_json(), &json!(3));

    let tags = payload.get("tags").unwrap().as_json();
    assert!(tags.as_array().unwrap().iter().all(|e| e.is_object()));
    let meta = payload.get("meta").unwrap().as_json();
    assert_eq!(meta.get("label").unwrap(), &json!("keep *this*"));
    assert!(meta.get("blurb").unwrap().is_object());

    let card = &doc.cards()[0];
    assert!(card.payload().get("body").unwrap().as_json().is_object());
    assert_eq!(
        card.payload().get("caption").unwrap().as_json(),
        &json!("raw *text*")
    );
}

#[test]
fn conform_is_idempotent() {
    let quill = quill();
    let (mut doc, first) = parse_bound(&quill, MD);
    let before = bytes(&doc);
    let second = quill.conform(&mut doc).expect("conform");
    assert_eq!(bytes(&doc), before, "a second conform moves no bytes");
    assert_eq!(
        format!("{first:?}"),
        format!("{second:?}"),
        "and re-emits identical diagnostics"
    );
}

/// The no-op guard: an already-canonical document is untouched, comments
/// included. Without it every conform would clear `nested_comments` document-wide
/// and move bytes on a document nobody edited.
#[test]
fn conform_preserves_comments_and_untouched_bytes() {
    let quill = quill();
    let md = "\
~~~card-yaml
$quill: conform_test@1.0.0
# a leading comment
note: plain text
meta:
  # a nested comment
  label: inner
~~~

Body.
";
    let (mut doc, _) = parse_bound(&quill, md);
    let before = bytes(&doc);
    let markdown_before = doc.to_markdown();
    assert!(
        markdown_before.contains("# a nested comment"),
        "comments survive the first conform: {markdown_before}"
    );
    quill.conform(&mut doc).expect("conform");
    assert_eq!(bytes(&doc), before);
    assert_eq!(doc.to_markdown(), markdown_before);
}

/// A marker anywhere in the value is the state; the field stays as authored.
#[test]
fn fill_marked_fields_are_skipped() {
    let quill = quill();
    let md = "\
~~~card-yaml
$quill: conform_test@1.0.0
subject: !must_fill Q3 **results**
meta:
  label: keep *this*
  blurb: !must_fill and **this**
~~~

Body.
";
    let (mut doc, _) = parse_bound(&quill, md);
    let payload = doc.main().payload();
    assert!(
        payload.get("subject").unwrap().as_json().is_string(),
        "a root marker skips the field"
    );
    assert!(
        payload.get("meta").unwrap().as_json()["blurb"].is_string()
            && payload.get("meta").unwrap().as_json()["label"] == json!("keep *this*"),
        "a marker on one property skips the whole field, its clean siblings included"
    );
    assert!(doc.main().payload().is_fill("subject"));

    let before = bytes(&doc);
    quill.conform(&mut doc).expect("conform");
    assert_eq!(bytes(&doc), before, "and a repeat conform moves nothing");
}

/// Nothing conforms under the wrong schema: the check runs before any mutation.
#[test]
fn wrong_quill_errors_before_any_mutation() {
    let quill = quill();
    let md = "~~~card-yaml\n$quill: other_quill\nsubject: hi\n~~~\n\nBody.";
    let mut doc = Document::parse(md).expect("transport parse").document;
    let before = bytes(&doc);
    let err = quill.conform(&mut doc).expect_err("name mismatch errors");
    assert_eq!(
        err.diagnostics()[0].code.as_deref(),
        Some("quill::name_mismatch")
    );
    assert_eq!(bytes(&doc), before, "the document is untouched");
    assert!(quill.parse(md).is_err(), "and the bound parse fails too");
}

/// A value the strict write refuses rests authored, carries a `conform::*`
/// warning, and the document still opens, validates, and renders.
#[test]
fn non_conforming_value_rests_authored_with_a_diagnostic() {
    let quill = quill();
    let md = "~~~card-yaml\n$quill: conform_test@1.0.0\nsubject: 42\n~~~\n\nBody.";
    let (doc, warnings) = parse_bound(&quill, md);
    let diag = warnings
        .iter()
        .find(|d| d.code.as_deref().is_some_and(|c| c.starts_with("conform::")))
        .expect("a conform diagnostic");
    assert_eq!(diag.code.as_deref(), Some("conform::field_richtext_decode"));
    assert_eq!(diag.path.as_deref(), Some("main.subject"));
    assert_eq!(
        doc.main().payload().get("subject").unwrap().as_json(),
        &json!(42),
        "the value stays authored: no silent retype"
    );
    // The render floor still coerces it at the plate, so the document renders
    // exactly as it did before conform existed.
    quill.compile_data(&doc).expect("still renders");
    assert!(
        !quill
            .validate(&doc)
            .iter()
            .any(|d| d.severity == crate::Severity::Error),
        "and validates clean: the render floor accepts the scalar"
    );
}

/// The seeder is a schema-aware writer, so its output is already at rest.
#[test]
fn conform_is_a_no_op_on_seeds() {
    let quill = quill_from_yaml(
        r#"
quill:
  name: seed_rest
  version: "1.0"
  backend: typst
  description: Seed rest
main:
  fields:
    subject:
      type: richtext
      example: "Q3 **results**"
    note:
      type: plaintext
      example: "a *literal* line"
card_kinds:
  entry:
    fields:
      caption:
        type: plaintext
        example: "raw *text*"
"#,
    );

    let mut doc = quill.seed_document();
    assert_eq!(
        doc.main().payload().get("note").unwrap().as_json(),
        &json!("a *literal* line"),
        "a seeded plaintext field rests as its literal string"
    );
    let before = bytes(&doc);
    let diags = quill.conform(&mut doc).expect("conform");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(bytes(&doc), before, "seed_document is already at rest");

    // A card seeded with an overlay commits through the same dispatch.
    let overlay = SeedOverlay::from_json(&json!({ "caption": "overlaid *text*" })).unwrap();
    let card = quill.seed_card("entry", Some(&overlay)).expect("kind exists");
    assert_eq!(
        card.payload().get("caption").unwrap().as_json(),
        &json!("overlaid *text*")
    );
    let mut doc2 = quill.seed_document();
    doc2.push_card(card).unwrap();
    let before2 = bytes(&doc2);
    quill.conform(&mut doc2).expect("conform");
    assert_eq!(bytes(&doc2), before2, "seed_card is already at rest");
}

/// Markdown-significant characters survive the emit → parse → conform loop for
/// a plaintext field: the corruption per-codec rest exists to prevent.
#[test]
fn plaintext_survives_the_markdown_round_trip() {
    let quill = quill();
    let md = "~~~card-yaml\n$quill: conform_test@1.0.0\nnote: 'a *literal* line'\n~~~\n\nBody.";
    let (doc, _) = parse_bound(&quill, md);

    let round_tripped = doc.to_markdown();
    let (again, _) = parse_bound(&quill, &round_tripped);
    assert_eq!(
        again.main().payload().get("note").unwrap().as_json(),
        &json!("a *literal* line")
    );
    assert_eq!(bytes(&again), bytes(&doc), "and the loop is byte-stable");
}

/// A typed writer that lands on an already-conformed document changes nothing:
/// the flip is one seam, so `set` and conform agree on plaintext.
#[test]
fn a_redundant_typed_write_moves_no_bytes() {
    let quill = quill();
    let (mut doc, _) = parse_bound(&quill, MD);
    let before = bytes(&doc);
    {
        let mut w = quill.writer(&mut doc);
        w.set("note", "a *literal* line").unwrap();
    }
    assert_eq!(bytes(&doc), before);
}

/// The plate is the render floor's shape and does not move: `plaintext` reaches
/// the backend as a content object whichever form was committed.
#[test]
fn the_plate_shape_for_plaintext_is_unchanged() {
    let quill = quill();
    let plate_note = |value: serde_json::Value| {
        let mut doc = Document::parse(
            "~~~card-yaml\n$quill: conform_test@1.0.0\n~~~\n\nBody.",
        )
        .expect("parse")
        .document;
        {
            let mut w = quill.writer(&mut doc);
            w.set("note", value).unwrap();
        }
        let plate = quill.compile_data(&doc).expect("compiles");
        plate["note"].clone()
    };

    // String input and content-object input alike: the plate carries content.
    let from_string = plate_note(json!("a *literal* line"));
    let from_object = plate_note(json!(quillmark_content::serial::to_canonical_value(
        &quillmark_content::from_plaintext("a *literal* line")
    )));
    assert!(from_string.is_object(), "plate keeps the content object");
    assert_eq!(from_string, from_object, "both commit inputs, one plate");
    assert_eq!(from_string["text"], json!("a *literal* line"));
}

/// The revise lane's plaintext arm: a byte-identical revise is a byte no-op,
/// where the markdown codec would have eaten the escapes.
#[test]
fn byte_identical_plaintext_revise_is_byte_stable() {
    let quill = quill();
    let md = r#"~~~card-yaml
$quill: conform_test@1.0.0
note: 'a \*b\* line'
~~~

Body.
"#;
    let (mut doc, _) = parse_bound(&quill, md);
    let before = bytes(&doc);
    let text = quill
        .reader(&doc)
        .get("note")
        .unwrap()
        .expect("note is present");
    let crate::ReadValue::Plaintext(text) = text else {
        panic!("plaintext field reads as plaintext");
    };
    assert_eq!(text, r"a \*b\* line");
    {
        let mut w = quill.writer(&mut doc);
        let delta = w.revise_field("note", &text).expect("revise");
        assert!(
            delta
                .ops
                .iter()
                .all(|op| matches!(op, quillmark_content::Op::Retain(_))),
            "a no-change revise is all-retain: {delta:?}"
        );
    }
    assert_eq!(bytes(&doc), before, "a no-change revise moves no bytes");
}

/// An edit through the plaintext revise lane lands at rest and diffs literally.
#[test]
fn plaintext_revise_commits_the_literal_string() {
    let quill = quill();
    let (mut doc, _) = parse_bound(&quill, MD);
    {
        let mut w = quill.writer(&mut doc);
        w.revise_field("note", "a *literal* line, revised")
            .expect("revise");
    }
    assert_eq!(
        doc.main().payload().get("note").unwrap().as_json(),
        &json!("a *literal* line, revised")
    );
}

/// Read-repair: a legacy row loaded through the bound door converges, and is
/// eligible for rewrite under its current schema tag.
#[test]
fn a_stored_row_converges_through_the_bound_door() {
    let quill = quill();
    // A row written before the invariant: an authored richtext string and a
    // typed-writer plaintext content object resting side by side.
    let mut legacy = Document::parse(
        "~~~card-yaml\n$quill: conform_test@1.0.0\nsubject: Q3 **results**\n~~~\n\nBody.",
    )
    .expect("parse")
    .document;
    legacy
        .main_mut()
        .store_field(
            "note",
            QuillValue::from_json(quillmark_content::serial::to_canonical_value(
                &quillmark_content::from_plaintext("a *literal* line"),
            )),
        )
        .unwrap();
    let stored = serde_json::to_string(&StoredDocument::from(legacy)).unwrap();

    let mut reloaded =
        Document::try_from(serde_json::from_str::<StoredDocument>(&stored).unwrap()).unwrap();
    quill.conform(&mut reloaded).expect("conform");
    assert!(
        reloaded.main().payload().get("subject").unwrap().as_json().is_object(),
        "the authored richtext string converges to the corpus"
    );
    assert_eq!(
        reloaded.main().payload().get("note").unwrap().as_json(),
        &json!("a *literal* line"),
        "and the object-rest plaintext converges to its literal string"
    );
}

/// The legacy envelope, end to end: a hand-authored `@0.92.0` blob (markdown
/// `body`, payload verbatim) migrates forward, conforms, and re-stores fully
/// canonical under the current tag. The `0.92.0 → 0.93.0` hop cold-imports the
/// body and carries the payload untouched, so every content field arrives at
/// the transport door's as-authored rest and conform is what finishes the job.
#[test]
fn a_0_92_0_row_migrates_then_converges() {
    let quill = quill();
    let legacy = json!({
        "schema": "quillmark/document@0.92.0",
        "main": {
            "payload": { "items": [
                { "type": "quill", "value": "conform_test@1.0.0" },
                { "type": "kind", "value": "main" },
                { "type": "field", "key": "subject", "value": "Q3 **results**" },
                // Written by a typed writer of that era: plaintext rested as a
                // content object, the form emit would markdown-escape.
                { "type": "field", "key": "note", "value":
                    quillmark_content::serial::to_canonical_value(
                        &quillmark_content::from_plaintext("a *literal* line")) },
                { "type": "field", "key": "qty", "value": 3 },
            ]},
            "body": "Main **body**."
        },
        "cards": [{
            "payload": { "items": [
                { "type": "kind", "value": "entry" },
                { "type": "field", "key": "body", "value": "card **body**" },
            ]},
            "body": "Entry body."
        }]
    })
    .to_string();

    let mut doc = Document::try_from(
        serde_json::from_str::<StoredDocument>(&legacy).expect("a 0.92.0 blob still loads"),
    )
    .expect("and migrates forward");
    // The hop cold-imports the body; the payload arrives verbatim.
    assert_eq!(doc.main().body_markdown(), "Main **body**.");
    assert!(doc.main().payload().get("subject").unwrap().as_json().is_string());

    let diags = quill.conform(&mut doc).expect("the quill matches");
    assert!(diags.is_empty(), "{diags:?}");
    assert!(
        doc.main().payload().get("subject").unwrap().as_json().is_object(),
        "the authored richtext string converges to the corpus"
    );
    assert_eq!(
        doc.main().payload().get("note").unwrap().as_json(),
        &json!("a *literal* line"),
        "and the object-rest plaintext converges to its literal string"
    );
    assert!(doc.cards()[0].payload().get("body").unwrap().as_json().is_object());

    // Re-stored under the current tag, the row is at rest: byte-equal to the
    // same document authored as markdown and taken through the bound door, and
    // a second conform is a no-op.
    let restored = bytes(&doc);
    assert!(restored.contains("quillmark/document@0.93.0"));
    let (authored, _) = parse_bound(
        &quill,
        "\
~~~card-yaml
$quill: conform_test@1.0.0
$kind: main
subject: Q3 **results**
note: a *literal* line
qty: 3
~~~

Main **body**.

~~~card-yaml
$kind: entry
body: card **body**
~~~

Entry body.
",
    );
    assert_eq!(restored, bytes(&authored), "one document, two ingress routes");
    quill.conform(&mut doc).expect("the quill matches");
    assert_eq!(bytes(&doc), restored);
}

/// A `@0.92.0` field holding an object that is not a decodable content is the
/// non-conforming case, not a corruption: it rests as stored under a
/// `conform::*` warning, and the document still renders.
#[test]
fn a_legacy_field_that_is_not_content_rests_as_stored() {
    let quill = quill();
    let legacy = json!({
        "schema": "quillmark/document@0.92.0",
        "main": {
            "payload": { "items": [
                { "type": "quill", "value": "conform_test@1.0.0" },
                { "type": "kind", "value": "main" },
                { "type": "field", "key": "subject", "value": { "prose": "an older shape" } },
            ]},
            "body": "Body."
        }
    })
    .to_string();

    let mut doc = Document::try_from(
        serde_json::from_str::<StoredDocument>(&legacy).expect("loads"),
    )
    .expect("migrates");
    let before = bytes(&doc);
    let diags = quill.conform(&mut doc).expect("the quill matches");
    assert_eq!(
        diags[0].code.as_deref(),
        Some("conform::field_richtext_decode")
    );
    assert_eq!(bytes(&doc), before, "the value is left exactly as stored");
}
