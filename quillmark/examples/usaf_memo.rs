use quillmark::{Quill, Workflow};
use quillmark_core::OutputFormat;
use quillmark_fixtures::demo;
use quillmark_typst::TypstBackend;

fn main() {
    // Use the fixtures demo helper which centralizes file IO and printing.
    demo(
        "usaf_memo.md",
        "usaf_memo",
        "usaf_memo_glue.typ",
        "usaf_memo_output.pdf",
        |markdown: &str, quill_path: &std::path::Path| {
            // setup engine
            let backend = Box::new(TypstBackend::default());
            let quill = Quill::from_path(quill_path.to_path_buf()).expect("Failed to load quill");
            let engine = Workflow::new(backend, quill).expect("Failed to create engine");

            // process glue
            let glued = engine.process_glue(markdown).expect("Failed to process glue");

            // render end to end
            let rendered = engine
                .render(markdown, Some(OutputFormat::Pdf))
                .expect("Failed to render");

            Ok((glued.into_bytes(), rendered.artifacts[0].bytes.clone()))
        },
    )
    .expect("Demo failed");
}
