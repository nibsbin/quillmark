# quillmark-acroform

AcroForm backend for Quillmark that fills PDF form fields with templated values.

## Overview

This backend reads PDF forms from a quill's `form.pdf` file, renders field values
using MiniJinja templates, and returns filled PDFs.

## Quill Structure

```
my_form_quill/
├── Quill.toml
└── form.pdf
```

The `Quill.toml` must specify `backend = "acroform"`:

```toml
[Quill]
name = "my_form"
backend = "acroform"
```

## Template Sources

Fields can be templated in two ways:

1. **Tooltip metadata**: `description__{{template}}` - Extracts template after `__` separator
2. **Field value**: Uses the current field value as a template if no tooltip template exists

## License

Apache-2.0
