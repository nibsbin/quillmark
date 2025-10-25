use quillmark::{ParsedDocument, Quill, Quillmark};

fn main() {
    // Create a simple quill in memory
    let quill_json = r#"{
        "files": {
            "Quill.toml": {
                "contents": "[Quill]\nname = \"test-defaults\"\nbackend = \"typst\"\nglue_file = \"glue.typ\"\ndescription = \"Test quill\"\n\n[fields]\ntitle = { description = \"Title\" }\nauthor = { description = \"Author\", default = \"Anonymous\" }\nstatus = { description = \"Status\", default = \"draft\" }\n"
            },
            "glue.typ": {
                "contents": "Title: {{ title }}\nAuthor: {{ author }}\nStatus: {{ status }}"
            }
        }
    }"#;

    let quill = Quill::from_json(quill_json).expect("Failed to load quill");

    println!("✓ Loaded quill: {}", quill.name);
    println!("✓ Default values from schema:");
    let defaults = quill.extract_defaults();
    for (name, value) in &defaults {
        println!("  - {}: default = {:?}", name, value.as_json());
    }
    println!();

    // Create Quillmark engine and register the quill
    let mut engine = Quillmark::new();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow_from_quill_name("test-defaults")
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

    // Process through glue - this applies defaults
    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    println!("✓ Glue output (with defaults applied):");
    println!("{}", glue_output);
    println!();

    // Verify defaults were applied
    if glue_output.contains("Author: Anonymous") {
        println!("✓ SUCCESS: Default author was applied!");
    }
    if glue_output.contains("Status: draft") {
        println!("✓ SUCCESS: Default status was applied!");
    }
}
