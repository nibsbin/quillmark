//! Round-trip tests for comments, `!must_fill`, and custom tags.
//!
//! Both own-line and trailing inline YAML comments round-trip at their
//! source position. Own-line comments in a block's payload (below the
//! `$quill` / `$kind` metadata header) also round-trip. Comments whose
//! host disappears at emit
//! time (empty-mapping omission, programmatic field removal) degrade to
//! own-line comments at the same indent so the comment text is preserved
//! even when its position shifts. `!must_fill` on scalars and sequences round-
//! trips. String quoting is normalised to saphyr's canonical form (plain
//! when safe, quoted when the value would otherwise be misread on
//! re-parse) — type fidelity is guaranteed; the exact quoting style is
//! not.

use crate::document::Document;

// ── Category: Block scalars ───────────────────────────────────────────────────

/// A block-scalar value (markdown) whose content contains `#` heading lines
/// round-trips intact. Regression guard: the prescan comment-stripper must not
/// treat `#`-leading lines inside literal blocks as YAML comments, which would
/// silently delete markdown headings (and mis-read `- ` / `key:` lines).
#[test]
fn block_scalar_with_markdown_headings_round_trips() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nbio: |-\n  ## About me\n\n  - first point\n  Plain line.\ntitle: Resume\n~~~\n";

    let doc = Document::parse(src).unwrap().document;
    assert_eq!(
        doc.main().payload().get("bio").and_then(|v| v.as_str()),
        Some("## About me\n\n- first point\nPlain line."),
        "markdown heading / bullet / plain lines inside a block scalar must survive parse",
    );
    // The block ended; the field after it parses normally.
    assert_eq!(
        doc.main().payload().get("title").and_then(|v| v.as_str()),
        Some("Resume"),
    );

    // Full round-trip is value-stable and idempotent.
    let emitted = doc.to_markdown();
    let doc2 = Document::parse(&emitted).unwrap().document;
    assert_eq!(
        doc2.main().payload().get("bio").and_then(|v| v.as_str()),
        Some("## About me\n\n- first point\nPlain line."),
        "block-scalar content must survive a full round-trip\nGot:\n{}",
        emitted
    );
    assert_eq!(emitted, doc2.to_markdown(), "round-trip must be idempotent");
}

/// An array authored as `- |` block-scalar items (a `richtext[]` field)
/// preserves `#` heading / `- ` bullet content per element. Regression for the
/// sequence-item path of the block-scalar prescan fix.
#[test]
fn block_scalar_sequence_items_round_trip() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nsections:\n  - |-\n    ## First\n    body one\n  - |-\n    ## Second\n    body two\n~~~\n";

    let doc = Document::parse(src).unwrap().document;
    let arr = doc
        .main()
        .payload()
        .get("sections")
        .and_then(|v| v.as_array())
        .expect("sections array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_str(), Some("## First\nbody one"));
    assert_eq!(arr[1].as_str(), Some("## Second\nbody two"));
}

// ── Category: Custom tags ─────────────────────────────────────────────────────

/// `!must_fill` tags round-trip; other custom tags are rejected with a warning
/// and the tag is dropped.
#[test]
fn custom_tags_lose_tag_but_keep_value() {
    // `!must_fill` case: round-trip with fill preserved.
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nmemo_from: !must_fill 2d lt example\n~~~\n";
    let doc = Document::parse(src).unwrap().document;

    let fm = doc.main().payload();
    assert_eq!(
        fm.get("memo_from").and_then(|v| v.as_str()),
        Some("2d lt example"),
        "string value must survive tag parsing"
    );
    assert!(fm.is_fill("memo_from"), "fill marker must be recorded");

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("memo_from: !must_fill"),
        "`!must_fill` tag must round-trip\nGot:\n{}",
        emitted
    );

    let doc2 = Document::parse(&emitted).unwrap().document;
    assert!(
        doc2.main().payload().is_fill("memo_from"),
        "fill marker must survive a full round-trip"
    );

    // Non-`!must_fill` tag case: warning + dropped tag.
    let src2 = "~~~card-yaml\n$quill: q\n$kind: main\nmemo_from: !include value.txt\n~~~\n";
    let out = Document::parse(src2).unwrap();
    assert!(
        out.warnings
            .iter()
            .any(|w| w.code.as_deref() == Some("parse::unsupported_yaml_tag")),
        "expected unsupported_yaml_tag warning; got: {:?}",
        out.warnings
    );
    let emitted2 = out.document.to_markdown();
    assert!(
        !emitted2.contains("!include"),
        "unknown tag must not re-appear on emit\nGot:\n{}",
        emitted2
    );
}

