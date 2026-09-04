//! An image island lowers to `#image(..)` inside the helper package's
//! `lib.typ`, and Typst resolves an image path against the root of the file
//! holding the call. A quill asset is therefore reachable from a content field
//! under the path it has in the quill, rooted or relative, the same path the
//! plate reaches it by.

use quillmark_core::{Backend, FileTreeNode, OutputFormat, Quill, RenderOptions};
use quillmark_typst::TypstBackend;
use std::collections::HashMap;

mod common;
use common::content;

const YAML: &str = r#"
quill:
  name: content_image_asset
  version: 0.1.0
  backend: typst
  description: content image resolving a quill asset
typst:
  plate_file: plate.typ
main:
  fields:
    body:
      type: richtext
      description: a paragraph carrying an image island
"#;

const PLATE: &str = r#"
#import "@local/quillmark-helper:0.1.0": data
#set page(width: 300pt, height: 300pt, margin: 20pt)

#image("assets/logo.svg", width: 16pt)

#data.body
"#;

const LOGO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"
viewBox="0 0 16 16"><rect width="16" height="16" fill="blue"/></svg>"#;

fn quill_with_logo() -> Quill {
    let mut root = FileTreeNode::Directory {
        files: HashMap::new(),
    };
    for (path, contents) in [
        ("Quill.yaml", YAML.as_bytes()),
        ("plate.typ", PLATE.as_bytes()),
        ("assets/logo.svg", LOGO),
    ] {
        root.insert(
            path,
            FileTreeNode::File {
                contents: contents.to_vec(),
            },
        )
        .expect("insert quill file");
    }
    Quill::from_tree(root).expect("load quill")
}

#[test]
fn a_content_image_resolves_a_quill_asset() {
    let data = serde_json::json!({
        "body": content("![logo](assets/logo.svg)\n\n![rooted](/assets/logo.svg)"),
    });

    let session = TypstBackend
        .open(&quill_with_logo(), &data)
        .expect("an image island naming a quill asset compiles");
    assert_eq!(session.page_count(), 1);

    let result = session
        .render(&RenderOptions::default().with_output_format(OutputFormat::Pdf))
        .expect("render ok");
    assert!(!result.artifacts[0].bytes.is_empty(), "produced a PDF");
}
