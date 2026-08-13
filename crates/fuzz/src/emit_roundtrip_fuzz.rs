//! For any input `Document::parse` accepts, parse → emit → re-parse yields an
//! equal document and an identical emission. An input the first parse rejects
//! is discarded; a panic anywhere downstream is a bug.

use proptest::prelude::*;
use quillmark_core::Document;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn fuzz_emit_roundtrip_arbitrary(s in "\\PC{0,1000}") {
        let doc_a = match Document::parse(&s) {
            Ok(d) => d.document,
            Err(_) => return Ok(()), // invalid input: discard
        };

        let emit1 = doc_a.to_markdown();

        let doc_b = Document::parse(&emit1).unwrap_or_else(|e| {
            panic!(
                "emit_roundtrip: re-parse of emitted document failed.\nError: {}\nInput: {:.200}\nEmitted:\n{}",
                e, s, emit1
            )
        })
        .document;

        prop_assert_eq!(
            &doc_a,
            &doc_b,
            "emit_roundtrip: doc_a != doc_b after parse→emit→re-parse.\nEmitted:\n{}",
            emit1
        );

        let emit2 = doc_b.to_markdown();
        prop_assert_eq!(
            &emit1,
            &emit2,
            "emit_roundtrip: emit1 != emit2 (not idempotent on canonical form).\nInput: {:.200}",
            s
        );
    }

    #[test]
    fn fuzz_emit_roundtrip_payload_shaped(
        quill in "[a-z][a-z0-9_]{0,20}",
        key in "[a-z][a-z0-9_]{0,15}",
        value in "\\PC{0,100}"
    ) {
        let src = format!("~~~card-yaml\n$quill: {}\n$kind: main\n{}: \"{}\"\n~~~\n\nBody.\n",
            quill, key, value.replace('\\', "\\\\").replace('"', "\\\""));

        let doc_a = match Document::parse(&src) {
            Ok(d) => d.document,
            Err(_) => return Ok(()),
        };

        let emit1 = doc_a.to_markdown();

        let doc_b = Document::parse(&emit1).unwrap_or_else(|e| {
            panic!(
                "fuzz payload-shaped: re-parse failed.\nError: {}\nSrc:\n{}\nEmitted:\n{}",
                e, src, emit1
            )
        })
        .document;

        prop_assert_eq!(
            &doc_a,
            &doc_b,
            "fuzz payload-shaped: doc_a != doc_b.\nEmitted:\n{}",
            emit1
        );

        let emit2 = doc_b.to_markdown();
        prop_assert_eq!(
            &emit1,
            &emit2,
            "fuzz payload-shaped: emit not idempotent."
        );
    }

    #[test]
    fn fuzz_emit_roundtrip_with_cards(
        quill in "[a-z][a-z0-9_]{0,20}",
        card_kind in "[a-z][a-z0-9_]{0,15}",
        card_key in "[a-z][a-z0-9_]{0,15}",
        card_value in "[a-zA-Z0-9 ]{0,50}"
    ) {
        let src = format!(
            "~~~card-yaml\n$quill: {}\n$kind: main\ntitle: \"test\"\n~~~\n\nBody here.\n\n~~~card-yaml\n$kind: {}\n{}: \"{}\"\n~~~\n\nCard body.\n",
            quill, card_kind, card_key, card_value
        );

        let doc_a = match Document::parse(&src) {
            Ok(d) => d.document,
            Err(_) => return Ok(()),
        };

        let emit1 = doc_a.to_markdown();

        let doc_b = Document::parse(&emit1).unwrap_or_else(|e| {
            panic!(
                "fuzz with-cards: re-parse failed.\nError: {}\nEmitted:\n{}",
                e, emit1
            )
        })
        .document;

        prop_assert_eq!(&doc_a, &doc_b);

        let emit2 = doc_b.to_markdown();
        prop_assert_eq!(&emit1, &emit2);
    }
}
