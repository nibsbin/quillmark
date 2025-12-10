# Quillmark CLI

Command-line interface for the Quillmark Markdown rendering system.

## Overview

`quillmark-cli` is a standalone executable that renders Markdown files with YAML frontmatter into PDF, SVG, and other formats using Quillmark templates.

## Installation

### From crates.io (Recommended)

```bash
cargo install quillmark-cli
```

The binary will be installed to `~/.cargo/bin/quillmark` (ensure `~/.cargo/bin` is in your PATH).

### From Git Repository

```bash
# Install latest from main branch
cargo install --git https://github.com/nibsbin/quillmark quillmark-cli

# Install from specific branch or tag
cargo install --git https://github.com/nibsbin/quillmark --branch main quillmark-cli
```

### From Local Source

```bash
# From workspace root
cargo install --path bindings/quillmark-cli

# Or build without installing
cargo build --release -p quillmark-cli
# Binary will be at: target/release/quillmark
```

## Quick Start

Render a markdown file using a quill template:

```bash
quillmark render document.md --quill path/to/quill
```

The output will be saved as `document.pdf` by default.

## Usage

### Basic Rendering

```bash
# Render to PDF (default format)
quillmark render memo.md --quill ./quills/usaf_memo

# Specify output file
quillmark render memo.md --quill ./quills/usaf_memo -o output/final.pdf

# Render to different format
quillmark render memo.md --quill ./quills/usaf_memo --format svg
```

### Using QUILL Field in Frontmatter

If your markdown file has a `QUILL` field in the frontmatter, you can omit the `--quill` flag:

```markdown
---
QUILL: usaf_memo
title: My Memo
---

Content here...
```

```bash
quillmark render memo.md
```

### Advanced Options

```bash
# Output to stdout (useful for piping)
quillmark render memo.md --quill ./quills/usaf_memo --stdout > output.pdf

# Only process plate template (for debugging)
quillmark render memo.md --quill ./quills/usaf_memo --plate-only -o plate_output.typ

# Verbose output
quillmark render memo.md --quill ./quills/usaf_memo --verbose

# Quiet mode (suppress all non-error output)
quillmark render memo.md --quill ./quills/usaf_memo --quiet
```

## Command Reference

### `quillmark render`

Render a markdown file to the specified output format.

**Usage:**
```
quillmark render [OPTIONS] <MARKDOWN_FILE>
```

**Arguments:**
- `<MARKDOWN_FILE>` - Path to markdown file with YAML frontmatter

**Options:**
- `-q, --quill <PATH>` - Path to quill directory (overrides QUILL frontmatter field)
- `-o, --output <FILE>` - Output file path (default: derived from input filename)
- `-f, --format <FORMAT>` - Output format: pdf, svg, txt (default: pdf)
- `--stdout` - Write output to stdout instead of file
- `--plate-only` - Only process plate template, don't render final output
- `-v, --verbose` - Show detailed processing information
- `--quiet` - Suppress all non-error output

## Examples

### Example: Render USAF Memo

```bash
quillmark render \
  quillmark-fixtures/resources/tonguetoquill-collection/quills/usaf_memo/usaf_memo.md \
  --quill quillmark-fixtures/resources/tonguetoquill-collection/quills/usaf_memo \
  -o usaf_memo_output.pdf
```

### Example: Generate SVG

```bash
quillmark render document.md \
  --quill ./quills/my_template \
  --format svg \
  -o output.svg
```

### Example: Pipeline Usage

```bash
# Render and immediately view with a PDF viewer
quillmark render memo.md --quill ./quills/usaf_memo --stdout | evince -

# Render to stdout and pipe to another tool
quillmark render memo.md --quill ./quills/usaf_memo --stdout > final.pdf
```

### Example: Debug Template Processing

```bash
# Generate only the plate template to inspect intermediate Typst code
quillmark render memo.md \
  --quill ./quills/usaf_memo \
  --plate-only \
  -o debug_plate.typ
```

## Error Handling

The CLI provides clear error messages for common issues:

- **Missing markdown file**: `Markdown file not found: path/to/file.md`
- **Missing quill**: `Quill directory not found: path/to/quill`
- **No QUILL field**: `No QUILL field in frontmatter and --quill not specified`
- **Parse errors**: Line numbers and context for YAML or markdown issues
- **Template errors**: Compilation diagnostics from the rendering backend

## Exit Codes

- `0` - Success
- `1` - Error occurred (see stderr for details)

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Locally

```bash
cargo run -- render example.md --quill path/to/quill
```

## Design Documentation

For architectural details and design decisions, see:
- [CLI Design Document](../../prose/designs/CLI.md)
- [Implementation Plan](../../prose/plans/cli-basic-render.md)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../../LICENSE) for details.
