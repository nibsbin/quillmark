use quillmark::{ParsedDocument, Quill, Quillmark};

fn main() {
    // Create a simple quill in memory
    let quill_json = r#"{
        "files": {
            "Quill.toml": {
                "contents": "[Quill]\nname = \"test-defaults\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n\n[fields]\ntitle = { description = \"Title\" }\nauthor = { description = \"Author\", default = \"Anonymous\" }\nstatus = { description = \"Status\", default = \"draft\" }\n"
            },
            "plate.typ": {
                "contents": "Title: {{ title }}\nAuthor: {{ author }}\nStatus: {{ status }}"
            }
        }
    }"#;

    let quill = Quill::from_json(quill_json).expect("Failed to load quill");

    println!("✓ Loaded quill: {}", quill.name);
    println!("✓ Default values from schema:");
    let defaults = quill.extract_defaults();
    for (name, value) in defaults {
        println!("  - {}: default = {:?}", name, value.as_json());
    }
    println!();

    // Create Quillmark engine and register the quill
    let mut engine = Quillmark::new();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test-defaults")
        .expect("Failed to get workflow");

    // Parse markdown with only title (missing author and status)
    let markdown = r#"---
title: My Test Document
---

This is a test.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    println!("✓ Fields in original parsed document:");
    for (key, value) in parsed.fields() {
        println!("  - {}: {}", key, value.as_json());
    }
    println!();

    // Render plate - this applies defaults and returns the print
    let print_output = workflow
        .render_plate(&parsed)
        .expect("Failed to render plate");

    println!("✓ Print output (with defaults applied):");
    println!("{}", print_output);
    println!();

    // Verify defaults were applied
    if print_output.contains("Author: Anonymous") {
        println!("✓ SUCCESS: Default author was applied!");
    }
    if print_output.contains("Status: draft") {
        println!("✓ SUCCESS: Default status was applied!");
    }
}
