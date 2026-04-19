//! # Dynamic Assets Tests

use quillmark::{Quillmark, RenderError};
use quillmark_fixtures::{quills_path, resource_path};

#[test]
fn test_with_asset_basic() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let taro_picture = std::fs::read(resource_path("taro.png")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow
        .add_asset("taro.png", taro_picture)
        .expect("Should add asset");
    assert!(workflow.quill_ref().starts_with("taro@"));
}

#[test]
fn test_with_asset_collision() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow
        .add_asset("taro.png", vec![1, 2, 3])
        .expect("Should add first asset");
    let result = workflow.add_asset("taro.png", vec![4, 5, 6]);
    assert!(matches!(
        result,
        Err(RenderError::DynamicAssetCollision { .. })
    ));
}

#[test]
fn test_with_assets_multiple() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let assets = vec![
        ("chart1.png".to_string(), vec![1, 2, 3]),
        ("chart2.png".to_string(), vec![4, 5, 6]),
        ("data.csv".to_string(), vec![7, 8, 9]),
    ];
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow
        .add_assets(assets)
        .expect("Should add multiple assets");
    assert!(workflow.quill_ref().starts_with("taro@"));
}

#[test]
fn test_clear_assets() {
    let engine = Quillmark::new();
    let quill = engine.quill_from_path(quills_path("taro")).unwrap();
    let mut workflow = engine.workflow(&quill).unwrap();
    workflow.add_asset("taro.png", vec![1, 2, 3]).unwrap();
    workflow.add_asset("more_taro.png", vec![4, 5, 6]).unwrap();
    workflow.clear_assets();
    workflow
        .add_asset("taro.png", vec![7, 8, 9])
        .expect("Should add again after clearing");
    workflow
        .add_asset("more_taro.png", vec![10, 11, 12])
        .expect("Should add again after clearing");
    assert!(workflow.quill_ref().starts_with("taro@"));
}