/// `!fill` is not an alias for `!must_fill`: it is treated like any other
/// noncanonical tag — dropped with an unsupported-tag warning, the value kept,
/// and no fill marker recorded.
#[test]
fn fill_tag_is_rejected_as_noncanonical() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nmemo_from: !fill draft\n~~~\n";
    let out = Document::parse(src).unwrap();

    let fm = out.document.main().payload();
    assert_eq!(fm.get("memo_from").and_then(|v| v.as_str()), Some("draft"));
    assert!(
        !fm.is_fill("memo_from"),
        "`!fill` must not record a fill marker"
    );
    assert!(
        out.warnings
            .iter()
            .any(|w| w.code.as_deref() == Some("parse::unsupported_yaml_tag")),
        "`!fill` must warn as an unsupported tag; got: {:?}",
        out.warnings
    );

    // The dropped tag means a plain value: no `!must_fill`, no `!fill` on emit.
    let emitted = out.document.to_markdown();
    assert!(
        !emitted.contains("!fill") && !emitted.contains("!must_fill"),
        "rejected tag must not reappear on emit\nGot:\n{}",
        emitted
    );
}

/// `!must_fill` in a position prescan cannot lift — inside a flow collection
/// or on a bare sequence element — would be silently dropped by the YAML
/// parser, so it is reported with a warning rather than lost quietly.
/// Block-style nested markers (the supported form) do not warn.
#[test]
fn unsupported_fill_position_warns_not_silently_dropped() {
    let code = "parse::fill_marker_unsupported_position";
    let warns = |src: &str| {
        Document::parse(src)
            .unwrap()
            .warnings
            .iter()
            .any(|w| w.code.as_deref() == Some(code))
    };

    // Flow collection containing a marker.
    assert!(
        warns("~~~card-yaml\n$quill: q\n$kind: main\naddr: {street: !must_fill, city: x}\n~~~\n"),
        "flow-collection marker must warn"
    );
    // Bare sequence element.
    assert!(
        warns("~~~card-yaml\n$quill: q\n$kind: main\ntags:\n  - !must_fill\n  - a\n~~~\n"),
        "bare sequence-element marker must warn"
    );
    // Supported block-style nested marker must NOT warn.
    assert!(
        !warns(
            "~~~card-yaml\n$quill: q\n$kind: main\naddr:\n  street: !must_fill\n  city: x\n~~~\n"
        ),
        "block-style nested marker must not warn"
    );
    // A quoted scalar that merely contains the literal text must NOT warn.
    assert!(
        !warns("~~~card-yaml\n$quill: q\n$kind: main\nnote: \"see !must_fill docs\"\n~~~\n"),
        "quoted literal must not warn"
    );
}

