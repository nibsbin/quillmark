# quillmark-cli

Command-line interface for Quillmark, a template-first Markdown rendering system.

## Installation

```bash
cargo install --path quillmark-cli
```

## Usage

Basic usage:

```bash
quillmark-cli <quill_path> <markdown_file>
```

This will render the markdown file using the specified quill template and output a PDF with the same name as the input file (with `.pdf` extension).

### Arguments

- `<quill_path>`: Path to the quill template directory (must contain a `Quill.toml` file)
- `<markdown_file>`: Path to the markdown file to render

### Options

- `-o, --output <OUTPUT>`: Specify a custom output path for the PDF (optional)
- `-h, --help`: Display help information

## Examples

Render a markdown file using the "taro" quill template:

```bash
quillmark-cli path/to/taro document.md
```

Specify a custom output path:

```bash
quillmark-cli path/to/taro document.md -o output.pdf
```

## Features

- Renders Markdown with YAML frontmatter to PDF
- Template-based styling via Quill templates
- Automatic output path generation
- Clear error messages

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
