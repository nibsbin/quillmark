use crate::document::assemble::decompose;
use crate::document::Document;

#[test]
fn test_empty_input_dedicated_error() {
    // Empty input gets a dedicated message distinct from the missing-root one.
    for input in ["", "   ", "\n\n\t\n"] {
        let err = decompose(input).unwrap_err().to_string();
        assert!(
            err.contains("Empty markdown input"),
            "expected dedicated empty-input message for {input:?}, got: {err}"
        );
    }
}

#[test]
fn test_missing_quill_diagnostic_code() {
    // Documents with no `~~~card-yaml` block at all surface the dedicated
    // `parse::missing_quill` code.
    let cases = [
        "# Hello World\n\nNo payload here.",
        "Just prose, no card-yaml block.",
    ];
    for input in cases {
        let err = decompose(input).unwrap_err();
        let diag = err.to_diagnostic();
        assert_eq!(
            diag.code.as_deref(),
            Some("parse::missing_quill"),
            "expected parse::missing_quill for {input:?}, got: {:?}",
            diag.code
        );
    }
}

#[test]
fn test_malformed_quill_reference_carries_code_and_grammar_hint() {
    // Uppercase name → dedicated code plus the canonical grammar as `hint`.
    let err =
        decompose("~~~card-yaml\n$quill: Resume@2.1.0\n$kind: main\n~~~\n\nBody\n").unwrap_err();
    let diag = err.to_diagnostic();
    assert_eq!(diag.code.as_deref(), Some("parse::invalid_quill_reference"));
    assert_eq!(
        diag.hint.as_deref(),
        Some(crate::version::quill_ref_hint()),
        "the malformed-reference diagnostic must carry the canonical grammar hint"
    );
}

#[test]
fn test_root_dash_frontmatter_without_quill_reports_missing_quill() {
    // `---` is an accepted opener for the root block. A `---` block without
    // `$quill` surfaces the standard MissingQuill error: not a
    // "use `~~~card-yaml` instead of `---`" hint, which would be misleading.
    let err = decompose("---\nquill: usaf_memo\ntitle: Memo\n---\n\nBody\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("must declare `$quill: <name>`"), "got: {msg}");
    assert!(!msg.contains("`---` YAML frontmatter"), "stale hint: {msg}");
    assert!(
        !msg.contains("Replace the opening `---`"),
        "stale hint: {msg}"
    );
}

#[test]
fn test_missing_block_with_bare_yaml_calls_out_missing_fence() {
    let err = decompose("$quill: usaf_memo\n$kind: main\ntitle: Memo\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("missing the `~~~` fence"), "got: {msg}");
}

// -----------------------------------------------------------------------
// `---` YAML-frontmatter root-block support (accept-but-don't-emit).
//
// LLMs trained on the broader internet overwhelmingly write `---` … `---`
// YAML frontmatter when generating Markdown. The parser accepts this shape
// for the document's first (root) block only. Composable cards still
// require the canonical `~~~card-yaml` / `~~~` fences, and the emitter is
// unchanged: `to_markdown()` always emits the canonical form.
// -----------------------------------------------------------------------

#[test]
fn test_dash_root_block_parses_equivalent_to_card_yaml() {
    let dash_md = "---\n$quill: test_quill\n$kind: main\ntitle: Test\n---\n\nBody.";
    let canonical_md = "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: Test\n~~~\n\nBody.";
    let dash_doc = decompose(dash_md).expect("--- root block should parse");
    let canonical_doc = decompose(canonical_md).expect("canonical root block parses");
    // PartialEq on Document ignores warnings; just compares main + cards.
    assert_eq!(dash_doc, canonical_doc);
    assert_eq!(dash_doc.quill_reference().name, "test_quill");
    assert_eq!(
        dash_doc
            .main()
            .payload()
            .get("title")
            .unwrap()
            .as_str()
            .unwrap(),
        "Test"
    );
    assert_eq!(dash_doc.main().body_markdown(), "Body.");
}

#[test]
fn test_dash_root_block_emits_canonical_card_yaml() {
    // Re-emitting a `---`-parsed document MUST produce the canonical bare
    // `~~~` fence shape. Normalisation on first emit is intended.
    let dash_md = "---\n$quill: test_quill\n$kind: main\ntitle: Test\n---\n\nBody.";
    let doc = decompose(dash_md).unwrap();
    let emitted = doc.to_markdown();
    assert!(
        emitted.starts_with("~~~\n"),
        "expected canonical opener, got: {emitted:?}"
    );
    assert!(
        !emitted.contains("---\n"),
        "stray dash fence in emit: {emitted:?}"
    );
}

#[test]
fn test_dash_root_with_composable_card_yaml_parses() {
    // `---` root, canonical composable card after: the common LLM shape.
    let markdown = "---\n$quill: test_quill\n$kind: main\ntitle: Test\n---\n\nBody.\n\n\
                    ~~~card-yaml\n$kind: note\nlabel: a\n~~~\n\nNote body.";
    let doc = decompose(markdown).expect("mixed shape should parse");
    assert_eq!(doc.quill_reference().name, "test_quill");
    assert_eq!(doc.cards().len(), 1);
    assert_eq!(doc.cards()[0].kind(), Some("note"));
    assert_eq!(doc.cards()[0].body_markdown(), "Note body.");
}

#[test]
fn test_dash_opener_in_composable_card_position_errors() {
    // After the root `~~~card-yaml` block, a `---` … `---` block with YAML
    // keys between is rejected with the "expected `~~~card-yaml`" error.
    let markdown = "~~~card-yaml\n$quill: test_quill\n$kind: main\n~~~\n\nBody.\n\n\
                    ---\n$kind: note\nlabel: a\n---\n\nNote body.";
    let err = decompose(markdown).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("composable cards") || msg.contains("Composable card"),
        "expected composable-card rejection, got: {msg}"
    );
    assert!(msg.contains("~~~"), "got: {msg}");
}

#[test]
fn test_dash_opener_with_tilde_closer_falls_through() {
    // Mixed fences: a `---` opener with no matching `---` closer is not
    // frontmatter, per CommonMark the lone `---` is a thematic break. No root
    // block is recognised, so the document surfaces MissingQuill.
    let markdown = "---\n$quill: test_quill\n$kind: main\ntitle: T\n~~~\n\nBody.";
    let err = decompose(markdown).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Missing required root"), "got: {msg}");
}

#[test]
fn test_tilde_opener_with_dash_closer_falls_through() {
    // The mirror: a `~~~` opener with no `~~~` closer (only a `---`) is an
    // unclosed CommonMark code block to EOF, not a card-yaml block. With no
    // closed root block the document surfaces MissingQuill.
    let markdown = "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: T\n---\n\nBody.";
    let err = decompose(markdown).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Missing required root"), "got: {msg}");
}

#[test]
fn test_with_payload() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test Document
author: Test Author
~~~

# Hello World

This is the body.";

    let doc = decompose(markdown).unwrap();

    assert_eq!(
        doc.main().body_markdown(),
        "# Hello World\n\nThis is the body."
    );
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Test Document"
    );
    assert_eq!(
        doc.main()
            .payload()
            .get("author")
            .unwrap()
            .as_str()
            .unwrap(),
        "Test Author"
    );
    assert_eq!(doc.main().payload().len(), 2); // title, author
    assert_eq!(doc.cards().len(), 0);
    assert_eq!(doc.quill_reference().name, "test_quill");
}

