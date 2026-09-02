# CLI

> **Package**: `quillmark-cli` → binary `quillmark`
> **Implementation**: `crates/bindings/cli/`

## TL;DR

`quillmark-cli` is a `clap` surface over the engine holding no logic of its own:
`render` turns a quill + markdown into PDF/SVG/PNG, `merge` turns a quill + a
merge spec + rows into one artifact per document, and
`schema`/`blueprint`/`validate`/`info` introspect a quill without rendering it.
Commands, options, and examples are the
[CLI reference](../../docs/cli/reference.md); this page is the contract behind
them.

## Contract

- **Only `render` and `merge` need the engine.** They construct `Quillmark` to
  resolve the quill's backend; the four introspection verbs load the quill with
  `quillmark::quill_from_path` and read the pure config-read operations a
  `Quill` already carries ([QUILL.md](QUILL.md)).
- **`merge` is plan first, always.** The interpreter ([MERGE.md](MERGE.md))
  plans the whole input and the report prints before anything renders;
  `--dry-run` stops there, a clean plan renders, `--force` renders the clean
  documents and reports the rest. The CLI owns only the edges: CSV/TSV parsing
  (cells as strings, a header row, a stripped BOM), JSON rows or `documents`,
  the report's presentation (spreadsheet row numbering for a tabular input,
  warnings collapsed per code and anchor, `--json` verbatim), the render loop
  (one live session per rayon worker, `update` per document), and
  `manifest.json` beside the artifacts. Exit 1 whenever the report holds an
  error or a render failed.
- **Seeded fallback.** `render` with no `MARKDOWN_FILE` renders the quill's
  seeded document: each field's `example:`, with `default:`/blank interpolated,
  so a quill renders with no input file. Output defaults to
  `example.{format}`.
- **Parsing is not relaxed for the CLI.** A `MARKDOWN_FILE` needs a root bare
  `~~~` block (`~~~card-yaml` is also accepted) carrying a `$quill` line,
  exactly as every other surface requires.
- **Both backends by default.** The binary inherits `quillmark`'s default
  features, `typst` and `pdfform`.
- **Every artifact reaches disk.** `svg` and `png` render one artifact per page,
  and a multi-page render writes `out-1.svg`, `out-2.svg`, …. `--stdout` carries
  one artifact and refuses such a render.
