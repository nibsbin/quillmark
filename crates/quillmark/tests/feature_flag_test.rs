//! # Feature Flag Tests
//!
//! Tests for conditional backend registration based on cargo feature flags.
//!
//! ## Test Coverage
//!
//! This test suite validates:
//! - **Auto-registration** - Backends registered only when features enabled
//! - **Feature isolation** - No backend registered when feature disabled
//! - **Zero-config setup** - Engine creation works regardless of enabled features
//!
//! ## Feature System
//!
//! Quillmark uses cargo features for optional backend inclusion:
//! - `typst` (default) - Typst backend for PDF/SVG rendering
//!
//! When `Quillmark::new()` is called, only backends with enabled features
//! are registered automatically.
//!
//! ## Airmark Deployment Constraints
//!
//! The airmark deployment MUST NOT expose MCP or ephemeral endpoints.
//! Quillmark has no MCP server implementation — no `mcp` or `ephemeral`
//! backend exists in the feature set. The tests below assert this invariant.
//!
//! ## Test Strategy
//!
//! Tests use conditional compilation to verify correct behavior:
//! - `#[cfg(feature = "typst")]` - Test when feature is enabled
//! - `#[cfg(not(feature = "typst"))]` - Test when feature is disabled
//!
//! ## Design Reference
//!
//! See `prose/designs/ARCHITECTURE.md` section on Backend Auto-Registration.

use quillmark::Quillmark;

#[test]
#[cfg(feature = "typst")]
fn test_typst_backend_auto_registered() {
    let engine = Quillmark::new();
    let backends = engine.registered_backends();

    assert!(
        backends.contains(&"typst"),
        "Typst backend should be auto-registered when feature is enabled"
    );
}

#[test]
#[cfg(not(feature = "typst"))]
fn test_typst_backend_not_registered() {
    let engine = Quillmark::new();
    let backends = engine.registered_backends();

    assert!(
        !backends.contains(&"typst"),
        "Typst backend should not be registered when feature is disabled"
    );
    assert_eq!(
        backends.len(),
        0,
        "No backends should be registered when all features are disabled"
    );
}

#[test]
fn test_no_mcp_or_ephemeral_backends() {
    // Airmark deployment must never expose MCP or ephemeral endpoints.
    // Neither backend type exists in quillmark's feature set; this test
    // guards against accidental future additions.
    let engine = Quillmark::new();
    let backends = engine.registered_backends();

    assert!(
        !backends.contains(&"mcp"),
        "MCP backend must not be registered in any deployment"
    );
    assert!(
        !backends.contains(&"ephemeral"),
        "Ephemeral backend must not be registered in any deployment"
    );
}
