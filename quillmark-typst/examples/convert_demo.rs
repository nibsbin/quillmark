use quillmark_core::{parameterize, test_context};
use quillmark_typst::mark_to_typst;
use std::fs;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Get the workspace examples directory
    let examples_dir = match test_context::examples_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Error finding examples directory: {}", err);
            std::process::exit(1);
        }
    };
    
    // Create output directory within examples
    let output_dir = match test_context::create_output_dir("converted") {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("Error creating output directory: {}", err);
            std::process::exit(1);
        }
    };
    
    let (input_file, output_file) = if args.len() >= 2 {
        let input = &args[1];
        let output = if args.len() >= 3 {
            // Put custom output file in examples/converted/ directory
            let output_path = Path::new(&args[2]);
            let filename = output_path.file_name().unwrap_or(std::ffi::OsStr::new("output.typ"));
            output_dir.join(filename)
        } else {
            // Generate output filename by changing extension and put in examples/converted/
            let path = Path::new(input);
            let stem = path.file_stem().unwrap_or(std::ffi::OsStr::new("output"));
            output_dir.join(format!("{}.typ", stem.to_string_lossy()))
        };
        (input.clone(), output)
    } else {
        // Use the example file if no arguments provided
        let input_file = examples_dir.join("sample.md");
        let output_file = output_dir.join("sample_output.typ");
        (input_file.to_string_lossy().to_string(), output_file)
    };
    
    println!("Converting {} to {}", input_file, output_file.display());
    
    let markdown_content = match fs::read_to_string(&input_file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{}': {}", input_file, err);
            std::process::exit(1);
        }
    };

    println!("=== Original Markdown ===");
    println!("{}", markdown_content);
    
    println!("\n=== Converting to Typst ===");
    
    // Parse the markdown to separate frontmatter from body
    let parsed_doc = match parameterize(&markdown_content) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("Error parsing markdown: {}", err);
            std::process::exit(1);
        }
    };
    
    // Show frontmatter fields if present
    let frontmatter_fields: Vec<_> = parsed_doc.fields().keys()
        .filter(|k| *k != "BODY")
        .collect();
    
    if !frontmatter_fields.is_empty() {
        println!("Frontmatter fields found: {:?}", frontmatter_fields);
        for field in &frontmatter_fields {
            if let Some(value) = parsed_doc.get_field(field) {
                println!("  {}: {:?}", field, value);
            }
        }
        println!();
    }
    
    // Convert only the body to Typst
    let body = parsed_doc.body().unwrap_or("");
    let typst_output = mark_to_typst(body);
    
    println!("{}", typst_output);
    
    // Write the output to file
    if let Err(err) = fs::write(&output_file, &typst_output) {
        eprintln!("Error writing file '{}': {}", output_file.display(), err);
        std::process::exit(1);
    }
    
    println!("\n=== Conversion Complete ===");
    println!("Output written to: {}", output_file.display());
    println!("\nYou can now edit '{}' and run this tool again to see the updated .typ file.", input_file);
}