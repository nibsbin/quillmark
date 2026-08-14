//! The obligation axis' relational form: `must_fill_when:`, its evaluation over
//! the resolved ladder, and the load-time resolution that keeps a declared rule
//! from being one that never fires.

use crate::quill::quill_from_yaml;
use crate::{Document, Quill, QuillConfig};

/// Every `validation::must_fill` as `(path, trigger)`, sorted.
fn obligations(quill: &Quill, md: &str) -> Vec<(String, String)> {
    let doc = Document::parse(md).expect("parse").document;
    let mut out: Vec<(String, String)> = quill
        .validate(&doc)
        .iter()
        .filter(|d| d.code.as_deref() == Some("validation::must_fill"))
        .map(|d| {
            (
                d.path.clone().expect("every must_fill anchors at a path"),
                d.args["trigger"].as_str().expect("trigger arg").to_string(),
            )
        })
        .collect();
    out.sort();
    out
}

fn paths(quill: &Quill, md: &str) -> Vec<String> {
    obligations(quill, md).into_iter().map(|(p, _)| p).collect()
}

/// The three constraints issue #1202 cites from `usaf_memo`, in the shape the
/// fixture declares them.
const CUI: &str = r#"
quill: { name: cui, version: 1.0.0, backend: typst, description: conditional }
main:
  fields:
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI, SECRET]
      default: ""
    cui_controlled_by:
      type: string
      default: ""
      must_fill_when: { field: classification, equals: CUI }
    memo_for:
      type: array
      items: { type: string }
      default: []
    distribution:
      type: array
      items: { type: string }
      default: []
      must_fill_when: { field: memo_for, contains: SEE DISTRIBUTION }
"#;

fn md(fields: &str) -> String {
    format!("~~~card-yaml\n$quill: cui@1.0.0\n$kind: main\n{fields}~~~\n")
}

#[test]
fn a_dormant_condition_obliges_nothing() {
    let quill = quill_from_yaml(CUI);

    assert_eq!(
        paths(&quill, &md("classification: UNCLASSIFIED\n")),
        Vec::<String>::new(),
        "an unclassified memo owes no CUI office"
    );
}

/// The issue's headline evidence: a `classification: CUI` memo with no
/// controlled-by office used to validate clean.
#[test]
fn a_held_condition_obliges_the_blank_cell() {
    let quill = quill_from_yaml(CUI);

    assert_eq!(
        obligations(&quill, &md("classification: CUI\n")),
        [(
            "main.cui_controlled_by".to_string(),
            "conditional".to_string()
        )]
    );
}

#[test]
fn authoring_the_cell_discharges_the_conditional_obligation() {
    let quill = quill_from_yaml(CUI);

    assert!(paths(&quill, &md("classification: CUI\ncui_controlled_by: SAF/AA\n")).is_empty());
}

/// The split from plain `must_fill`, and the reason the message says "must not
/// be blank": a human writing the blank has made a call, but the call does not
/// satisfy a rule that demands a value.
#[test]
fn the_blank_does_not_discharge_a_conditional_obligation() {
    let quill = quill_from_yaml(CUI);

    assert_eq!(
        paths(&quill, &md("classification: CUI\ncui_controlled_by: \"\"\n")),
        ["main.cui_controlled_by"],
        "an explicitly blank office is the very gap the rule closes"
    );
}

#[test]
fn contains_reads_array_membership() {
    let quill = quill_from_yaml(CUI);

    assert_eq!(
        paths(&quill, &md("memo_for:\n  - SEE DISTRIBUTION\n")),
        ["main.distribution"]
    );
    assert!(
        paths(&quill, &md("memo_for:\n  - ORG1/SYMBOL\n")).is_empty(),
        "a normally-addressed memo owes no distribution list"
    );
    assert!(
        paths(
            &quill,
            &md("memo_for:\n  - SEE DISTRIBUTION\ndistribution:\n  - ORG1/SYMBOL\n")
        )
        .is_empty(),
        "an authored list discharges it"
    );
}

