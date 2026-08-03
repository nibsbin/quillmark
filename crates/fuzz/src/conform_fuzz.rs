//! Property-based fuzz tests for the bound door (`Quill::conform` /
//! `Quill::parse`): the resting-form invariant, stated as three properties.
//!
//! - **C1 (idempotence):** `conform` is a fixed point. A second call moves no
//!   bytes and re-emits identical diagnostics.
//! - **C2 (lane convergence):** parse-then-conform equals typed-write, per
//!   content field. The authored value the parse lane leaves in the payload
//!   (here, the opaque `store_field` that models it) conforms to exactly what
//!   `commit_field` stores; a value the strict write refuses stays authored in
//!   both lanes. A field whose type carries no content is conform's to leave
//!   alone: the typed write remains its canonicalizer.
//! - **C3 (markdown loop):** a document that leaves through the markdown
//!   surface and comes back through the bound door settles. Exactly, for a
//!   `plaintext` field: the literal codec is lossless both ways, which is the
//!   whole reason its rest is a string. For `richtext` the emitted markdown is
//!   the lossy projection (`project_content_field`: island ids, content-only
//!   marks, and a container around a non-paragraph line do not survive), so the
//!   loop is a **fixed point after one pass**, not the identity: the document
//!   settles rather than drifting.
//!
//! The quill is fixed and the values are generated: the resting form is a
//! property of the codec, so what needs coverage is the value space, not the
//! schema space (`coerce_fuzz` fuzzes the schema space under both leniencies).

use proptest::prelude::*;
use quillmark_core::{Document, Quill, QuillValue};

const QUILL_YAML: &str = r#"
quill:
  name: conform_fuzz
  version: "1.0"
  backend: typst
  description: Conform fuzz
main:
  fields:
    rich:
      type: richtext
    rich_inline:
      type: richtext
      inline: true
    plain:
      type: plaintext
    plain_inline:
      type: plaintext
      inline: true
    riches:
      type: array
      items:
        type: richtext
    pair:
      type: object
      properties:
        label:
          type: plaintext
        blurb:
          type: richtext
    qty:
      type: integer
"#;

/// The declared content fields, by name: the ones with a resting form to hold.
const CONTENT_FIELDS: [&str; 6] = [
    "rich",
    "rich_inline",
    "plain",
    "plain_inline",
    "riches",
    "pair",
];

fn quill() -> Quill {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "Quill.yaml".to_string(),
        quillmark_core::quill::FileTreeNode::File {
            contents: QUILL_YAML.as_bytes().to_vec(),
        },
    );
    Quill::from_tree(quillmark_core::quill::FileTreeNode::Directory { files })
        .expect("the fuzz quill loads")
}

fn blank_doc() -> Document {
    Document::parse("~~~card-yaml\n$quill: conform_fuzz@1.0.0\n~~~\n\nBody.\n")
        .expect("blank document parses")
        .document
}

fn bytes(doc: &Document) -> String {
    serde_json::to_string(doc).expect("the storage DTO serializes")
}

/// Text with the characters that make the two codecs disagree: markdown
/// delimiters, escapes, and the newlines that decide inline-ness.
fn arb_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("a".to_string()),
            Just(" ".to_string()),
            Just("*".to_string()),
            Just("**".to_string()),
            Just("_".to_string()),
            Just("\\".to_string()),
            Just("`".to_string()),
            Just("#".to_string()),
            Just("- ".to_string()),
            Just("[x](y)".to_string()),
            Just("\n".to_string()),
            Just("\n\n".to_string()),
        ],
        0..12,
    )
    .prop_map(|parts| parts.concat())
}

/// A value for one of the declared content fields, shaped to its type.
fn arb_field_value() -> impl Strategy<Value = (&'static str, serde_json::Value)> {
    prop_oneof![
        arb_text().prop_map(|t| ("rich", serde_json::Value::String(t))),
        arb_text().prop_map(|t| ("rich_inline", serde_json::Value::String(t))),
        arb_text().prop_map(|t| ("plain", serde_json::Value::String(t))),
        arb_text().prop_map(|t| ("plain_inline", serde_json::Value::String(t))),
        prop::collection::vec(arb_text(), 0..3)
            .prop_map(|ts| ("riches", serde_json::json!(ts))),
        (arb_text(), arb_text())
            .prop_map(|(l, b)| ("pair", serde_json::json!({ "label": l, "blurb": b }))),
        // Shapes the strict write refuses: they must rest authored, not retype.
        Just(("rich", serde_json::json!(42))),
        Just(("plain", serde_json::json!({ "not": "content" }))),
        Just(("qty", serde_json::json!("3"))),
    ]
}

