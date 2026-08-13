# quillmark-fixtures

Sample Quill templates and markdown files backing [Quillmark](https://github.com/borb-sh/quillmark)'s tests and examples, plus the helpers that resolve their paths.

## Usage

```rust
let sample_md = quillmark_fixtures::resource_path("sample.md");

// Resolves to the latest version directory
let usaf_memo = quillmark_fixtures::quills_path("usaf_memo");
```

## Resources

- **Quill templates** under `resources/quills/<name>/<version>/`, each with a `Quill.yaml` and either a Typst `plate.typ` or a PDF-form template. `quills_path` resolves the latest version.
  - Typst backend: `usaf_memo`, `taro`, `classic_resume`, `cmu_letter`, `table_demo`
  - `pdfform` backend: `sample_form`, `richtext_form`

- **Sample markdown** under `resources/`
  - `sample.md` - markdown constructs only, no card-yaml block
  - `card_yaml_demo.md` - a card-yaml document
  - `extended_metadata_demo.md` - composable cards under one main card
  - `ambiguous_strings.md` - field values YAML would otherwise coerce away from strings

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../../LICENSE) for details.