/// `!must_fill` on a nested mapping leaf is captured on that leaf's tree
/// node and round-trips at the correct depth.
#[test]
fn nested_must_fill_round_trips() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\naddr:\n  street: !must_fill\n  city: Springfield\n~~~\n";
    let doc = Document::parse(src).unwrap().document;

    let fm = doc.main().payload();
    let addr = fm.get("addr").unwrap();
    assert!(
        addr.get("street").unwrap().fill(),
        "nested `street` must carry the fill marker"
    );
    assert!(!addr.get("city").unwrap().fill(), "`city` must not");
    assert_eq!(addr.get("city").unwrap().as_str(), Some("Springfield"));
    // The marker is on the leaf node, not the parent object.
    assert!(!addr.fill());

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("street: !must_fill") && emitted.contains("city: Springfield"),
        "nested fill must round-trip at depth\nGot:\n{}",
        emitted
    );

    let doc2 = Document::parse(&emitted).unwrap().document;
    assert!(doc2
        .main()
        .payload()
        .get("addr")
        .unwrap()
        .get("street")
        .unwrap()
        .fill());
}

/// `!must_fill` on a nested mapping (object-valued node) is rejected, the
/// same as at the top level.
#[test]
fn nested_must_fill_on_mapping_is_rejected() {
    let src =
        "~~~card-yaml\n$quill: q\n$kind: main\nouter:\n  inner: !must_fill\n    leaf: 1\n~~~\n";
    let err = Document::parse(src);
    assert!(
        err.is_err(),
        "`!must_fill` on an object-valued nested key must be rejected"
    );
}

/// `!must_fill` on a top-level mapping is rejected at parse.
#[test]
fn fill_tag_mapping_rejected() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nx: !must_fill {a: 1}\n~~~\n";
    let err = Document::parse(src).unwrap_err();
    assert!(
        err.to_string().contains("!must_fill") && err.to_string().contains("mapping"),
        "expected mapping-rejection error; got: {}",
        err
    );
}

/// `!must_fill` round-trips across every supported shape: each scalar type
/// on a shared document (including the bare-null form, `key: !must_fill`
/// with no value), and top-level sequences in block, flow, and empty form.
/// Sequences get their own sibling table below — each needs its own document
/// and a full-document equality check, which doesn't fit the shared-document
/// loop the scalar types use.
#[test]
fn fill_tag_all_scalar_types_round_trip() {
    let src = concat!(
        "~~~card-yaml\n$quill: q\n$kind: main\n",
        "s: !must_fill hello\n",
        "i: !must_fill 42\n",
        "f: !must_fill 3.14\n",
        "b: !must_fill true\n",
        "n: !must_fill\n",
        "~~~\n",
    );

    let doc = Document::parse(src).unwrap().document;
    let fm = doc.main().payload();

    assert_eq!(fm.get("s").and_then(|v| v.as_str()), Some("hello"));
    assert_eq!(fm.get("i").and_then(|v| v.as_i64()), Some(42));
    #[allow(clippy::approx_constant)]
    let expected_f = 3.14;
    assert_eq!(fm.get("f").and_then(|v| v.as_f64()), Some(expected_f));
    assert_eq!(fm.get("b").and_then(|v| v.as_bool()), Some(true));
    assert!(fm.get("n").map(|v| v.is_null()).unwrap_or(false));

    for key in ["s", "i", "f", "b", "n"] {
        assert!(fm.is_fill(key), "{} must be fill-tagged", key);
    }

    let emitted = doc.to_markdown();
    // Bare-null shape (`n`): emits `key: !must_fill` with no trailing value,
    // same shape a standalone top-level bare-null field takes.
    assert!(
        emitted.contains("n: !must_fill\n"),
        "bare `!must_fill` must round-trip as `key: !must_fill`\nGot:\n{}",
        emitted
    );

    let doc2 = Document::parse(&emitted).unwrap().document;
    for key in ["s", "i", "f", "b", "n"] {
        assert!(
            doc2.main().payload().is_fill(key),
            "{} must remain fill-tagged after round-trip",
            key
        );
    }

    // Sequence shapes: block, flow (normalises to block on emit), and empty.
    // Each needs its own document and a full-document equality check, so they
    // get a sibling loop rather than a row in the scalar loop above.
    struct SeqCase {
        label: &'static str,
        key: &'static str,
        src: &'static str,
        expected_items: &'static [&'static str],
        emitted_contains: &'static str,
    }

    let seq_cases = [
        SeqCase {
            label: "block sequence",
            key: "recipient",
            src: "~~~card-yaml\n$quill: q\n$kind: main\nrecipient: !must_fill\n  - Dr. Who\n  - 1 TARDIS Lane\n~~~\n",
            expected_items: &["Dr. Who", "1 TARDIS Lane"],
            emitted_contains: "recipient: !must_fill\n",
        },
        SeqCase {
            label: "flow sequence normalises to block form",
            key: "tags",
            src: "~~~card-yaml\n$quill: q\n$kind: main\ntags: !must_fill [a, b, c]\n~~~\n",
            expected_items: &["a", "b", "c"],
            emitted_contains: "tags: !must_fill",
        },
        SeqCase {
            label: "empty sequence",
            key: "items",
            src: "~~~card-yaml\n$quill: q\n$kind: main\nitems: !must_fill []\n~~~\n",
            expected_items: &[],
            emitted_contains: "items: !must_fill []\n",
        },
    ];

    for case in seq_cases {
        let doc = Document::parse(case.src).unwrap().document;
        let fm = doc.main().payload();
        assert!(
            fm.is_fill(case.key),
            "[{}] key must be fill-tagged",
            case.label
        );

        let arr = fm.get(case.key).and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            arr.len(),
            case.expected_items.len(),
            "[{}] array length",
            case.label
        );
        for (item, expected) in arr.iter().zip(case.expected_items) {
            assert_eq!(
                item.as_str(),
                Some(*expected),
                "[{}] array element",
                case.label
            );
        }

        let emitted = doc.to_markdown();
        assert!(
            emitted.contains(case.emitted_contains),
            "[{}] must emit `{}`\nGot:\n{}",
            case.label,
            case.emitted_contains,
            emitted
        );

        let doc2 = Document::parse(&emitted).unwrap().document;
        assert!(
            doc2.main().payload().is_fill(case.key),
            "[{}] fill marker must survive round-trip",
            case.label
        );
        assert_eq!(doc2, doc, "[{}] full round-trip must be equal", case.label);
    }
}

