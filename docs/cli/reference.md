# CLI Reference

Command-line interface for Quillmark rendering.

## Installation

```bash
cargo install quillmark-cli
```

## Commands

### render

Render a markdown document to the specified output format.

```bash
quillmark render [OPTIONS] <QUILL_PATH> [MARKDOWN_FILE]
```

**Arguments:**

- `<QUILL_PATH>`: Path to quill directory
- `[MARKDOWN_FILE]`: Path to markdown file with a root card-yaml block (optional, when omitted, the quill's seeded document is rendered, each field populated from its `example:` value, with `default:` used as fallback)

The file must open with a `~~~` block containing a `$quill:` key identifying the quill. The `~~~card-yaml` opener is also accepted.

**Options:**

- `-o <PATH>` / `--output <PATH>`: Output file path (default: input filename with format extension, e.g. `input.pdf`; `example.<format>` when no markdown file is given)
- `-f <FORMAT>` / `--format <FORMAT>`: Output format: `pdf`, `svg`, `png` (default: `pdf`)
- `--output-data <DATA_FILE>`: Write compiled JSON data to a file
- `-v` / `--verbose`: Show detailed processing information on stderr
- `--quiet`: Suppress warnings and the output-destination line
- `--stdout`: Write the artifact to stdout instead of a file (and ignore `-o`); refused when the render produces more than one page

**Streams:** under `--stdout` the artifact owns stdout, and progress, warnings, and errors all go to stderr, so `quillmark render ./my-quill input.md --stdout --verbose > out.pdf` writes a valid PDF. Without `--stdout`, the one stdout line is `Output written to: <path>`, which `--quiet` suppresses.

**Pages:** `svg` and `png` render one artifact per page. A multi-page document writes one numbered file per page — `out.svg` becomes `out-1.svg`, `out-2.svg`, … — so no unnumbered file claims to be the whole document. `--stdout` carries one artifact and refuses a multi-page render.

**Examples:**

```bash
# Render to PDF
quillmark render ./invoice-quill input.md -o output.pdf

# Render to SVG
quillmark render ./my-quill input.md -f svg -o output.svg

# Emit compiled data for inspection
quillmark render ./my-quill input.md --output-data data.json

# Output to stdout
quillmark render ./my-quill input.md --stdout > output.pdf

# Render the quill's seeded document
quillmark render ./my-quill
```

### schema

Output the quill's field schema as YAML, including main-card and card-kind field definitions with UI hints.

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
quillmark schema ./my-quill -o schema.yaml
```

### blueprint

Print a quill's Markdown blueprint: an annotated document showing the quill's fields, constraints, and examples, itself a valid document an author can fill in.

```bash
quillmark blueprint [OPTIONS] <QUILL_PATH>
```

**Arguments:**

- `<QUILL_PATH>`: Path to quill directory

**Options:**

- `-o <FILE>` / `--output <FILE>`: Output file (default: stdout)

**Examples:**

```bash
# Print blueprint to stdout
quillmark blueprint ./my-quill

# Save blueprint to file
quillmark blueprint ./my-quill -o blueprint.md
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

**Fields shown:** name, description, version, author, backend, field count, and
card count (when nonzero), plus a metadata section for any non-standard
`Quill.yaml` keys: the standard keys (`backend`, `version`, `author`,
`description`) are excluded from it. The text output additionally shows a
defaults count when nonzero; `--json` has no defaults count.

**Examples:**

```bash
# Display quill info
quillmark info ./my-quill

# Output as JSON
quillmark info ./my-quill --json
```

### merge

Generate one document per input row, or per group of rows, through a merge
spec, and render them all. Full guide: [Bulk Generation](../integration/bulk-generation.md).

```bash
quillmark merge [OPTIONS] <QUILL_PATH> <SPEC_FILE> <INPUT_FILE> --out <DIR>
```

**Arguments:**

- `<QUILL_PATH>`: Path to quill directory
- `<SPEC_FILE>`: The merge spec, YAML (or JSON by `.json` extension): `$quill`, `mode`, `map`, `output`, …
- `<INPUT_FILE>`: `.csv` / `.tsv` (a header row; cells are strings), or `.json`: an array of row objects, or an object carrying a `documents` array in the values form

**Options:**

- `--out <DIR>`: Output directory; every artifact and `manifest.json` land here (required unless `--dry-run`)
- `--dry-run`: Plan and print the report, render nothing
- `--force`: Render the documents no error touches and report the rest
- `--json`: Emit the report and the manifest as one JSON object on stdout (rows 0-based)
- `-f <FORMAT>` / `--format <FORMAT>`: `pdf` (default), `svg`, `png`; appended to the spec's `output` stem
- `--delimiter <CHAR>`: Field delimiter for a tabular input (default `,`, or a tab for `.tsv`)
- `--jobs <N>`: Render threads (default: every core)
- `--quiet`: Suppress the report and the summary line

**Report:** printed to stderr, one line per error and one per distinct warning (a warning repeated across rows shows its count and first row). A tabular input is numbered as a spreadsheet numbers it (the header is row 1); a JSON input by index. The plan runs whole before anything renders; a spec-level problem (an unknown target, a column the input lacks, a `$quill` that does not pair) stops it before the first row.

**Manifest:** `manifest.json` in `--out` lists every planned document: `key`, `rows`, `filename`, `input_hash`, `status` (`rendered`, `failed`, `skipped`), `files`.

**Exit status:** 1 whenever the report holds an error or a render failed, `--force` included.

**Examples:**

```bash
# Headers are field names: the spec is $quill and output
quillmark merge ./certificate certs.yaml attendees.csv --out ./certs

# Check the batch without rendering
quillmark merge ./certificate certs.yaml attendees.csv --dry-run

# Machine-readable report for CI
quillmark merge ./certificate certs.yaml attendees.csv --dry-run --json

# Render what is clean, report the rest, four threads
quillmark merge ./invoice invoices.yaml lines.tsv --out ./invoices --force --jobs 4
```

## Exit Codes

- `0`: success
- `1`: error (invalid arguments, file not found, parse error, compilation error, etc.)
