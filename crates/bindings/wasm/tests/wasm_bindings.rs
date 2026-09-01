//! Real-browser wasm32 coverage: node/vitest cannot prove the
//! `serde_wasm_bindgen` boundary produces the right JS types under a real
//! browser's instantiation, so the assertions stay at the crossing.

use wasm_bindgen_test::*;

use quillmark_wasm::{Document, Quill, Quillmark, RenderOptions};

mod common;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn small_quill_tree() -> wasm_bindgen::JsValue {
    common::tree(&[
        (
            "Quill.yaml",
            b"quill:\n  name: test_quill\n  backend: typst\n  description: Test quill for WASM bindings\n\ntypst:\n  plate_file: plate.typ\n",
        ),
        ("plate.typ", b"= Title\n\nThis is a test."),
    ])
}

const SIMPLE_MARKDOWN: &str =
    "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: Hello\n~~~\n\n# Hello\n";

#[wasm_bindgen_test]
fn test_render_from_document() {
    let engine = Quillmark::new();
    let quill = Quill::from_tree(small_quill_tree()).expect("quill failed");

    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let result = engine
        .render(&quill, &doc, Some(RenderOptions::default()))
        .expect("render from Document failed");

    assert!(
        !result.artifacts.is_empty(),
        "should produce at least one artifact"
    );
    assert_eq!(
        result.warnings.len(),
        0,
        "no warnings expected for matching quill_ref"
    );
}

/// Guards the declared `Uint8Array` type against serde's default `Vec<u8>`
/// serializer reverting to `Array<number>`.
#[wasm_bindgen_test]
fn test_artifact_bytes_is_uint8array() {
    use serde::Serialize;
    use wasm_bindgen::{JsCast, JsValue};

    let engine = Quillmark::new();
    let quill = Quill::from_tree(small_quill_tree()).expect("quill failed");
    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let result = engine
        .render(&quill, &doc, Some(RenderOptions::default()))
        .expect("render failed");
    assert!(!result.artifacts.is_empty(), "should produce artifacts");

    // The same serializer Tsify uses for `into_wasm_abi`.
    let serializer = serde_wasm_bindgen::Serializer::new();
    let js_result = result
        .serialize(&serializer)
        .expect("RenderResult serialization");
    let artifacts = js_sys::Reflect::get(&js_result, &JsValue::from_str("artifacts"))
        .expect("artifacts present");
    let arr = js_sys::Array::from(&artifacts);
    assert!(arr.length() > 0, "artifacts array non-empty");

    let first = arr.get(0);
    let bytes = js_sys::Reflect::get(&first, &JsValue::from_str("bytes")).expect("bytes present");
    assert!(
        bytes.is_instance_of::<js_sys::Uint8Array>(),
        "artifact.bytes must be a Uint8Array at the WASM boundary, not a number[]"
    );
    let typed = bytes.unchecked_into::<js_sys::Uint8Array>();
    assert!(typed.length() > 0, "Uint8Array has bytes");
}

#[wasm_bindgen_test]
fn test_open_session_render() {
    let engine = Quillmark::new();
    let quill = Quill::from_tree(small_quill_tree()).expect("quill failed");

    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let session = engine.open(&quill, &doc).expect("open failed");
    assert!(session.page_count() > 0, "session should expose page count");

    let result = session
        .render(Some(RenderOptions::default()))
        .expect("session render failed");
    assert!(!result.artifacts.is_empty(), "should produce artifacts");
}

/// A plate placing a two-paragraph richtext `body` field.
fn region_quill_tree() -> wasm_bindgen::JsValue {
    common::tree(&[
        (
            "Quill.yaml",
            b"quill:\n  name: region_quill\n  version: 0.1.0\n  backend: typst\n  description: region + navigation wasm surface test\ntypst:\n  plate_file: plate.typ\nmain:\n  fields:\n    body:\n      type: richtext\n      description: a two-paragraph body\n",
        ),
        (
            "plate.typ",
            b"#import \"@local/quillmark-helper:0.1.0\": data\n#set page(width: 612pt, height: 792pt, margin: 72pt)\n#set text(size: 11pt)\n\n#data.body\n",
        ),
    ])
}

