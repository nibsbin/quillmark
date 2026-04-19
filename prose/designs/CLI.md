# CLI

> **Status**: Implemented
> **Package**: `quillmark-cli` → binary `quillmark`
> **Implementation**: `crates/bindings/cli/src/`

## Commands

### `render`

```
quillmark render [OPTIONS] <QUILL_PATH> [MARKDOWN_FILE]
```

Options:
- `-o, --output <FILE>` — output file path (default: derived from input filename)
- `-f, --format <FORMAT>` — `pdf`, `svg`, `png`, or `txt` (default: `pdf`)
- `--stdout` — write output to stdout
- `--output-data <FILE>` — write compiled JSON data to file
- `--verbose` — detailed processing output
- `--quiet` — suppress non-error output

### `schema`

```
quillmark schema <QUILL_PATH> [-o <FILE>]
```

Outputs the Quill's public schema contract as YAML to stdout or file.

### `validate`

```
quillmark validate <QUILL_PATH>
```

Validates quill configuration.

### `info`

```
quillmark info <QUILL_PATH> [--json]
```

Displays quill metadata (name, version, description, backend, field/card counts).

## Project Structure

```
crates/bindings/cli/src/
├── main.rs
├── commands/
│   ├── mod.rs
│   ├── info.rs
│   ├── render.rs
│   ├── schema.rs
│   └── validate.rs
├── output.rs
└── errors.rs
```

## Dependencies

- `clap` — argument parsing
- `quillmark` — rendering engine
- `quillmark-core` — types
- `anyhow` — error handling
- `serde_json` — JSON output
