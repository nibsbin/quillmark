use super::*;
use quillmark_core::FileTreeNode;
use serde_json::json;
use std::collections::HashMap;

const QUILL_YAML: &str = r#"
quill:
  name: certificate
  version: 1.2.0
  backend: typst
  description: merge tests
typst:
  plate_file: plate.typ
main:
  fields:
    recipient: { type: string }
    event: { type: string, default: "" }
    awarded_on: { type: date }
    qty: { type: integer, default: 0 }
    tags: { type: array, items: { type: string }, default: [] }
    cohort:
      type: object
      properties:
        lead: { type: string, default: "" }
        size: { type: integer, default: 0 }
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI]
      default: ""
      variants:
        CUI:
          poc: { type: string }
card_kinds:
  line_item:
    fields:
      desc: { type: richtext, inline: true }
      qty: { type: integer }
"#;

fn quill() -> Quill {
    let mut files = HashMap::new();
    files.insert(
        "Quill.yaml".to_string(),
        FileTreeNode::File {
            contents: QUILL_YAML.as_bytes().to_vec(),
        },
    );
    files.insert(
        "plate.typ".to_string(),
        FileTreeNode::File {
            contents: b"#import \"@local/quillmark-helper:0.1.0\": data\n#data.recipient".to_vec(),
        },
    );
    Quill::from_tree(FileTreeNode::Directory { files }).expect("test quill loads")
}

fn rows(header: &[&str], data: &[&[&str]]) -> Input {
    Input::Rows(
        data.iter()
            .map(|cells| {
                header
                    .iter()
                    .zip(cells.iter())
                    .map(|(h, c)| (h.to_string(), Value::String(c.to_string())))
                    .collect()
            })
            .collect(),
    )
}

fn spec(yaml: &str) -> MergeSpec {
    MergeSpec::from_yaml(yaml).expect("spec parses")
}

fn error_codes(plan: &MergePlan) -> Vec<String> {
    plan.errors()
        .filter_map(|d| d.diagnostic.code.clone())
        .collect()
}

const CERT_SPEC: &str = r#"
$quill: certificate@1.2.0
map:
  recipient:   { column: Name }
  event:       { value: "Rustconf 2026" }
  awarded_on:  { column: Date, format: "%m/%d/%Y" }
  qty:         { column: Qty }
  tags:        { column: Tags, split: "," }
  cohort.lead: { column: Instructor }
output: "{recipient}-certificate"
"#;

const HEADER: &[&str] = &["Name", "Date", "Qty", "Tags", "Instructor", "Notes"];

#[test]
fn document_mode_lowers_transforms_and_localizes_the_bad_cell() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(CERT_SPEC),
        &rows(
            HEADER,
            &[
                &["Ada", "3/1/2026", "2", "a, b", "Grace", "x"],
                &["Linus", "3/2/2026", "abc", "", "Grace", ""],
                &["", "", "", "", "", ""],
            ],
        ),
    );

    assert_eq!(plan.skipped_empty, 1);
    assert_eq!(plan.documents.len(), 1, "{:#?}", plan.report);
    let doc = &plan.documents[0];
    assert_eq!(doc.filename, "Ada-certificate");
    assert_eq!(doc.key, "Ada-certificate", "no `key:`, so the output name keys the row");
    assert_eq!(doc.rows, [0]);
    let fields = q.reader(&doc.document).values().fields.unwrap();
    assert_eq!(fields["recipient"], json!("Ada"));
    assert_eq!(fields["event"], json!("Rustconf 2026"));
    assert_eq!(fields["awarded_on"], json!("2026-03-01"), "date re-spelled ISO");
    assert_eq!(fields["qty"], json!(2), "the typed write coerced the cell");
    assert_eq!(fields["tags"], json!(["a", "b"]), "split trims pieces");
    assert_eq!(fields["cohort"], json!({"lead": "Grace"}), "a dotted target assembles the container");
    let ext = &doc.document.main().ext().unwrap()["merge"];
    assert_eq!(ext["row_key"], json!("Ada-certificate"));
    assert_eq!(ext["spec_hash"], json!(spec(CERT_SPEC).hash()));

    let errs: Vec<_> = plan.errors().collect();
    assert_eq!(errs.len(), 1, "{errs:#?}");
    let e = errs[0];
    assert_eq!(e.row, Some(1));
    assert_eq!(e.column.as_deref(), Some("Qty"));
    assert_eq!(e.diagnostic.path.as_deref(), Some("main.qty"));
    assert_eq!(e.diagnostic.code.as_deref(), Some("edit::field_coercion_failed"));

    let unmapped: Vec<_> = plan
        .warnings()
        .filter(|w| w.diagnostic.code.as_deref() == Some("merge::unmapped_column"))
        .collect();
    assert_eq!(unmapped.len(), 1, "one warning per unmapped column, once");
    assert_eq!(unmapped[0].row, None);
    assert_eq!(unmapped[0].column.as_deref(), Some("Notes"));

    assert!(!plan.is_clean());
    assert_eq!(plan.clean_documents().count(), 1, "a forced render renders the clean row");
}

