//! Compiles only without the `typst` feature, so it runs solely under CI's
//! zero-backend job — the one configuration where `registered_backends()` can
//! be empty.

#[test]
#[cfg(not(feature = "typst"))]
fn test_typst_backend_not_registered() {
    let engine = quillmark::Quillmark::new();
    let backends = engine.registered_backends();
    assert!(!backends.contains(&"typst"));
    assert_eq!(backends.len(), 0, "no feature, no backend");
}
