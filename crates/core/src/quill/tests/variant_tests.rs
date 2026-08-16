//! Enum variants: fields that exist only for one enum value.
//!
//! The axis has four surfaces that must agree on one answer — load, the render
//! floor, validation, and the authoring projections — and the shape of bug it
//! invites is a disagreement between them. Each test below pins one surface to
//! the same reading of the same schema.

use crate::document::Document;
use crate::quill::{blank, build_transform_schema, quill_from_yaml, FieldSchema, Quill, QuillConfig};
use crate::value::QuillValue;
use serde_json::json;

/// A quill whose `classification` enum brings a `CUI` field set into play:
/// `controlled_by` obliged in that world (no `default:`), `category` optional.
fn quill_yaml() -> &'static str {
    r#"
quill:
  name: variant_probe
  version: "0.1.0"
  backend: typst
  description: Enum variant probe

typst:
  plate_file: plate.typ

main:
  fields:
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI, SECRET]
      default: ""
      variants:
        CUI:
          controlled_by: { type: string }
          category: { type: string, default: "" }
        SECRET:
          declassify_on: { type: string }
    title:
      type: string
      default: ""
"#
}

fn config() -> QuillConfig {
    QuillConfig::from_yaml(quill_yaml()).expect("variant probe loads")
}

fn quill() -> Quill {
    quill_from_yaml(quill_yaml())
}

fn doc(fields: &str) -> Document {
    let markdown =
        format!("~~~\n$quill: variant_probe@0.1.0\n$kind: main\n{fields}~~~\n");
    Document::parse(&markdown).expect("document parses").document
}

fn plate(document: &Document) -> serde_json::Value {
    config()
        .compile_data(document)
        .expect("compile_data succeeds")["classification"]
        .clone()
}

fn codes(document: &Document) -> Vec<(String, String)> {
    quill()
        .validate(document)
        .into_iter()
        .map(|d| {
            (
                d.code.unwrap_or_default(),
                d.path.unwrap_or_default(),
            )
        })
        .collect()
}

fn field(yaml: &str) -> FieldSchema {
    let value = QuillValue::from_yaml_str(yaml).unwrap();
    FieldSchema::from_quill_value("classification".to_string(), &value).unwrap()
}

fn load_error(fields: &str) -> String {
    let yaml = format!(
        r#"
quill:
  name: bad
  version: "0.1.0"
  backend: typst
  description: bad

typst:
  plate_file: plate.typ

main:
  fields:
{fields}
"#
    );
    let err = QuillConfig::from_yaml(&yaml).expect_err("expected a load error");
    format!("{err:?}")
}

// ---------------------------------------------------------------- load errors

#[test]
fn variants_on_a_non_enum_field_is_a_load_error() {
    assert!(load_error(
        "    name:\n      type: string\n      variants:\n        A:\n          x: { type: string }\n"
    )
    .contains("quill::variants_on_non_enum"));
}

#[test]
fn a_variant_keyed_by_a_non_member_is_a_load_error() {
    let err = load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        B:\n          x: { type: string }\n",
    );
    assert!(err.contains("quill::variant_unknown_value"));
}

/// The blank is not a member, so it owns no field set: the same rule
/// `quill::enum_blank_member` states from the `values:` side.
#[test]
fn a_variant_keyed_by_the_blank_is_a_load_error() {
    let err = load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        \"\":\n          x: { type: string }\n",
    );
    assert!(err.contains("quill::variant_unknown_value"));
}

#[test]
fn a_variant_field_named_value_collides_with_the_discriminant() {
    let err = load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        A:\n          value: { type: string }\n",
    );
    assert!(err.contains("quill::variant_reserved_field_name"));
}

/// A hoisted field earns the flat path's key gate: without it a variant could
/// declare `$kind` and forge document metadata.
#[test]
fn a_variant_field_key_obeys_the_field_name_gate() {
    let err = load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        A:\n          $kind: { type: string }\n",
    );
    assert!(err.contains("quill::invalid_field_name"));
}

#[test]
fn an_empty_variant_and_an_empty_variants_map_are_load_errors() {
    assert!(load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        A: {}\n"
    )
    .contains("quill::variant_empty"));
    assert!(
        load_error("    c:\n      type: enum\n      values: [A]\n      variants: {}\n")
            .contains("quill::variant_empty")
    );
}

/// Content and dates lower to Typst through top-level name tables that do not
/// descend into a container, so declaring one inside a variant would load clean
/// and reach the plate as a raw dict. The ceiling is enforced, not discovered.
#[test]
fn a_variant_carries_plain_data_only() {
    for ty in ["richtext", "plaintext", "date", "datetime"] {
        let err = load_error(&format!(
            "    c:\n      type: enum\n      values: [A]\n      variants:\n        A:\n          x: {{ type: {ty} }}\n"
        ));
        assert!(
            err.contains("quill::variant_field_type"),
            "type: {ty} should be refused inside a variant, got {err}"
        );
    }
}

