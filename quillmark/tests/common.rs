use quillmark_fixtures::{example_output_dir, resource_path, write_example_output};
use std::error::Error;

/// Demo helper that centralizes example plumbing.
///
/// It reads the given resource markdown, computes the quill path, then calls the
/// provided `runner` closure to perform backend-specific work. The `runner`
/// should return a tuple of (glue_bytes, output_bytes) which this helper will
/// write to the example output directory and print a short preview.
    pub fn demo(
    resource_name: &str,
    quill_dir: &str,
    asset_resources: Option<Vec<&str>>,
    glue_output: &str,
    render_output: &str,
) -> Result<(), Box<dyn Error>> {
    // Load the sample markdown
    let markdown = std::fs::read_to_string(resource_path(resource_name))?;

    // quill path (folder)
    let quill_path = resource_path(quill_dir);

    // Default engine flow used by examples: Typst backend, Quill from path, Workflow
    let quill = quillmark::Quill::from_path(quill_path.clone()).expect("Failed to load quill");
    let engine = quillmark::Quillmark::new();
    let mut workflow = engine.load(&quill).expect("Failed to load workflow");

    if let Some(assets) = &asset_resources {
        let full_assets: Vec<(String, Vec<u8>)> = assets.iter()
            .map(|name| (name.to_string(), std::fs::read(resource_path(name)).unwrap()))
            .collect();
        workflow = workflow.with_assets(full_assets)?;
    }

    // process glue
    let glued = workflow.process_glue(&markdown)?;

    // write outputs
    let glued_bytes = glued.into_bytes();
    write_example_output(glue_output, &glued_bytes)?;

    println!(
        "Glue outputted to:: {}",
        example_output_dir().join(glue_output).display()
    );

    // render output
    let rendered = workflow.render(&markdown, Some(quillmark_core::OutputFormat::Pdf))?;
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
