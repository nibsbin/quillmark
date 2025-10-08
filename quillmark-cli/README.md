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

### Examples

#### Example 1: Render a USAF Memo

```bash
# Using the usaf_memo quill from fixtures
quillmark-cli \
  --quill ./quillmark-fixtures/resources/usaf_memo \
  --markdown ./my-memo.md \
  --output ./my-memo.pdf
```

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

#### Example 2: Render with a Simple Quill

```bash
# Using the taro quill (simpler, no external packages)
quillmark-cli \
  --quill ./quillmark-fixtures/resources/taro \
  --markdown ./simple-doc.md \
  --output ./simple-doc.pdf
```

Create `simple-doc.md`:
```markdown
---
author: John Doe
ice_cream: Chocolate
title: My Document
---

This is a simple document.
```

#### Example 3: Testing Your Own Quill

```bash
# Test your custom quill during development
quillmark-cli \
  --quill ./path/to/my-quill \
  --markdown ./test.md \
  --output ./test.pdf
```

## Features

- PDF rendering using Typst backend
- Structured error diagnostics
- Minimal dependencies
- Clean, focused CLI interface

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
