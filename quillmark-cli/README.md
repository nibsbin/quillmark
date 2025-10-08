# quillmark-cli

Minimal CLI for testing Quillmark packages locally.

## Overview

`quillmark-cli` is a simple command-line tool for rendering Markdown files to PDF using Quillmark Typst quills. It's designed for local testing and development of quill templates.

## Installation

```bash
cargo install quillmark-cli
```

Or install from source:

```bash
git clone https://github.com/nibsbin/quillmark
cd quillmark/quillmark-cli
cargo install --path .
```

## Usage

```bash
quillmark-cli --quill <QUILL_DIR> --markdown <MARKDOWN_FILE> --output <OUTPUT_PDF>
```

### Arguments

- `--quill <QUILL_DIR>` - Path to the quill directory containing the template
- `--markdown <MARKDOWN_FILE>` - Path to the markdown file to render
- `--output <OUTPUT_PDF>` - Output PDF file path (default: `output.pdf`)

### Example

```bash
# Render a memo using the usaf_memo quill
quillmark-cli \
  --quill ./quillmark-fixtures/resources/usaf_memo \
  --markdown ./my-memo.md \
  --output ./my-memo.pdf
```

## Features

- PDF rendering using Typst backend
- Structured error diagnostics
- Minimal dependencies
- Clean, focused CLI interface

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
