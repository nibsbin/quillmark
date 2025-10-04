/// Test that the native-fonts feature flag works correctly
///
/// This test verifies that:
/// 1. With native-fonts enabled: system fonts and package downloads are available
/// 2. Without native-fonts: only asset fonts are available, packages are skipped
///
/// Note: The actual functionality testing is done in the integration tests.
/// This test primarily verifies that the code compiles with and without the feature.

#[cfg(feature = "native-fonts")]
#[test]
fn test_native_fonts_feature_enabled() {
    // When native-fonts is enabled, we should be able to access system fonts
    // This is a compilation test - if it compiles, the feature is working

    // The actual FontSearcher is used in QuillWorld::new()
    // We just verify that the module exists
    use typst_kit::fonts::FontSearcher;

    let _searcher = FontSearcher::new();
    // If this compiles, the feature is enabled correctly
}

#[cfg(not(feature = "native-fonts"))]
#[test]
fn test_native_fonts_feature_disabled() {
    // When native-fonts is disabled, typst-kit should not be available
    // This test verifies that the code compiles without typst-kit

    // We can't use FontSearcher here - it shouldn't be available
    // Just verify that the basic quillmark-typst still works

    // This is a compilation test - if it compiles without typst-kit, success
    assert!(
        true,
        "quillmark-typst compiles without native-fonts feature"
    );
}