/// The stored value of every content field, the comparison C2 and C3 make.
fn rest(doc: &Document) -> Vec<Option<serde_json::Value>> {
    CONTENT_FIELDS
        .iter()
        .map(|name| doc.main().payload().get(name).map(|v| v.as_json().clone()))
        .collect()
}

proptest! {
    // C1: `conform` is a fixed point, on bytes and on diagnostics.
    #[test]
    fn conform_is_idempotent((name, value) in arb_field_value()) {
        let quill = quill();
        let mut doc = blank_doc();
        doc.main_mut()
            .store_field(name, QuillValue::from_json(value))
            .expect("the opaque store accepts any value");

        let first = quill.conform(&mut doc).expect("the quill matches");
        let once = bytes(&doc);
        let second = quill.conform(&mut doc).expect("the quill matches");
        prop_assert_eq!(once, bytes(&doc), "a second conform moved bytes");
        prop_assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    // C2: the authored value converges to what the typed write commits, and a
    // refusal leaves the same authored value in both lanes.
    #[test]
    fn conform_converges_on_the_typed_write((name, value) in arb_field_value()) {
        let quill = quill();
        let schema = quill
            .config()
            .main
            .fields
            .get(name)
            .expect("a declared field");

        let mut conformed = blank_doc();
        conformed
            .main_mut()
            .store_field(name, QuillValue::from_json(value.clone()))
            .expect("the opaque store accepts any value");
        let diags = quill.conform(&mut conformed).expect("the quill matches");

        let mut written = blank_doc();
        let write = written
            .main_mut()
            .commit_field(name, QuillValue::from_json(value.clone()), schema);

        if !CONTENT_FIELDS.contains(&name) {
            // Not conform's business: the value rests exactly as authored, with
            // nothing to report, whatever the typed write would have made of it.
            prop_assert!(diags.is_empty(), "{diags:?}");
            prop_assert_eq!(
                conformed.main().payload().get(name).map(|v| v.as_json().clone()),
                Some(value),
            );
            return Ok(());
        }

        match write {
            Ok(()) => {
                prop_assert!(diags.is_empty(), "the write took it but conform did not: {diags:?}");
                prop_assert_eq!(bytes(&conformed), bytes(&written));
            }
            Err(_) => {
                // Refused: the value rests authored, and conform says so.
                prop_assert_eq!(
                    conformed.main().payload().get(name).map(|v| v.as_json().clone()),
                    Some(value),
                );
                prop_assert!(
                    diags.iter().any(|d| d.code.as_deref().is_some_and(|c| c.starts_with("conform::"))),
                    "a refused value must carry a conform diagnostic",
                );
            }
        }
    }

    // C3: out through markdown and back in through the bound door settles.
    #[test]
    fn the_markdown_loop_settles((name, value) in arb_field_value()) {
        let quill = quill();
        // A conformed document always re-parses through the bound door; a
        // failure here is one of the bugs this property exists to catch.
        let loop_once = |doc: &Document| -> Document {
            quill
                .parse(&doc.to_markdown())
                .unwrap_or_else(|e| panic!("re-parse of an emitted document failed: {e}"))
                .document
        };

        let mut doc = blank_doc();
        doc.main_mut()
            .store_field(name, QuillValue::from_json(value))
            .expect("the opaque store accepts any value");
        quill.conform(&mut doc).expect("the quill matches");

        let once = loop_once(&doc);
        let twice = loop_once(&once);
        prop_assert_eq!(rest(&twice), rest(&once), "the loop did not settle");

        // The plaintext codec is lossless both ways, so its rest survives the
        // first pass too: the literal string is the whole value.
        if name.starts_with("plain") {
            prop_assert_eq!(rest(&once), rest(&doc));
        }
    }
}
