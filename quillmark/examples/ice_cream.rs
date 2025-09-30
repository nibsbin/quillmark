use quillmark::{Quill, Workflow};
use quillmark_core::OutputFormat;
use quillmark_fixtures::{example_output_dir, resource_path, write_example_output};
use quillmark_typst::TypstBackend;

fn main() {
    // Load the sample markdown
    let markdown = std::fs::read_to_string(resource_path("ice_cream.md")).unwrap();

    //load quill
    let quill_path = resource_path("ice_cream");

    //setup engine
    let backend = Box::new(TypstBackend::default());
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    let engine = Workflow::new(backend, quill).expect("Failed to create engine");

    // process glue
    let glued = engine
        .process_glue(&markdown)
        .expect("Failed to process glue");
    write_example_output("ice_cream.typ", glued.as_bytes()).unwrap();

    //render end to end
    let rendered = engine
        .render(&markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render");
    println!("Generated {} bytes", rendered.artifacts[0].bytes.len());
    write_example_output("ice_cream.pdf", &rendered.artifacts[0].bytes).unwrap();

    println!(
        "Rendered output bytes: {}",
        rendered.artifacts[0].bytes.len()
    );
    println!(
        "Access files:\n- Glue: {}\n- Output: {}",
        example_output_dir().join("ice_cream.typ").display(),
        example_output_dir().join("ice_cream.pdf").display()
    );
}
