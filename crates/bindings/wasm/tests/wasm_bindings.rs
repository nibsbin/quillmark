use wasm_bindgen_test::*;

use quillmark_wasm::{Document, Quillmark, RenderOptions};

mod common;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn small_quill_tree() -> wasm_bindgen::JsValue {
    common::tree(&[
        (
            "Quill.yaml",
            b"quill:\n  name: test_quill\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill for WASM bindings\n",
        ),
        ("plate.typ", b"= Title\n\nThis is a test."),
    ])
}

const SIMPLE_MARKDOWN: &str = "---\nQUILL: test_quill\ntitle: Hello\n---\n\n# Hello\n";

#[wasm_bindgen_test]
fn test_parse_markdown_static() {
    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    assert_eq!(doc.quill_ref(), "test_quill");
}

#[wasm_bindgen_test]
fn test_document_body_and_warnings() {
    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    // Body at EOF: no F2 separator to strip, so trailing content newlines are
    // preserved verbatim. `toMarkdown` carries the body through unchanged.
    assert!(doc.to_markdown().contains("# Hello\n"));
    // warnings() returns JsValue (array) — just verify it's defined
    let warnings = doc.warnings();
    assert!(!warnings.is_undefined());
}

#[wasm_bindgen_test]
fn test_quill_from_tree() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");
    let _ = quill;
}

/// Rendering with a QUILL ref that differs from the quill name must yield
/// exactly one warning with code `quill::ref_mismatch` and still produce an artifact.
#[wasm_bindgen_test]
fn test_render_ref_mismatch_warning() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let mismatch_md = "---\nQUILL: other_quill\ntitle: Mismatch\n---\n\n# Content\n";
    let doc = Document::from_markdown(mismatch_md).expect("fromMarkdown failed");
    let result = quill
        .render(&doc, Some(RenderOptions::default()))
        .expect("render should succeed despite mismatch");

    assert_eq!(result.warnings.len(), 1, "expected exactly one warning");
    assert_eq!(
        result.warnings[0].code.as_deref(),
        Some("quill::ref_mismatch"),
        "warning code should be quill::ref_mismatch"
    );
    assert!(!result.artifacts.is_empty(), "artifact must be produced");
}

/// `quill.render(Document, opts)` — render via pre-parsed document.
#[wasm_bindgen_test]
fn test_render_from_document() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let result = quill
        .render(&doc, Some(RenderOptions::default()))
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

/// `quill.open(Document)` returns a render session supporting page_count + render.
#[wasm_bindgen_test]
fn test_open_session_render() {
    let engine = Quillmark::new();
    let quill = engine.quill(small_quill_tree()).expect("quill failed");

    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let session = quill.open(&doc).expect("open failed");
    assert!(session.page_count() > 0, "session should expose page count");

    let result = session
        .render(Some(RenderOptions::default()))
        .expect("session render failed");
    assert!(!result.artifacts.is_empty(), "should produce artifacts");
}

/// `toMarkdown` emits canonical Quillmark Markdown and round-trips cleanly.
#[wasm_bindgen_test]
fn test_to_markdown_round_trip() {
    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let emitted = doc.to_markdown();
    assert!(
        !emitted.is_empty(),
        "toMarkdown must return non-empty output"
    );

    // Re-parse: the emitted document must parse back cleanly
    let doc2 = Document::from_markdown(&emitted).expect("re-parse of emitted markdown failed");
    assert_eq!(
        doc2.quill_ref(),
        doc.quill_ref(),
        "quill_ref must survive round-trip"
    );
}

/// Plain object (`Record<string, Uint8Array>`) must be accepted by
/// `engine.quill` equivalently to `Map<string, Uint8Array>`.
#[wasm_bindgen_test]
fn test_quill_from_object_tree() {
    let entries: &[(&str, &[u8])] = &[
        (
            "Quill.yaml",
            b"quill:\n  name: test_quill\n  backend: typst\n  plate_file: plate.typ\n  description: Test quill for WASM bindings\n",
        ),
        ("plate.typ", b"= Title\n\nThis is a test."),
    ];

    let engine = Quillmark::new();
    let from_map = engine
        .quill(common::tree(entries))
        .expect("Map form failed");
    let from_obj = engine
        .quill(common::tree_object(entries))
        .expect("Object form failed");

    assert_eq!(from_map.backend_id(), from_obj.backend_id());

    // Both handles render the same document to the same artifact count/format.
    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let doc2 = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let r_map = from_map
        .render(&doc, Some(RenderOptions::default()))
        .expect("render from Map form");
    let r_obj = from_obj
        .render(&doc2, Some(RenderOptions::default()))
        .expect("render from object form");
    assert_eq!(r_map.artifacts.len(), r_obj.artifacts.len());
}

