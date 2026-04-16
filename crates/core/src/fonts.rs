//! Font manifest and provider types for centralized font storage.
//!
//! Quill bundles are published in a **dehydrated** form: font files are stripped
//! from the ZIP and replaced by a `fonts.json` sidecar that records each
//! removed path together with the MD5 hex of its bytes.  Loading a published
//! Quill **rehydrates** the bundle: the manifest drives fetches from a
//! [`FontProvider`] and the bytes are written back to their original tree paths
//! before the backend ever sees the [`FileTreeNode`].
//!
//! After rehydration the in-memory [`crate::Quill`] is indistinguishable from
//! the pre-strip source.  The Typst backend therefore requires no changes.

use std::collections::{HashMap, HashSet};
use std::error::Error as StdError;

use serde::{Deserialize, Serialize};

use crate::quill::FileTreeNode;

// ── FontManifest ─────────────────────────────────────────────────────────────

/// Dehydration record written to `fonts.json` at the ZIP root by the publisher.
///
/// Maps every stripped font path to the MD5 hex of its bytes.  Multiple paths
/// may share the same hash when byte-identical fonts are embedded more than once
/// (e.g. under both `assets/fonts/` and `packages/*/fonts/`).
///
/// Rust is the canonical schema owner.  The JSON Schema at
/// `crates/core/schemas/fonts-manifest.schema.json` is derived from this type
/// via `schemars` and committed to the repository; CI fails on drift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FontManifest {
    /// Schema version.  Must be `1`.
    pub version: u32,
    /// Map from file path (relative to ZIP root) to lowercase MD5 hex.
    pub files: HashMap<String, String>,
}

// ── FontProvider ─────────────────────────────────────────────────────────────

/// Supplies raw font bytes by MD5 content hash during rehydration.
///
/// Implementations are called **at most once per unique hash** in a single
/// [`rehydrate_tree`] call.  The trait is intentionally sync so it remains
/// usable inside Typst's sync font-loading path and in WASM without async.
pub trait FontProvider {
    /// Return the raw font bytes for `md5` (lowercase hex), or `None` if the
    /// hash is not available.
    ///
    /// Returning `None` for any hash that appears in a manifest causes
    /// [`rehydrate_tree`] to fail.
    fn fetch(&self, md5: &str) -> Option<Vec<u8>>;
}

// ── MapProvider ──────────────────────────────────────────────────────────────

/// A [`FontProvider`] backed by an in-memory `md5 → bytes` map.
///
/// Used by the WASM binding: Node reads `fonts.json`, fetches every unique hash
/// from the store, builds a `Map<string, Uint8Array>`, and hands it to Rust as
/// a `MapProvider`.  From Rust's perspective fonts are already present when
/// [`rehydrate_tree`] runs.
pub struct MapProvider {
    map: HashMap<String, Vec<u8>>,
}

impl MapProvider {
    /// Create a `MapProvider` from a pre-populated `md5-hex → bytes` map.
    pub fn new(map: HashMap<String, Vec<u8>>) -> Self {
        Self { map }
    }
}

impl FontProvider for MapProvider {
    fn fetch(&self, md5: &str) -> Option<Vec<u8>> {
        self.map.get(md5).cloned()
    }
}

// ── rehydrate_tree ────────────────────────────────────────────────────────────

/// Rehydrate `root` in-place using `provider`.
///
/// # Algorithm
///
/// 1. Look for `fonts.json` at the tree root.  If absent, return `Ok(())` — the
///    tree is either a local dev tree (fonts already present) or a bundle that
///    pre-dates centralization.
/// 2. Parse the [`FontManifest`] and reject unknown schema versions.
/// 3. Collect the unique set of MD5 hashes from `manifest.files`.
/// 4. Call `provider.fetch(md5)` once per unique hash.  **Fail immediately** if
///    any hash is not available.
/// 5. Insert font bytes at every path listed in the manifest.
///
/// After this call the tree is indistinguishable from the pre-strip source.
///
/// # Errors
///
/// - `fonts.json` is present but cannot be parsed as a [`FontManifest`].
/// - `manifest.version` is not `1`.
/// - Any hash in the manifest cannot be resolved by `provider`.
/// - A path in the manifest cannot be inserted into the tree.
pub fn rehydrate_tree(
    root: &mut FileTreeNode,
    provider: &dyn FontProvider,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let manifest_bytes = match root.get_file("fonts.json") {
        Some(b) => b.to_vec(),
        None => return Ok(()),
    };

    let manifest: FontManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("Failed to parse fonts.json: {}", e))?;

    if manifest.version != 1 {
        return Err(format!(
            "Unsupported fonts.json version {} (expected 1)",
            manifest.version
        )
        .into());
    }

    // Fetch each unique hash exactly once.
    let unique_hashes: HashSet<&str> = manifest.files.values().map(String::as_str).collect();

    let mut resolved: HashMap<&str, Vec<u8>> = HashMap::with_capacity(unique_hashes.len());
    for hash in &unique_hashes {
        match provider.fetch(hash) {
            Some(bytes) => {
                resolved.insert(hash, bytes);
            }
            None => {
                return Err(
                    format!("Font provider could not resolve hash: {}", hash).into(),
                );
            }
        }
    }

    // Write bytes back to their original paths.
    for (path, hash) in &manifest.files {
        let bytes = resolved
            .get(hash.as_str())
            .expect("invariant: all hashes were resolved above")
            .clone();
        root.insert(path, FileTreeNode::File { contents: bytes })
            .map_err(|e| format!("Failed to insert rehydrated font at '{}': {}", path, e))?;
    }

    Ok(())
}
