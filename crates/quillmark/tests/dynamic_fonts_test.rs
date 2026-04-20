//! # Dynamic Fonts Tests

use quillmark::{Quillmark, RenderError};
use quillmark_fixtures::quills_path;

#[test]
fn test_with_font_basic() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow
        .add_font("custom.ttf", vec![1, 2, 3, 4, 5])
        .expect("Should add font");
    assert!(workflow.quill_ref().starts_with("taro@"));
}

#[test]
fn test_with_font_collision() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow
        .add_font("custom.ttf", vec![1, 2, 3])
        .expect("Should add first font");
    let result = workflow.add_font("custom.ttf", vec![4, 5, 6]);
    assert!(matches!(
        result,
        Err(RenderError::DynamicFontCollision { .. })
    ));
}

#[test]
fn test_with_fonts_multiple() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let fonts = vec![
        ("font1.ttf".to_string(), vec![1, 2, 3]),
        ("font2.otf".to_string(), vec![4, 5, 6]),
        ("font3.woff".to_string(), vec![7, 8, 9]),
    ];
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow
        .add_fonts(fonts)
        .expect("Should add multiple fonts");
    assert!(workflow.quill_ref().starts_with("taro@"));
}

#[test]
fn test_clear_fonts() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow.add_font("font1.ttf", vec![1, 2, 3]).unwrap();
    workflow.add_font("font2.ttf", vec![4, 5, 6]).unwrap();
    workflow.clear_fonts();
    workflow
        .add_font("font1.ttf", vec![7, 8, 9])
        .expect("Should add again after clearing");
    workflow
        .add_font("font2.ttf", vec![10, 11, 12])
        .expect("Should add again after clearing");
    assert!(workflow.quill_ref().starts_with("taro@"));
}

#[test]
fn test_with_font_and_asset_together() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow
        .add_asset("chart.png", vec![1, 2, 3])
        .expect("Should add asset");
    workflow
        .add_font("custom.ttf", vec![4, 5, 6])
        .expect("Should add font");
    assert!(workflow.quill_ref().starts_with("taro@"));
}

#[test]
fn test_dynamic_font_names() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow.add_font("font1.ttf", vec![1, 2, 3]).unwrap();
    workflow.add_font("font2.otf", vec![4, 5, 6]).unwrap();
    let mut font_names = workflow.dynamic_font_names();
    font_names.sort();
    assert_eq!(font_names, vec!["font1.ttf", "font2.otf"]);
}
