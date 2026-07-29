//! Decode-lane fuzz targets: the four boundaries that take caller-supplied
//! **JSON** rather than markdown or a typed Rust value.
//!
//! | Lane | Entry | Binding surface |
//! |---|---|---|
//! | Storage DTO | `Document: TryFrom<StoredDocument>` (serde) | `Document.fromJson` |
//! | Live card wire | `Card: TryFrom<CardWire>` | card reads/writes |
//! | Canonical content | `serial::from_canonical_value` | `install`, body reads |
//! | Op wire | `change_bundle_from_value` | `applyChange` |
//!
//! The existing targets all enter through markdown (`parse_fuzz`,
//! `emit_roundtrip_fuzz`) or a typed value (`coerce_fuzz`, `convert_fuzz`), so
//! none of these were covered. They are where a browser consumer feeds back a
//! restored blob, a server response, or an editor's own change stream — and a
//! panic on any of them traps the WASM module, losing the document rather than
//! the operation. #1093 was exactly this shape, found by reading.
//!
//! Two properties, per lane:
//!
//! - **No panic on arbitrary JSON.** `Err` is a fine answer; unwinding is not.
//! - **Round-trip equality where the lane has an inverse.** A decoder that
//!   accepts is a decoder that must preserve, which "did not panic" cannot see.

use proptest::prelude::*;
use quillmark_core::{Card, CardWire, Document};
use serde_json::{json, Value};

