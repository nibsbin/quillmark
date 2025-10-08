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

The simplest usage is to specify the markdown file and include the quill path in the frontmatter:

```bash
quillmark-cli <MARKDOWN_FILE>
```

The markdown file should include a `quill` field in its frontmatter:

```markdown
---
quill: path/to/quill
title: My Document
---

Document content...
```

### Command-Line Options

```bash
quillmark-cli [OPTIONS] <MARKDOWN>

Arguments:
  <MARKDOWN>  Path to the markdown file to render

Options:
  -q, --quill <QUILL>    Path to the quill directory (optional override)
  -o, --output <OUTPUT>  Output PDF file path [default: output.pdf]
  -h, --help             Print help
```

- `<MARKDOWN>` - Path to the markdown file to render (required)
- `--quill <QUILL_DIR>` - Optional path to override the quill specified in frontmatter
- `--output <OUTPUT_PDF>` - Output PDF file path (default: `output.pdf`)

### Examples

#### Example 1: Simple Usage with Frontmatter

Create `my-memo.md`:
```markdown
---
quill: ./quillmark-fixtures/resources/usaf_memo
from: HQ AFGSC/A3TW
to: ALL MAJCOM UNITS
subject: Test Memorandum
date: 2024-01-15
---

This is the body of the memorandum.
```

Render it:
```bash
quillmark-cli my-memo.md
```

This creates `output.pdf` using the quill specified in the frontmatter.

#### Example 2: With Custom Output Path

```bash
quillmark-cli my-memo.md --output my-memo.pdf
```

#### Example 3: Override Quill with Flag

```bash
# Use a different quill than specified in frontmatter
quillmark-cli my-memo.md --quill ./path/to/different-quill --output test.pdf
```

#### Example 4: Complete Example

Create `simple-doc.md`:
```markdown
---
quill: ./quillmark-fixtures/resources/taro
author: John Doe
ice_cream: Chocolate
title: My Document
---

This is a simple document.
```

Render it:
```bash
quillmark-cli simple-doc.md --output simple-doc.pdf
```

## Features

- PDF rendering using Typst backend
- Structured error diagnostics
- Minimal dependencies
- Clean, focused CLI interface

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