#[test]
fn test_complex_yaml_payload() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Complex Document
tags:
  - test
  - yaml
metadata:
  version: 1.0
  nested:
    field: value
~~~

Content here.";

    let doc = decompose(markdown).unwrap();

    assert_eq!(doc.main().body_markdown(), "Content here.");
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Complex Document"
    );

    let tags = doc
        .main()
        .payload()
        .get("tags")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].as_str().unwrap(), "test");
    assert_eq!(tags[1].as_str().unwrap(), "yaml");
}

#[test]
fn test_invalid_yaml() {
    // Root card-yaml block with invalid YAML payload.
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: [invalid yaml
author: missing close bracket
~~~

Content here.";

    let result = decompose(markdown);
    assert!(result.is_err());
    // Error message includes location context
    assert!(result.unwrap_err().to_string().contains("YAML error"));
}

#[test]
fn test_unclosed_payload() {
    // An unclosed root fence is delegated to CommonMark (a code block running
    // to EOF), so no root block is recognised and the document fails with
    // MissingQuill rather than a hard "never closed" error.
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test
author: Test Author

Content without closing fence";

    let result = decompose(markdown);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required root"));
}

// Extended metadata tests

#[test]
fn test_basic_card_block() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Main Document
~~~

Main body content.

~~~card-yaml
$kind: items
name: Item 1
~~~

Body of item 1.";

    let doc = decompose(markdown).unwrap();

    // Global body is followed by a card block: blank-line separator stripped,
    // so the trailing `\n\n` from the source becomes a single `\n`.
    assert_eq!(doc.main().body_markdown(), "Main body content.");
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Main Document"
    );

    assert_eq!(doc.cards().len(), 1);
    let card = &doc.cards()[0];
    assert_eq!(card.kind(), Some("items"));
    assert_eq!(
        card.payload().get("name").unwrap().as_str().unwrap(),
        "Item 1"
    );
    // Last card body at EOF: no blank-line separator to strip.
    assert_eq!(card.body_markdown(), "Body of item 1.");
}

