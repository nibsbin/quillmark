# Hello Quill Example

This directory contains a minimal quill template that demonstrates the QuillMark Typst backend functionality.

## Structure

```
hello-quill/
├── main.typ        # Main Typst template file
├── packages/       # Directory for Typst packages (empty in this example)
└── assets/         # Directory for assets like fonts and images (empty in this example)
```

## main.typ

The `main.typ` file contains a basic Typst template with:
- Page setup (8.5" x 11", 1" margins)
- Font configuration (Times New Roman, 12pt)
- A content placeholder (`$content$`) where Typst content will be inserted
- Basic styling and layout

## Testing the Example

To test this quill template:

```bash
# Run the example from the project root
cargo run --package quillmark-typst --example hello-quill-example
```

This will:
1. Load the hello-quill template
2. Process sample Typst content (not markdown)
3. Compile to both PDF and SVG formats
4. Save the output files for inspection

## Modifying the Template

You can edit `main.typ` to customize:
- Page layout and margins
- Typography and fonts
- Document structure
- Content layout around the `$content$` placeholder

After making changes, run the example again to see the updated output.

## Content Format

**Important**: The quillmark-typst backend expects valid Typst content, not markdown. The example provides Typst syntax directly:

```typst
= Heading
_italic text_
*bold text*
#quote[blockquote]
```

Markdown-to-Typst conversion will be handled by a separate module in the future.

## Adding Assets

To add fonts or images:
1. Place font files (.ttf, .otf) in the `assets/` directory
2. Reference them in your template using `#set text(font: "Your Font Name")`
3. Place images in `assets/` and reference them with `#image("assets/filename.ext")`

## Adding Packages

To include Typst packages:
1. Place package directories in the `packages/` folder
2. Each package should have its own subdirectory with a `typst.toml` manifest
3. Import packages in your template using `#import "package-name"`

This demonstrates the power of dynamic quill loading - templates are loaded at runtime, making it easy to create reusable document templates that can be shared and modified without recompiling the application.