# AcroForm Backend

> **Status**: Implemented
> **Implementation**: `crates/backends/quillmark-acroform/`

## Quill Structure

```
example_quill/
├── Quill.toml
├── form.pdf
└── example.md
```

```toml
[Quill]
name = "usaf_form_8"
backend = "acroform"
example = "usaf_form_8.md"
description = "Certificate of aircrew qualification"
```

No plate file; `plate_extension_types` is empty and plate content is ignored.

## Compilation

1. Read PDF form and extract field names/values via `acroform` crate
2. For each field, render the templated value with MiniJinja using injected JSON context
3. Write rendered values back to an output PDF form
4. Return output PDF as byte vector

## Templating

- **Tooltip-based**: extract template from field description/tooltip using `description__{{template}}` format
- **Value-based**: use the current field value as a template if no tooltip template exists

## Reference

- `backends/quillmark-typst` — reference backend implementation
- `quillmark-fixtures/resources/usaf_form_8` — test quill
