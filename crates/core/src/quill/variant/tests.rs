//! The variant axis: the load-time hoist, and the projections that read it.

use crate::quill::schema::{QUILLMARK_MUST_FILL_KEY, QUILLMARK_VARIANT_OF_KEY};
use crate::quill::{blank, build_transform_schema, quill_from_yaml, QuillConfig};
use crate::{Document, Quill};

const CLASSIFICATION: &str = r#"
    classification:
      type: enum
      values: [UNCLASSIFIED, CUI, SECRET]
      default: ""
      variants:
        CUI:
          cui_controlled_by: { type: string }
          cui_category: { type: string, default: "" }
        SECRET:
          declassify_on: { type: date }
"#;

fn yaml(fields: &str) -> String {
    format!(
        "quill: {{ name: vf, version: 1.0.0, backend: typst, description: variants }}\nmain:\n  fields:\n{fields}"
    )
}

fn config(fields: &str) -> QuillConfig {
    QuillConfig::from_yaml_with_warnings(&yaml(fields))
        .map(|(config, _)| config)
        .unwrap_or_else(|e| panic!("schema should load: {e:?}"))
}

fn load_err(fields: &str) -> Vec<String> {
    QuillConfig::from_yaml_with_warnings(&yaml(fields))
        .err()
        .unwrap_or_default()
        .iter()
        .filter_map(|d| d.code.clone())
        .collect()
}

fn md(fields: &str) -> String {
    format!("~~~\n$quill: vf@1.0.0\n$kind: main\n{fields}~~~\n")
}

fn doc(fields: &str) -> Document {
    Document::parse(&md(fields)).expect("document parses").document
}