// ── Category: Canonical quoting style ────────────────────────────────────────

/// Quoting style is normalized on emit — saphyr picks the canonical form
/// (plain when safe, quoted when the unquoted form would be re-parsed as
/// the wrong type). The original quoting in the source is not preserved,
/// but values survive round-trip with type fidelity, which is what
/// callers actually depend on.
#[test]
fn quoting_normalises_to_canonical_form_with_type_fidelity() {
    // Mix of single-quoted, unquoted, and double-quoted strings — all of
    // them safe-to-emit-plain after parse.
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nsingle_q: 'hello'\nunquoted: world\ndouble_q: \"already\"\nambiguous: \"on\"\nnumeric_str: \"01234\"\n~~~\n";

    let doc = Document::parse(src).unwrap().document;
    let emitted = doc.to_markdown();

    // Source-side single-quoting is dropped; safe strings emit plain.
    assert!(
        !emitted.contains("'hello'"),
        "original single-quote style must not survive\nGot:\n{}",
        emitted
    );

    // Ambiguous values stay quoted so they re-parse as strings.
    assert!(
        emitted.contains("\"on\"") || emitted.contains("'on'"),
        "ambiguous string `on` must stay quoted\nGot:\n{}",
        emitted
    );
    assert!(
        emitted.contains("\"01234\"") || emitted.contains("'01234'"),
        "numeric-looking string `01234` must stay quoted\nGot:\n{}",
        emitted
    );

    // Values survive the full round-trip with the right types.
    let doc2 = Document::parse(&emitted).unwrap().document;
    for (key, expected) in [
        ("single_q", "hello"),
        ("unquoted", "world"),
        ("double_q", "already"),
        ("ambiguous", "on"),
        ("numeric_str", "01234"),
    ] {
        assert_eq!(
            doc2.main().payload().get(key).and_then(|v| v.as_str()),
            Some(expected),
            "field {key} must round-trip as string {expected:?}",
        );
    }

    // And emission is idempotent: a second round-trip is byte-equal.
    let emitted2 = doc2.to_markdown();
    assert_eq!(emitted, emitted2, "round-trip must be idempotent");
}

