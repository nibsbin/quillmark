use crate::document::Document;

fn assert_round_trip(label: &str, src: &str) {
    let a = Document::parse(src)
        .unwrap_or_else(|e| panic!("{}: parse failed: {}", label, e))
        .document;
    let emitted = a.to_markdown();
    let b = Document::parse(&emitted)
        .unwrap_or_else(|e| panic!("{}: re-parse failed: {}\nEmitted:\n{}", label, e, emitted))
        .document;
    assert_eq!(
        a, b,
        "{}: round-trip produced different Documents.\nEmitted:\n{}",
        label, emitted
    );
}

#[test]
fn number_scientific_notation_round_trip() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nbig: 1e10\n~~~\n";
    assert_round_trip("1e10", src);

    let doc = Document::parse(src).unwrap().document;
    let v = doc.main().payload().get("big").unwrap();
    assert!(
        v.as_f64().is_some(),
        "1e10 must parse as a number, got {:?}",
        v
    );
}

#[test]
fn emitted_number_representation_matches_parse() {
    struct Case {
        src_value: &'static str,
        key: &'static str,
    }

    let cases = [
        Case {
            src_value: "42",
            key: "count",
        },
        Case {
            src_value: "3.14",
            key: "pi",
        },
        Case {
            src_value: "0",
            key: "zero",
        },
        Case {
            src_value: "-7",
            key: "neg",
        },
        Case {
            src_value: "9999999999999",
            key: "big",
        },
    ];

    for case in &cases {
        let src = format!(
            "~~~card-yaml\n$quill: q\n$kind: main\n{}: {}\n~~~\n",
            case.key, case.src_value
        );
        let doc = Document::parse(&src).unwrap().document;
        let emitted = doc.to_markdown();
        let doc2 = Document::parse(&emitted)
            .unwrap_or_else(|e| {
                panic!(
                    "re-parse failed for {}: {}\nEmitted:\n{}",
                    case.src_value, e, emitted
                )
            })
            .document;
        let v1 = doc.main().payload().get(case.key).unwrap();
        let v2 = doc2.main().payload().get(case.key).unwrap();
        assert_eq!(
            v1, v2,
            "number {} changed representation after emit/re-parse\nEmitted:\n{}",
            case.src_value, emitted
        );
    }
}
