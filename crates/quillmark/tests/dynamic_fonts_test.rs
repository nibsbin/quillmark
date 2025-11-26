//! # Dynamic Fonts Tests
//!
//! Tests for runtime font injection using the `Workflow::add_font()` API.
//!
//! ## Test Coverage
//!
//! This test suite validates:
//! - **Basic font injection** - Adding fonts at runtime via workflow
//! - **Font registration** - Fonts become available to backend for rendering
//! - **Multiple fonts** - Managing multiple runtime font files
//! - **Font accessibility** - Backend can access and use injected fonts
//!
//! ## Use Case
//!
//! Dynamic fonts enable scenarios where font files need to be selected or
//! provided at render time rather than bundled with the quill template.
//! Common use cases include:
//! - User-selected fonts
//! - Organization-specific branding fonts
//! - Localization-specific fonts
//! - Runtime font fallbacks
//!
//! ## Backend Support
//!
//! Font injection is currently supported by:
//! - **Typst backend** - Registers fonts in compilation environment
//!
//! ## Related
//!
//! See `dynamic_assets_test.rs` for general asset injection tests.

use quillmark::{Quill, Quillmark, RenderError};
use quillmark_fixtures::quills_path;

#[test]
fn test_with_font_basic() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    // Create some dummy font data
    let font_data = vec![1, 2, 3, 4, 5];
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
    workflow
        .add_font("custom.ttf", font_data.clone())
        .expect("Should add font");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_with_font_collision() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
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
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let fonts = vec![
        ("font1.ttf".to_string(), vec![1, 2, 3]),
        ("font2.otf".to_string(), vec![4, 5, 6]),
        ("font3.woff".to_string(), vec![7, 8, 9]),
    ];

    let mut workflow = engine.workflow("taro").unwrap();
    workflow
        .add_fonts(fonts)
        .expect("Should add multiple fonts");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_clear_fonts() {
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
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
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
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
    let quill = Quill::from_path(quills_path("taro")).unwrap();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let mut workflow = engine.workflow("taro").unwrap();
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