// ── Category: Comment positions round-trip ───────────────────────────────────

/// A comment placed at each of these structural positions in a card-yaml
/// document survives a markdown round-trip: parse, emit, and confirm the
/// comment (and any position-specific marker) lands where it should; then
/// re-parse the emission and re-emit — a stable second round confirms the
/// first is idempotent.
#[test]
fn comment_position_round_trips() {
    struct Case {
        label: &'static str,
        src: &'static str,
        /// Substrings that must appear in the first emission.
        contains: &'static [&'static str],
        /// Substrings that must NOT appear (guards against a comment
        /// degrading to a different position, e.g. inline -> own-line).
        not_contains: &'static [&'static str],
        /// A field whose string value must remain intact after the
        /// round-trip (guards against comment parsing corrupting the value).
        value_check: Option<(&'static str, &'static str)>,
        /// This case must not raise the nested-comments-dropped warning.
        no_drop_warning: bool,
    }

    let cases = [
        Case {
            label: "top-level own-line comment",
            src: "~~~card-yaml\n$quill: q\n$kind: main\n# recipient's full name\nrecipient: Jane\nauthor: Alice\n~~~\n\nBody.\n",
            contains: &["# recipient's full name"],
            not_contains: &[],
            value_check: Some(("recipient", "Jane")),
            no_drop_warning: false,
        },
        Case {
            label: "top-level trailing inline comment",
            src: "~~~card-yaml\n$quill: q\n$kind: main\ntitle: My Document # this is a comment\n~~~\n\nBody.\n",
            contains: &["title: My Document # this is a comment"],
            not_contains: &["My Document\n# this is a comment"],
            value_check: Some(("title", "My Document")),
            no_drop_warning: false,
        },
        Case {
            label: "nested sequence comments (leading/between/trailing)",
            src: "~~~card-yaml\n$quill: q\n$kind: main\nitems:\n  # before-first\n  - a\n  # between\n  - b\n  # after-last\n~~~\n",
            contains: &["# before-first", "# between", "# after-last"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: true,
        },
        Case {
            label: "nested mapping comments (leading/trailing)",
            src: "~~~card-yaml\n$quill: q\n$kind: main\nouter:\n  # leading\n  inner: 1\n  # trailing\n~~~\n",
            contains: &["# leading", "# trailing"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: false,
        },
        Case {
            label: "nested sequence item trailing inline comment",
            src: "~~~card-yaml\n$quill: q\n$kind: main\nitems:\n  - a # inline\n  - b\n~~~\n",
            contains: &["- a # inline"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: false,
        },
        Case {
            label: "nested mapping field trailing inline comment",
            src: "~~~card-yaml\n$quill: q\n$kind: main\nouter:\n  inner: 1 # tail\n~~~\n",
            contains: &["inner: 1 # tail"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: false,
        },
        Case {
            label: "inline comment on a container key",
            src: "~~~card-yaml\n$quill: q\n$kind: main\nouter: # describes outer\n  inner: 1\n~~~\n",
            contains: &["outer: # describes outer\n  inner: 1"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: false,
        },
        Case {
            label: "own-line comment below $quill header (root payload)",
            src: "~~~card-yaml\n$quill: q\n$kind: main\n# main entry\ntitle: Hi\n~~~\n",
            contains: &["~~~\n$quill: q\n$kind: main\n# main entry\n"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: false,
        },
        Case {
            label: "own-line comment below $kind header (card payload)",
            src: "~~~card-yaml\n$quill: q\n$kind: main\n~~~\n\n~~~card-yaml\n$kind: foo\n# the foo card\nx: 1\n~~~\n",
            contains: &["~~~\n$kind: foo\n# the foo card\n"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: false,
        },
        Case {
            label: "own-line comments flanking an inline comment",
            src: "~~~card-yaml\n$quill: q\n$kind: main\n# header\ntitle: Hi # tail\n# footer\n~~~\n",
            contains: &["# header\n", "title: Hi # tail\n", "# footer\n"],
            not_contains: &[],
            value_check: None,
            no_drop_warning: false,
        },
    ];

    for case in cases {
        let out = Document::parse(case.src).unwrap();
        if case.no_drop_warning {
            assert!(
                !out.warnings
                    .iter()
                    .any(|w| w.code.as_deref() == Some("parse::comments_in_nested_yaml_dropped")),
                "[{}] no dropped-comment warning expected; nested comments are now preserved",
                case.label
            );
        }

        let emitted = out.document.to_markdown();
        for needle in case.contains {
            assert!(
                emitted.contains(needle),
                "[{}] comment must survive round-trip at its position\nGot:\n{}",
                case.label,
                emitted
            );
        }
        for needle in case.not_contains {
            assert!(
                !emitted.contains(needle),
                "[{}] comment must not degrade to a different position\nGot:\n{}",
                case.label,
                emitted
            );
        }

        let doc2 = Document::parse(&emitted).unwrap().document;
        if let Some((field, expected)) = case.value_check {
            assert_eq!(
                doc2.main().payload().get(field).and_then(|v| v.as_str()),
                Some(expected),
                "[{}] value must remain intact after round-trip",
                case.label
            );
        }

        let emitted2 = doc2.to_markdown();
        assert_eq!(
            emitted, emitted2,
            "[{}] round-trip must be idempotent",
            case.label
        );
    }
}

/// Inline comment with `!must_fill` round-trips with the tag intact.
#[test]
fn fill_with_inline_comment_round_trips() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\ndept: !must_fill Sales # placeholder\n~~~\n";

    let doc = Document::parse(src).unwrap().document;
    assert!(
        doc.main().payload().is_fill("dept"),
        "fill marker must be set"
    );

    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("dept: !must_fill Sales # placeholder"),
        "`!must_fill` and inline comment must round-trip together\nGot:\n{}",
        emitted
    );

    let doc2 = Document::parse(&emitted).unwrap().document;
    let emitted2 = doc2.to_markdown();
    assert_eq!(emitted, emitted2, "round-trip must be idempotent");
}

/// Multiple inline comments — top-level scalar, nested scalar, sequence
/// item — all preserved in one document.
#[test]
fn mixed_inline_comments_round_trip() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\ntitle: Hello # greeting\nitems:\n  - a # first\n  - b\nouter:\n  inner: 1 # nested tail\n~~~\n";

    let doc = Document::parse(src).unwrap().document;
    let emitted = doc.to_markdown();

    assert!(emitted.contains("title: Hello # greeting"));
    assert!(emitted.contains("- a # first"));
    assert!(emitted.contains("inner: 1 # nested tail"));

    let doc2 = Document::parse(&emitted).unwrap().document;
    let emitted2 = doc2.to_markdown();
    assert_eq!(emitted, emitted2, "round-trip must be idempotent");
}

/// Orphan inline comment whose host is removed via `Payload::remove`
/// degrades to an own-line comment instead of being silently dropped.
#[test]
fn orphan_inline_after_remove_degrades_to_own_line() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nfield: value # tail\nother: 2\n~~~\n";

    let mut doc = Document::parse(src).unwrap().document;
    // Remove the host field. The inline comment is now orphaned in items.
    doc.main_mut().payload_mut().remove("field");

    let emitted = doc.to_markdown();
    // Comment text preserved as own-line.
    assert!(
        emitted.contains("# tail"),
        "orphan comment text must be preserved\nGot:\n{}",
        emitted
    );
    // It must NOT have ended up inline on any value line.
    assert!(
        !emitted.contains("\" # tail"),
        "orphan comment must not appear inline on another line\nGot:\n{}",
        emitted
    );

    // Re-parsing the emitted form yields a stable round-trip.
    let doc2 = Document::parse(&emitted).unwrap().document;
    let emitted2 = doc2.to_markdown();
    assert_eq!(
        emitted, emitted2,
        "post-orphan round-trip must be idempotent"
    );
}

/// Inline comment on an empty-mapping field — the field is omitted on emit
/// per the canonical-emission rule, but the inline trailer survives as an
/// own-line comment at the same indent so its text is not lost.
#[test]
fn inline_on_empty_mapping_degrades_to_own_line() {
    use crate::QuillValue;

    // Construct programmatically since `key: {}` doesn't appear in source.
    let src = "~~~card-yaml\n$quill: q\n$kind: main\n~~~\n";
    let mut doc = Document::parse(src).unwrap().document;
    let _ = doc
        .main_mut()
        .payload_mut()
        .insert("empty", QuillValue::from_json(serde_json::json!({})));
    // Append an inline comment item right after the empty-mapping field.
    {
        let fm = doc.main_mut().payload_mut();
        let items = fm.items().to_vec();
        let mut new_items = items;
        new_items.push(crate::PayloadItem::comment_inline("notes about empty"));
        *fm = crate::document::Payload::from_items(new_items);
    }

    let emitted = doc.to_markdown();
    // Empty-mapping host is omitted. Trailer surfaces as own-line.
    assert!(
        !emitted.contains("empty:"),
        "empty mapping must be omitted\nGot:\n{}",
        emitted
    );
    assert!(
        emitted.contains("# notes about empty"),
        "inline trailer for an omitted host must degrade to own-line\nGot:\n{}",
        emitted
    );
}

/// A `!must_fill` marker on the *first* key of a sequence-item mapping
/// (`- key: !must_fill`) sits on the dash line, which Case 4 never sees.
/// Prescan inspects it inline so the marker survives parse and round-trips.
#[test]
fn seq_item_inline_first_key_fill_round_trips() {
    let src = "~~~card-yaml\n$quill: q\n$kind: main\nrecipients:\n  - name: !must_fill\n    role: lead\n~~~\n\nBody.\n";
    let doc = Document::parse(src).unwrap().document;
    let emitted = doc.to_markdown();
    assert!(
        emitted.contains("- name: !must_fill"),
        "first-key fill marker must survive round-trip\nGot:\n{}",
        emitted
    );
    // Non-fill sibling keys are unaffected.
    assert!(emitted.contains("role: lead"), "Got:\n{}", emitted);
    let emitted2 = Document::parse(&emitted).unwrap().document.to_markdown();
    assert_eq!(emitted, emitted2, "round-trip must be idempotent");
}

/// A `!must_fill` marker on an own-line (non-first) key inside an array element
/// survives both the markdown round-trip and the storage (serde) round-trip,
/// and a second emit cycle is idempotent.
#[test]
fn array_element_nested_fill_survives_markdown_and_storage() {
    let src = "~~~card-yaml\n$quill: q@0.1\n$kind: main\nrecipients:\n  - name: Alice\n    org: !must_fill\n~~~\n\nBody.\n";
    let doc = Document::parse(src).unwrap().document;
    let emitted = doc.to_markdown();
    assert!(emitted.contains("org: !must_fill"), "Got:\n{}", emitted);
    assert!(emitted.contains("name: Alice"), "Got:\n{}", emitted);

    // Storage (serde) round-trip preserves the nested marker on recipients[0].org.
    let restored: Document = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(
        doc, restored,
        "nested array-element fill must survive storage"
    );
    assert_eq!(
        emitted,
        restored.to_markdown(),
        "markdown must be identical after a storage round-trip"
    );
}
