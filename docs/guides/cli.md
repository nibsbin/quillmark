# CLI Reference

Command-line interface for Quillmark rendering.

## Installation

```bash
cargo install quillmark-cli
```

## Commands

### render

Render markdown documents to PDF, SVG, or text. Optionally emit compiled JSON data.

```bash
quillmark render [OPTIONS] [MARKDOWN_FILE]
```

**Arguments:**

- `[MARKDOWN_FILE]`: Path to markdown file with YAML frontmatter (optional — when omitted, the quill's example content is used, which requires `--quill`)

**Options:**

- `-q <PATH>` / `--quill <PATH>`: Path to quill directory (optional if the markdown frontmatter contains a `QUILL` field)
- `-o <PATH>` / `--output <PATH>`: Output file path (default: derived from input filename, e.g. `input.pdf`)
- `-f <FORMAT>` / `--format <FORMAT>`: Output format: `pdf`, `svg`, `txt` (default: `pdf`)
- `--output-data <DATA_FILE>`: Write compiled JSON data (after coercion/defaults/transform_fields) to a file
- `-v` / `--verbose`: Show detailed processing information
- `--quiet`: Suppress all non-error output
- `--stdout`: Write output to stdout instead of file

**Examples:**

```bash
# Render to PDF
quillmark render -q ./invoice-quill input.md -o output.pdf

# Render to SVG
quillmark render -q ./my-quill input.md -f svg -o output.svg

# Emit compiled data for inspection
quillmark render -q ./my-quill input.md --output-data data.json

# Output to stdout
quillmark render -q ./my-quill input.md --stdout > output.pdf

# Render the quill's built-in example
quillmark render -q ./my-quill
```

### schema

Extract JSON schema from a quill's field definitions.

```bash
quillmark schema [OPTIONS] <QUILL_PATH>
```

**Arguments:**

- `<QUILL_PATH>`: Path to quill directory

**Options:**

- `-o <FILE>` / `--output <FILE>`: Output file (default: stdout)

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
quillmark validate [OPTIONS] <QUILL_PATH>
```

**Arguments:**

- `<QUILL_PATH>`: Path to quill directory

**Options:**

- `-v` / `--verbose`: Show verbose output with all validation details

**Examples:**

```bash
# Validate quill structure
quillmark validate ./my-quill

# Verbose validation
quillmark validate ./my-quill -v
```

### info

Display metadata and information about a quill.

```bash
quillmark info [OPTIONS] <QUILL_PATH>
```

**Arguments:**

- `<QUILL_PATH>`: Path to quill directory

**Options:**

- `--json`: Output as machine-readable JSON instead of human-readable format

**Examples:**

```bash
# Display quill info
quillmark info ./my-quill

# Output as JSON
quillmark info ./my-quill --json

# Use with other tools
quillmark info ./my-quill --json | jq '.name'
```

## Exit Codes

- `0`: Success
- `1`: Error (invalid arguments, file not found, parse error, compilation error, etc.)

## Common Workflows

### Batch Rendering

```bash
#!/bin/bash
# Render multiple documents

for file in inputs/*.md; do
    output="outputs/$(basename "$file" .md).pdf"
    quillmark render -q ./my-quill "$file" -o "$output"
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
quillmark render -q ./my-quill input.md -f pdf -o output.pdf
quillmark render -q ./my-quill input.md -f svg -o output.svg
quillmark render -q ./my-quill input.md -f txt -o output.txt
```

## Environment Variables

- `RUST_LOG`: Set logging level (e.g., `RUST_LOG=debug quillmark render ...`)
- `NO_COLOR`: Disable colored output

## Notes

- When `--output` is omitted, the output filename is derived from the input filename (e.g., `input.md` → `input.pdf`)
- Use `--stdout` to send output to stdout instead of a file
- Use `--` to separate options from positional arguments if needed
- Verbose mode shows template composition and compilation details