/// Which boxes land where is geometry, pinned in
/// `backends/typst/tests/content_regions.rs`.
#[wasm_bindgen_test]
fn test_session_region_shapes_cross_the_boundary() {
    let engine = Quillmark::new();
    let quill = Quill::from_tree(region_quill_tree()).expect("quill failed");
    let md = "~~~card-yaml\n$quill: region_quill\n$kind: main\nbody: |\n  First paragraph, alpha.\n\n  Second paragraph, beta.\n~~~\n\nignored\n";
    let doc = Document::from_markdown(md).expect("fromMarkdown failed");
    let session = engine.open(&quill, &doc).expect("open failed");

    let regions_js = session.regions().expect("regions");
    let regions: Vec<serde_json::Value> =
        serde_wasm_bindgen::from_value(regions_js).expect("regions deserialize");
    let body: Vec<&serde_json::Value> = regions.iter().filter(|r| r["field"] == "body").collect();
    let first = *body
        .first()
        .unwrap_or_else(|| panic!("expected a body region, got: {regions:?}"));
    assert!(
        first.get("span").and_then(|s| s.as_array()).is_some(),
        "a content segment region carries a span: {first}"
    );

    // A click on the segment's ink resolves to a ContentHit, and `locate` maps
    // the offset back to a caret rect: both crossing as typed structs.
    let rect = first["rect"].as_array().expect("rect array");
    let page = first["page"].as_u64().unwrap() as usize;
    let (cx, cy) = (
        rect[0].as_f64().unwrap() as f32 + 5.0,
        rect[3].as_f64().unwrap() as f32 - 3.0,
    );
    let hit = session
        .position_at(page, cx, cy, None)
        .expect("positionAt inside content resolves");
    assert_eq!(hit.field, "body");
    let caret = session.locate("body", hit.pos).expect("locate");
    assert_eq!(caret.span, Some([hit.pos, hit.pos]));
}

#[wasm_bindgen_test]
fn test_json_dto_round_trip() {
    let md = "~~~card-yaml\n$quill: test_quill\n$kind: main\ntitle: Hello\nsubject: !must_fill A Subject\n~~~\n\n# Hello\n\n~~~card-yaml\n$kind: note\nfor: someone\n~~~\n\nNote body.\n";
    let doc = Document::from_markdown(md).expect("fromMarkdown failed");

    let dto = doc.to_stored();
    assert!(
        dto.contains("\"quillmark/document@0.112.0\""),
        "DTO string must carry the schema version, got: {dto}"
    );

    let restored = Document::from_stored(&dto).expect("fromStored failed");
    assert!(
        restored.equals(&doc),
        "fromStored(toStored(doc)) must equal doc"
    );
    assert_eq!(restored.quill_ref(), doc.quill_ref());
}

/// A plain object tree must be accepted equivalently to a `Map` tree.
#[wasm_bindgen_test]
fn test_quill_from_object_tree() {
    let entries: &[(&str, &[u8])] = &[
        (
            "Quill.yaml",
            b"quill:\n  name: test_quill\n  backend: typst\n  description: Test quill for WASM bindings\n\ntypst:\n  plate_file: plate.typ\n",
        ),
        ("plate.typ", b"= Title\n\nThis is a test."),
    ];

    let engine = Quillmark::new();
    let from_map = Quill::from_tree(common::tree(entries)).expect("Map form failed");
    let from_obj = Quill::from_tree(common::tree_object(entries)).expect("Object form failed");

    assert_eq!(from_map.backend_id(), from_obj.backend_id());

    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let doc2 = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let r_map = engine
        .render(&from_map, &doc, Some(RenderOptions::default()))
        .expect("render from Map form");
    let r_obj = engine
        .render(&from_obj, &doc2, Some(RenderOptions::default()))
        .expect("render from object form");
    assert_eq!(r_map.artifacts.len(), r_obj.artifacts.len());
}

