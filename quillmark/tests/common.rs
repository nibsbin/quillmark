use quillmark_fixtures::{example_output_dir, quills_path, write_example_output};
use std::error::Error;

/// Demo helper that centralizes example plumbing.
///
/// It loads the quill and uses its markdown template, then processes and renders it.
pub fn demo(
    quill_dir: &str,
    glue_output: &str,
    render_output: &str,
    use_resource_path: bool,
) -> Result<(), Box<dyn Error>> {
    // quill path (folder)
    let quill_path = if use_resource_path {
        quillmark_fixtures::resource_path(quill_dir)
    } else {
        quills_path(quill_dir)
    };
    // Default engine flow used by examples: Typst backend, Quill from path, Workflow
    let quill = quillmark::Quill::from_path(quill_path.clone()).expect("Failed to load quill");

    // Load the markdown template from the quill
    let markdown = quill
        .example
        .as_ref()
        .ok_or("Quill does not have a markdown template")?
        .clone();

    // Parse the markdown once
    let parsed = quillmark::ParsedDocument::from_markdown(&markdown)?;

    let engine = quillmark::Quillmark::new();
    let workflow = engine
        .workflow_from_quill(&quill)
        .expect("Failed to load workflow");

    // process glue
    let glued = workflow.process_glue(&parsed)?;

    // write outputs
    let glued_bytes = glued.into_bytes();
    write_example_output(glue_output, &glued_bytes)?;

    println!(
        "Glue outputted to: {}",
        example_output_dir().join(glue_output).display()
    );

    // render output
    let rendered = workflow.render(&parsed, Some(quillmark_core::OutputFormat::Pdf))?;
    let output_bytes = rendered.artifacts[0].bytes.clone();

    write_example_output(render_output, &output_bytes)?;

    println!("------------------------------");
    println!(
        "Access glue output: {}",
        example_output_dir().join(glue_output).display()
    );
    println!(
        "Access render output: {}",
        example_output_dir().join(render_output).display()
    );

    Ok(())
}
