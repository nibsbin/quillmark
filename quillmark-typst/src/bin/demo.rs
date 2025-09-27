use quillmark_typst::mark_to_typst;
use std::fs;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let (input_file, output_file) = if args.len() >= 2 {
        let input = &args[1];
        let output = if args.len() >= 3 {
            args[2].clone()
        } else {
            // Generate output filename by changing extension
            let path = Path::new(input);
            let stem = path.file_stem().unwrap_or(std::ffi::OsStr::new("output"));
            format!("{}.typ", stem.to_string_lossy())
        };
        (input.clone(), output)
    } else {
        // Use the example file if no arguments provided
        ("../examples/sample.md".to_string(), "sample_output.typ".to_string())
    };
    
    println!("Converting {} to {}", input_file, output_file);
    
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
    let typst_output = mark_to_typst(&markdown_content);
    
    println!("{}", typst_output);
    
    // Write the output to file
    if let Err(err) = fs::write(&output_file, &typst_output) {
        eprintln!("Error writing file '{}': {}", output_file, err);
        std::process::exit(1);
    }
    
    println!("\n=== Conversion Complete ===");
    println!("Output written to: {}", output_file);
    println!("\nYou can now edit '{}' and run this tool again to see the updated .typ file.", input_file);
}