// ── "N composable cards parse with correct kind/payload/order" (table-driven) ─
// Each row parses `markdown` and checks the subset of {quill reference, main
// payload fields, main body, per-card kind/fields/body in order} it sets. An
// unset/`None` entry is not checked.
#[test]
fn cards_parse_with_correct_kind_payload_and_order() {
    enum Expect {
        Str(&'static str),
        I64(i64),
        F64(f64),
    }

    fn check_field(payload: &crate::document::Payload, key: &str, expect: &Expect, ctx: &str) {
        let v = payload
            .get(key)
            .unwrap_or_else(|| panic!("{ctx}: missing field {key:?}"));
        match expect {
            Expect::Str(s) => assert_eq!(v.as_str().unwrap(), *s, "{ctx}: field {key:?}"),
            Expect::I64(i) => assert_eq!(v.as_i64().unwrap(), *i, "{ctx}: field {key:?}"),
            Expect::F64(f) => assert_eq!(v.as_f64().unwrap(), *f, "{ctx}: field {key:?}"),
        }
    }

    struct ExpectedCard {
        kind: Option<&'static str>,
        fields: Vec<(&'static str, Expect)>,
        body: Option<&'static str>,
    }

    struct Case {
        name: &'static str,
        markdown: &'static str,
        quill: Option<&'static str>,
        main_fields: Vec<(&'static str, Expect)>,
        main_payload_len: Option<usize>,
        main_body_eq: Option<&'static str>,
        main_body_contains: Vec<&'static str>,
        cards: Vec<ExpectedCard>,
    }

    let cases = vec![
        Case {
            name: "multiple_card_blocks",
            markdown: "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: items
name: Item 1
tags: [a, b]
~~~

First item body.

~~~card-yaml
$kind: items
name: Item 2
tags: [c, d]
~~~

Second item body.",
            quill: None,
            main_fields: vec![],
            main_payload_len: None,
            main_body_eq: None,
            main_body_contains: vec![],
            cards: vec![
                ExpectedCard {
                    kind: Some("items"),
                    fields: vec![("name", Expect::Str("Item 1"))],
                    body: None,
                },
                ExpectedCard {
                    kind: Some("items"),
                    fields: vec![("name", Expect::Str("Item 2"))],
                    body: None,
                },
            ],
        },
        Case {
            name: "mixed_global_and_cards",
            markdown: "~~~card-yaml
$quill: test_quill
$kind: main
title: Global
author: John Doe
~~~

Global body.

~~~card-yaml
$kind: sections
title: Section 1
~~~

Section 1 content.

~~~card-yaml
$kind: sections
title: Section 2
~~~

Section 2 content.",
            quill: None,
            main_fields: vec![("title", Expect::Str("Global"))],
            main_payload_len: None,
            main_body_eq: Some("Global body."),
            main_body_contains: vec![],
            cards: vec![
                ExpectedCard {
                    kind: Some("sections"),
                    fields: vec![],
                    body: None,
                },
                ExpectedCard {
                    kind: None,
                    fields: vec![],
                    body: None,
                },
            ],
        },
        Case {
            name: "adjacent_blocks_different_kinds",
            markdown: "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: items
name: Item 1
~~~

Item 1 body

~~~card-yaml
$kind: sections
title: Section 1
~~~

Section 1 body",
            quill: None,
            main_fields: vec![],
            main_payload_len: None,
            main_body_eq: None,
            main_body_contains: vec![],
            cards: vec![
                ExpectedCard {
                    kind: Some("items"),
                    fields: vec![("name", Expect::Str("Item 1"))],
                    body: None,
                },
                ExpectedCard {
                    kind: Some("sections"),
                    fields: vec![("title", Expect::Str("Section 1"))],
                    body: None,
                },
            ],
        },
        Case {
            name: "order_preservation",
            markdown: "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: items
id: 1
~~~

First

~~~card-yaml
$kind: items
id: 2
~~~

Second

~~~card-yaml
$kind: items
id: 3
~~~

Third",
            quill: None,
            main_fields: vec![],
            main_payload_len: None,
            main_body_eq: None,
            main_body_contains: vec![],
            cards: vec![
                ExpectedCard {
                    kind: Some("items"),
                    fields: vec![("id", Expect::I64(1))],
                    body: None,
                },
                ExpectedCard {
                    kind: Some("items"),
                    fields: vec![("id", Expect::I64(2))],
                    body: None,
                },
                ExpectedCard {
                    kind: Some("items"),
                    fields: vec![("id", Expect::I64(3))],
                    body: None,
                },
            ],
        },
        Case {
            name: "product_catalog_integration",
            markdown: "~~~card-yaml
$quill: test_quill
$kind: main
title: Product Catalog
author: John Doe
date: 2024-01-01
~~~

This is the main catalog description.

~~~card-yaml
$kind: products
name: Widget A
price: 19.99
sku: WID-001
~~~

The **Widget A** is our most popular product.

~~~card-yaml
$kind: products
name: Gadget B
price: 29.99
sku: GAD-002
~~~

The **Gadget B** is perfect for professionals.

~~~card-yaml
$kind: reviews
product: Widget A
rating: 5
~~~

\"Excellent product! Highly recommended.\"

~~~card-yaml
$kind: reviews
product: Gadget B
rating: 4
~~~

\"Very good, but a bit pricey.\"",
            quill: None,
            main_fields: vec![
                ("title", Expect::Str("Product Catalog")),
                ("author", Expect::Str("John Doe")),
                ("date", Expect::Str("2024-01-01")),
            ],
            main_payload_len: Some(3),
            main_body_eq: None,
            main_body_contains: vec!["main catalog description"],
            cards: vec![
                ExpectedCard {
                    kind: Some("products"),
                    fields: vec![
                        ("name", Expect::Str("Widget A")),
                        ("price", Expect::F64(19.99)),
                    ],
                    body: None,
                },
                ExpectedCard {
                    kind: Some("products"),
                    fields: vec![("name", Expect::Str("Gadget B"))],
                    body: None,
                },
                ExpectedCard {
                    kind: Some("reviews"),
                    fields: vec![
                        ("product", Expect::Str("Widget A")),
                        ("rating", Expect::I64(5)),
                    ],
                    body: None,
                },
                ExpectedCard {
                    kind: None,
                    fields: vec![],
                    body: None,
                },
            ],
        },
        Case {
            name: "quill_with_card_blocks",
            markdown: "~~~card-yaml
$quill: document
$kind: main
title: Test Document
~~~

Main body.

~~~card-yaml
$kind: sections
name: Section 1
~~~

Section 1 body.",
            quill: Some("document"),
            main_fields: vec![("title", Expect::Str("Test Document"))],
            main_payload_len: None,
            main_body_eq: Some("Main body."),
            main_body_contains: vec![],
            cards: vec![ExpectedCard {
                kind: Some("sections"),
                fields: vec![],
                body: None,
            }],
        },
        Case {
            name: "card_consecutive_blocks",
            markdown: "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: a
id: 1
~~~

~~~card-yaml
$kind: a
id: 2
~~~",
            quill: None,
            main_fields: vec![],
            main_payload_len: None,
            main_body_eq: None,
            main_body_contains: vec![],
            cards: vec![
                ExpectedCard {
                    kind: Some("a"),
                    fields: vec![],
                    body: None,
                },
                ExpectedCard {
                    kind: Some("a"),
                    fields: vec![],
                    body: None,
                },
            ],
        },
        // A multi-card document (root + two composable cards, prose thematic
        // break in the root body) exercising the shapes described in
        // markdown-spec.md. `***` (thematic break) has no content
        // representation and is dropped by the projection; the surrounding
        // paragraphs survive: hence `main_body_contains` rather than
        // `main_body_eq`.
        Case {
            name: "spec_example",
            markdown: "~~~card-yaml
$quill: blog_post
$kind: main
title: My Document
~~~

Main document body.

***

More content after horizontal rule.

~~~card-yaml
$kind: section
heading: Introduction
~~~

Introduction content.

~~~card-yaml
$kind: section
heading: Conclusion
~~~

Conclusion content.
",
            quill: Some("blog_post"),
            main_fields: vec![("title", Expect::Str("My Document"))],
            main_payload_len: None,
            main_body_eq: None,
            main_body_contains: vec![
                "Main document body.",
                "More content after horizontal rule.",
            ],
            cards: vec![
                ExpectedCard {
                    kind: Some("section"),
                    fields: vec![("heading", Expect::Str("Introduction"))],
                    body: Some("Introduction content."),
                },
                ExpectedCard {
                    kind: Some("section"),
                    fields: vec![("heading", Expect::Str("Conclusion"))],
                    body: Some("Conclusion content."),
                },
            ],
        },
    ];

    for case in &cases {
        let doc = decompose(case.markdown)
            .unwrap_or_else(|e| panic!("{}: parse failed: {e}", case.name));

        if let Some(quill) = case.quill {
            assert_eq!(
                doc.quill_reference().name,
                quill,
                "{}: quill name",
                case.name
            );
        }
        for (key, expect) in &case.main_fields {
            check_field(
                doc.main().payload(),
                key,
                expect,
                &format!("{}: main", case.name),
            );
        }
        if let Some(len) = case.main_payload_len {
            assert_eq!(
                doc.main().payload().len(),
                len,
                "{}: main payload len",
                case.name
            );
        }
        if let Some(body) = case.main_body_eq {
            assert_eq!(doc.main().body_markdown(), body, "{}: main body", case.name);
        }
        for needle in &case.main_body_contains {
            assert!(
                doc.main().body_markdown().contains(needle),
                "{}: main body missing {needle:?}",
                case.name
            );
        }

        assert_eq!(
            doc.cards().len(),
            case.cards.len(),
            "{}: cards len",
            case.name
        );
        for (i, expected) in case.cards.iter().enumerate() {
            let card = &doc.cards()[i];
            let ctx = format!("{}: card[{i}]", case.name);
            if let Some(kind) = expected.kind {
                assert_eq!(card.kind(), Some(kind), "{ctx} kind");
            }
            for (key, expect) in &expected.fields {
                check_field(card.payload(), key, expect, &ctx);
            }
            if let Some(body) = expected.body {
                assert_eq!(card.body_markdown(), body, "{ctx} body");
            }
        }
    }
}

#[test]
fn test_empty_card_metadata() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: items
~~~

Body without metadata.";

    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.cards().len(), 1);
    let card = &doc.cards()[0];
    assert_eq!(card.kind(), Some("items"));
    assert!(card.payload().is_empty());
    assert_eq!(card.body_markdown(), "Body without metadata.");
}

#[test]
fn test_card_block_without_body() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: items
name: Item
~~~";

    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.cards().len(), 1);
    let card = &doc.cards()[0];
    assert_eq!(card.kind(), Some("items"));
    assert_eq!(card.body_markdown(), ""); // empty, not absent
}