/// Every diagnostic as `(code, path)`, sorted so assertions read as sets.
fn diags(quill: &Quill, doc: &Document) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = quill
        .validate(doc)
        .iter()
        .map(|d| {
            (
                d.code.clone().unwrap_or_default(),
                d.path.clone().unwrap_or_default(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn hoist_lands_variant_fields_after_their_discriminant() {
    let config = config(&format!("{CLASSIFICATION}    trailing:\n      type: string\n"));
    let names: Vec<&str> = config.main.fields.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "classification",
            "cui_controlled_by",
            "cui_category",
            "declassify_on",
            "trailing",
        ],
        "variants hoist in declaration order, immediately after the discriminant"
    );
    let stamp = |name: &str| {
        config.main.fields[name]
            .variant_of
            .as_ref()
            .map(|v| (v.field.as_str(), v.value.as_str()))
    };
    assert_eq!(stamp("cui_controlled_by"), Some(("classification", "CUI")));
    assert_eq!(stamp("declassify_on"), Some(("classification", "SECRET")));
    assert_eq!(stamp("trailing"), None);
    assert!(
        config.main.fields["classification"].variants.is_none(),
        "the hoist drains `variants:`: the flat field map is the one carrier"
    );
}

/// The variant scopes the obligation; it does not add a second axis.
#[test]
fn a_variant_field_keeps_its_own_type_and_obligation() {
    let config = config(CLASSIFICATION);
    assert!(config.main.fields["cui_controlled_by"].must_fill());
    assert!(!config.main.fields["cui_category"].must_fill());
}

#[test]
fn variants_on_a_non_enum_field_is_a_load_error() {
    let codes = load_err(
        "    note:\n      type: string\n      variants:\n        CUI:\n          x: { type: string }\n",
    );
    assert!(codes.contains(&"quill::variants_on_non_enum".to_string()), "{codes:?}");
}

#[test]
fn a_variant_key_outside_the_domain_is_a_load_error() {
    let codes = load_err(
        "    level:\n      type: enum\n      values: [a, b]\n      variants:\n        c:\n          x: { type: string }\n",
    );
    assert!(codes.contains(&"quill::variant_unknown_value".to_string()), "{codes:?}");
}

/// The blank is no member, so a field under it would exist under "nobody chose".
#[test]
fn the_blank_owns_no_variant() {
    let codes = load_err(
        "    level:\n      type: enum\n      values: [a, b]\n      variants:\n        \"\":\n          x: { type: string }\n",
    );
    assert!(codes.contains(&"quill::variant_unknown_value".to_string()), "{codes:?}");
}

/// One namespace per card: a name resolving to two schemas would make a field's
/// type depend on the discriminant's value.
#[test]
fn a_variant_field_colliding_with_a_flat_field_is_a_load_error() {
    let codes = load_err(
        "    level:\n      type: enum\n      values: [a, b]\n      variants:\n        a:\n          note: { type: integer }\n    note:\n      type: string\n",
    );
    assert!(codes.contains(&"quill::variant_field_collision".to_string()), "{codes:?}");
}

#[test]
fn two_variants_declaring_one_name_is_a_load_error() {
    let codes = load_err(
        "    level:\n      type: enum\n      values: [a, b]\n      variants:\n        a:\n          note: { type: string }\n        b:\n          note: { type: integer }\n",
    );
    assert!(codes.contains(&"quill::variant_field_collision".to_string()), "{codes:?}");
}

#[test]
fn variants_below_card_level_is_a_load_error() {
    let codes = load_err(
        "    row:\n      type: object\n      properties:\n        level:\n          type: enum\n          values: [a]\n          variants:\n            a:\n              x: { type: string }\n",
    );
    assert!(codes.contains(&"quill::variant_placement".to_string()), "{codes:?}");
}

#[test]
fn variants_do_not_nest() {
    let codes = load_err(
        "    level:\n      type: enum\n      values: [a]\n      variants:\n        a:\n          inner:\n            type: enum\n            values: [x]\n            variants:\n              x:\n                deep: { type: string }\n",
    );
    assert!(codes.contains(&"quill::variant_placement".to_string()), "{codes:?}");
}

/// `variant_of` is emission-only; the authoring spelling stays nested.
#[test]
fn authoring_variant_of_directly_is_a_load_error() {
    let codes = load_err(
        "    note:\n      type: string\n      variant_of:\n        field: level\n        value: a\n",
    );
    assert!(codes.contains(&"quill::field_parse_error".to_string()), "{codes:?}");
}

#[test]
fn schema_emits_the_hoisted_form_with_variant_of() {
    let schema = config(CLASSIFICATION).schema();
    let field = &schema["main"]["fields"]["cui_controlled_by"];
    assert_eq!(field["variant_of"]["field"], "classification");
    assert_eq!(field["variant_of"]["value"], "CUI");
    assert!(
        schema["main"]["fields"]["classification"]
            .get("variants")
            .is_none(),
        "the declaration nests, the emission hoists: one shape reaches consumers"
    );
}

#[test]
fn transform_schema_scopes_must_fill_with_variant_of() {
    let wire = build_transform_schema(&config(CLASSIFICATION));
    let json = wire.as_json();
    let field = &json["properties"]["cui_controlled_by"];
    assert_eq!(field[QUILLMARK_VARIANT_OF_KEY]["value"], "CUI");
    // Separate answers: must be filled, in the world it belongs to.
    assert_eq!(field[QUILLMARK_MUST_FILL_KEY], serde_json::json!(true));
    assert!(
        json["properties"]["classification"]
            .get(QUILLMARK_VARIANT_OF_KEY)
            .is_none(),
        "an unconditional field carries no scope"
    );
}

/// The plate reads a variant field unconditionally, so the floor stays total.
#[test]
fn an_out_of_play_field_is_still_blank_filled_at_the_floor() {
    let plate = config(CLASSIFICATION)
        .compile_data(&doc("classification: UNCLASSIFIED\n"))
        .expect("blank-filled render is total over every declared field");
    assert_eq!(plate["cui_controlled_by"], "");
    assert_eq!(plate["declassify_on"], "");
}

#[test]
fn an_unauthored_obligation_waits_for_its_variant() {
    let quill = quill_from_yaml(&yaml(CLASSIFICATION));

    let unclassified = diags(&quill, &doc("classification: UNCLASSIFIED\n"));
    assert!(
        !unclassified
            .iter()
            .any(|(_, path)| path == "main.cui_controlled_by"),
        "an out-of-play cell obliges nothing: {unclassified:?}"
    );

    // One cell changed, and the cells a strict loop must fill are now reported.
    let cui = diags(&quill, &doc("classification: CUI\n"));
    assert!(
        cui.contains(&(
            "validation::must_fill".to_string(),
            "main.cui_controlled_by".to_string()
        )),
        "an in-play must-fill cell warns unauthored: {cui:?}"
    );
    assert!(
        !cui.iter().any(|(_, path)| path == "main.cui_category"),
        "a defaulted variant field stays skippable: {cui:?}"
    );
}

#[test]
fn an_authored_value_outside_its_variant_warns() {
    let quill = quill_from_yaml(&yaml(CLASSIFICATION));
    let found = diags(
        &quill,
        &doc("classification: UNCLASSIFIED\ncui_controlled_by: SAF/AA\n"),
    );
    assert!(
        found.contains(&(
            "validation::out_of_variant".to_string(),
            "main.cui_controlled_by".to_string()
        )),
        "{found:?}"
    );
}

#[test]
fn out_of_variant_never_gates_render() {
    let document = doc("classification: UNCLASSIFIED\ncui_controlled_by: SAF/AA\n");
    let severities: Vec<_> = quill_from_yaml(&yaml(CLASSIFICATION))
        .validate(&document)
        .iter()
        .filter(|d| d.code.as_deref() == Some("validation::out_of_variant"))
        .map(|d| d.severity)
        .collect();
    assert_eq!(severities, [crate::Severity::Warning]);
    // Carried, not dropped: a flip must not cost the author their answer.
    let plate = config(CLASSIFICATION)
        .compile_data(&document)
        .expect("still renders");
    assert_eq!(plate["cui_controlled_by"], "SAF/AA");
}

#[test]
fn a_blank_or_absent_out_of_play_field_is_silent() {
    let quill = quill_from_yaml(&yaml(CLASSIFICATION));
    for fields in [
        "classification: UNCLASSIFIED\n",
        "classification: UNCLASSIFIED\ncui_controlled_by: \"\"\n",
        "classification: UNCLASSIFIED\ncui_controlled_by: null\n",
    ] {
        let found = diags(&quill, &doc(fields));
        assert!(
            !found
                .iter()
                .any(|(code, _)| code == "validation::out_of_variant"),
            "{fields:?} should be silent: {found:?}"
        );
    }
}

/// The marker is document-sovereign: the schema never suppresses one.
#[test]
fn a_marker_on_an_out_of_play_field_still_warns() {
    let quill = quill_from_yaml(&yaml(CLASSIFICATION));
    let found = diags(
        &quill,
        &doc("classification: UNCLASSIFIED\ncui_controlled_by: !must_fill\n"),
    );
    assert!(
        found.contains(&(
            "validation::must_fill".to_string(),
            "main.cui_controlled_by".to_string()
        )),
        "{found:?}"
    );
}

#[test]
fn the_blueprint_shows_one_world_and_names_the_others() {
    // Blank default: no variant is active, so every one of them is skipped.
    let blueprint = config(CLASSIFICATION).blueprint();
    assert!(!blueprint.contains("cui_controlled_by:"), "{blueprint}");
    assert!(
        blueprint.contains("# when CUI: cui_controlled_by, cui_category"),
        "{blueprint}"
    );
    assert!(
        blueprint.contains("# when SECRET: declassify_on"),
        "{blueprint}"
    );
}

#[test]
fn the_blueprint_emits_the_variant_its_own_cell_names() {
    let blueprint = config(&CLASSIFICATION.replace(r#"default: """#, "default: CUI")).blueprint();
    assert!(
        blueprint.contains("cui_controlled_by: !must_fill"),
        "{blueprint}"
    );
    assert!(
        !blueprint.contains("# when CUI:"),
        "the shown world needs no when line: {blueprint}"
    );
    assert!(
        blueprint.contains("# when SECRET: declassify_on"),
        "{blueprint}"
    );
    assert!(!blueprint.contains("declassify_on:"), "{blueprint}");
}

#[test]
fn a_blueprint_carrying_variants_still_round_trips() {
    let config = config(CLASSIFICATION);
    let blueprint = config.blueprint();
    let parsed = Document::parse(&blueprint).expect("blueprint parses").document;
    assert_eq!(parsed.to_markdown(), blueprint, "blueprint round-trips");
    config.compile_data(&parsed).expect("every blueprint renders");
}

#[test]
fn seeding_commits_only_the_active_variant() {
    let active = CLASSIFICATION
        .replace(r#"default: """#, "default: CUI")
        .replace(
            "cui_controlled_by: { type: string }",
            "cui_controlled_by: { type: string, example: SAF/AA }",
        )
        .replace(
            "declassify_on: { type: date }",
            "declassify_on: { type: date, example: 2030-01-01 }",
        );
    let seeded = quill_from_yaml(&yaml(&active)).seed_document();
    let payload = seeded.main().payload();
    assert!(payload.get("cui_controlled_by").is_some());
    assert!(
        payload.get("declassify_on").is_none(),
        "an out-of-play example never seeds"
    );
}

#[test]
fn variants_work_on_a_card_kind() {
    let config = QuillConfig::from_yaml_with_warnings(
        r#"
quill: { name: vf, version: 1.0.0, backend: typst, description: variants }
main:
  fields:
    title: { type: string, default: "" }
card_kinds:
  indorsement:
    fields:
      action:
        type: enum
        values: [approve, disapprove]
        default: approve
        variants:
          disapprove:
            reason: { type: string }
"#,
    )
    .map(|(config, _)| config)
    .expect("loads");
    let card = config.card_kind("indorsement").unwrap();
    let names: Vec<&str> = card.fields.keys().map(String::as_str).collect();
    assert_eq!(names, ["action", "reason"]);
    assert_eq!(
        card.fields["reason"]
            .variant_of
            .as_ref()
            .map(|v| v.value.as_str()),
        Some("disapprove")
    );
}

#[test]
fn two_discriminants_on_one_card_stay_independent() {
    let two = "    kind:\n      type: enum\n      values: [letter, memo]\n      default: letter\n      variants:\n        memo:\n          memo_for: { type: string }\n    urgency:\n      type: enum\n      values: [routine, flash]\n      default: flash\n      variants:\n        flash:\n          callback: { type: string }\n";
    let config = config(two);
    assert_eq!(
        config.main.fields.keys().map(String::as_str).collect::<Vec<_>>(),
        ["kind", "memo_for", "urgency", "callback"]
    );

    // `kind` reads its default `letter`, so `memo_for` is out of play, while
    // `urgency` reads `flash` and brings `callback` in.
    let quill = quill_from_yaml(&yaml(two));
    let found = diags(&quill, &doc(""));
    assert!(
        !found.iter().any(|(_, path)| path == "main.memo_for"),
        "the out-of-play axis obliges nothing: {found:?}"
    );
    assert!(
        found.contains(&(
            "validation::must_fill".to_string(),
            "main.callback".to_string()
        )),
        "the in-play axis obliges its own field: {found:?}"
    );
}

/// A hoisted field is a card field, so it earns the flat path's key gate.
/// Without it a variant could name a `$` key and the blueprint would emit a
/// document with a duplicate mapping key, breaking its parseability guarantee.
#[test]
fn a_variant_field_name_must_be_a_valid_field_key() {
    for bad in ["$kind", "$quill", "$body", "Foo", "bad-name", "9x", "a b"] {
        let codes = load_err(&format!(
            "    level:\n      type: enum\n      values: [a]\n      default: a\n      variants:\n        a:\n          \"{bad}\": {{ type: string, default: \"\" }}\n"
        ));
        assert!(
            codes.contains(&"quill::invalid_field_name".to_string()),
            "{bad:?} must be rejected: {codes:?}"
        );
    }
}

/// The variant axis and coercion must read one discriminant. Coercion adopts a
/// bare scalar's canonical text, so an axis reading `as_str()` alone would call
/// the field out of play while the plate puts it in.
#[test]
fn a_bare_scalar_discriminant_reads_as_its_coerced_text() {
    let bare = "    flag:\n      type: enum\n      values: [\"true\", \"false\"]\n      default: \"false\"\n      variants:\n        \"true\":\n          why: { type: string }\n";
    let quill = quill_from_yaml(&yaml(bare));

    let authored = doc("flag: true\nwhy: because\n");
    assert!(
        !diags(&quill, &authored)
            .iter()
            .any(|(code, _)| code == "validation::out_of_variant"),
        "the field is in play: the plate lowers `flag: true` to `\"true\"`"
    );
    assert_eq!(
        config(bare).compile_data(&authored).expect("renders")["why"],
        "because"
    );
    // The converse leak: an in-play obligation must not go silent either.
    assert!(
        diags(&quill, &doc("flag: true\n")).contains(&(
            "validation::must_fill".to_string(),
            "main.why".to_string()
        )),
        "an in-play must-fill cell still warns under a bare-scalar discriminant"
    );
}

/// `Quill::validate` reads the payload before coercion, so a content leaf rests
/// at `""` and a typed dict at `{}` rather than at the coerced blank. Comparing
/// against the blank alone would warn at an already-clear field, with no edit
/// that resolves it.
#[test]
fn an_uncoerced_blank_out_of_play_field_is_silent() {
    let shapes = "    level:\n      type: enum\n      values: [p, q]\n      default: p\n      variants:\n        q:\n          rt: { type: richtext }\n          obj: { type: object, properties: { a: { type: string } } }\n          arr: { type: array, items: { type: string } }\n";
    let found = diags(
        &quill_from_yaml(&yaml(shapes)),
        &doc("level: p\nrt: \"\"\nobj: {}\narr: []\n"),
    );
    assert!(
        !found
            .iter()
            .any(|(code, _)| code == "validation::out_of_variant"),
        "{found:?}"
    );
}

#[test]
fn blank_is_unchanged_by_the_variant_axis() {
    let config = config(CLASSIFICATION);
    assert_eq!(
        blank(&config.main.fields["cui_controlled_by"]).into_json(),
        serde_json::json!("")
    );
}
