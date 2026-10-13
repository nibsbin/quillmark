//! Content nested to the import bound renders. Typst refuses past a show-rule
//! depth of 64, which each quote and list level spends from.

use quillmark_content::{import::from_markdown, serial::to_canonical_value, MAX_NESTING_DEPTH};
use quillmark_core::{backend::Backend, types::RenderOptions};
use quillmark_typst::TypstBackend;

mod common;

#[test]
fn content_nested_to_the_bound_renders() {
    let quill = common::quill_with_plate(
        &common::yaml("main:\n  fields: {}\n"),
        "#import \"@local/quillmark-helper:0.2.0\" as qm\n#qm.data.at(\"$body\", default: [])\n",
    );
    let n = MAX_NESTING_DEPTH;
    let alternating = |a: &str, b: &str| -> String {
        (0..n).map(|i| if i % 2 == 0 { a } else { b }).collect()
    };
    let shapes = [
        format!("{}x", "> ".repeat(n)),
        format!("{}x", "- ".repeat(n)),
        format!("{}x", "1. ".repeat(n)),
        format!("{}x", alternating("> ", "- ")),
        format!("{}x", alternating("1. ", "> ")),
        format!("{}**x**", "> ".repeat(n - 1)),
    ];
    for markdown in shapes {
        let content = from_markdown(&markdown).expect("imports at the bound").content;
        let deepest = content.lines.iter().map(|l| l.containers.len()).max().unwrap_or(0);
        assert_eq!(deepest + content.marks.len(), n, "{markdown:?} reaches the bound");
        let data = serde_json::json!({ "$body": to_canonical_value(&content) });
        TypstBackend
            .open(&quill, &data, common::test_date())
            .and_then(|session| session.render(&RenderOptions::default()))
            .unwrap_or_else(|e| panic!("{markdown:?} renders: {e}"));
    }
}
