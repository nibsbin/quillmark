# CLI

> **Package**: `quillmark-cli` → binary `quillmark`
> **Implementation**: `crates/bindings/cli/`

## TL;DR

`quillmark-cli` is a `clap` surface over the engine holding no logic of its own:
`render` turns a quill + markdown into PDF/SVG/PNG, and
`schema`/`blueprint`/`validate`/`info` introspect a quill without rendering it.
Commands, options, and examples are the
[CLI reference](../../docs/cli/reference.md); this page is the contract behind
them.

## Contract

- **Only `render` needs the engine.** It constructs `Quillmark` to resolve the
  quill's backend; the four introspection verbs load the quill with
  `quillmark::quill_from_path` and read the pure config-read operations a
  `Quill` already carries ([QUILL.md](QUILL.md)).
- **Seeded fallback.** `render` with no `MARKDOWN_FILE` renders the quill's
  seeded document: each field's `example:`, with `default:`/zero interpolated
, so a quill renders with no input file. Output defaults to
  `example.{format}`.
- **Parsing is not relaxed for the CLI.** A `MARKDOWN_FILE` needs a root bare
  `~~~` block (`~~~card-yaml` is also accepted) carrying a `$quill` line,
  exactly as every other surface requires.
- **Both backends by default.** The binary inherits `quillmark`'s default
  features, `typst` and `pdfform`.
