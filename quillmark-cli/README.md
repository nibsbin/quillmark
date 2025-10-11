# quillmark-cli

A simple command-line interface for rendering Markdown files to PDF using Quillmark.

## Installation

Build from source:

```bash
cargo build --release -p quillmark-cli
```

The binary will be available at `target/release/quillmark-cli`.

## Usage

```bash
quillmark-cli <quill_path> <markdown_file>
```

### Arguments

- `<quill_path>` - Path to the quill template directory
- `<markdown_file>` - Path to the markdown file to render

### Output

The CLI generates a PDF file with the same name as the markdown file, in the same directory.
For example, if you render `document.md`, the output will be `document.pdf`.

## Example

```bash
# Render a markdown file using the taro quill template
quillmark-cli quillmark-fixtures/resources/taro my-document.md

# This will create my-document.pdf in the current directory
```

## Error Handling

The CLI provides clear error messages for common issues:

- Missing or invalid quill template
- File not found errors
- Template compilation errors
- Rendering errors

All errors are printed to stderr with detailed diagnostic information.

## License

Licensed under the Apache License, Version 2.0. See the main repository LICENSE for details.
