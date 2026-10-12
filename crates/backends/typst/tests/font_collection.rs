//! A font collection a quill ships loads every face it holds.

use quillmark_core::backend::Backend;
use quillmark_core::quill::{FileTreeNode, Quill};
use quillmark_typst::TypstBackend;

mod common;

const YAML: &str = "quill:\n  name: t\n  version: \"1.0\"\n  backend: typst\n  \
                    description: d\n\ntypst:\n  plate_file: plate.typ\n";

/// A one-face collection holding `ttf`: a `ttcf` header, then the font with
/// each table offset moved past the header.
fn collection_of(ttf: &[u8]) -> Vec<u8> {
    const HEADER: u32 = 16;
    let mut font = ttf.to_vec();
    let tables = usize::from(u16::from_be_bytes([font[4], font[5]]));
    for i in 0..tables {
        let at = 12 + 16 * i + 8;
        let offset = u32::from_be_bytes(font[at..at + 4].try_into().unwrap());
        font[at..at + 4].copy_from_slice(&(offset + HEADER).to_be_bytes());
    }
    let mut out = b"ttcf".to_vec();
    out.extend(0x0001_0000u32.to_be_bytes());
    out.extend(1u32.to_be_bytes());
    out.extend(HEADER.to_be_bytes());
    out.extend(font);
    out
}

#[test]
fn a_ttc_under_assets_fonts_sets_its_family() {
    let figtree = std::fs::read(
        quillmark_fixtures::quills_path("taro").join("assets/fonts/Figtree-Regular.ttf"),
    )
    .expect("read Figtree");
    let ttc = collection_of(&figtree);
    let mut root = FileTreeNode::Directory {
        files: Default::default(),
    };
    for (path, contents) in [
        ("Quill.yaml", YAML.as_bytes()),
        ("plate.typ", b"#set text(font: \"Figtree\")\nhello\n"),
        ("assets/fonts/Figtree.ttc", &ttc),
    ] {
        root.insert(path, FileTreeNode::File { contents: contents.to_vec() })
            .expect("insert");
    }
    let q = Quill::from_tree(root).expect("load quill");
    let session = TypstBackend
        .open(&q, &serde_json::json!({}), common::test_date())
        .expect("the plate compiles");
    assert!(
        session.warnings().is_empty(),
        "the collection's family resolves: {:?}",
        session.warnings()
    );
}