/// `[]` is an array's blank, so an empty list is not an answer to a rule that
/// demands one.
#[test]
fn an_empty_array_is_blank_for_the_obligation() {
    let quill = quill_from_yaml(CUI);

    assert_eq!(
        paths(&quill, &md("memo_for:\n  - SEE DISTRIBUTION\ndistribution: []\n")),
        ["main.distribution"]
    );
}

/// The condition reads the render ladder, not the raw payload: a `default:` the
/// document never spells is still what the page will say.
#[test]
fn the_condition_reads_the_resolved_default() {
    let quill = quill_from_yaml(
        r#"
quill: { name: dft, version: 1.0.0, backend: typst, description: defaulted condition }
main:
  fields:
    mode:  { type: enum, values: [draft, final], default: final }
    stamp: { type: string, default: "", must_fill_when: { field: mode, equals: final } }
"#,
    );
    let doc = "~~~card-yaml\n$quill: dft@1.0.0\n$kind: main\n~~~\n";

    assert_eq!(
        paths(&quill, doc),
        ["main.stamp"],
        "the rule fires on the document that renders, not the subset a human typed"
    );
}

#[test]
fn in_and_nonblank_read_their_own_shapes() {
    let quill = quill_from_yaml(
        r#"
quill: { name: ops, version: 1.0.0, backend: typst, description: operators }
main:
  fields:
    level:    { type: enum, values: [low, high, critical], default: "" }
    category: { type: string, default: "" }
    owner:    { type: string, default: "", must_fill_when: { field: level, in: [high, critical] } }
    poc:      { type: string, default: "", must_fill_when: { field: category, nonblank: true } }
"#,
    );
    let doc = |fields: &str| format!("~~~card-yaml\n$quill: ops@1.0.0\n$kind: main\n{fields}~~~\n");

    assert!(paths(&quill, &doc("level: low\n")).is_empty());
    assert_eq!(paths(&quill, &doc("level: high\n")), ["main.owner"]);
    assert_eq!(paths(&quill, &doc("level: critical\n")), ["main.owner"]);
    assert_eq!(paths(&quill, &doc("category: PRVCY\n")), ["main.poc"]);
    assert!(
        paths(&quill, &doc("category: \"\"\n")).is_empty(),
        "a blank condition field leaves the rule dormant"
    );
}

/// A rule replaces the `default:`-presence derivation rather than stacking on
/// it, so one cell never draws two contradicting diagnostics.
#[test]
fn a_rule_suppresses_the_unconditional_derivation() {
    let quill = quill_from_yaml(
        r#"
quill: { name: sup, version: 1.0.0, backend: typst, description: suppression }
main:
  fields:
    mode:  { type: enum, values: [a, b], default: a }
    extra: { type: string, must_fill_when: { field: mode, equals: b } }
"#,
    );
    let doc = |fields: &str| format!("~~~card-yaml\n$quill: sup@1.0.0\n$kind: main\n{fields}~~~\n");

    assert!(
        paths(&quill, &doc("mode: a\n")).is_empty(),
        "a defaultless field carrying a rule is not also unconditionally obliged"
    );
    assert_eq!(
        obligations(&quill, &doc("mode: b\n")),
        [("main.extra".to_string(), "conditional".to_string())],
        "and when the condition holds it draws exactly one diagnostic"
    );
}

/// Marker-wins, the precedent the other two triggers already set.
#[test]
fn a_marker_outranks_the_conditional_trigger() {
    let quill = quill_from_yaml(CUI);

    assert_eq!(
        obligations(
            &quill,
            &md("classification: CUI\ncui_controlled_by: !must_fill \"\"\n")
        ),
        [("main.cui_controlled_by".to_string(), "marker".to_string())],
        "one diagnostic, and the marker's actionable hint wins"
    );
}