#[test]
fn a_variant_field_may_not_be_a_container_or_carry_a_group() {
    assert!(load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        A:\n          x: { type: object, properties: { y: { type: string } } }\n"
    )
    .contains("quill::nested_object_not_supported"));
    assert!(load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        A:\n          x: { type: array, items: { type: string } }\n"
    )
    .contains("quill::nested_array_not_supported"));
    // Variant fields inherit the discriminant's group; declaring one is the
    // same dead knob a nested `ui.group` always was.
    assert!(load_error(
        "    c:\n      type: enum\n      values: [A]\n      variants:\n        A:\n          x: { type: string, ui: { group: g } }\n"
    )
    .contains("quill::nested_group_not_supported"));
}

/// Two spellings of one name would coerce a live value under the other world's
/// type, failing `validation::type_mismatch` on a document valid against the
/// world it selected.
#[test]
fn a_name_two_worlds_declare_differently_is_a_load_error() {
    let err = load_error(
        "    c:\n      type: enum\n      values: [A, B]\n      variants:\n        A:\n          note: { type: string }\n        B:\n          note: { type: integer }\n",
    );
    assert!(err.contains("quill::variant_field_collision"), "{err}");
    assert!(err.contains("'A'") && err.contains("'B'"), "{err}");
}

/// Repetition is how a shared field set is spelled, so the gate is disagreement
/// rather than repetition.
#[test]
fn a_name_two_worlds_declare_identically_loads() {
    let config = QuillConfig::from_yaml(&quill_yaml().replace(
        "        SECRET:\n          declassify_on: { type: string }",
        "        SECRET:\n          declassify_on: { type: string }\n          controlled_by: { type: string }",
    ))
    .expect("an identically-repeated variant field loads");
    let doc = Document::parse(
        "~~~\n$quill: variant_probe@0.1.0\n$kind: main\nclassification:\n  value: SECRET\n  controlled_by: SAF/AA\n  declassify_on: 20301231\n~~~\n",
    )
    .expect("parses")
    .document;
    let data = config.compile_data(&doc).expect("compile_data succeeds");
    assert_eq!(data["classification"]["controlled_by"], json!("SAF/AA"));
}

/// A variant turns its field into a container, and a container may not sit
/// inside one.
#[test]
fn variants_below_card_level_is_a_load_error() {
    let err = load_error(
        "    o:\n      type: object\n      properties:\n        c:\n          type: enum\n          values: [A]\n          variants:\n            A:\n              x: { type: string }\n",
    );
    assert!(err.contains("quill::variant_placement"));
}

// ----------------------------------------------------------------- the blank

/// The blank activates no variant, so the container carries nothing but the
/// unanswered discriminant.
#[test]
fn the_blank_of_a_variant_bearing_enum_is_the_container_holding_the_blank() {
    let schema = field(
        "type: enum\nvalues: [A]\nvariants:\n  A:\n    x: { type: string }\n",
    );
    assert_eq!(blank(&schema).into_json(), json!({ "value": "" }));
}

/// A plain enum is untouched: `variants:` is the one thing that changes the
/// resting shape.
#[test]
fn a_variantless_enum_still_blanks_to_the_bare_string() {
    let schema = field("type: enum\nvalues: [A]\n");
    assert_eq!(blank(&schema).into_json(), json!(""));
}

// ------------------------------------------------------- the wire (per-world)

#[test]
fn an_empty_document_renders_the_container_with_no_variant_fields() {
    assert_eq!(plate(&doc("")), json!({ "value": "" }));
}

/// Inside the world a plate branches into, every declared field is present —
/// that is what makes an unguarded read total there.
#[test]
fn the_live_world_arrives_complete_and_blank_filled() {
    assert_eq!(
        plate(&doc("classification:\n  value: CUI\n")),
        json!({ "value": "CUI", "controlled_by": "", "category": "" })
    );
}

/// The container is a closed shape: a value belonging to a world nobody selected
/// never reaches the plate, so a payload cannot arrive under a tag that disowns
/// it.
#[test]
fn a_dormant_worlds_fields_never_reach_the_plate() {
    assert_eq!(
        plate(&doc(
            "classification:\n  value: UNCLASSIFIED\n  controlled_by: SAF/AA\n"
        )),
        json!({ "value": "UNCLASSIFIED" })
    );
}

