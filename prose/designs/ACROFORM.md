# backends/quillmark-acroform

This is an optional backend for `quillmark` that maps markdown/YAML context to PDF AcroForms.

## Quill

The backend handles Quills with the following structure:

example_quill/
- Quill.toml
- form.pdf # we will hardcode this file name for now
- usaf_form_8.md

Quill.toml uses the default auto glue, so it does not have a glue file.
```
[Quill]
name = "usaf_form_8"
backend = "acroform"
example = "usaf_form_8.md"
description = "Certificate of aircrew qualification"
```

## Functionality

The backends/quillmark-acroform backend fills PDF form fields using MiniJinja templating. Fields are templated in two ways:

1. **Tooltip-based templating**: Extract template from field description/tooltip using `description__{{template}}` format
2. **Value-based templating**: Use the current field value as a template if no tooltip template exists

The backend uses the workspace's minijinja dependency to render templates with the glue JSON context. PDF manipulation is handled using the `acroform` crate: https://crates.io/crates/acroform

## Compilation

1. Use acroform to read the PDF form and extract field names/values
2. For each field, use minijinja to render the templated value with the glue JSON context
3. Use acroform to write the rendered field values back to an output PDF form
4. Return the output PDF form as a byte vector

## Considerations

- For the backend compile implementation, expect a JSON string for glue_content input.
- If you need any dependencies like `serde_json`, pin them in the workspace `Cargo.toml`.
- Do not register any filters because we are using the default auto glue

## Resources

- See the `backends/quillmark-typst` crate for an example of backend implementation.
- Use the `quillmark-fixtures/resources/usaf_form_8` as a test Quill.