/// `quill.metadata` exposes the snapshot of `Quill.yaml` expected by
/// downstream consumers: `name`, `backend`, `description`, `version`,
/// `supportedFormats`, and the raw `schema` mirroring Quill.yaml's
/// `main:` and `card_types:` sections.
#[wasm_bindgen_test]
fn test_quill_metadata_snapshot() {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let engine = Quillmark::new();
    let quill = engine
        .quill(common::tree(&[
            (
                "Quill.yaml",
                b"quill:\n  name: meta_quill\n  backend: typst\n  version: \"0.2.1\"\n  plate_file: plate.typ\n  description: Metadata quill\n\nmain:\n  fields:\n    title:\n      type: string\n      description: The title\n\ncard_types:\n  indorsement:\n    title: Indorsement\n    fields:\n      signature_block:\n        type: string\n",
            ),
            ("plate.typ", b"= Title"),
        ]))
        .expect("quill failed");

    let meta: JsValue = quill.metadata();
    assert!(meta.is_object(), "metadata must be a plain JS object");

    let get = |key: &str| -> JsValue { Reflect::get(&meta, &JsValue::from_str(key)).unwrap() };

    assert_eq!(get("name").as_string().as_deref(), Some("meta_quill"));
    assert_eq!(get("backend").as_string().as_deref(), Some("typst"));
    assert_eq!(
        get("description").as_string().as_deref(),
        Some("Metadata quill")
    );
    assert_eq!(get("version").as_string().as_deref(), Some("0.2.1"));
    // `author` defaults to "Unknown" when the YAML omits it.
    assert_eq!(get("author").as_string().as_deref(), Some("Unknown"));

    let formats = get("supportedFormats");
    assert!(
        js_sys::Array::is_array(&formats),
        "supportedFormats must be an array"
    );
    let formats_arr = js_sys::Array::from(&formats);
    assert!(
        formats_arr.length() > 0,
        "supportedFormats must be non-empty"
    );

    // schema mirrors Quill.yaml: { main: CardSchema, cardTypes: { [name]: CardSchema } }
    let schema = get("schema");
    assert!(schema.is_object(), "schema must be an object");

    let main = Reflect::get(&schema, &JsValue::from_str("main")).unwrap();
    assert!(main.is_object(), "schema.main must be present");
    let main_fields = Reflect::get(&main, &JsValue::from_str("fields")).unwrap();
    assert!(
        main_fields.is_object(),
        "schema.main.fields must be an object"
    );
    let title_field = Reflect::get(&main_fields, &JsValue::from_str("title")).unwrap();
    assert!(
        title_field.is_object(),
        "schema.main.fields.title must be present from Quill.yaml"
    );

    // Named card-types live under schema.cardTypes (does NOT include main).
    let card_types = Reflect::get(&schema, &JsValue::from_str("cardTypes")).unwrap();
    assert!(card_types.is_object(), "schema.cardTypes must be an object");
    assert!(
        Reflect::get(&card_types, &JsValue::from_str("main"))
            .unwrap()
            .is_undefined(),
        "schema.cardTypes must NOT contain the main card"
    );
    let indorsement = Reflect::get(&card_types, &JsValue::from_str("indorsement")).unwrap();
    assert!(
        indorsement.is_object(),
        "schema.cardTypes.indorsement must be present"
    );
    let indorsement_fields = Reflect::get(&indorsement, &JsValue::from_str("fields")).unwrap();
    assert!(
        indorsement_fields.is_object(),
        "named card-type entries are CardSchema-shaped (have `fields`)"
    );
    let signature_block =
        Reflect::get(&indorsement_fields, &JsValue::from_str("signature_block")).unwrap();
    assert!(
        signature_block.is_object(),
        "card-type field must round-trip from Quill.yaml"
    );
}

/// `doc.clone()` returns an independent handle: mutations on the clone
/// must not affect the original, and parse-time warnings must survive.
#[wasm_bindgen_test]
fn test_document_clone_independence() {
    use wasm_bindgen::JsValue;

    let doc = Document::from_markdown(SIMPLE_MARKDOWN).expect("fromMarkdown failed");
    let mut clone = doc.clone_doc();

    // Mutate the clone; the original must keep its original title.
    clone
        .set_field("title", JsValue::from_str("Changed"))
        .expect("setField on clone");

    // Emit both and check the title survived on each side independently.
    let original_md = doc.to_markdown();
    let clone_md = clone.to_markdown();

    assert!(
        original_md.contains("title: \"Hello\""),
        "original frontmatter must be untouched after clone mutation\nGot:\n{}",
        original_md
    );
    assert!(
        clone_md.contains("title: \"Changed\""),
        "clone frontmatter must reflect the mutation\nGot:\n{}",
        clone_md
    );

    // Warnings are a JS array on both handles. Length-equality is the
    // observable guarantee for parse-warning preservation.
    let orig_warns = js_sys::Array::from(&doc.warnings());
    let clone_warns = js_sys::Array::from(&clone.warnings());
    assert_eq!(
        orig_warns.length(),
        clone_warns.length(),
        "clone must preserve parse-time warnings"
    );
}