// ── card-kind / global-field name collision: is_ok()-only cases (table-driven) ───
// `test_allowed_card_field_collision` (below) exercises the same collision
// shape and additionally checks the resulting values.
#[test]
fn card_kind_global_field_name_collision_is_allowed() {
    let cases: &[(&str, &str)] = &[
        (
            "scalar global field",
            "~~~card-yaml
$quill: test_quill
$kind: main
items: \"global value\"
~~~

Body

~~~card-yaml
$kind: items
name: Item
~~~

Item body",
        ),
        (
            "array-valued global field",
            "~~~card-yaml
$quill: test_quill
$kind: main
items:
  - name: Global Item 1
    value: 100
~~~

Global body

~~~card-yaml
$kind: items
name: Scope Item 1
~~~

Scope item 1 body",
        ),
        (
            "empty-array global field",
            "~~~card-yaml
$quill: test_quill
$kind: main
items: []
~~~

Global body

~~~card-yaml
$kind: items
name: Item 1
~~~

Item 1 body",
        ),
    ];

    for (label, markdown) in cases {
        assert!(
            decompose(markdown).is_ok(),
            "{label}: name collision should be allowed"
        );
    }
}

#[test]
fn test_uppercase_payload_keys_accepted_at_parse() {
    // Spec §3.4: data-field names match [A-Za-z_][A-Za-z0-9_]*. Only
    // `$`-prefixed keys are system metadata, so uppercase user fields parse,
    // are preserved verbatim (case is significant), and round-trip bare
    // through emit.
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: section
BODY: Test
~~~";

    let doc = decompose(markdown).unwrap();
    assert_eq!(
        doc.cards()[0]
            .payload()
            .get("BODY")
            .unwrap()
            .as_str()
            .unwrap(),
        "Test"
    );
    // Verbatim, no lowercasing: the uppercase key survives emit unquoted.
    assert!(
        doc.to_markdown().contains("BODY: Test"),
        "uppercase field name must round-trip bare and verbatim"
    );
}

#[test]
fn test_delimiter_inside_fenced_code_block_backticks() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test
~~~

Here is some code:

```yaml
~~~card-yaml
$kind: code_example
fake: payload
~~~
```

More content.
";

    let doc = decompose(markdown).unwrap();
    // The card-yaml inside the code block should NOT be parsed as metadata.
    assert!(doc.main().body_markdown().contains("fake: payload"));
    assert!(doc.main().payload().get("fake").is_none());
    assert_eq!(doc.cards().len(), 0);
}

/// Flow-sequence YAML (`[a, b]`) reaches the payload as an array: the block
/// form is covered by `emit_tests.rs::round_trip_sequence`.
#[test]
fn test_flow_sequence_array_field_parses() {
    let markdown = "~~~card-yaml
$quill: usaf_memo
$kind: main
memo_for: [ORG/SYMBOL, OTHER/SYMBOL]
~~~

This is the memo body.";

    let doc = decompose(markdown).unwrap();
    let memo_for = doc.main().payload().get("memo_for").unwrap();
    let items = memo_for.as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].as_str().unwrap(), "ORG/SYMBOL");
    assert_eq!(items[1].as_str().unwrap(), "OTHER/SYMBOL");
}

#[test]
fn test_root_without_kind_is_accepted_and_synthesised() {
    // markdown-spec.md §3.3: the root's `$kind` is `main` by position. An omitted
    // `$kind` parses successfully; the parser synthesises the entry at the
    // canonical position so `doc.main().kind()` is always `Some("main")`
    // and canonical emission writes `$kind: main` back out.
    let markdown = "~~~card-yaml
$quill: test_quill
title: Test
~~~

Body content.";

    let doc = decompose(markdown).expect("root without $kind should parse");
    assert_eq!(doc.main().kind(), Some("main"));
    assert_eq!(doc.main().quill().unwrap().name.as_str(), "test_quill");

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("$kind: main"),
        "canonical emission should synthesise $kind: main; got: {emitted}"
    );

    // The synthesised line lives in canonical position: after $quill, before
    // any user field.
    let quill_pos = emitted.find("$quill:").expect("emitted lacks $quill");
    let kind_pos = emitted.find("$kind:").expect("emitted lacks $kind");
    let title_pos = emitted.find("title:").expect("emitted lacks title");
    assert!(
        quill_pos < kind_pos && kind_pos < title_pos,
        "canonical order is $quill < $kind < user fields; got: {emitted}"
    );

    // Round-trip the emitted form: it parses again and equals the original.
    let reparsed = decompose(&emitted).expect("emitted form re-parses");
    assert_eq!(doc, reparsed);
}

#[test]
fn test_root_with_non_main_kind_is_error() {
    // Only `$kind: main` is valid on the root. Other values still error.
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: other
title: Test
~~~";
    let err = decompose(markdown).unwrap_err().to_string();
    assert!(
        err.contains("$kind: other") && err.contains("reserved for the document root"),
        "expected non-main-root error, got: {err}"
    );
}

#[test]
fn test_over_nested_body_surfaces_body_import_error() {
    // A body whose container nesting exceeds MAX_NESTING_DEPTH cannot import into
    // the content; the parse fails with the dedicated `parse::body_import` code
    // (such a body never rendered: the backend rejected the same depth).
    let deep = ">".repeat(crate::error::MAX_NESTING_DEPTH + 5);
    let markdown =
        format!("~~~card-yaml\n$quill: test_quill\n$kind: main\n~~~\n\n{deep} too deep\n");
    let err = decompose(&markdown).unwrap_err();
    assert_eq!(
        err.to_diagnostic().code.as_deref(),
        Some("parse::body_import")
    );
}

#[test]
fn test_canonical_root_with_kind_round_trips_byte_equal() {
    // §9.1: a canonical document is a parse-emit fixed point. Adding the
    // implicit-kind synthesis must not perturb canonical input: when
    // `$kind: main` is already written, the emitter produces the same line.
    // The canonical body carries a single trailing newline (the content
    // projection's block terminator) so the document is a parse-emit fixed point.
    let canonical = "~~~\n$quill: test_quill\n$kind: main\ntitle: Test\n~~~\n\nBody.\n";
    let doc = decompose(canonical).unwrap();
    assert_eq!(doc.to_markdown(), canonical);
}

#[test]
fn test_non_root_block_declaring_quill_is_error() {
    // Only the root block binds the document to a quill. A composable card
    // declaring `$quill` is a structural parse error.
    let markdown = "~~~card-yaml
$quill: first
$kind: main
~~~

~~~card-yaml
$quill: second
$kind: note
~~~";

    let err = decompose(markdown).unwrap_err().to_string();
    assert!(err.contains("must not declare `$quill`"), "got: {err}");
}

#[test]
fn test_invalid_quill_ref() {
    let markdown = "~~~card-yaml
$quill: Invalid-Name
$kind: main
~~~";

    let result = decompose(markdown);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid $quill reference"));
}

#[test]
fn test_quill_empty_value() {
    let markdown = "~~~card-yaml
$quill:
~~~";

    let result = decompose(markdown);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid $quill reference"));
}