#[test]
fn an_empty_cell_is_absent_and_a_constant_blank_is_authored() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(
            r#"
$quill: certificate@1.2.0
map:
  recipient:  { column: Name }
  event:      { value: "" }
  awarded_on: { column: Date, format: "%m/%d/%Y" }
  qty:        { column: Qty }
output: "{awarded_on}"
"#,
        ),
        &rows(&["Name", "Date", "Qty"], &[&["", "3/1/2026", "  "]]),
    );
    assert!(plan.is_clean(), "{:#?}", plan.report);
    assert_eq!(plan.documents.len(), 1);
    let fields = q.reader(&plan.documents[0].document).values().fields.unwrap();
    assert!(!fields.contains_key("recipient"), "absent, not authored-empty");
    assert!(!fields.contains_key("qty"), "a blank integer cell is not a coercion error");
    assert_eq!(fields["event"], json!(""), "a constant is verbatim, the blank included");
    let w: Vec<_> = plan.warnings().collect();
    assert_eq!(w.len(), 1, "{w:#?}");
    assert_eq!(w[0].diagnostic.code.as_deref(), Some("validation::must_fill"));
    assert_eq!(w[0].diagnostic.path.as_deref(), Some("main.recipient"));
    assert_eq!(w[0].row, Some(0));
    assert_eq!(w[0].column.as_deref(), Some("Name"), "reverse-mapped from the path");
}

#[test]
fn a_header_that_is_a_target_maps_by_identity_and_an_entry_overrides_it() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(
            r#"
$quill: certificate@1.2.0
map:
  recipient: { column: Name }
output: "{recipient}"
"#,
        ),
        &rows(
            &["Name", "recipient", "qty", "cohort.lead", "$body", "Notes"],
            &[&["Ada", "ignored", "4", "Grace", "Well *done*.", "x"]],
        ),
    );
    assert!(plan.is_clean(), "{:#?}", plan.report);
    let doc = &plan.documents[0].document;
    let values = q.reader(doc).values();
    let fields = values.fields.unwrap();
    assert_eq!(fields["recipient"], json!("Ada"), "the explicit entry wins over the identity header");
    assert_eq!(fields["qty"], json!(4));
    assert_eq!(fields["cohort"], json!({"lead": "Grace"}), "a dotted header maps by identity too");
    assert_eq!(values.body.unwrap(), "Well *done*.");
    let unmapped: Vec<_> = plan
        .warnings()
        .filter(|w| w.diagnostic.code.as_deref() == Some("merge::unmapped_column"))
        .map(|w| w.column.clone().unwrap())
        .collect();
    assert_eq!(unmapped, ["recipient", "Notes"], "an overridden identity header is ignored, and says so");
}

#[test]
fn a_key_column_keys_the_row_and_must_be_unique() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(
            r#"
$quill: certificate@1.2.0
key: ID
map:
  recipient: { column: Name }
output: "{recipient}"
"#,
        ),
        &rows(
            &["ID", "Name"],
            &[&["e-1", "Ada"], &["e-1", "Bob"], &["", "Cy"], &["e-2", "Dee"]],
        ),
    );
    let keys: Vec<&str> = plan.documents.iter().map(|d| d.key.as_str()).collect();
    assert_eq!(keys, ["e-1", "e-2"]);
    assert_eq!(
        q.reader(&plan.documents[0].document).values().ext.unwrap().unwrap()["merge"]["row_key"],
        json!("e-1")
    );
    let errs: Vec<_> = plan.errors().collect();
    assert_eq!(errs.len(), 2, "{errs:#?}");
    assert_eq!(errs[0].row, Some(1));
    assert_eq!(errs[0].column.as_deref(), Some("ID"));
    assert_eq!(errs[0].diagnostic.code.as_deref(), Some("merge::duplicate_key"));
    assert_eq!(errs[1].row, Some(2));
    assert_eq!(errs[1].diagnostic.code.as_deref(), Some("merge::missing_key"));
}