#[wasm_bindgen_test]
fn test_quill_metadata_and_schemas() {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let get = |obj: &JsValue, key: &str| Reflect::get(obj, &JsValue::from_str(key)).unwrap();
    let get_str = |obj: &JsValue, key: &str| get(obj, key).as_string();

    let engine = Quillmark::new();
    let quill = Quill::from_tree(common::tree(&[
        (
            "Quill.yaml",
            b"quill:\n  name: meta_quill\n  backend: typst\n  version: \"0.2.1\"\n  description: Metadata quill\nmain:\n  fields:\n    title:\n      type: string\n      ui:\n        group: Header\ncard_kinds:\n  indorsement:\n    fields:\n      signature_block:\n        type: string\n",
        ),
        ("plate.typ", b"= Title"),
    ]))
    .expect("quill load");

    let meta = quill.metadata().unwrap();
    assert_eq!(get_str(&meta, "name").as_deref(), Some("meta_quill"));
    assert_eq!(get_str(&meta, "version").as_deref(), Some("0.2.1"));
    assert_eq!(get_str(&meta, "backend").as_deref(), Some("typst"));
    assert_eq!(get_str(&meta, "author").as_deref(), Some("Unknown"));
    assert_eq!(
        get_str(&meta, "description").as_deref(),
        Some("Metadata quill")
    );
    // `supportedFormats` is an engine capability, not a `metadata` key.
    let formats = engine.supported_formats(&quill).expect("supported_formats");
    assert!(js_sys::Array::from(&formats).length() > 0);
    assert!(get(&meta, "schema").is_undefined());

    let schema = quill.schema().unwrap();
    let main_fields = get(&get(&schema, "main"), "fields");
    assert!(get(&get(&main_fields, "title"), "ui").is_object());
    assert!(get(&main_fields, "QUILL").is_undefined());
    let card_fields = get(&get(&get(&schema, "card_kinds"), "indorsement"), "fields");
    assert!(get(&card_fields, "CARD").is_undefined());
}

/// What the seeds *contain* is core's (`core/src/quill/seed/tests.rs`).
#[wasm_bindgen_test]
fn test_seed_verbs_cross_the_boundary() {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let get = |obj: &JsValue, key: &str| Reflect::get(obj, &JsValue::from_str(key)).unwrap();

    let quill = Quill::from_tree(common::tree(&[
        (
            "Quill.yaml",
            b"quill:\n  name: seed_quill\n  backend: typst\n  version: \"1.0\"\n  description: Seed quill\nmain:\n  fields:\n    byline:\n      type: string\n      example: FIRST LAST\ncard_kinds:\n  note:\n    fields:\n      text:\n        type: string\n        example: NOTE EXAMPLE\n",
        ),
        ("plate.typ", b"= Title"),
    ]))
    .expect("quill load");

    assert!(!quill.seed_document().to_markdown().is_empty());
    let main = quill.seed_main().unwrap();
    assert_eq!(get(&main, "kind").as_string().as_deref(), Some("main"));
    let note = quill.seed_card("note", JsValue::UNDEFINED).unwrap();
    assert_eq!(get(&note, "kind").as_string().as_deref(), Some("note"));
    assert!(
        quill
            .seed_card("missing", JsValue::UNDEFINED)
            .unwrap()
            .is_undefined(),
        "unknown kind must be undefined"
    );

    // `main.seed[kind]` is the per-kind overlay slot the write verbs target.
    let seed_of = |doc: &Document, kind: &str| -> JsValue {
        let seed = get(&doc.main().unwrap(), "seed");
        if seed.is_null() || seed.is_undefined() {
            return JsValue::UNDEFINED;
        }
        get(&seed, kind)
    };
    let mut doc =
        Document::from_markdown("~~~card-yaml\n$quill: seed_quill@1.0\n$kind: main\n~~~\n")
            .expect("fromMarkdown failed");
    assert!(seed_of(&doc, "note").is_undefined());
    doc.store_seed_overlay("note", js_sys::JSON::parse("{\"text\":\"WRITTEN\"}").unwrap())
        .unwrap();
    assert_eq!(
        get(&seed_of(&doc, "note"), "text").as_string().as_deref(),
        Some("WRITTEN")
    );
    doc.remove_seed_overlay("note").unwrap();
    assert!(seed_of(&doc, "note").is_undefined());
}

#[wasm_bindgen_test]
fn test_document_clone_independence() {
    use wasm_bindgen::JsValue;

    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let mut clone = doc.clone_doc();

    clone
        .store_field(JsValue::from_str("title"), JsValue::from_str("Changed"))
        .expect("storeField on clone");

    let original_md = doc.to_markdown();
    let clone_md = clone.to_markdown();

    assert!(
        original_md.contains("title: \"Hello\""),
        "original payload must be untouched after clone mutation\nGot:\n{}",
        original_md
    );
    assert!(
        clone_md.contains("title: \"Changed\""),
        "clone payload must reflect the mutation\nGot:\n{}",
        clone_md
    );

    let orig_warns = js_sys::Array::from(&doc.warnings().unwrap());
    let clone_warns = js_sys::Array::from(&clone.warnings().unwrap());
    assert_eq!(
        orig_warns.length(),
        clone_warns.length(),
        "clone must preserve parse-time warnings"
    );
}