#[test]
fn test_card_with_unknown_meta_key_is_error() {
    // `$`-prefixed metadata keys are a closed set `{quill, kind, id, ext, seed}`.
    // Any other `$key` is a parse error.
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$foo: bar
$kind: note
~~~";

    let err = decompose(markdown).unwrap_err().to_string();
    assert!(
        err.contains("Unknown `$foo`"),
        "expected unknown-key parse error, got: {err}"
    );
}

#[test]
fn dollar_keys_at_any_position_in_payload_work() {
    // `$`-prefixed reserved keys are ordinary YAML; they may appear at any
    // position in the block's mapping. Emit preserves source order so that
    // any comments adjacent to a `$` line round-trip in place.
    let markdown = "~~~card-yaml
title: First
$quill: test_quill
author: Bob
$kind: main
~~~

Body.";

    let doc = decompose(markdown).expect("payload with $-keys mid-mapping should parse");
    assert_eq!(doc.main().quill().unwrap().to_string(), "test_quill");
    assert_eq!(doc.main().kind(), Some("main"));
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str(),
        Some("First")
    );
    assert_eq!(
        doc.main().payload().get("author").unwrap().as_str(),
        Some("Bob")
    );
    // `$` keys do not appear in the user-field accessors.
    assert!(doc.main().payload().get("$quill").is_none());
    assert!(doc.main().payload().get("$kind").is_none());

    // Round-trip stability: emit then re-parse produces an equal Document.
    let emitted = doc.to_markdown();
    let reparsed = decompose(&emitted).expect("round-trip should re-parse");
    assert_eq!(doc, reparsed);
}

#[test]
fn fill_on_dollar_key_is_rejected() {
    // `!must_fill` is not permitted on `$` metadata keys: they are extracted
    // into typed metadata and have no placeholder semantics.
    let markdown = "~~~card-yaml
$quill: !must_fill test_quill
$kind: main
~~~";
    let err = decompose(markdown).unwrap_err().to_string();
    assert!(
        err.contains("`!must_fill`") && err.contains("$quill"),
        "expected !must_fill-on-$ rejection, got: {err}"
    );
}

#[test]
fn test_blank_lines_in_payload() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test Document
author: Test Author

description: This has a blank line above it
tags:
  - one
  - two
~~~

# Hello World

This is the body.";

    let doc = decompose(markdown).unwrap();
    assert_eq!(
        doc.main().body_markdown(),
        "# Hello World\n\nThis is the body."
    );
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Test Document"
    );
    assert_eq!(
        doc.main()
            .payload()
            .get("author")
            .unwrap()
            .as_str()
            .unwrap(),
        "Test Author"
    );
    assert_eq!(
        doc.main()
            .payload()
            .get("description")
            .unwrap()
            .as_str()
            .unwrap(),
        "This has a blank line above it"
    );
    let tags = doc
        .main()
        .payload()
        .get("tags")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(tags.len(), 2);
}

#[test]
fn test_blank_lines_in_scope_blocks() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: items
name: Item 1

price: 19.99

tags:
  - electronics
  - gadgets
~~~

Body of item 1.";

    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.cards().len(), 1);
    let card = &doc.cards()[0];
    assert_eq!(card.kind(), Some("items"));
    assert_eq!(
        card.payload().get("name").unwrap().as_str().unwrap(),
        "Item 1"
    );
    assert_eq!(
        card.payload().get("price").unwrap().as_f64().unwrap(),
        19.99
    );
    let tags = card.payload().get("tags").unwrap().as_array().unwrap();
    assert_eq!(tags.len(), 2);
}

#[test]
fn test_triple_dash_between_paragraphs_is_delegated() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test
~~~

First paragraph.

---

Second paragraph.";

    let doc = decompose(markdown).unwrap();
    let body = doc.main().body_markdown();
    assert!(body.contains("First paragraph."));
    assert!(body.contains("Second paragraph."));
    // `---` is delegated to CommonMark (thematic break / setext underline),
    // never treated as a card fence: the document stays a single card.
    assert!(doc.cards().is_empty(), "--- must not split a card");
}

#[test]
fn test_lone_triple_dash_in_body_is_delegated() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test
~~~

First paragraph.
---

Second paragraph.";

    let doc = decompose(markdown).unwrap();
    let body = doc.main().body_markdown();
    assert!(body.contains("First paragraph."));
    assert!(body.contains("Second paragraph."));
    // `---` is delegated to CommonMark (thematic break / setext underline),
    // never treated as a card fence: the document stays a single card.
    assert!(doc.cards().is_empty(), "--- must not split a card");
}

#[test]
fn test_multiple_blank_lines_in_yaml() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test


author: John Doe


version: 1.0
~~~

Body content.";

    let doc = decompose(markdown).unwrap();
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Test"
    );
    assert_eq!(
        doc.main()
            .payload()
            .get("author")
            .unwrap()
            .as_str()
            .unwrap(),
        "John Doe"
    );
    assert_eq!(
        doc.main()
            .payload()
            .get("version")
            .unwrap()
            .as_f64()
            .unwrap(),
        1.0
    );
}

// --- demo_file_test ---

#[test]
fn test_extended_metadata_demo_file() {
    let markdown = include_str!("../../../../fixtures/resources/extended_metadata_demo.md");
    let doc = decompose(markdown).unwrap();

    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Extended Metadata Demo"
    );
    assert_eq!(
        doc.main()
            .payload()
            .get("author")
            .unwrap()
            .as_str()
            .unwrap(),
        "Quillmark Team"
    );
    // version is parsed as a number by YAML
    assert_eq!(
        doc.main()
            .payload()
            .get("version")
            .unwrap()
            .as_f64()
            .unwrap(),
        1.0
    );

    // Verify body
    assert!(doc
        .main()
        .body_markdown()
        .contains("card-yaml metadata format"));

    // 5 cards total: 3 features + 2 use_cases
    assert_eq!(doc.cards().len(), 5);

    let features_count = doc
        .cards()
        .iter()
        .filter(|c| c.kind() == Some("features"))
        .count();
    let use_cases_count = doc
        .cards()
        .iter()
        .filter(|c| c.kind() == Some("use_cases"))
        .count();
    assert_eq!(features_count, 3);
    assert_eq!(use_cases_count, 2);

    // Check first card is a feature
    assert_eq!(doc.cards()[0].kind(), Some("features"));
    assert_eq!(
        doc.cards()[0]
            .payload()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap(),
        "Tag Directives"
    );
}

#[test]
fn test_input_size_limit() {
    let size = crate::error::MAX_INPUT_SIZE + 1;
    let large_markdown = "a".repeat(size);

    let result = decompose(&large_markdown);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Input too large"));
}

