# CLI Reference

Command-line interface for Quillmark rendering.

## Installation

The CLI is available when installing Quillmark:

```bash
# Using uv
uv tool install quillmark

# Using pip
pip install quillmark
```

## Commands

### render

Render markdown documents to PDF, SVG, or text. Optionally emit compiled JSON data.

```bash
quillmark render [OPTIONS] <INPUT>
```

**Arguments:**

- `<INPUT>`: Markdown file path or `-` for stdin

**Options:**

- `--quill <PATH>`: Path to quill directory (required unless using `--quill-name`)
- `--quill-name <NAME>`: Use registered quill by name
- `--output <PATH>` / `-o <PATH>`: Output file path (default: stdout)
- `--format <FORMAT>` / `-f <FORMAT>`: Output format: `pdf`, `svg`, `txt`
- `--output-data <PATH>`: Write compiled JSON data (after coercion/defaults/transform_fields) to a file
- `--verbose` / `-v`: Enable verbose logging
- `--quiet` / `-q`: Suppress non-error output
- `--stdout`: Force output to stdout

**Examples:**

```bash
# Render to PDF
quillmark render --quill ./invoice-quill input.md -o output.pdf

# Render to SVG
quillmark render --quill ./my-quill input.md -f svg -o output.svg

# Emit compiled data for inspection
quillmark render --quill ./my-quill input.md --output-data data.json

# Render from stdin
cat input.md | quillmark render --quill ./my-quill -o output.pdf

# Output to stdout
quillmark render --quill ./my-quill input.md --stdout > output.pdf
```

### schema

Extract JSON schema from a quill's field definitions.

```bash
quillmark schema [OPTIONS] <QUILL>
```

**Arguments:**

- `<QUILL>`: Path to quill directory

**Options:**

- `--output <PATH>` / `-o <PATH>`: Output file (default: stdout)

**Examples:**

```bash
# Print schema to stdout
quillmark schema ./my-quill

# Save schema to file
quillmark schema ./my-quill -o schema.json

# Use with other tools
quillmark schema ./my-quill | jq '.properties.title'
```

### validate

Validate quill configuration and structure.

```bash
quillmark validate [OPTIONS] <QUILL>
```

**Arguments:**

- `<QUILL>`: Path to quill directory

**Options:**

- `--verbose` / `-v`: Show detailed validation info
- `--quiet` / `-q`: Only show errors

**Examples:**

```bash
# Validate quill structure
quillmark validate ./my-quill

# Verbose validation
quillmark validate ./my-quill -v

# Quiet mode (exit code only)
quillmark validate ./my-quill -q && echo "Valid"
```

## Exit Codes

- `0`: Success
- `1`: General error (invalid arguments, file not found)
- `2`: Parse error (invalid YAML/markdown)
- `3`: Template error (template composition failed)
- `4`: Compilation error (backend compilation failed)

## Common Workflows

### Batch Rendering

```bash
#!/bin/bash
# Render multiple documents

for file in inputs/*.md; do
    output="outputs/$(basename "$file" .md).pdf"
    quillmark render --quill ./my-quill "$file" -o "$output"
done
```

### Validation in CI

```bash
#!/bin/bash
# Validate quills in CI pipeline

set -e  # Exit on error

quillmark validate ./quills/invoice
quillmark validate ./quills/report
quillmark validate ./quills/letter

echo "✓ All quills valid"
```

### Format Conversion

```bash
# Generate multiple formats
quillmark render --quill ./my-quill input.md -f pdf -o output.pdf
quillmark render --quill ./my-quill input.md -f svg -o output.svg
quillmark render --quill ./my-quill input.md -f txt -o output.txt
```

## Environment Variables

- `RUST_LOG`: Set logging level (e.g., `RUST_LOG=debug quillmark render ...`)
- `NO_COLOR`: Disable colored output

## Notes

- When `--output` is omitted, output goes to stdout
- Use `--` to separate options from positional arguments if needed
- Binary output (PDF, etc.) to stdout requires `--stdout` or redirection
- Verbose mode shows template composition and compilation details
