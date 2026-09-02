use super::*;
use quillmark_core::FileTreeNode;
use serde_json::json;
use std::collections::HashMap;

const QUILL_YAML: &str = r#"
quill:
  name: certificate
  version: 1.2.0
  backend: typst
  description: merge spike
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
    Quill::from_tree(FileTreeNode::Directory { files }).expect("spike quill loads")
}

fn rows(header: &[&str], data: &[&[&str]]) -> Vec<Row> {
    data.iter()
        .map(|cells| {
            header
                .iter()
                .zip(cells.iter())
                .map(|(h, c)| (h.to_string(), Value::String(c.to_string())))
                .collect()
        })
        .collect()
}

fn errors(plan: &MergePlan) -> Vec<&RowDiagnostic> {
    plan.report
        .iter()
        .filter(|d| d.diagnostic.severity == Severity::Error)
        .collect()
}

fn warnings(plan: &MergePlan) -> Vec<&RowDiagnostic> {
    plan.report
        .iter()
        .filter(|d| d.diagnostic.severity == Severity::Warning)
        .collect()
}

fn spec(yaml: &str) -> MergeSpec {
    MergeSpec::from_yaml(yaml).expect("spec parses")
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
output: "{recipient}-certificate.pdf"
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
    assert_eq!(doc.filename, "Ada-certificate.pdf");
    assert_eq!(doc.rows, [0]);
    let fields = q.reader(&doc.document).values().fields.unwrap();
    assert_eq!(fields["recipient"], json!("Ada"));
    assert_eq!(fields["event"], json!("Rustconf 2026"));
    assert_eq!(fields["awarded_on"], json!("2026-03-01"), "date re-spelled ISO");
    assert_eq!(fields["qty"], json!(2), "the typed write coerced the cell");
    assert_eq!(fields["tags"], json!(["a", "b"]), "split trims pieces");
    assert_eq!(fields["cohort"], json!({"lead": "Grace"}), "dotted address assembles the container");
    assert_eq!(
        doc.document.main().ext().unwrap()["merge"],
        json!({"row_key": "0"})
    );

    let errs = errors(&plan);
    assert_eq!(errs.len(), 1, "{errs:#?}");
    let e = errs[0];
    assert_eq!(e.row, Some(1));
    assert_eq!(e.column.as_deref(), Some("Qty"));
    assert_eq!(e.diagnostic.path.as_deref(), Some("main.qty"));
    assert_eq!(e.diagnostic.code.as_deref(), Some("edit::field_coercion_failed"));

    let unmapped: Vec<_> = warnings(&plan)
        .into_iter()
        .filter(|w| w.diagnostic.code.as_deref() == Some("merge::unmapped_column"))
        .collect();
    assert_eq!(unmapped.len(), 1, "one warning per unmapped column, once");
    assert_eq!(unmapped[0].row, None);
    assert!(unmapped[0].diagnostic.message.contains("'Notes'"));

    assert!(!plan.is_clean());
    assert_eq!(plan.clean_documents().count(), 1, "`--force` renders the clean row");
}

#[test]
fn an_empty_cell_is_absent_and_surfaces_as_the_obligation_warning() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(
            r#"
$quill: certificate@1.2.0
map:
  recipient:  { column: Name }
  awarded_on: { column: Date, format: "%m/%d/%Y" }
  qty:        { column: Qty }
output: "{awarded_on}.pdf"
"#,
        ),
        &rows(&["Name", "Date", "Qty"], &[&["", "3/1/2026", ""]]),
    );
    assert!(plan.is_clean(), "{:#?}", plan.report);
    assert_eq!(plan.documents.len(), 1);
    let fields = q.reader(&plan.documents[0].document).values().fields.unwrap();
    assert!(!fields.contains_key("recipient"), "absent, not authored-empty");
    assert!(!fields.contains_key("qty"), "an empty integer cell is not a coercion error");
    let w = warnings(&plan);
    assert_eq!(w.len(), 1, "{w:#?}");
    assert_eq!(w[0].diagnostic.code.as_deref(), Some("validation::must_fill"));
    assert_eq!(w[0].diagnostic.path.as_deref(), Some("main.recipient"));
    assert_eq!(w[0].row, Some(0));
    assert_eq!(w[0].column.as_deref(), Some("Name"), "reverse-mapped from the path");
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
output: "{recipient}.pdf"
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
    let w = warnings(&plan);
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
    let errs = errors(&plan);
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert_eq!(errs[0].row, Some(1));
    assert_eq!(errs[0].diagnostic.code.as_deref(), Some("merge::output_collision"));
}

const INVOICE_SPEC: &str = r#"
$quill: certificate@1.2.0
mode: cards
group_by: "Invoice #"
map:
  recipient: { column: Customer }
  event:     { column: "Invoice #" }
  awarded_on: { value: "2026-03-01" }
cards:
  line_item:
    map:
      desc: { column: Description }
      qty:  { column: Qty }
output: "invoice-{event}.pdf"
"#;

const INVOICE_HEADER: &[&str] = &["Invoice #", "Customer", "Description", "Qty"];