#[test]
fn test_yaml_size_limit() {
    let mut markdown = String::from("~~~card-yaml\n$quill: test_quill\n$kind: main\n");
    let size = crate::error::MAX_YAML_SIZE + 1;
    markdown.push_str("data: \"");
    markdown.push_str(&"x".repeat(size));
    markdown.push_str("\"\n~~~\n\nBody");

    let result = decompose(&markdown);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Input too large"));
}

#[test]
fn test_yaml_depth_limit() {
    let mut yaml_content = String::new();
    for i in 0..110 {
        yaml_content.push_str(&"  ".repeat(i));
        yaml_content.push_str(&format!("level{}: value\n", i));
    }

    let markdown = format!(
        "~~~card-yaml\n$quill: test_quill\n$kind: main\n{}~~~\n\nBody",
        yaml_content
    );
    let result = decompose(&markdown);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.to_lowercase().contains("budget")
            || err_msg.to_lowercase().contains("depth")
            || err_msg.contains("YAML"),
        "Expected depth/budget error, got: {}",
        err_msg
    );
}

// Guillemet preservation tests

/// Guillemet/chevron sequences (`<<...>>`) must survive parsing unmodified in
/// every context: body, YAML string values, YAML arrays, nested maps, code
/// blocks, inline code, and card bodies/fields. A single integrative document
/// exercises all of these.
#[test]
fn test_chevrons_preserved_in_all_contexts() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
title: Test <<with chevrons>>
items:
  - \"<<first>>\"
  - \"<<second>>\"
metadata:
  description: \"<<nested value>>\"
~~~

<<body>> text.

```
<<in code block>>
```

`<<inline code>>` and <<plain>>

~~~card-yaml
$kind: items
description: \"<<card yaml>>\"
~~~

Use <<card body>> here.";

    let doc = decompose(markdown).unwrap();

    // Payload scalar, array, nested map.
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Test <<with chevrons>>"
    );
    let items = doc
        .main()
        .payload()
        .get("items")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(items[0].as_str().unwrap(), "<<first>>");
    assert_eq!(items[1].as_str().unwrap(), "<<second>>");
    let metadata = doc
        .main()
        .payload()
        .get("metadata")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        metadata.get("description").unwrap().as_str().unwrap(),
        "<<nested value>>"
    );

    // Body chevrons in the content projection: code contexts (fenced + inline)
    // protect them verbatim; plain-text `<<word>>` follows CommonMark HTML rules
    // (`<word>` reads as an inline tag). This pins the exact projected body.
    let body = doc.main().body_markdown();
    assert_eq!(
        body,
        "\\<> text.\n\n```\n<<in code block>>\n```\n\n`<<inline code>>` and \\<>"
    );

    // Card yaml (a YAML scalar, never markdown) preserves chevrons verbatim.
    let card = &doc.cards()[0];
    assert_eq!(
        card.payload().get("description").unwrap().as_str().unwrap(),
        "<<card yaml>>"
    );
    assert_eq!(card.body_markdown(), "Use \\<> here.");
}

#[test]
fn test_multiline_chevrons_projection() {
    // A plain-text `<<text ... >>` spanning a line follows CommonMark HTML rules
    // in the content projection. Pin the exact projected body.
    let markdown = "~~~card-yaml\n$quill: test_quill\n$kind: main\n~~~\n\n<<text\nacross lines>>";
    let doc = decompose(markdown).unwrap();
    let body = doc.main().body_markdown();
    assert_eq!(body, "\\<>");
}

#[test]
fn test_unmatched_chevrons_preserved() {
    let markdown = "~~~card-yaml\n$quill: test_quill\n$kind: main\n~~~\n\n<<unmatched";
    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.main().body_markdown(), "\\<\\<unmatched");
}

// Robustness tests

/// CRLF and mixed line endings must parse identically to LF.
#[test]
fn test_line_ending_normalization() {
    for markdown in [
        "~~~card-yaml\r\n$quill: test_quill\r\n$kind: main\r\ntitle: Test\r\n~~~\r\n\r\nBody content.",
        "~~~card-yaml\n$quill: test_quill\r\n$kind: main\r\ntitle: Test\r\n~~~\n\nBody.",
    ] {
        let doc = decompose(markdown).unwrap();
        assert_eq!(
            doc.main().payload().get("title").unwrap().as_str().unwrap(),
            "Test"
        );
    }
}

#[test]
fn test_payload_at_eof_no_trailing_newline() {
    let markdown = "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: Test\n~~~";
    let doc = decompose(markdown).unwrap();
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Test"
    );
    assert_eq!(doc.main().body_markdown(), "");
}

// Unicode handling

#[test]
fn test_unicode_in_yaml_keys() {
    // Unicode is welcome in *values* (next test); field *names* are
    // restricted to ASCII [A-Za-z_][A-Za-z0-9_]* (spec §3.4), so a non-ASCII
    // name is rejected at parse.
    let markdown =
        "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitre: Bonjour\nタイトル: こんにちは\n~~~\n\nBody.";
    let err = decompose(markdown).unwrap_err();
    assert!(
        err.to_string().contains("field names must match"),
        "non-ASCII field name is a parse error: {err}"
    );

    // A conforming name with a Unicode value parses fine.
    let ok = "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitre: こんにちは\n~~~\n";
    let doc = decompose(ok).unwrap();
    assert_eq!(
        doc.main().payload().get("titre").unwrap().as_str().unwrap(),
        "こんにちは"
    );
}

// YAML edge cases

// ── Single-field YAML scalar types (table-driven) ─────────────────────────────
// `|` (literal) and `>` (folded) block scalars are exercised nowhere else in
// the workspace: keep them as explicit rows here.
#[test]
fn single_field_yaml_scalar_types() {
    enum Check {
        StrEq(&'static str),
        StrContains(&'static [&'static str]),
        I64Eq(i64),
        F64Eq(f64),
        BoolEq(bool),
    }

    let cases: &[(&str, &str, &[(&str, Check)])] = &[
        (
            "literal block scalar (`|`)",
            "~~~card-yaml
$quill: test_quill
$kind: main
description: |
  This is a
  multiline string
  with preserved newlines.
~~~

Body.",
            &[(
                "description",
                Check::StrContains(&["multiline string", "\n"]),
            )],
        ),
        (
            "folded block scalar (`>`)",
            "~~~card-yaml
$quill: test_quill
$kind: main
description: >
  This is a folded
  string that becomes
  a single line.
~~~

Body.",
            &[("description", Check::StrContains(&["folded"]))],
        ),
        (
            "empty string",
            "~~~card-yaml\n$quill: test_quill\n$kind: main\nempty: \"\"\n~~~\n\nBody.",
            &[("empty", Check::StrEq(""))],
        ),
        (
            "special characters in a quoted string",
            "~~~card-yaml\n$quill: test_quill\n$kind: main\nspecial: \"colon: here, and [brackets]\"\n~~~\n\nBody.",
            &[(
                "special",
                Check::StrEq("colon: here, and [brackets]"),
            )],
        ),
        (
            "int/float/bool scalars",
            "~~~card-yaml
$quill: test_quill
$kind: main
count: 42
price: 19.99
active: true
items:
  - first
  - 100
  - true
~~~

Body.",
            &[
                ("count", Check::I64Eq(42)),
                ("price", Check::F64Eq(19.99)),
                ("active", Check::BoolEq(true)),
            ],
        ),
    ];

    for (label, markdown, fields) in cases {
        let doc = decompose(markdown).unwrap_or_else(|e| panic!("{label}: parse failed: {e}"));
        for (key, check) in *fields {
            let v = doc
                .main()
                .payload()
                .get(key)
                .unwrap_or_else(|| panic!("{label}: missing field {key:?}"));
            match check {
                Check::StrEq(s) => assert_eq!(v.as_str().unwrap(), *s, "{label}: {key}"),
                Check::StrContains(needles) => {
                    let s = v.as_str().unwrap();
                    for n in *needles {
                        assert!(s.contains(n), "{label}: {key} missing {n:?} in {s:?}");
                    }
                }
                Check::I64Eq(i) => assert_eq!(v.as_i64().unwrap(), *i, "{label}: {key}"),
                Check::F64Eq(f) => assert_eq!(v.as_f64().unwrap(), *f, "{label}: {key}"),
                Check::BoolEq(b) => assert_eq!(v.as_bool().unwrap(), *b, "{label}: {key}"),
            }
        }
    }
}

