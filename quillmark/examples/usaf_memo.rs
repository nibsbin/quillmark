use quillmark::QuillEngine;
use quillmark_fixtures::{write_example_output,resource_path};
use quillmark_typst::TypstBackend;
use quillmark_core::{OutputFormat};

fn main() {


    // Load the sample markdown
    let markdown = std::fs::read_to_string(resource_path("usaf-memo.md")).unwrap();

    //load quill
    let quill_path = resource_path("usaf-memo");

    //setup engine
    let backend = Box::new(TypstBackend::default());
    let engine = QuillEngine::new(
        backend,
        quill_path
    ).expect("Failed to create engine");

    //render
    let result = engine.render_with_format(&markdown, Some(OutputFormat::Pdf)).expect("Failed to render");
    let content = result.artifacts[0].bytes.clone();

    //print content
    println!("Generated {} bytes", content.len());

    // save result
    write_example_output("usaf-memo-output.pdf", &content).unwrap();

}