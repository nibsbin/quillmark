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

The CLI requires both a markdown file and a quill path:

```bash
quillmark-cli <MARKDOWN_FILE> --quill-path <QUILL_DIR>
```

### Command-Line Options

```bash
quillmark-cli [OPTIONS] --quill-path <QUILL_PATH> <MARKDOWN>

Arguments:
  <MARKDOWN>  Path to the markdown file to render

Options:
      --quill-path <QUILL_PATH>  Path to the quill directory
  -o, --output <OUTPUT>          Output PDF file path [default: output.pdf]
  -h, --help                     Print help
```

- `<MARKDOWN>` - Path to the markdown file to render (required)
- `--quill-path <QUILL_DIR>` - Path to the quill directory (required)
- `--output <OUTPUT_PDF>` - Output PDF file path (default: `output.pdf`)

### Quill Validation

If your markdown includes a `QUILL` field in the frontmatter, the CLI will warn you if it doesn't match the quill loaded from `--quill-path`. The quill specified by `--quill-path` is always used as the authoritative source.

Example warning:
```
Warning: Markdown specifies quill 'expected_quill' but using quill 'actual_quill' from --quill-path
```

### Examples

#### Example 1: Basic Usage

Create `my-memo.md`:
```markdown
---
from: HQ AFGSC/A3TW
to: ALL MAJCOM UNITS
subject: Test Memorandum
date: 2024-01-15
---

This is the body of the memorandum.
```

Render it:
```bash
quillmark-cli my-memo.md --quill-path ./quillmark-fixtures/resources/usaf_memo
```

This creates `output.pdf` using the specified quill.

#### Example 2: With Custom Output Path

```bash
quillmark-cli my-memo.md --quill-path ./my-quill --output my-memo.pdf
```

#### Example 3: With QUILL Field (Validation)

Create `document.md`:
```markdown
---
QUILL: usaf_memo
from: HQ AFGSC/A3TW
to: ALL MAJCOM UNITS
subject: Test
---

Document content...
```

Render it:
```bash
quillmark-cli document.md --quill-path ./quillmark-fixtures/resources/usaf_memo
```

If the loaded quill's name matches `usaf_memo`, no warning is shown. If it doesn't match, you'll see a warning, but the PDF will still be generated using the quill from `--quill-path`.

#### Example 4: Testing Your Own Quill

```bash
# Test your custom quill during development
quillmark-cli test.md --quill-path ./path/to/my-quill --output test.pdf
```

## Features

- PDF rendering using Typst backend
- Structured error diagnostics
- Minimal dependencies
- Clean, focused CLI interface

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