#[test]
fn rules_apply_on_composable_cards() {
    let quill = quill_from_yaml(
        r#"
quill: { name: crd, version: 1.0.0, backend: typst, description: card rules }
main:
  fields:
    title: { type: string, default: "" }
card_kinds:
  indorsement:
    fields:
      action: { type: enum, values: [approve, disapprove], default: "" }
      reason: { type: string, default: "", must_fill_when: { field: action, equals: disapprove } }
"#,
    );
    let doc = |action: &str| {
        format!(
            "~~~card-yaml\n$quill: crd@1.0.0\n$kind: main\n~~~\n\n\
             ~~~card-yaml\n$kind: indorsement\naction: {action}\n~~~\n"
        )
    };

    assert!(paths(&quill, &doc("approve")).is_empty());
    assert_eq!(
        paths(&quill, &doc("disapprove")),
        ["cards.indorsement[0].reason"],
        "the anchor is the card-qualified path an editor can navigate to"
    );
}

#[test]
fn the_diagnostic_names_both_ways_out() {
    let quill = quill_from_yaml(CUI);
    let doc = Document::parse(&md("classification: CUI\n"))
        .expect("parse")
        .document;
    let diag = quill
        .validate(&doc)
        .into_iter()
        .find(|d| d.code.as_deref() == Some("validation::must_fill"))
        .expect("conditional obligation");

    assert_eq!(diag.severity, crate::Severity::Warning, "never a render gate");
    assert_eq!(
        diag.message,
        "Field `main.cui_controlled_by` must not be blank: `classification` is `CUI`."
    );
    assert_eq!(
        diag.hint.as_deref(),
        Some(
            "Either author `main.cui_controlled_by`, or change `classification` away from `CUI`."
        )
    );
    assert_eq!(diag.args["conditionField"], "classification");
    assert_eq!(diag.args["conditionOperator"], "equals");
    assert_eq!(diag.args["conditionOperand"], "CUI");
}

/// A conditional obligation never gates render: the cell blank-fills like any
/// other absent field.
#[test]
fn an_outstanding_conditional_obligation_still_renders() {
    let quill = quill_from_yaml(CUI);
    let doc = Document::parse(&md("classification: CUI\n"))
        .expect("parse")
        .document;

    let plate = quill.compile_data(&doc).expect("blank-filled render");
    assert_eq!(plate["cui_controlled_by"], "");
}

// ---- projections ----------------------------------------------------------

/// The blueprint states the rule as prose and stamps no marker: whether the
/// obligation binds is a fact about a filled-in document, and the blueprint is
/// the empty form.
#[test]
fn the_blueprint_states_the_rule_without_stamping_a_marker() {
    let quill = quill_from_yaml(CUI);
    let text = quill.config().blueprint();

    assert!(
        text.contains("# required when classification is CUI\ncui_controlled_by:"),
        "blueprint missing the rule line:\n{text}"
    );
    assert!(
        text.contains("# required when memo_for contains SEE DISTRIBUTION\ndistribution:"),
        "blueprint missing the array rule line:\n{text}"
    );
    assert!(
        !text.contains("cui_controlled_by: !must_fill"),
        "a conditional obligation must not stamp an unconditional marker:\n{text}"
    );
}

/// The blueprint is a document by construction, so its rule lines must survive
/// the round-trip that contract rests on.
#[test]
fn the_blueprint_still_round_trips() {
    let quill = quill_from_yaml(CUI);
    let text = quill.config().blueprint();

    Document::parse(&text).expect("blueprint round-trips through the parser");
}

#[test]
fn the_transform_schema_carries_the_rule_as_an_annotation() {
    let quill = quill_from_yaml(CUI);
    let schema = crate::quill::build_transform_schema(quill.config()).into_json();
    let field = &schema["properties"]["cui_controlled_by"];

    assert_eq!(
        field[crate::quill::QUILLMARK_MUST_FILL_WHEN_KEY],
        serde_json::json!({ "field": "classification", "operator": "equals", "operand": "CUI" })
    );
    assert_eq!(
        field["quillmark:must_fill"], false,
        "a conditional obligation is not an unconditional one"
    );
    assert!(
        field.get("if").is_none() && field.get("then").is_none(),
        "the rule is an annotation, never an enforcing keyword: a stock validator \
         must keep accepting what the engine accepts"
    );
}

// ---- load-time resolution -------------------------------------------------

fn load_error(yaml: &str) -> Vec<String> {
    QuillConfig::from_yaml_with_warnings(yaml)
        .err()
        .expect("expected a load error")
        .iter()
        .filter_map(|d| d.code.clone())
        .collect()
}

