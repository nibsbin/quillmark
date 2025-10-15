# quillmark-acroform

This is an optional backend for `quillmark` that maps markdown/YAML context to PDF AcroForms.

## Quill

The backend handles Quills with the following structure:

example_quill/
- Quill.toml
- form.pdf # we will hardcode this file name for now
- usaf_form_8.md

Quill.toml uses the default JSON glue, so it does not have a glue file.
```
[Quill]
name = "usaf_form_8"
backend = "acroform"
example = "usaf_form_8.md"
description = "Certificate of aircrew qualification"
```

## Functionality

Instead of relying on quillmark-core’s templating, quillmark-acroform will use the workspace’s minijinja dependency to impute values into form fields that have jinja-style templating expressions. Use this crate to accomplish PDF field retrieval and manipulation:https://crates.io/crates/acroform

## Compilation workflow

1. Use acroform to read the PDF form and extract field names/values
2. For each field, use minijinja to render the field value with the parsed document context
3. Use acroform to write the rendered field values back to an output PDF form
4. Return the output PDF form as a byte vector

## Considerations

- For your backend compile implementation, expect a JSON string for glue_content input.
- If you need any dependencies like `serde_json`, pin them in the workspace `Cargo.toml`.

## Resources

- See the `quillmark-typst` crate for an example of backend implementation.
- Use the `quillmark-fixtures/resources/usaf_form_8` as a test Quill.