#[test]
fn a_variant_cell_is_addressable_and_its_obligation_reverse_maps() {
    let q = quill();
    let cui = r#"
$quill: certificate@1.2.0
map:
  recipient:            { column: Name }
  awarded_on:           { column: Date, format: "%m/%d/%Y" }
  classification.value: { value: CUI }
  classification.poc:   { column: POC }
output: "{recipient}"
"#;
    let plan = plan(
        &q,
        &spec(cui),
        &rows(
            &["Name", "Date", "POC"],
            &[&["Ada", "3/1/2026", "Capt Smith"], &["Bob", "03/01/2026", ""]],
        ),
    );
    assert!(plan.is_clean(), "{:#?}", plan.report);
    let fields = q.reader(&plan.documents[0].document).values().fields.unwrap();
    assert_eq!(fields["classification"], json!({"value": "CUI", "poc": "Capt Smith"}));
    assert_eq!(
        q.reader(&plan.documents[1].document).values().fields.unwrap()["awarded_on"],
        json!("2026-03-01"),
        "a zero-padded cell parses under the same pattern as an unpadded one"
    );
    let w: Vec<_> = plan.warnings().collect();
    assert_eq!(w.len(), 1, "{w:#?}");
    assert_eq!(w[0].row, Some(1));
    assert_eq!(w[0].diagnostic.path.as_deref(), Some("main.classification.poc"));
    assert_eq!(w[0].column.as_deref(), Some("POC"));
}

#[test]
fn an_output_collision_means_the_pattern_does_not_key_the_input() {
    let plan = plan(
        &quill(),
        &spec(CERT_SPEC),
        &rows(
            HEADER,
            &[
                &["Ada", "3/1/2026", "1", "", "", ""],
                &["Ada", "3/2/2026", "1", "", "", ""],
            ],
        ),
    );
    assert_eq!(plan.documents.len(), 1);
    let errs: Vec<_> = plan.errors().collect();
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert_eq!(errs[0].row, Some(1));
    assert_eq!(errs[0].diagnostic.code.as_deref(), Some("merge::output_collision"));
}

const INVOICE_SPEC: &str = r#"
$quill: certificate@1.2.0
mode: cards
group_by: "Invoice #"
map:
  recipient:  { column: Customer }
  event:      { column: "Invoice #" }
  awarded_on: { value: "2026-03-01" }
  $body:      { column: Note }
cards:
  line_item:
    map:
      desc:  { column: Description }
      qty:   { column: Qty }
      $body: { column: Detail }
output: "invoice-{event}"
"#;

const INVOICE_HEADER: &[&str] = &["Invoice #", "Customer", "Note", "Description", "Qty", "Detail"];

#[test]
fn cards_mode_groups_rows_in_first_appearance_order_and_keeps_row_order() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(INVOICE_SPEC),
        &rows(
            INVOICE_HEADER,
            &[
                &["INV-2", "Bolt", "Net 30", "Gear", "1", "steel"],
                &["INV-1", "Acme", "", "Widget", "2", ""],
                &["INV-2", "Bolt", "Net 30", "Sprocket", "3", ""],
                &["INV-1", "Acme", "", "Gadget", "4", "brass"],
            ],
        ),
    );
    assert!(plan.is_clean(), "{:#?}", plan.report);
    let keys: Vec<&str> = plan.documents.iter().map(|d| d.key.as_str()).collect();
    assert_eq!(keys, ["INV-2", "INV-1"]);
    assert_eq!(plan.documents[0].rows, [0, 2]);
    assert_eq!(plan.documents[0].filename, "invoice-INV-2");
    let values = q.reader(&plan.documents[0].document).values();
    assert_eq!(values.fields.unwrap()["recipient"], json!("Bolt"));
    assert_eq!(values.body.unwrap(), "Net 30", "a main `$body` target fills the main body");
    let cards = values.cards.unwrap();
    let descs: Vec<&Value> = cards.iter().map(|c| &c.fields.as_ref().unwrap()["desc"]).collect();
    assert_eq!(descs, [&json!("Gear"), &json!("Sprocket")]);
    assert_eq!(cards[1].fields.as_ref().unwrap()["qty"], json!(3));
    assert_eq!(cards[0].body.as_deref(), Some("steel"), "a card `$body` target fills the card body");
    assert_eq!(cards[1].body.as_deref(), Some(""));
    assert_eq!(
        plan.documents[0].document.main().ext().unwrap()["merge"]["row_key"],
        json!("INV-2")
    );
}

