use quillmark::{Quill, Quillmark, RenderError};
use quillmark_fixtures::resource_path;

#[test]
fn test_with_font_basic() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    // Create some dummy font data
    let font_data = vec![1, 2, 3, 4, 5];
    engine.register_quill(quill);

    let mut workflow = engine.workflow_from_quill_name("taro").unwrap();
    workflow
        .add_font("custom.ttf", font_data.clone())
        .expect("Should add font");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_with_font_collision() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let mut workflow = engine.workflow_from_quill_name("taro").unwrap();
    workflow
        .add_font("custom.ttf", vec![1, 2, 3])
        .expect("Should add first font");

    // Should fail - font already exists
    let result = workflow.add_font("custom.ttf", vec![4, 5, 6]);
    assert!(matches!(
        result,
        Err(RenderError::DynamicFontCollision { .. })
    ));
}

#[test]
fn test_with_fonts_multiple() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let fonts = vec![
        ("font1.ttf".to_string(), vec![1, 2, 3]),
        ("font2.otf".to_string(), vec![4, 5, 6]),
        ("font3.woff".to_string(), vec![7, 8, 9]),
    ];

    let mut workflow = engine.workflow_from_quill_name("taro").unwrap();
    workflow
        .add_fonts(fonts)
        .expect("Should add multiple fonts");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_clear_fonts() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let mut workflow = engine.workflow_from_quill_name("taro").unwrap();
    workflow
        .add_font("font1.ttf", vec![1, 2, 3])
        .expect("Should add first font");
    workflow
        .add_font("font2.ttf", vec![4, 5, 6])
        .expect("Should add second font");
    workflow.clear_fonts();

    // After clearing, should be able to add the same filenames again
    workflow
        .add_font("font1.ttf", vec![7, 8, 9])
        .expect("Should add font1.ttf again after clearing");
    workflow
        .add_font("font2.ttf", vec![10, 11, 12])
        .expect("Should add font2.ttf again after clearing");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_with_font_and_asset_together() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let mut workflow = engine.workflow_from_quill_name("taro").unwrap();
    workflow
        .add_asset("chart.png", vec![1, 2, 3])
        .expect("Should add asset");
    workflow
        .add_font("custom.ttf", vec![4, 5, 6])
        .expect("Should add font");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_dynamic_font_names() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let mut workflow = engine.workflow_from_quill_name("taro").unwrap();
    workflow
        .add_font("font1.ttf", vec![1, 2, 3])
        .expect("Should add first font");
    workflow
        .add_font("font2.otf", vec![4, 5, 6])
        .expect("Should add second font");

    let mut font_names = workflow.dynamic_font_names();
    font_names.sort();

    assert_eq!(font_names, vec!["font1.ttf", "font2.otf"]);
}

#[test]
fn test_with_real_font_file() {
    use std::fs;

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    // Load a real font file from usaf_memo fixture
    let font_path = resource_path("usaf_memo/assets/DejaVuSansMono.ttf");
    let font_data = fs::read(&font_path).expect("Should read font file");

    let mut workflow = engine.workflow_from_quill_name("taro").unwrap();
    workflow
        .add_font("DejaVuSansMono.ttf", font_data)
        .expect("Should add real font");

    assert_eq!(workflow.quill_name(), "taro");
    assert_eq!(workflow.dynamic_font_names(), vec!["DejaVuSansMono.ttf"]);
}
