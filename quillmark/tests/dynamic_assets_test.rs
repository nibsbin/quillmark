use quillmark::{Quill, QuillmarkEngine, RenderError};
use quillmark_fixtures::resource_path;

#[test]
fn test_with_asset_basic() {
    let mut engine = QuillmarkEngine::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    let taro_picture = std::fs::read(resource_path("taro.png")).unwrap();
    engine.register_quill(quill);

    let workflow = engine.load("taro").unwrap();
    let workflow = workflow
        .with_asset("taro.png", taro_picture.to_vec())
        .expect("Should add asset");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_with_asset_collision() {
    let mut engine = QuillmarkEngine::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let workflow = engine.load("taro").unwrap();
    let workflow = workflow
        .with_asset("taro.png", vec![1, 2, 3])
        .expect("Should add first asset");

    // Should fail - asset already exists
    let result = workflow.with_asset("taro.png", vec![4, 5, 6]);
    assert!(matches!(
        result,
        Err(RenderError::DynamicAssetCollision { .. })
    ));
}

#[test]
fn test_with_assets_multiple() {
    let mut engine = QuillmarkEngine::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let assets = vec![
        ("chart1.png".to_string(), vec![1, 2, 3]),
        ("chart2.png".to_string(), vec![4, 5, 6]),
        ("data.csv".to_string(), vec![7, 8, 9]),
    ];

    let workflow = engine.load("taro").unwrap();
    let workflow = workflow
        .with_assets(assets)
        .expect("Should add multiple assets");

    assert_eq!(workflow.quill_name(), "taro");
}

#[test]
fn test_clear_assets() {
    let mut engine = QuillmarkEngine::new();
    let quill = Quill::from_path(resource_path("taro")).unwrap();
    engine.register_quill(quill);

    let workflow = engine.load("taro").unwrap();
    let workflow = workflow
        .with_asset("taro.png", vec![1, 2, 3])
        .expect("Should add first asset")
        .with_asset("more_taro.png", vec![4, 5, 6])
        .expect("Should add second asset")
        .clear_assets();

    // After clearing, should be able to add the same filenames again
    let workflow = workflow
        .with_asset("taro.png", vec![7, 8, 9])
        .expect("Should add taro.png again after clearing")
        .with_asset("more_taro.png", vec![10, 11, 12])
        .expect("Should add more_taro.png again after clearing");

    assert_eq!(workflow.quill_name(), "taro");
}
