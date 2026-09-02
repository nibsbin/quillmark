# Quillmark CLI

`quillmark`, the command-line interface to [Quillmark](https://github.com/borb-sh/quillmark): renders Markdown with card-yaml blocks through a quill into PDF, SVG, or PNG.

Maintained by [TTQ](https://tonguetoquill.com).

## Installation

```bash
cargo install quillmark-cli
```

The binary lands at `~/.cargo/bin/quillmark`. To build from a checkout instead:

```bash
cargo install --path crates/bindings/cli
```

## Quick start

```bash
# Render a document; output defaults to document.pdf
quillmark render ./quills/usaf_memo document.md

# Omit the markdown file to render the quill's seeded document
quillmark render ./quills/usaf_memo -o preview.pdf

# Pipe the artifact instead of writing a file
quillmark render ./quills/usaf_memo document.md --stdout | evince -
```

## Commands

Every command takes a `<QUILL_PATH>` pointing at a quill directory.

### `quillmark render [OPTIONS] <QUILL_PATH> [MARKDOWN_FILE]`

Renders a document. With `MARKDOWN_FILE` omitted, the quill's seeded document is
rendered instead, so a quill previews without any authored input.

- `-o, --output <FILE>` — output path (default: the input filename with the format's extension)
- `-f, --format <FORMAT>` — `pdf` (default), `svg`, or `png`
- `--stdout` — write the artifact to stdout; all chatter moves to stderr
- `--output-data <DATA_FILE>` — also write the compiled JSON data handed to the backend
- `-v, --verbose` — progress detail on stderr
- `--quiet` — suppress non-error output

### `quillmark schema <QUILL_PATH> [-o <FILE>]`

Prints the quill's field schema as YAML.

### `quillmark blueprint <QUILL_PATH> [-o <FILE>]`

Prints an annotated Markdown blueprint: a starting document with every declared
field, `!must_fill` where a value is expected.

### `quillmark validate <QUILL_PATH> [-v]`

Checks the quill's configuration: `Quill.yaml` parse errors, `example:`/`default:`
literals against their declared types, and referenced files. `-v` adds advisory
warnings such as missing field descriptions. Exits 1 on any error.

### `quillmark merge <QUILL_PATH> <SPEC_FILE> <INPUT_FILE> --out <DIR>`

Generates one document per input row (or per group of rows) through a merge
spec and renders them all: mail merge over the quill's schema. Rows come from
`.csv` / `.tsv` / `.json`; the spec pins `$quill`, maps columns onto fields,
and patterns the output name. The whole batch is planned and reported before
anything renders.

- `--dry-run` — plan and report only
- `--force` — render the rows no error touches, report the rest
- `--json` — the report and manifest as one JSON object on stdout
- `-f, --format <FORMAT>` — `pdf` (default), `svg`, or `png`
- `--delimiter <CHAR>`, `--jobs <N>`, `--quiet`

### `quillmark info <QUILL_PATH> [--json]`

Prints quill metadata — name, version, author, backend, field and card counts.
`--json` emits the same as one JSON object.

## Exit codes

`0` on success, `1` on any error, with diagnostics on stderr.

## Links

- [CLI design document](../../../prose/canon/CLI.md)
- [Changelog](https://github.com/borb-sh/quillmark/blob/main/CHANGELOG.md) and [releases](https://github.com/borb-sh/quillmark/releases)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../../../LICENSE).
