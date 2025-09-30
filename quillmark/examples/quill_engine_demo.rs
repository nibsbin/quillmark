use quillmark::{Quill, Quillmark};
use quillmark_core::OutputFormat;
use quillmark_fixtures::{example_output_dir, resource_path, write_example_output};

fn main() {
    // Step 1: Create Quillmark with auto-registered backends
    let mut engine = Quillmark::new();

    println!(
        "Quillmark initialized with backends: {:?}",
        engine.registered_backends()
    );

    // Step 2: Create Quill from path
    let ice_cream_quill_path = resource_path("ice_cream");
    let ice_cream_quill =
        Quill::from_path(ice_cream_quill_path).expect("Failed to load ice_cream quill");

    let usaf_memo_quill_path = resource_path("usaf_memo");
    let usaf_memo_quill =
        Quill::from_path(usaf_memo_quill_path).expect("Failed to load usaf_memo quill");

    // Step 3: Register Quills to Quillmark
    println!("Registering quill: {}", ice_cream_quill.name);
    engine.register_quill(ice_cream_quill);

    println!("Registering quill: {}", usaf_memo_quill.name);
    engine.register_quill(usaf_memo_quill);

    println!("Registered quills: {:?}", engine.registered_quills());

    // Step 4: Load workflow by quill name and render

    // Render ice_cream document
    let ice_cream_markdown = std::fs::read_to_string(resource_path("ice_cream.md")).unwrap();
    let ice_cream_workflow = engine
        .load("ice_cream")
        .expect("Failed to load ice_cream workflow");

    println!(
        "\nRendering with quill: {}",
        ice_cream_workflow.quill_name()
    );
    println!("Using backend: {}", ice_cream_workflow.backend_id());

    let ice_cream_result = ice_cream_workflow
        .render(&ice_cream_markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render ice_cream");

    write_example_output("ice_cream-engine.pdf", &ice_cream_result.artifacts[0].bytes).unwrap();
    println!(
        "Generated ice_cream PDF: {} bytes",
        ice_cream_result.artifacts[0].bytes.len()
    );

    // Render usaf_memo document
    let usaf_memo_markdown = std::fs::read_to_string(resource_path("usaf_memo.md")).unwrap();
    let usaf_memo_workflow = engine
        .load("usaf_memo")
        .expect("Failed to load usaf_memo workflow");

    println!(
        "\nRendering with quill: {}",
        usaf_memo_workflow.quill_name()
    );
    println!("Using backend: {}", usaf_memo_workflow.backend_id());

    let usaf_memo_result = usaf_memo_workflow
        .render(&usaf_memo_markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render usaf_memo");

    write_example_output("usaf_memo-engine.pdf", &usaf_memo_result.artifacts[0].bytes).unwrap();
    println!(
        "Generated usaf_memo PDF: {} bytes",
        usaf_memo_result.artifacts[0].bytes.len()
    );

    println!("\nOutput files:");
    println!(
        "- Ice Cream: {}",
        example_output_dir().join("ice_cream-engine.pdf").display()
    );
    println!(
        "- USAF Memo: {}",
        example_output_dir().join("usaf_memo-engine.pdf").display()
    );
}