#[test]
fn cards_mode_localizes_a_card_cell_and_refuses_a_non_constant_main_column() {
    let plan = plan(
        &quill(),
        &spec(INVOICE_SPEC),
        &rows(
            INVOICE_HEADER,
            &[
                &["INV-1", "Acme", "", "Widget", "2", ""],
                &["INV-2", "Bolt", "", "Gear", "1", ""],
                &["INV-1", "Acme", "", "Gadget", "x", ""],
                &["INV-2", "Bolt Inc", "", "Sprocket", "3", ""],
                &["INV-3", "Cog", "Net 30", "Pin", "1", ""],
                &["INV-3", "Cog", "Net 60", "Nut", "1", ""],
                &["", "Nobody", "", "Orphan", "1", ""],
            ],
        ),
    );
    assert!(plan.documents.is_empty());
    let errs: Vec<_> = plan.errors().collect();
    assert_eq!(errs.len(), 4, "{errs:#?}");

    let card_err = errs
        .iter()
        .find(|e| e.diagnostic.code.as_deref() == Some("edit::field_coercion_failed"))
        .unwrap();
    assert_eq!(card_err.row, Some(2), "the card's own source row, not the group's first");
    assert_eq!(card_err.column.as_deref(), Some("Qty"));
    assert_eq!(card_err.diagnostic.path.as_deref(), Some("cards.line_item[1].qty"));

    let conflicts: Vec<_> = errs
        .iter()
        .filter(|e| e.diagnostic.code.as_deref() == Some("merge::group_conflict"))
        .collect();
    assert_eq!(conflicts.len(), 2);
    assert_eq!(conflicts[0].row, Some(3));
    assert_eq!(conflicts[0].column.as_deref(), Some("Customer"));
    assert_eq!(conflicts[0].diagnostic.path.as_deref(), Some("main.recipient"));
    assert_eq!(conflicts[1].row, Some(5), "a main body differing within a group is a conflict too");
    assert_eq!(conflicts[1].diagnostic.path.as_deref(), Some("main.body"));

    let orphan = errs
        .iter()
        .find(|e| e.diagnostic.code.as_deref() == Some("merge::missing_group_key"))
        .unwrap();
    assert_eq!(orphan.row, Some(6));
    assert_eq!(orphan.column.as_deref(), Some("Invoice #"));
}

#[test]
fn cards_mode_refuses_an_identity_header_both_cards_declare() {
    let plan = plan(
        &quill(),
        &spec(
            r#"
$quill: certificate@1.2.0
mode: cards
group_by: inv
cards:
  line_item:
    map:
      desc: { column: Description }
output: "{recipient}"
"#,
        ),
        &rows(&["inv", "recipient", "Description", "qty"], &[&["1", "Ada", "Widget", "2"]]),
    );
    assert!(plan.documents.is_empty());
    let errs: Vec<_> = plan.errors().collect();
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert_eq!(errs[0].diagnostic.code.as_deref(), Some("merge::ambiguous_column"));
    assert_eq!(errs[0].column.as_deref(), Some("qty"));
}

#[test]
fn spec_level_refusals_come_first_and_stop_the_plan() {
    let plan = plan(
        &quill(),
        &spec(
            r#"
$quill: certificate@2
mode: cards
map:
  nope: { column: Name }
  recipient: { column: Missing, value: "both" }
  qty.digits: { column: Name }
  cohort.nope: { column: Name }
  classification.value.x: { column: Name }
output: "x-{nope"
"#,
        ),
        &rows(&["Name"], &[&["Ada"]]),
    );
    assert!(plan.documents.is_empty());
    let codes = error_codes(&plan);
    for expected in [
        "merge::quill_mismatch",
        "merge::spec_mode",
        "merge::spec_unknown_target",
        "merge::spec_mapping",
        "merge::spec_output",
    ] {
        assert!(codes.contains(&expected.to_string()), "{expected} missing from {codes:?}");
    }
    assert_eq!(
        codes.iter().filter(|c| *c == "merge::spec_unknown_target").count(),
        4,
        "an undeclared field, a scalar stepped into, an undeclared property, a step past the discriminant"
    );
    assert!(plan.errors().all(|e| e.row.is_none()));
    assert!(
        !codes.contains(&"merge::unknown_column".to_string()),
        "header checks wait for a spec that holds"
    );
}

