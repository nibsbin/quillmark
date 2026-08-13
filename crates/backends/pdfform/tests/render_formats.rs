//! SVG and PNG are views of the *flattened* form — values baked into the page
//! content — so they render without viewer appearance synthesis.

use quillmark::{Document, OutputFormat, Quillmark, RenderOptions};

const FILLED: &str = "~~~\n\
$quill: sample_form\n\
$kind: main\n\
full_name: Ada Lovelace\n\
comments:\n\
  - First comment line.\n\
agree: true\n\
favorite_color: green\n\
~~~\n";

fn render(format: OutputFormat, ppi: Option<f32>) -> Vec<quillmark_core::Artifact> {
    let quill = quillmark::quill_from_path(quillmark_fixtures::quills_path("sample_form"))
        .expect("load sample_form quill");
    let engine = Quillmark::new();
    let doc = Document::parse(FILLED).expect("parse markdown").document;
    let mut opts = RenderOptions::default().with_output_format(format);
    opts.ppi = ppi;
    engine
        .render(&quill, &doc, &opts)
        .unwrap_or_else(|e| panic!("render {format:?}: {e:?}"))
        .artifacts
}

#[test]
fn renders_svg_per_page() {
    let artifacts = render(OutputFormat::Svg, None);
    assert!(!artifacts.is_empty(), "at least one SVG page");
    for art in &artifacts {
        assert_eq!(art.output_format, OutputFormat::Svg);
        let text = std::str::from_utf8(&art.bytes).expect("SVG is UTF-8");
        assert!(text.contains("<svg"), "artifact must be an SVG document");
    }
}

#[test]
fn renders_png_per_page() {
    let artifacts = render(OutputFormat::Png, Some(96.0));
    assert!(!artifacts.is_empty(), "at least one PNG page");
    for art in &artifacts {
        assert_eq!(art.output_format, OutputFormat::Png);
        assert!(
            art.bytes
                .starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            "artifact must carry the PNG signature"
        );
    }
}

