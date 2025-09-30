use quillmark::{Quill, Workflow};
use quillmark_core::OutputFormat;
use quillmark_fixtures::{example_output_dir, resource_path, write_example_output};
use quillmark_typst::TypstBackend;

fn main() {
    // Load the sample markdown
    let markdown = std::fs::read_to_string(resource_path("usaf_memo.md")).unwrap();

    //load quill
    let quill_path = resource_path("usaf-memo");

    //setup engine
    let backend = Box::new(TypstBackend::default());
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    let engine = Workflow::new(backend, quill).expect("Failed to create engine");

    // process glue
    let glued = engine
        .process_glue(&markdown)
        .expect("Failed to process glue");
    write_example_output("usaf-memo-glue.typ", glued.as_bytes()).unwrap();

    println!(
        "Processed glue content preview: \n\n{}...\n",
        &glued[..std::cmp::min(500, glued.len())]
    );

    //render end to end
    let rendered = engine
        .render(&markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render");
    println!("Generated {} bytes", rendered.artifacts[0].bytes.len());
    write_example_output("usaf-memo-output.pdf", &rendered.artifacts[0].bytes).unwrap();

    println!(
        "Rendered output bytes: {}",
        rendered.artifacts[0].bytes.len()
    );

    println!(
        "Access files:\n- Glue: {}\n- Output: {}",
        example_output_dir().join("usaf-memo-glue.typ").display(),
        example_output_dir().join("usaf-memo-output.pdf").display()
    );
}