#[test]
fn test_yaml_nested_objects() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
config:
  database:
    host: localhost
    port: 5432
  cache:
    enabled: true
~~~

Body.";
    let doc = decompose(markdown).unwrap();
    let config = doc
        .main()
        .payload()
        .get("config")
        .unwrap()
        .as_object()
        .unwrap();
    let db = config.get("database").unwrap().as_object().unwrap();
    assert_eq!(db.get("host").unwrap().as_str().unwrap(), "localhost");
    assert_eq!(db.get("port").unwrap().as_i64().unwrap(), 5432);
}

// Card block edge cases

#[test]
fn test_card_with_body_containing_dashes() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
~~~

~~~card-yaml
$kind: items
name: Item
~~~

Some text with --- dashes in it.";
    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.cards().len(), 1);
    assert!(doc.cards()[0].body_markdown().contains("--- dashes"));
}

// Error handling

#[test]
fn test_invalid_card_kind_names_are_rejected() {
    // `$kind` is name-validated at parse time against `[a-z_][a-z0-9_]*`.
    for kind in ["ITEMS", "123items", "my-items", "Invalid-Name", ""] {
        let markdown = format!(
            "~~~card-yaml\n$quill: test_quill\n$kind: main\n~~~\n\n~~~card-yaml\n$kind: {kind}\n~~~\n\nBody."
        );
        let err = decompose(&markdown).unwrap_err().to_string();
        assert!(
            err.contains("Invalid `$kind`"),
            "kind {kind:?} should be rejected; got: {err}"
        );
    }
}

#[test]
fn test_yaml_syntax_error_missing_colon() {
    let markdown = "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle Test\n~~~\n\nBody.";
    let result = decompose(markdown);
    assert!(result.is_err());
}

// Body extraction edge cases

#[test]
fn test_body_with_leading_newlines() {
    // The body is a content; markdown is its projection. Leading blank lines are
    // canonicalized away at import, so the emitted body does not carry them.
    let markdown =
        "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: Test\n~~~\n\n\n\nBody with leading newlines.";
    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.main().body_markdown(), "Body with leading newlines.");
}

#[test]
fn test_body_with_trailing_newlines() {
    // Body at EOF: no blank-line separator to strip, source's trailing
    // newlines are preserved verbatim as authored content.
    let markdown = "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: Test\n~~~\n\nBody.\n\n\n";
    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.main().body_markdown(), "Body.");
}

// ── Blank-line separator stripping: parse-side normalisation ──────────────────
// See `assemble.rs::strip_blank_separator` and `markdown-spec.md §4` (rule D1).

#[test]
fn test_blank_separator_strip_global_body_followed_by_card_lf() {
    // Global body followed by a card block: the source's tail `\n\n` is
    // (content line terminator) + (blank-line separator). Strip exactly the
    // separator `\n`, leaving `\n` as the content terminator.
    let markdown =
        "~~~card-yaml\n$quill: q\n$kind: main\n~~~\n\nbody\n\n~~~card-yaml\n$kind: x\n~~~\n";
    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.main().body_markdown(), "body");
}

#[test]
fn test_blank_separator_strip_global_body_followed_by_card_crlf() {
    // CRLF line endings: strip exactly one `\r\n` as the blank-line separator.
    let markdown =
        "~~~card-yaml\r\n$quill: q\r\n$kind: main\r\n~~~\r\n\r\nbody\r\n\r\n~~~card-yaml\r\n$kind: x\r\n~~~\r\n";
    let doc = decompose(markdown).unwrap();
    // CRLF normalizes to LF; the blank-line separator is stripped and the value
    // projection carries no trailing newline: identical to the `_lf` sibling.
    assert_eq!(doc.main().body_markdown(), "body");
}

#[test]
fn test_blank_separator_strip_card_body_followed_by_card() {
    // First card body is followed by another fence → separator stripped.
    // Last card body is at EOF → preserved verbatim.
    let markdown = "~~~card-yaml\n$quill: q\n$kind: main\n~~~\n\n~~~card-yaml\n$kind: a\n~~~\n\nfirst\n\n~~~card-yaml\n$kind: b\n~~~\n\nsecond\n";
    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.cards()[0].body_markdown(), "first");
    assert_eq!(doc.cards()[1].body_markdown(), "second");
}

#[test]
fn test_blank_separator_strip_preserves_author_blank_lines() {
    // Author wrote two blank lines after the body. Only the blank-line
    // separator (last `\n`) is stripped; the author's blank line is preserved.
    let markdown =
        "~~~card-yaml\n$quill: q\n$kind: main\n~~~\n\nbody\n\n\n~~~card-yaml\n$kind: x\n~~~\n";
    let doc = decompose(markdown).unwrap();
    assert_eq!(doc.main().body_markdown(), "body");
}

#[test]
fn test_f2_strip_does_not_overstrip_content_newlines() {
    // Content-fidelity: a body whose authored content ends with multiple
    // newlines (e.g. a code block with trailing blank lines) must survive
    // round-trip.
    let markdown =
        "~~~card-yaml\n$quill: q\n$kind: main\n~~~\n\n```\ncode\n```\n\n\n~~~card-yaml\n$kind: x\n~~~\n";
    let doc = decompose(markdown).unwrap();
    let emitted = doc.to_markdown();
    let reparsed = Document::parse(&emitted).unwrap().document;
    assert_eq!(doc.main().body_markdown(), reparsed.main().body_markdown());
    // The code block content survives; trailing blank lines are dropped (the
    // content projection is a value, not a file, so it carries no final newline).
    assert!(
        doc.main().body_markdown().ends_with("```"),
        "expected code block, got {:?}",
        doc.main().body_markdown()
    );
}