#[test]
fn cards_mode_groups_rows_in_first_appearance_order_and_keeps_row_order() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(INVOICE_SPEC),
        &rows(
            INVOICE_HEADER,
            &[
                &["INV-2", "Bolt", "Gear", "1"],
                &["INV-1", "Acme", "Widget", "2"],
                &["INV-2", "Bolt", "Sprocket", "3"],
                &["INV-1", "Acme", "Gadget", "4"],
            ],
        ),
    );
    assert!(plan.is_clean(), "{:#?}", plan.report);
    let keys: Vec<&str> = plan.documents.iter().map(|d| d.key.as_str()).collect();
    assert_eq!(keys, ["INV-2", "INV-1"]);
    assert_eq!(plan.documents[0].rows, [0, 2]);
    assert_eq!(plan.documents[0].filename, "invoice-INV-2.pdf");
    let values = q.reader(&plan.documents[0].document).values();
    assert_eq!(values.fields.unwrap()["recipient"], json!("Bolt"));
    let cards = values.cards.unwrap();
    let descs: Vec<&Value> = cards.iter().map(|c| &c.fields.as_ref().unwrap()["desc"]).collect();
    assert_eq!(descs, [&json!("Gear"), &json!("Sprocket")]);
    assert_eq!(cards[1].fields.as_ref().unwrap()["qty"], json!(3));
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
                &["INV-1", "Acme", "Widget", "2"],
                &["INV-2", "Bolt", "Gear", "1"],
                &["INV-1", "Acme", "Gadget", "x"],
                &["INV-2", "Bolt Inc", "Sprocket", "3"],
                &["", "Nobody", "Orphan", "1"],
            ],
        ),
    );
    assert!(plan.documents.is_empty());
    let errs = errors(&plan);
    assert_eq!(errs.len(), 3, "{errs:#?}");

    let card_err = errs.iter().find(|e| e.diagnostic.code.as_deref() == Some("edit::field_coercion_failed")).unwrap();
    assert_eq!(card_err.row, Some(2), "the card's own source row, not the group's first");
    assert_eq!(card_err.column.as_deref(), Some("Qty"));
    assert_eq!(card_err.diagnostic.path.as_deref(), Some("cards.line_item[1].qty"));

    let conflict = errs.iter().find(|e| e.diagnostic.code.as_deref() == Some("merge::group_conflict")).unwrap();
    assert_eq!(conflict.row, Some(3));
    assert_eq!(conflict.column.as_deref(), Some("Customer"));
    assert_eq!(conflict.diagnostic.path.as_deref(), Some("main.recipient"));

    let orphan = errs.iter().find(|e| e.diagnostic.code.as_deref() == Some("merge::missing_group_key")).unwrap();
    assert_eq!(orphan.row, Some(4));
    assert_eq!(orphan.column.as_deref(), Some("Invoice #"));
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
output: "x.pdf"
"#,
        ),
        &rows(&["Name"], &[&["Ada"]]),
    );
    assert!(plan.documents.is_empty());
    let codes: Vec<&str> = errors(&plan)
        .iter()
        .filter_map(|e| e.diagnostic.code.as_deref())
        .collect();
    assert!(codes.contains(&"merge::quill_mismatch"), "{codes:?}");
    assert!(codes.contains(&"merge::spec_mode"), "{codes:?}");
    assert!(codes.contains(&"merge::spec_unknown_target"), "{codes:?}");
    assert!(codes.contains(&"merge::spec_mapping"), "{codes:?}");
    assert!(codes.contains(&"merge::unknown_column"), "{codes:?}");
    assert!(errors(&plan).iter().all(|e| e.row.is_none()));
}

#[test]
fn a_bad_date_reports_the_cell_and_leaves_the_field_absent() {
    let q = quill();
    let plan = plan(
        &q,
        &spec(CERT_SPEC),
        &rows(HEADER, &[&["Ada", "2026-03-01", "1", "", "", ""]]),
    );
    let errs = errors(&plan);
    assert_eq!(errs.len(), 1, "{errs:#?}");
    assert_eq!(errs[0].diagnostic.code.as_deref(), Some("merge::date_format"));
    assert_eq!(errs[0].row, Some(0));
    assert_eq!(errs[0].column.as_deref(), Some("Date"));
    assert_eq!(errs[0].diagnostic.path.as_deref(), Some("main.awarded_on"));
    assert_eq!(plan.clean_documents().count(), 0);
    let _ = q;
}

/// The render loop the CLI would run, against a real backend: one-shot
/// `engine.render` per document versus one session updated per document.
#[test]
fn taro_batch_renders_through_one_session_or_one_shot_per_document() {
    use quillmark::Quillmark;
    use quillmark_core::{LiveSession, OutputFormat, RenderOptions};
    use std::time::Instant;

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LiveSession>();
    assert_send_sync::<Quill>();

    let q = quillmark::quill_from_path(quillmark_fixtures::quills_path("taro")).unwrap();
    let spec = spec(
        r#"
$quill: taro@0.1.0
map:
  author: { column: Author }
  title:  { column: Title }
output: "{author}.pdf"
"#,
    );
    let n = 20;
    let data: Vec<[String; 2]> = (0..n)
        .map(|i| [format!("Author {i}"), format!("Taro tasting no. {i}")])
        .collect();
    let rows: Vec<Row> = data
        .iter()
        .map(|[a, t]| {
            IndexMap::from([
                ("Author".to_string(), json!(a)),
                ("Title".to_string(), json!(t)),
            ])
        })
        .collect();
    let planned = Instant::now();
    let plan = plan(&q, &spec, &rows);
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