fn quill_with(field: &str) -> String {
    format!(
        r#"
quill: {{ name: lt, version: 1.0.0, backend: typst, description: load-time }}
main:
  fields:
    classification: {{ type: enum, values: [UNCLASSIFIED, CUI], default: "" }}
    tags: {{ type: array, items: {{ type: string }}, default: [] }}
{field}
"#
    )
}

#[test]
fn a_rule_naming_an_undeclared_field_is_a_load_error() {
    assert!(load_error(&quill_with(
        "    office: { type: string, default: \"\", \
         must_fill_when: { field: clasification, equals: CUI } }"
    ))
    .contains(&"quill::must_fill_when_unknown_field".to_string()));
}

#[test]
fn a_self_referencing_rule_is_a_load_error() {
    assert!(load_error(&quill_with(
        "    office: { type: string, default: \"\", \
         must_fill_when: { field: office, nonblank: true } }"
    ))
    .contains(&"quill::must_fill_when_self_reference".to_string()));
}

/// The check that turns a declared rule into a checked one: an operand outside
/// the condition field's domain is unenforceable prose in predicate syntax.
#[test]
fn an_out_of_domain_operand_is_a_load_error() {
    assert!(load_error(&quill_with(
        "    office: { type: string, default: \"\", \
         must_fill_when: { field: classification, equals: cui } }"
    ))
    .contains(&"quill::must_fill_when_domain".to_string()));
}

#[test]
fn an_operator_the_condition_field_cannot_answer_is_a_load_error() {
    assert!(
        load_error(&quill_with(
            "    office: { type: string, default: \"\", \
             must_fill_when: { field: classification, contains: CUI } }"
        ))
        .contains(&"quill::must_fill_when_operator".to_string()),
        "`contains` needs an array"
    );
    assert!(
        load_error(&quill_with(
            "    office: { type: string, default: \"\", \
             must_fill_when: { field: tags, equals: x } }"
        ))
        .contains(&"quill::must_fill_when_operator".to_string()),
        "`equals` needs a scalar"
    );
}

#[test]
fn a_rule_beside_must_fill_is_a_load_error() {
    assert!(load_error(&quill_with(
        "    office: { type: string, must_fill: true, \
         must_fill_when: { field: classification, equals: CUI } }"
    ))
    .contains(&"quill::field_parse_error".to_string()));
}

#[test]
fn a_rule_in_a_nested_position_is_a_load_error() {
    assert!(load_error(&quill_with(
        "    rows:\n      type: array\n      items:\n        type: object\n        \
         properties:\n          kind: { type: string }\n          note:\n            \
         type: string\n            must_fill_when: { field: kind, nonblank: true }"
    ))
    .contains(&"quill::nested_must_fill_when".to_string()));
}

#[test]
fn a_malformed_operator_set_is_a_load_error() {
    for rule in [
        "{ field: classification }",
        "{ field: classification, equals: CUI, nonblank: true }",
        "{ field: classification, matches: CUI }",
        "{ field: classification, nonblank: false }",
    ] {
        assert!(
            load_error(&quill_with(&format!(
                "    office: {{ type: string, default: \"\", must_fill_when: {rule} }}"
            )))
            .contains(&"quill::field_parse_error".to_string()),
            "expected a parse error for {rule}"
        );
    }
}

/// A rule keyed on the blank is legitimate: "obliged when nobody has chosen".
#[test]
fn a_rule_keyed_on_the_blank_loads_and_fires() {
    let quill = quill_from_yaml(
        r#"
quill: { name: bl, version: 1.0.0, backend: typst, description: blank-keyed }
main:
  fields:
    level:  { type: enum, values: [low, high], default: "" }
    reason: { type: string, default: "", must_fill_when: { field: level, equals: "" } }
"#,
    );
    let doc = |fields: &str| format!("~~~card-yaml\n$quill: bl@1.0.0\n$kind: main\n{fields}~~~\n");

    assert_eq!(paths(&quill, &doc("")), ["main.reason"]);
    assert!(paths(&quill, &doc("level: low\n")).is_empty());
}
