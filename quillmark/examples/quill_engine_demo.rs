use quillmark::{Quillmark, Quill};
use quillmark_fixtures::{write_example_output, resource_path, example_output_dir};
use quillmark_core::OutputFormat;

fn main() {
    // Step 1: Create Quillmark with auto-registered backends
    let mut engine = Quillmark::new();
    
    println!("Quillmark initialized with backends: {:?}", engine.registered_backends());
    
    // Step 2: Create Quill from path
    let ice_cream_quill_path = resource_path("ice-cream");
    let ice_cream_quill = Quill::from_path(ice_cream_quill_path).expect("Failed to load ice-cream quill");
    
    let usaf_memo_quill_path = resource_path("usaf-memo");
    let usaf_memo_quill = Quill::from_path(usaf_memo_quill_path).expect("Failed to load usaf-memo quill");
    
    // Step 3: Register Quills to Quillmark
    println!("Registering quill: {}", ice_cream_quill.name);
    engine.register_quill(ice_cream_quill);
    
    println!("Registering quill: {}", usaf_memo_quill.name);
    engine.register_quill(usaf_memo_quill);
    
    println!("Registered quills: {:?}", engine.registered_quills());
    
    // Step 4: Load workflow by quill name and render
    
    // Render ice-cream document
    let ice_cream_markdown = std::fs::read_to_string(resource_path("ice_cream.md")).unwrap();
    let ice_cream_workflow = engine.load("ice-cream").expect("Failed to load ice-cream workflow");
    
    println!("\nRendering with quill: {}", ice_cream_workflow.quill_name());
    println!("Using backend: {}", ice_cream_workflow.backend_id());
    
    let ice_cream_result = ice_cream_workflow.render(&ice_cream_markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render ice-cream");
    
    write_example_output("ice-cream-engine.pdf", &ice_cream_result.artifacts[0].bytes).unwrap();
    println!("Generated ice-cream PDF: {} bytes", ice_cream_result.artifacts[0].bytes.len());
    
    // Render usaf-memo document
    let usaf_memo_markdown = std::fs::read_to_string(resource_path("usaf_memo.md")).unwrap();
    let usaf_memo_workflow = engine.load("usaf-memo").expect("Failed to load usaf-memo workflow");
    
    println!("\nRendering with quill: {}", usaf_memo_workflow.quill_name());
    println!("Using backend: {}", usaf_memo_workflow.backend_id());
    
    let usaf_memo_result = usaf_memo_workflow.render(&usaf_memo_markdown, Some(OutputFormat::Pdf))
        .expect("Failed to render usaf-memo");
    
    write_example_output("usaf-memo-engine.pdf", &usaf_memo_result.artifacts[0].bytes).unwrap();
    println!("Generated usaf-memo PDF: {} bytes", usaf_memo_result.artifacts[0].bytes.len());
    
    println!("\nOutput files:");
    println!("- Ice Cream: {}", example_output_dir().join("ice-cream-engine.pdf").display());
    println!("- USAF Memo: {}", example_output_dir().join("usaf-memo-engine.pdf").display());
}