/// The hand-authored spelling of a world with nothing filled in.
#[test]
fn a_bare_scalar_is_adopted_as_the_discriminant() {
    assert_eq!(
        plate(&doc("classification: SECRET\n")),
        json!({ "value": "SECRET", "declassify_on": "" })
    );
}

#[test]
fn the_discriminant_falls_to_the_default_and_carries_its_world() {
    let yaml = quill_yaml().replace("      default: \"\"\n      variants:", "      default: CUI\n      variants:");
    let config = QuillConfig::from_yaml(&yaml).unwrap();
    let data = config.compile_data(&doc("")).unwrap();
    assert_eq!(
        data["classification"],
        json!({ "value": "CUI", "controlled_by": "", "category": "" })
    );
}

/// Null ≡ absent holds through the container: a present-null discriminant
/// blank-fills exactly as an omitted one does.
#[test]
fn a_null_discriminant_blank_fills() {
    assert_eq!(
        plate(&doc("classification:\n  value:\n")),
        json!({ "value": "" })
    );
}

// -------------------------------------------------------------- resolve() parity

/// `resolve()`'s contract is byte-parity with the plate, and the container is
/// one cell carrying one rung (as a typed dictionary is).
#[test]
fn resolve_reports_the_container_as_one_cell_matching_the_plate() {
    let quill = quill();
    let document = doc("classification:\n  value: CUI\n  controlled_by: SAF/AA\n");
    let resolved = quill.resolve(&document);
    let row = resolved
        .main
        .fields
        .iter()
        .find(|f| f.name == "classification")
        .expect("classification row");
    assert_eq!(row.value.as_json(), &plate(&document));
    assert_eq!(row.source, crate::quill::resolved::FieldSource::Authored);

    // An unanswered container reports the rung that supplied its discriminant:
    // here the schema's `default: ""`, exactly as a plain enum would.
    let blank_doc = doc("");
    let resolved = quill.resolve(&blank_doc);
    let row = resolved
        .main
        .fields
        .iter()
        .find(|f| f.name == "classification")
        .unwrap();
    assert_eq!(row.value.as_json(), &plate(&blank_doc));
    assert_eq!(row.source, crate::quill::resolved::FieldSource::Default);
}

// ------------------------------------------------------------------ validation

/// The conditional-obligation payoff: a field with no `default:` is obliged in
/// its own world and silent everywhere else — the thing `must_fill` alone cannot
/// say.
#[test]
fn obligation_follows_the_selected_world() {
    let obliged = codes(&doc("classification:\n  value: CUI\n"));
    assert!(obliged.contains(&(
        "validation::must_fill".to_string(),
        "main.classification.controlled_by".to_string()
    )));
    // `category` declares a `default:`, so it stays skippable in the same world.
    assert!(!obliged
        .iter()
        .any(|(_, path)| path == "main.classification.category"));

    // The identical schema obliges nothing once another world is selected.
    let quiet = codes(&doc("classification:\n  value: UNCLASSIFIED\n"));
    assert!(!quiet
        .iter()
        .any(|(_, path)| path.starts_with("main.classification.")));
}

/// Flipping a discriminant in an editor must not cost the author their answers,
/// and must not hand them a document that refuses to render.
#[test]
fn a_stranded_value_is_kept_and_warned_never_gated() {
    let document = doc("classification:\n  value: UNCLASSIFIED\n  controlled_by: SAF/AA\n");
    let diags = quill().validate(&document);
    let stranded = diags
        .iter()
        .find(|d| d.code.as_deref() == Some("validation::out_of_variant"))
        .expect("out_of_variant warning");
    assert_eq!(stranded.severity, crate::Severity::Warning);
    assert_eq!(
        stranded.path.as_deref(),
        Some("main.classification.controlled_by")
    );
    assert_eq!(stranded.args["variant"], json!("CUI"));
    // Kept in the document…
    assert_eq!(
        document.main().payload().get("classification").unwrap().as_json()["controlled_by"],
        json!("SAF/AA")
    );
    // …and dropped from the wire.
    assert_eq!(plate(&document), json!({ "value": "UNCLASSIFIED" }));
}

/// A key no variant declares is an undeclared field, which every other surface
/// carries without comment; only a key some *other* world owns is the stranded
/// case worth naming.
#[test]
fn an_undeclared_key_draws_no_variant_warning() {
    let document = doc("classification:\n  value: CUI\n  note: hello\n");
    assert!(!quill()
        .validate(&document)
        .iter()
        .any(|d| d.code.as_deref() == Some("validation::out_of_variant")));
}

#[test]
fn the_domain_check_lands_on_the_discriminant_path() {
    let found = codes(&doc("classification:\n  value: NATO\n"));
    assert!(found.contains(&(
        "validation::enum_violation".to_string(),
        "main.classification.value".to_string()
    )));
}