#[test]
fn a_column_the_input_lacks_stops_the_plan_before_any_row() {
    let plan = plan(
        &quill(),
        &spec(CERT_SPEC),
        &rows(&["Name"], &[&["Ada"]]),
    );
    assert!(plan.documents.is_empty());
    let errs: Vec<_> = plan.errors().collect();
    assert!(errs.iter().all(|e| e.diagnostic.code.as_deref() == Some("merge::unknown_column")), "{errs:#?}");
    let columns: Vec<&str> = errs.iter().filter_map(|e| e.column.as_deref()).collect();
    assert_eq!(columns, ["Date", "Qty", "Tags", "Instructor"]);
}

#[test]
fn a_bad_date_reports_the_cell_and_leaves_the_field_absent() {
    let plan = plan(
        &quill(),
        &spec(CERT_SPEC),
        &rows(HEADER, &[&["Ada", "2026-03-01", "1", "", "", ""]]),
    );
    let errs: Vec<_> = plan.errors().collect();
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert_eq!(errs[0].diagnostic.code.as_deref(), Some("merge::date_format"));
    assert_eq!(errs[0].row, Some(0));
    assert_eq!(errs[0].column.as_deref(), Some("Date"));
    assert_eq!(errs[0].diagnostic.path.as_deref(), Some("main.awarded_on"));
    assert_eq!(plan.documents.len(), 1, "the row still lowers; the field is absent");
    assert_eq!(plan.clean_documents().count(), 0, "and an error on its row keeps it out of a forced render");
}

#[test]
fn the_documents_lane_skips_the_mapping_and_anchors_by_index() {
    let q = quill();
    let documents = vec![
        DocumentValues::new(IndexMap::from([
            ("recipient".to_string(), json!("Ada")),
            ("awarded_on".to_string(), json!("2026-03-01")),
            ("tags".to_string(), json!(["a", "b"])),
        ]))
        .with_body("Cited.")
        .with_cards(vec![CardValues::new(
            "line_item",
            IndexMap::from([("desc".to_string(), json!("Widget")), ("qty".to_string(), json!(2))]),
        )]),
        DocumentValues::new(IndexMap::from([
            ("recipient".to_string(), json!("Bob")),
            ("nope".to_string(), json!(1)),
        ])),
    ];
    let plan = plan(
        &q,
        &MergeSpec::new("certificate@1.2.0", "{recipient}"),
        &Input::Documents(documents),
    );
    assert_eq!(plan.documents.len(), 1, "{:#?}", plan.report);
    let values = q.reader(&plan.documents[0].document).values();
    assert_eq!(values.fields.unwrap()["tags"], json!(["a", "b"]), "native JSON rides through");
    assert_eq!(values.body.unwrap(), "Cited.");
    assert_eq!(values.cards.unwrap().len(), 1);
    assert_eq!(plan.documents[0].key, "Ada");
    let errs: Vec<_> = plan.errors().collect();
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert_eq!(errs[0].row, Some(1));
    assert_eq!(errs[0].column, None, "no column: nothing was mapped");
    assert_eq!(errs[0].diagnostic.code.as_deref(), Some("edit::unknown_field"));
    assert_eq!(errs[0].diagnostic.path.as_deref(), Some("main.nope"));
}

#[test]
fn the_spec_hash_ignores_spelling_and_tracks_meaning() {
    let a = spec(CERT_SPEC).hash();
    let b = spec(&CERT_SPEC.replace("  ", " ").replace("\"{recipient}-certificate\"", "'{recipient}-certificate'")).hash();
    let c = spec(&CERT_SPEC.replace("column: Qty", "column: Quantity")).hash();
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.len(), 64);
}