// Kind name validation

// Guillemet preprocessing

#[test]
fn test_guillemet_double_conversion_prevention() {
    let markdown =
        "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: Already «converted»\n~~~\n\nBody.";
    let doc = decompose(markdown).unwrap();
    assert_eq!(
        doc.main().payload().get("title").unwrap().as_str().unwrap(),
        "Already «converted»"
    );
}

#[test]
fn test_allowed_card_field_collision() {
    let markdown = "~~~card-yaml
$quill: test_quill
$kind: main
my_card: \"some global value\"
~~~

~~~card-yaml
$kind: my_card
title: \"My Card\"
~~~

Body
";
    let doc = decompose(markdown).unwrap();
    assert_eq!(
        doc.main()
            .payload()
            .get("my_card")
            .unwrap()
            .as_str()
            .unwrap(),
        "some global value"
    );
    assert_eq!(doc.cards().len(), 1);
    assert_eq!(doc.cards()[0].kind(), Some("my_card"));
    assert_eq!(
        doc.cards()[0]
            .payload()
            .get("title")
            .unwrap()
            .as_str()
            .unwrap(),
        "My Card"
    );
}

// ── to_plate_json round-trip snapshot ─────────────────────────────────────────

/// Verify to_plate_json produces the correct shape for a simple document.
#[test]
fn test_to_plate_json_simple() {
    let doc = Document::parse(
        "~~~card-yaml\n$quill: my_quill\n$kind: main\ntitle: Hello\n~~~\n\nBody text.\n",
    )
    .unwrap()
    .document;
    let json = doc.to_plate_json();

    assert_eq!(json["$quill"], "my_quill");
    assert_eq!(json["title"], "Hello");
    // `$body` crosses the seam as canonical content JSON, not a markdown string.
    assert_eq!(json["$body"]["text"], "Body text.");
    assert!(json["$body"]["lines"].is_array());
    assert!(json["$cards"].is_array());
    assert_eq!(json["$cards"].as_array().unwrap().len(), 0);
}

/// to_plate_json with cards produces a `$cards` array containing `$kind`,
/// fields, and `$body`.
#[test]
fn test_to_plate_json_with_cards() {
    let markdown = "~~~card-yaml
$quill: usaf_memo
$kind: main
title: Test
~~~

Global body.

~~~card-yaml
$kind: indorsement
for: ORG
~~~

Card body here.
";
    let doc = Document::parse(markdown).unwrap().document;
    let json = doc.to_plate_json();

    assert_eq!(json["$quill"], "usaf_memo");
    assert_eq!(json["title"], "Test");
    // `$body` (global and per-card) crosses as canonical content JSON; its `text`
    // is the content-only string (blank-line separator stripped on parse).
    assert_eq!(json["$body"]["text"], "Global body.");

    let cards = json["$cards"].as_array().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0]["$kind"], "indorsement");
    assert_eq!(cards[0]["for"], "ORG");
    assert_eq!(cards[0]["$body"]["text"], "Card body here.");
}

/// A kindless composable card (no `$kind`) carries no `$kind` key in the raw
/// plate: the serializer never fabricates `$kind: ""` (issue 1030, "absent on
/// undefined"). The schema-free serializer still emits `$body` for every card;
/// the schema-gated body drop is the render plate's job (`compile_data`).
#[test]
fn test_to_plate_json_kindless_card_omits_kind() {
    use crate::{Card, Payload};

    let mut doc = Document::parse("~~~card-yaml\n$quill: my_quill\n$kind: main\n~~~\n\nBody.\n")
        .unwrap()
        .document;
    doc.cards_vec_mut().push(Card::from_parts(
        Payload::new(),
        quillmark_content::Content::empty(),
    ));

    let json = doc.to_plate_json();
    let card = &json["$cards"][0];
    assert!(
        card.get("$kind").is_none(),
        "a kindless card must carry no $kind: {card}"
    );
    assert!(
        card.get("$body").is_some(),
        "the schema-free serializer still emits $body: {card}"
    );
}

/// to_plate_json parity: the `$quill` key appears first.
#[test]
fn test_to_plate_json_quill_first() {
    let doc = Document::parse(
        "~~~card-yaml\n$quill: my_quill\n$kind: main\nfoo: bar\nbaz: qux\n~~~\n",
    )
    .unwrap()
    .document;
    let json = doc.to_plate_json();
    let obj = json.as_object().unwrap();
    let keys: Vec<&String> = obj.keys().collect();
    assert_eq!(keys[0], "$quill");
}

/// Regression test for the `serde_json::Map::remove` / `shift_remove` bug.
///
/// `serde_json::Map::remove` with `preserve_order` uses `swap_remove` under
/// the hood (O(1), moves last element into removed slot): NOT the order-
/// preserving `shift_remove` (O(n)).  Payload field order must be
/// preserved.
#[test]
fn payload_field_order_preserved_after_quill_removal() {
    let md = "~~~card-yaml\n$quill: q\n$kind: main\nsender: Alice\nrecipient: Bob\ndate: March 15\nsubject: hi\n~~~\n";
    let doc = Document::parse(md).unwrap().document;
    let keys: Vec<&str> = doc.main().payload().keys().map(|s| s.as_str()).collect();
    // Fields must appear in YAML document order, not alphabetical or swap-order.
    assert_eq!(
        keys,
        vec!["sender", "recipient", "date", "subject"],
        "Payload fields must preserve insertion order"
    );
}

// ── `$id` is not a system key ────────────────────────────────────────────────

#[test]
fn card_id_is_rejected_as_an_unknown_system_key() {
    // `$id` is not a system key: it is rejected like any other unknown `$`
    // key, on a composable card and on the root alike.
    let md = "~~~\n$quill: q@0.1\n~~~\n\n~~~\n$kind: note\n$id: a\n~~~\n";
    let err = Document::parse(md).unwrap_err().to_string();
    assert!(
        err.contains("Unknown `$id`"),
        "expected unknown-key parse error, got: {err}"
    );

    let root = "~~~\n$quill: q@0.1\n$id: x\n~~~\n";
    assert!(Document::parse(root)
        .unwrap_err()
        .to_string()
        .contains("Unknown `$id`"));
}

#[test]
fn a_user_field_named_id_is_untouched() {
    // The unsigiled `id` is an ordinary user field.
    let md = "~~~\n$quill: q@0.1\n~~~\n\n~~~\n$kind: note\nid: a\n~~~\n";
    let doc = Document::parse(md).unwrap().document;
    assert_eq!(
        doc.cards()[0].payload().get("id").map(|v| v.as_json().clone()),
        Some(serde_json::json!("a"))
    );
}