/// The live world's fields still type-check; a dormant world's do not, since
/// nothing downstream reads them.
#[test]
fn only_the_live_worlds_fields_are_type_checked() {
    let bad_live = codes(&doc("classification:\n  value: CUI\n  controlled_by: [1, 2]\n"));
    assert!(bad_live
        .iter()
        .any(|(code, path)| code == "validation::type_mismatch"
            && path == "main.classification.controlled_by"));

    let bad_dormant = codes(&doc(
        "classification:\n  value: UNCLASSIFIED\n  controlled_by: [1, 2]\n",
    ));
    assert!(!bad_dormant
        .iter()
        .any(|(code, _)| code == "validation::type_mismatch"));
}

// --------------------------------------------------------- authoring surfaces

/// A blueprint is a document, so it shows one world and *names* the rest.
#[test]
fn the_blueprint_shows_one_world_and_names_the_others() {
    let bp = config().blueprint();
    assert!(bp.contains("# when CUI: controlled_by, category"));
    assert!(bp.contains("# when SECRET: declassify_on"));
    assert!(bp.contains("classification: # enum<UNCLASSIFIED | CUI | SECRET>"));
    assert!(bp.contains("value: \"\""));
    // The blank world owns no field set, so no variant cell is emitted.
    assert!(!bp.contains("controlled_by:"));
}

#[test]
fn the_blueprint_emits_the_default_worlds_cells_and_round_trips() {
    let yaml = quill_yaml().replace("      default: \"\"\n      variants:", "      default: CUI\n      variants:");
    let config = QuillConfig::from_yaml(&yaml).unwrap();
    let bp = config.blueprint();
    assert!(bp.contains("value: CUI"));
    assert!(bp.contains("controlled_by:"));
    // The blueprint round-trips through the parser by construction, and the
    // container survives it as the container: the shape an author edits is the
    // shape the parser reads back.
    let reparsed = Document::parse(&bp).expect("blueprint round-trips").document;
    let value = reparsed
        .main()
        .payload()
        .get("classification")
        .expect("classification survives the round trip")
        .as_json()
        .clone();
    assert_eq!(value["value"], json!("CUI"));
    assert!(value.get("controlled_by").is_some());
}

/// Which world is live decides which fields are even candidates, so the
/// discriminant must resolve before the field set is walked.
#[test]
fn seeding_resolves_the_discriminant_before_walking_the_field_set() {
    let yaml = quill_yaml()
        .replace("      default: \"\"\n", "      example: CUI\n")
        .replace(
            "          controlled_by: { type: string }",
            "          controlled_by: { type: string, example: SAF/AA }",
        );
    let quill = quill_from_yaml(&yaml);
    let seeded = quill.seed_document();
    let value = seeded
        .main()
        .payload()
        .get("classification")
        .expect("seeded classification")
        .as_json()
        .clone();
    assert_eq!(value["value"], json!("CUI"));
    assert_eq!(value["controlled_by"], json!("SAF/AA"));
    // `declassify_on` belongs to a world the seed did not select.
    assert!(value.get("declassify_on").is_none());
}

#[test]
fn the_transform_schema_flattens_every_world_into_one_container() {
    let schema = build_transform_schema(&config());
    let json = schema.as_json();
    let cls = &json["properties"]["classification"];
    assert_eq!(cls["type"], json!("object"));
    assert_eq!(
        cls["properties"]["value"]["enum"],
        json!(["", "UNCLASSIFIED", "CUI", "SECRET"])
    );
    assert_eq!(cls["properties"]["controlled_by"]["type"], json!("string"));
    assert_eq!(cls["properties"]["declassify_on"]["type"], json!("string"));
}

/// `variants:` on the declaration view states it instead, keyed by member.
#[test]
fn the_transform_schema_does_not_restate_which_member_owns_a_cell() {
    let schema = build_transform_schema(&config());
    let cls = &schema.as_json()["properties"]["classification"];
    for name in ["controlled_by", "category", "declassify_on"] {
        assert_eq!(
            cls["properties"][name]
                .as_object()
                .expect("variant cell projects as an object")
                .keys()
                .filter(|k| k.starts_with("quillmark:variant"))
                .count(),
            0
        );
    }
}

/// `schema()` is the declaration view and emits what the author wrote, so a
/// round-trip through it re-loads.
#[test]
fn the_declaration_schema_round_trips_through_a_reload() {
    let emitted = config().schema();
    let variants = &emitted["main"]["fields"]["classification"]["variants"];
    assert_eq!(variants["CUI"]["controlled_by"]["type"], json!("string"));
    assert_eq!(variants["SECRET"]["declassify_on"]["type"], json!("string"));
}
