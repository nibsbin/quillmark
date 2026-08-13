//! The quill authoring contract: every quill in the fixtures quiver loads and
//! renders. An empty document is the type-minimal valid input under zero-filled
//! render, so a plate that renders it degrades gracefully on any valid input.

#![cfg(feature = "typst")]

use quillmark::{Document, OutputFormat, Quillmark, RenderOptions};
use quillmark_fixtures::{quills_path, resource_path};
use std::fs;

fn quiver_quills() -> Vec<String> {
    let quills_dir = resource_path("quills");
    let mut names: Vec<String> = fs::read_dir(&quills_dir)
        .expect("quills directory should exist")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

#[test]
fn every_quill_in_quiver_renders() {
    let engine = Quillmark::new();

    for name in quiver_quills() {
        let quill = quillmark::quill_from_path(quills_path(&name))
            .unwrap_or_else(|e| panic!("quill '{name}' failed to load: {e:?}"));

        let config = quill.config();
        let markdown = format!(
            "~~~\n$quill: {}@{}\n$kind: main\n~~~\n",
            config.name, config.version
        );
        let parsed = Document::parse(&markdown)
            .unwrap_or_else(|e| {
                panic!("quill '{name}' empty document failed to parse: {e:?}\n---\n{markdown}")
            })
            .document;

        let result = engine.render(
            &quill,
            &parsed,
            &RenderOptions::default().with_output_format(OutputFormat::Pdf),
        );

        let rendered = result
            .unwrap_or_else(|e| panic!("quill '{name}' failed to render: {e:?}\n---\n{markdown}"));
        assert!(
            !rendered.artifacts.is_empty(),
            "quill '{name}': render produced no artifacts"
        );
    }
}

/// Every bundled quill's generated blueprint parses, round-trips idempotently,
/// and renders with its `!must_fill` markers zero-filled.
#[test]
fn every_quill_blueprint_round_trips_and_renders() {
    let engine = Quillmark::new();

    for name in quiver_quills() {
        let quill = quillmark::quill_from_path(quills_path(&name))
            .unwrap_or_else(|e| panic!("quill '{name}' failed to load: {e:?}"));

        let bp = quill.config().blueprint();
        let doc1 = Document::parse(&bp)
            .unwrap_or_else(|e| {
                panic!("quill '{name}' blueprint failed to parse: {e:?}\n---\n{bp}")
            })
            .document;
        let doc2 = Document::parse(&doc1.to_markdown())
            .unwrap_or_else(|e| panic!("quill '{name}' blueprint re-emit failed to parse: {e:?}"))
            .document;
        assert_eq!(doc1, doc2, "quill '{name}': blueprint must round-trip");

        let result = engine.render(
            &quill,
            &doc1,
            &RenderOptions::default().with_output_format(OutputFormat::Pdf),
        );
        result.unwrap_or_else(|e| {
            panic!("quill '{name}' blueprint failed to render: {e:?}\n---\n{bp}")
        });
    }
}