#[test]
fn the_input_hash_tracks_the_values_the_spec_and_the_quill() {
    let q = quill();
    let one = |csv: &[&str], yaml: &str| {
        let plan = plan(&q, &spec(yaml), &rows(&["Name", "Qty"], &[csv]));
        plan.documents[0].input_hash.clone()
    };
    let base = "$quill: certificate@1.2.0\nmap:\n  recipient: { column: Name }\n  qty: { column: Qty }\noutput: \"{recipient}\"\n";
    let same = one(&["Ada", "1"], base);
    assert_eq!(same, one(&["Ada", "1"], base));
    assert_ne!(same, one(&["Ada", "2"], base));
    assert_ne!(same, one(&["Ada", "1"], &base.replace("1.2.0", "1")), "the pin is part of the input");
    assert_ne!(same, one(&["Ada", "1"], &base.replace("{recipient}", "{recipient}-x")));
    assert_eq!(same.len(), 64);
}

#[test]
fn report_serializes_with_its_anchors() {
    let plan = plan(
        &quill(),
        &spec(CERT_SPEC),
        &rows(HEADER, &[&["Ada", "3/1/2026", "abc", "", "", ""]]),
    );
    let json = serde_json::to_value(&plan).unwrap();
    let report = json["report"].as_array().unwrap();
    let warning = &report[0];
    assert_eq!(warning["row"], Value::Null, "spec-level: no row");
    assert_eq!(warning["column"], json!("Notes"));
    let error = &report[1];
    assert_eq!(error["row"], json!(0));
    assert_eq!(error["column"], json!("Qty"));
    assert_eq!(error["diagnostic"]["path"], json!("main.qty"));
    assert_eq!(error["diagnostic"]["severity"], json!("error"));
    assert_eq!(json["skipped_empty"], json!(0));
    assert!(json["documents"].as_array().unwrap().is_empty());
}

/// The render loop the CLI runs, against a real backend: one-shot
/// `engine.render` per document versus one session updated per document.
#[test]
fn taro_batch_renders_through_one_session_or_one_shot_per_document() {
    use quillmark::Quillmark;
    use quillmark_core::{LiveSession, OutputFormat, RenderOptions};
    use std::time::Instant;

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LiveSession>();
    assert_send_sync::<Quill>();
    assert_send_sync::<Quillmark>();

    let q = quillmark::quill_from_path(quillmark_fixtures::quills_path("taro")).unwrap();
    let spec = MergeSpec::new("taro@0.1.0", "{author}");
    let n = 20;
    let data: Vec<[String; 2]> = (0..n)
        .map(|i| [format!("Author {i}"), format!("Taro tasting no. {i}")])
        .collect();
    let rows: Vec<Row> = data
        .iter()
        .map(|[a, t]| {
            IndexMap::from([
                ("author".to_string(), json!(a)),
                ("title".to_string(), json!(t)),
            ])
        })
        .collect();
    let planned = Instant::now();
    let plan = plan(&q, &spec, &Input::Rows(rows));
    let planned = planned.elapsed();
    assert!(plan.is_clean(), "{:#?}", plan.report);
    assert_eq!(plan.documents.len(), n);

    let engine = Quillmark::new();
    let opts = RenderOptions::default().with_output_format(OutputFormat::Pdf);

    let one_shot = Instant::now();
    let mut sizes_a = Vec::new();
    for doc in &plan.documents {
        let result = engine.render(&q, &doc.document, &opts).unwrap();
        sizes_a.push(result.artifacts[0].bytes.len());
    }
    let one_shot = one_shot.elapsed();

    let sessioned = Instant::now();
    let mut session = engine.open(&q, &plan.documents[0].document).unwrap();
    let mut sizes_b = Vec::new();
    for doc in &plan.documents {
        session.update(&doc.document).unwrap();
        let result = session.render(&opts).unwrap();
        sizes_b.push(result.artifacts[0].bytes.len());
    }
    let sessioned = sessioned.elapsed();

    assert!(sizes_a.iter().all(|&s| s > 1000));
    assert_eq!(sizes_a, sizes_b, "both loops produce the same artifacts");
    eprintln!(
        "plan {n} docs: {planned:?}; render one-shot: {one_shot:?} ({:?}/doc); \
         render via one session: {sessioned:?} ({:?}/doc)",
        one_shot / n as u32,
        sessioned / n as u32
    );
}