/// Arbitrary JSON, container-biased: the decoders branch on object keys and
/// array shapes, so a generator weighted toward scalars would spend its budget
/// on inputs that fail at the first `as_object()`.
fn arb_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i64>().prop_map(Value::from),
        // The key names the decoders actually dispatch on, mixed with noise, so
        // generated objects reach past the first branch often enough to matter.
        prop_oneof![
            Just("type".to_string()),
            Just("field".to_string()),
            Just("kind".to_string()),
            Just("schema".to_string()),
            Just("main".to_string()),
            Just("body".to_string()),
            Just("value".to_string()),
            Just("islands".to_string()),
            Just("lines".to_string()),
            Just("marks".to_string()),
            Just("text".to_string()),
            "\\PC{0,12}",
        ]
        .prop_map(Value::from),
    ];
    leaf.prop_recursive(6, 96, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
            prop::collection::hash_map(
                prop_oneof![
                    Just("type".to_string()),
                    Just("kind".to_string()),
                    Just("value".to_string()),
                    Just("key".to_string()),
                    Just("body".to_string()),
                    Just("payload".to_string()),
                    Just("items".to_string()),
                    "\\PC{0,8}",
                ],
                inner,
                0..6,
            )
            .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

/// A document with enough shape to exercise the payload and body lanes.
fn sample_doc() -> Document {
    Document::parse(
        "\
~~~
$quill: test_quill@0.1
$kind: main
# a comment
title: Hello
tags:
  - one # inline
  - two
nested:
  a:
    b: 1
~~~

Body **text** with a [link](https://example.com) and a list:

- one
- two

~~~
$kind: note
note_field: value
~~~

A second card's body.
",
    )
    .expect("fixture parses")
    .document
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// `Document`'s serde entry (the `StoredDocument` envelope) against
    /// arbitrary JSON. Every rejection must be an `Err`.
    #[test]
    fn storage_decode_never_panics(v in arb_json()) {
        let text = v.to_string();
        let _ = serde_json::from_str::<Document>(&text);
    }

    /// The same lane fed a *well-formed envelope* whose payload is arbitrary —
    /// past the `schema` discriminator, where the interesting decoding is.
    #[test]
    fn storage_decode_never_panics_past_the_tag(main in arb_json(), cards in arb_json()) {
        let blob = json!({
            "schema": "quillmark/document@0.93.0",
            "main": main,
            "cards": cards,
        });
        let _ = serde_json::from_str::<Document>(&blob.to_string());

        let legacy = json!({
            "schema": "quillmark/document@0.92.0",
            "main": blob["main"],
            "cards": blob["cards"],
        });
        let _ = serde_json::from_str::<Document>(&legacy.to_string());
    }

    /// The live card wire against arbitrary JSON, both halves: deserializing a
    /// `CardWire` and converting one into a `Card`.
    #[test]
    fn card_wire_decode_never_panics(v in arb_json()) {
        if let Ok(wire) = serde_json::from_value::<CardWire>(v) {
            let _ = Card::try_from(wire);
        }
    }

    /// Canonical content decode — the `install` lane.
    #[test]
    fn canonical_content_decode_never_panics(v in arb_json()) {
        let _ = quillmark_content::serial::from_canonical_value(&v);
    }

    /// The op wire — the `applyChange` lane.
    #[test]
    fn op_wire_decode_never_panics(v in arb_json()) {
        let _ = quillmark_content::change_bundle_from_value(&v);
        let _ = quillmark_content::line_op_from_value(&v);
        let _ = quillmark_content::mark_op_from_value(&v);
    }
}

/// Round-trip oracles. Separate from the arbitrary-input sweep because these
/// need a *valid* document to start from — the property is preservation, which
/// a rejected input cannot exercise.
#[cfg(test)]
mod roundtrip {
    use super::*;

    #[test]
    fn storage_dto_round_trips() {
        let doc = sample_doc();
        let json = serde_json::to_string(&doc).expect("serialize");
        let back: Document = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(doc, back, "storage round-trip lost data");
        // Byte-stable on re-serialization, not merely equal.
        assert_eq!(
            json,
            serde_json::to_string(&back).expect("re-serialize"),
            "storage round-trip is not byte-stable"
        );
    }

    #[test]
    fn card_wire_round_trips() {
        let doc = sample_doc();
        for card in std::iter::once(doc.main()).chain(doc.cards()) {
            let wire = CardWire::from(card);
            let back = Card::try_from(wire.clone()).expect("wire converts back");
            // The wire is the editable projection: nested comments do not ride
            // it (module doc), so compare on what it does carry.
            assert_eq!(
                CardWire::from(&back),
                wire,
                "card wire round-trip lost data for kind {:?}",
                wire.kind
            );
        }
    }

    #[test]
    fn canonical_content_round_trips() {
        let doc = sample_doc();
        for card in std::iter::once(doc.main()).chain(doc.cards()) {
            let body = card.body();
            let value = quillmark_content::serial::to_canonical_value(body);
            let back = quillmark_content::serial::from_canonical_value(&value)
                .expect("canonical content decodes");
            assert_eq!(&back, body, "canonical content round-trip lost data");
        }
    }

    /// The envelope's own guard: an unknown schema tag is a rejection, not a
    /// panic and not a silent fallback to the current version.
    #[test]
    fn unknown_schema_version_is_rejected() {
        let blob = r#"{"schema":"quillmark/document@99.0.0","main":{},"cards":[]}"#;
        assert!(serde_json::from_str::<Document>(blob).is_err());
    }
}

#[cfg(test)]
mod generator_probe {
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::{Config, TestRunner};

    /// The sweep is only worth its runtime if generated JSON gets *past* the
    /// outer shape checks. Probe the reach rather than assuming it.
    #[test]
    fn probe() {
        let mut runner = TestRunner::new(Config { cases: 2000, ..Config::default() });
        let (mut objects, mut content_deep, mut wire_ok, mut ops_deep) = (0, 0, 0, 0);
        let strat = arb_json();
        for _ in 0..2000 {
            let v = strat.new_tree(&mut runner).unwrap().current();
            if v.is_object() { objects += 1; }
            // Did the content decoder get past "not an object" into field work?
            match quillmark_content::serial::from_canonical_value(&v) {
                Ok(_) => content_deep += 1,
                Err(e) => {
                    let m = e.to_string();
                    if !m.contains("expected an object") && !m.contains("not an object") {
                        content_deep += 1;
                    }
                }
            }
            if serde_json::from_value::<CardWire>(v.clone()).is_ok() { wire_ok += 1; }
            if let Err(e) = quillmark_content::change_bundle_from_value(&v) {
                let m = e.to_string();
                if !m.contains("expected an object") && !m.contains("not an object") { ops_deep += 1; }
            }
        }
        println!(
            "objects={objects}/2000 content_past_outer={content_deep} \
             wire_deserialized={wire_ok} ops_past_outer={ops_deep}"
        );
        // Observed at the time of writing: ~984 objects, ~390 full CardWire
        // deserializations, ~1016 op-wire inputs past the outer shape check.
        // The floors are set well under those so ordinary generator drift does
        // not fail the build, while a change that collapses the reach — the
        // sweep passing because nothing gets in — does.
        assert!(objects > 200, "generator rarely produces objects: {objects}/2000");
        assert!(
            wire_ok > 50,
            "generator rarely reaches a full CardWire: {wire_ok}/2000 — the \
             card-wire sweep would be passing on rejected input"
        );
        assert!(
            ops_deep > 200,
            "generator rarely reaches the op wire: {ops_deep}/2000"
        );
    }
}
