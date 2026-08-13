//! `plate::unsupported_construct`: what a quill declares it does not typeset,
//! and what a body holding it draws.

use crate::quill::{quill_from_yaml, BlockConstruct, QuillConfig, UNSUPPORTED_CONSTRUCT};
use crate::{Document, Quill, Severity};

const YAML: &str = r#"
quill:
  name: decliner
  version: 0.1.0
  backend: typst
  description: A quill that declines constructs
main:
  body:
    unsupported: [rule, table]
  fields: {}
card_kinds:
  note:
    body: {}
    fields: {}
"#;

fn quill() -> Quill {
    quill_from_yaml(YAML)
}

/// `(code, path, construct, count)` for every warning `body` draws.
fn warn(body: &str) -> Vec<(String, String, String, u64)> {
    let markdown = format!("~~~card-yaml\n$quill: decliner\n~~~\n\n{body}\n");
    let parsed = quill().parse(&markdown).expect("parses and conforms");
    parsed
        .warnings
        .iter()
        .filter(|d| d.code.as_deref() == Some(UNSUPPORTED_CONSTRUCT))
        .map(|d| {
            assert_eq!(d.severity, Severity::Warning, "never fatal: {d:?}");
            (
                d.code.clone().unwrap(),
                d.path.clone().unwrap_or_default(),
                d.args["construct"].as_str().unwrap().to_string(),
                d.args["count"].as_u64().unwrap(),
            )
        })
        .collect()
}

fn constructs(body: &str) -> Vec<(String, u64)> {
    warn(body)
        .into_iter()
        .map(|(_, _, construct, count)| (construct, count))
        .collect()
}

/// The warning names the construct, counts it, and anchors on the body's
/// schema address: the path a consumer routes on.
#[test]
fn a_declined_construct_warns_against_its_body() {
    assert_eq!(
        warn("one\n\n***\n\ntwo"),
        [(
            UNSUPPORTED_CONSTRUCT.to_string(),
            "main.body".to_string(),
            "rule".to_string(),
            1
        )]
    );
}

/// One warning per construct, not per occurrence: the walk sees the whole body
/// at once, so a count rides `args` instead of forty identical diagnostics
/// reaching an editor's surface.
#[test]
fn occurrences_collapse_into_one_count() {
    assert_eq!(constructs("a\n\n***\n\nb\n\n***\n\nc\n\n***"), [("rule".to_string(), 3)]);
    // At any depth, and the whole set is one diagnostic each.
    assert_eq!(
        constructs("- ***\n\n| a |\n| --- |\n| 1 |\n\n***"),
        [("rule".to_string(), 2), ("table".to_string(), 1)]
    );
}

/// A construct the quill never declined draws nothing, however much of it the
/// body holds. Silence is the default and stays the default.
#[test]
fn an_undeclared_construct_is_silent() {
    assert_eq!(constructs("# Title\n\n> quoted\n\n- a\n- b\n\n```\nx\n```"), []);
    assert_eq!(constructs("just a paragraph"), []);
    assert_eq!(constructs(""), []);
}

/// A card kind declaring an empty `unsupported` (the default) is silent even
/// when the main card next to it is not.
#[test]
fn the_declaration_is_per_body() {
    let markdown = "~~~card-yaml\n$quill: decliner\n~~~\n\n***\n\n~~~\n$kind: note\n~~~\n\n***\n";
    let parsed = quill().parse(markdown).expect("parses");
    let paths: Vec<_> = parsed
        .warnings
        .iter()
        .filter(|d| d.code.as_deref() == Some(UNSUPPORTED_CONSTRUCT))
        .map(|d| d.path.clone().unwrap_or_default())
        .collect();
    assert_eq!(paths, ["main.body"], "the note's body declines nothing");
}

/// The walk is stateless, so a second pass over the same document re-emits the
/// identical set: the same contract `conform` holds.
#[test]
fn the_walk_is_idempotent() {
    let markdown = "~~~card-yaml\n$quill: decliner\n~~~\n\n***\n";
    let quill = quill();
    let mut document = Document::parse(markdown).expect("parses").document;
    quill.conform(&mut document).expect("conforms");
    let first = quill.unsupported_constructs(&document);
    assert_eq!(first, quill.unsupported_constructs(&document));
    assert_eq!(first.len(), 1);
}

/// A misspelled construct is a load error, not a declaration that quietly
/// matches nothing: the vocabulary is closed.
#[test]
fn an_unknown_construct_name_fails_the_load() {
    let yaml = YAML.replace("[rule, table]", "[rules]");
    assert!(
        QuillConfig::from_yaml_with_warnings(&yaml).is_err(),
        "`rules` is not a construct name"
    );
}

/// A run counts once, not once per item or per line: the count is a count of
/// constructs the author wrote.
#[test]
fn a_container_counts_once_per_run() {
    let quill = quill_from_yaml(&YAML.replace("[rule, table]", "[list, quote]"));
    let census = |body: &str| {
        let markdown = format!("~~~card-yaml\n$quill: decliner\n~~~\n\n{body}\n");
        let parsed = quill.parse(&markdown).expect("parses");
        parsed
            .warnings
            .iter()
            .filter(|d| d.code.as_deref() == Some(UNSUPPORTED_CONSTRUCT))
            .map(|d| {
                (
                    d.args["construct"].as_str().unwrap().to_string(),
                    d.args["count"].as_u64().unwrap(),
                )
            })
            .collect::<Vec<_>>()
    };
    // Three items are one list; a multi-paragraph item does not re-open it.
    assert_eq!(census("- a\n- b\n- c"), [("list".to_string(), 1)]);
    assert_eq!(census("- a\n\n  still a"), [("list".to_string(), 1)]);
    // A nested list is its own list, so two levels are two.
    assert_eq!(census("- a\n  - b\n  - c"), [("list".to_string(), 2)]);
    // Two lists with a paragraph between them are two.
    assert_eq!(census("- a\n\nbetween\n\n- b"), [("list".to_string(), 2)]);
    assert_eq!(census("> one\n>\n> two"), [("quote".to_string(), 1)]);
}

/// Every construct name in the vocabulary is reachable from content: a name
/// nothing can hold would be a declaration that never fires.
#[test]
fn every_construct_name_is_reachable() {
    for (construct, body) in [
        (BlockConstruct::Heading, "# h"),
        (BlockConstruct::Rule, "***"),
        (BlockConstruct::Code, "```\nx\n```"),
        (BlockConstruct::List, "- a"),
        (BlockConstruct::Quote, "> q"),
        (BlockConstruct::Table, "| a |\n| --- |\n| 1 |"),
        (BlockConstruct::Image, "![alt](cat.png)"),
    ] {
        let quill = quill_from_yaml(&YAML.replace("[rule, table]", &format!("[{construct}]")));
        let markdown = format!("~~~card-yaml\n$quill: decliner\n~~~\n\n{body}\n");
        let parsed = quill.parse(&markdown).expect("parses");
        let hits: Vec<_> = parsed
            .warnings
            .iter()
            .filter(|d| d.code.as_deref() == Some(UNSUPPORTED_CONSTRUCT))
            .collect();
        assert_eq!(hits.len(), 1, "{construct} did not fire on {body:?}");
        assert_eq!(hits[0].args["construct"].as_str().unwrap(), construct.as_str());
    }
}
