//! Every fixture quill's seeded document renders. The seed carries one card per
//! declared kind with every declared field filled from its `example:`/`default:`,
//! so this exercises each plate's whole dispatch: the sweep a plate edit needs
//! to be caught by, since a plate is compiled code that no unit test reaches.
//!
//! The quill list is read from the fixtures directory rather than spelled out,
//! so a new fixture is covered by existing.

#![cfg(feature = "typst")]

use quillmark::{Quillmark, RenderOptions};
use quillmark_fixtures::resource_path;

fn fixture_quill_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(resource_path("quills"))
        .expect("fixtures must carry a quills directory")
        .map(|e| e.expect("readable entry"))
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no fixture quills found");
    names
}

#[test]
fn every_fixture_quill_renders_its_seed_document() {
    let engine = Quillmark::new();

    for name in fixture_quill_names() {
        let quill = quillmark::quill_from_path(quillmark_fixtures::quills_path(&name))
            .unwrap_or_else(|e| panic!("{name} should load: {e:?}"));
        let format = engine
            .supported_formats(&quill)
            .unwrap_or_else(|e| panic!("{name}'s backend should resolve: {e:?}"))
            .first()
            .copied()
            .unwrap_or_else(|| panic!("{name}'s backend declares no output format"));

        let rendered = engine
            .render(
                &quill,
                &quill.seed_document(),
                &RenderOptions::default().with_output_format(format),
            )
            .unwrap_or_else(|e| panic!("{name} failed to render its seed to {format:?}: {e:?}"));

        assert!(
            rendered.artifacts.first().is_some_and(|a| !a.bytes.is_empty()),
            "{name} rendered no {format:?} bytes"
        );
    }
}
