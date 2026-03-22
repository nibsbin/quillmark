# CLI Tool Design for Quillmark

Status: **Implemented** (2026-03-22)  
Package: `quillmark-cli` (`crates/bindings/cli`)

## Commands
- `render [OPTIONS] <MARKDOWN_FILE?>`
  - `-q, --quill <PATH>`: quill directory (required when QUILL is missing/`__default__`).
  - `-o, --output <FILE>`: output path (defaults from input stem + extension).
  - `-f, --format <pdf|svg|png|txt>`: default `pdf`.
  - `--stdout`: write artifact bytes to stdout.
  - `--output-data <FILE>`: write `compile_data` JSON (post-coercion/validation/transform/defaults).
  - `-v/--verbose`, `--quiet`.
  - If `MARKDOWN_FILE` is omitted, `--quill` is required and the quill’s `example` is used.
- `schema <QUILL_PATH> [-o <FILE>]`: emit JSON schema.
- `info <QUILL_PATH> [--json]`: display metadata, schema counts, defaults/examples.
- `validate <QUILL_PATH> [-v]`: parse `Quill.yaml`, verify files, schema, defaults.

## Behavior Notes
- Render fails early if quill path is missing, QUILL is absent and no `--quill` is provided, or format is unsupported by the backend.
- Output format options map to core `OutputFormat` (Typst: pdf/svg/png/txt; AcroForm: pdf).
- Exit code `1` on any error; diagnostics printed via structured CLI formatter.

## Structure
`src/main.rs` wires subcommands; implementations live in `src/commands/{render,schema,validate,info}.rs`. Output path derivation in `src/output.rs`; shared error handling in `src/errors.rs`.
