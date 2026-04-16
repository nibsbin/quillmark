//! Tests for FontManifest parsing, rehydration, and schema drift detection.

use std::collections::HashMap;

use quillmark_core::fonts::{rehydrate_tree, FontManifest, FontProvider, MapProvider};
use quillmark_core::FileTreeNode;

// ── Fixture helpers ───────────────────────────────────────────────────────────

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/fonts-manifest")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()))
}

// ── FontManifest parsing ──────────────────────────────────────────────────────

#[test]
fn parse_valid_simple_fixture() {
    let json = fixture("valid-simple.json");
    let manifest: FontManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.files.len(), 2);
    assert_eq!(
        manifest.files["assets/fonts/Inter-Regular.ttf"],
        "3f2a8c1d9e4b5a7f0c8d6e3a1b4f9c2d"
    );
}

#[test]
fn parse_valid_dedup_fixture() {
    let json = fixture("valid-dedup.json");
    let manifest: FontManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.files.len(), 3);
    // Two paths share the same hash — the dedup case.
    let hash = &manifest.files["assets/fonts/Inter-Regular.ttf"];
    assert_eq!(
        &manifest.files["packages/ttq-classic-resume/fonts/Inter-Regular.ttf"],
        hash
    );
}

#[test]
fn roundtrip_serialization() {
    let mut files = HashMap::new();
    files.insert(
        "assets/fonts/Foo.ttf".to_string(),
        "aabbccdd".to_string(),
    );
    let original = FontManifest { version: 1, files };
    let json = serde_json::to_string(&original).unwrap();
    let decoded: FontManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(original, decoded);
}

// ── rehydrate_tree ────────────────────────────────────────────────────────────

/// Builds a minimal dehydrated tree: Quill.yaml + fonts.json, no font files.
fn dehydrated_tree(manifest_json: &str) -> FileTreeNode {
    let mut root = FileTreeNode::Directory {
        files: HashMap::new(),
    };
    root.insert(
        "Quill.yaml",
        FileTreeNode::File {
            contents: b"name: test\nbackend: typst\nversion: '0.1.0'\nauthor: Test\ncards: []\n"
                .to_vec(),
        },
    )
    .unwrap();
    root.insert(
        "fonts.json",
        FileTreeNode::File {
            contents: manifest_json.as_bytes().to_vec(),
        },
    )
    .unwrap();
    root
}

/// A [`FontProvider`] that serves synthetic font bytes keyed by md5.
struct FakeProvider(HashMap<String, Vec<u8>>);

impl FontProvider for FakeProvider {
    fn fetch(&self, md5: &str) -> Option<Vec<u8>> {
        self.0.get(md5).cloned()
    }
}

#[test]
fn rehydrate_inserts_font_at_correct_path() {
    let manifest_json = r#"{"version":1,"files":{"assets/fonts/Inter-Regular.ttf":"aabbccdd"}}"#;
    let mut tree = dehydrated_tree(manifest_json);

    let provider = FakeProvider({
        let mut m = HashMap::new();
        m.insert("aabbccdd".to_string(), b"FAKE_FONT_BYTES".to_vec());
        m
    });

    rehydrate_tree(&mut tree, &provider).unwrap();

    assert_eq!(
        tree.get_file("assets/fonts/Inter-Regular.ttf").unwrap(),
        b"FAKE_FONT_BYTES"
    );
}

#[test]
fn rehydrate_dedup_calls_provider_once_per_unique_hash() {
    let manifest_json = r#"{
      "version": 1,
      "files": {
        "assets/fonts/Inter-Regular.ttf": "aabbccdd",
        "packages/pkg/fonts/Inter-Regular.ttf": "aabbccdd"
      }
    }"#;
    let mut tree = dehydrated_tree(manifest_json);

    // Track how many times fetch is called.
    struct CountingProvider {
        bytes: Vec<u8>,
        calls: std::cell::Cell<usize>,
    }
    impl FontProvider for CountingProvider {
        fn fetch(&self, _md5: &str) -> Option<Vec<u8>> {
            self.calls.set(self.calls.get() + 1);
            Some(self.bytes.clone())
        }
    }

    let provider = CountingProvider {
        bytes: b"FONT".to_vec(),
        calls: std::cell::Cell::new(0),
    };
    rehydrate_tree(&mut tree, &provider).unwrap();

    // Both paths should now have font bytes.
    assert!(tree
        .get_file("assets/fonts/Inter-Regular.ttf")
        .is_some());
    assert!(tree
        .get_file("packages/pkg/fonts/Inter-Regular.ttf")
        .is_some());
    // Provider was called exactly once for the single unique hash.
    assert_eq!(provider.calls.get(), 1);
}

#[test]
fn rehydrate_no_op_when_fonts_json_absent() {
    let mut tree = FileTreeNode::Directory {
        files: HashMap::new(),
    };
    // No fonts.json — should succeed silently.
    let provider = MapProvider::new(HashMap::new());
    rehydrate_tree(&mut tree, &provider).unwrap();
}

#[test]
fn rehydrate_fails_on_missing_hash() {
    let manifest_json =
        r#"{"version":1,"files":{"assets/fonts/Missing.ttf":"deadbeef"}}"#;
    let mut tree = dehydrated_tree(manifest_json);

    // Provider has nothing.
    let provider = MapProvider::new(HashMap::new());
    let err = rehydrate_tree(&mut tree, &provider).unwrap_err();
    assert!(err.to_string().contains("deadbeef"), "{}", err);
}

#[test]
fn rehydrate_fails_on_unsupported_version() {
    let manifest_json = r#"{"version":99,"files":{}}"#;
    let mut tree = dehydrated_tree(manifest_json);

    let provider = MapProvider::new(HashMap::new());
    let err = rehydrate_tree(&mut tree, &provider).unwrap_err();
    assert!(err.to_string().contains("99"), "{}", err);
}

// ── Schema drift guard ────────────────────────────────────────────────────────

#[test]
fn fonts_manifest_schema_matches_committed_file() {
    let schema = schemars::schema_for!(FontManifest);
    let generated = serde_json::to_string_pretty(&schema).unwrap();

    let committed_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/fonts-manifest.schema.json");
    let committed = std::fs::read_to_string(&committed_path).unwrap_or_else(|_| {
        panic!(
            "schemas/fonts-manifest.schema.json not found at {}",
            committed_path.display()
        )
    });

    assert_eq!(
        committed.trim(),
        generated.trim(),
        "schemas/fonts-manifest.schema.json is out of date — \
         update the file to match the generated schema above"
    );
}
