use quillmark::{Workflow, Quill};
use quillmark_fixtures::{write_example_output,resource_path,example_output_dir};
use quillmark_typst::TypstBackend;
use quillmark_core::{OutputFormat};

fn main() {
    // Load the sample markdown
    let markdown = std::fs::read_to_string(resource_path("ice_cream.md")).unwrap();

    //load quill
    let quill_path = resource_path("ice-cream");

    //setup engine
    let backend = Box::new(TypstBackend::default());
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    let engine = Workflow::new(
        backend,
        quill
    ).expect("Failed to create engine");

    // process glue
    let glued = engine.process_glue(&markdown).expect("Failed to process glue");
    write_example_output("ice-cream.typ", glued.as_bytes()).unwrap();
     
    //render end to end
    let rendered = engine.render(&markdown, Some(OutputFormat::Pdf)).expect("Failed to render");
    println!("Generated {} bytes", rendered.artifacts[0].bytes.len());
    write_example_output("ice-cream.pdf", &rendered.artifacts[0].bytes).unwrap();

    println!("Rendered output bytes: {}", rendered.artifacts[0].bytes.len());
    println!("Access files:\n- Glue: {}\n- Output: {}", example_output_dir().join("ice-cream.typ").display(), example_output_dir().join("ice-cream.pdf").display());
}