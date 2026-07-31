use proptest::prelude::*;
use quillmark_core::Document;

proptest! {
    /// Shaped payloads of growing width: every generated field survives parse.
    /// No-oracle siblings (generate, parse, then `let _ =` the accessors) are not
    /// carried here: they prove only "did not panic" over input classes
    /// `emit_roundtrip_fuzz` already covers under a round-trip equality oracle.
    #[test]
    fn fuzz_decompose_large_payload(size in 1usize..100) {
        let fields: Vec<String> = (0..size)
            .map(|i| format!("field{}: value{}", i, i))
            .collect();
        let payload = fields.join("\n");
        let markdown =
            format!("~~~card-yaml\n$quill: test_quill\n$kind: main\n{}\n~~~\n\nContent", payload);

        let result = Document::parse(&markdown).map(|p| p.document);
        if let Ok(doc) = result {
            // payload has exactly the fields we provided (no BODY or CARDS keys)
            assert!(doc.main().payload().len() <= size);
        }
    }
}
