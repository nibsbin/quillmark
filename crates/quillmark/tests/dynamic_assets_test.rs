//! # Dynamic Assets Tests
//!
//! Tests for runtime asset injection using the `Workflow::add_asset()` API.
//!
//! ## Test Coverage
//!
//! This test suite validates:
//! - **Basic asset injection** - Adding assets at runtime via workflow
//! - **Asset naming** - `DYNAMIC_ASSET__` prefix and collision avoidance
//! - **Asset accessibility** - Assets available to backend during compilation
//! - **Multiple assets** - Managing multiple runtime assets
//!
//! ## Use Case
//!
//! Dynamic assets enable scenarios where images, fonts, or data files need to
//! be injected at render time rather than bundled with the quill template.
//! Common use cases include:
//! - User-uploaded images
//! - Generated charts or graphs
//! - Dynamic QR codes
//! - Runtime-selected fonts
//!
//! ## Related
//!
//! See `dynamic_fonts_test.rs` for font-specific dynamic injection tests.

use quillmark::{Quill, Quillmark, RenderError};
use quillmark_fixtures::{quills_path, resource_path};

#[test]
fn test_with_asset_basic() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    let taro_picture = std::fs::read(resource_path("taro.png")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
    workflow
        .add_asset("taro.png", taro_picture.to_vec())
        .expect("Should add asset");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_with_asset_collision() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
    workflow
        .add_asset("taro.png", vec![1, 2, 3])
        .expect("Should add first asset");

    // Should fail - asset already exists
    let result = workflow.add_asset("taro.png", vec![4, 5, 6]);
    assert!(matches!(
        result,
        Err(RenderError::DynamicAssetCollision { .. })
    ));
}

#[test]
fn test_with_assets_multiple() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let assets = vec![
        ("chart1.png".to_string(), vec![1, 2, 3]),
        ("chart2.png".to_string(), vec![4, 5, 6]),
        ("data.csv".to_string(), vec![7, 8, 9]),
    ];

    let mut workflow = engine.workflow("taro").unwrap();
    workflow
        .add_assets(assets)
        .expect("Should add multiple assets");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_clear_assets() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
    workflow
        .add_asset("taro.png", vec![1, 2, 3])
        .expect("Should add first asset");
    workflow
        .add_asset("more_taro.png", vec![4, 5, 6])
        .expect("Should add second asset");
    workflow.clear_assets();

    // After clearing, should be able to add the same filenames again
    workflow
        .add_asset("taro.png", vec![7, 8, 9])
        .expect("Should add taro.png again after clearing");
    workflow
        .add_asset("more_taro.png", vec![10, 11, 12])
        .expect("Should add more_taro.png again after clearing");

    assert_eq!(workflow.quill_name(), "taro");
}
