# QuillMark Typst Backend Example

This directory contains an example demonstrating the markdown-to-Typst conversion functionality.

## Files

- `sample.md` - Example markdown file with various formatting elements
- `sample_converted.typ` - Generated Typst output from the conversion
- `README.md` - This file

## Running the Example

### Using the demo example

From the `quillmark-typst` directory:

```bash
# Use default files (sample.md -> out/sample_output.typ)
cargo run --example demo

# Specify input file (output will be out/input_filename.typ)
cargo run --example demo -- ../examples/sample.md

# Specify both input and output files (output will be in out/ directory)
cargo run --example demo -- ../examples/sample.md custom_output.typ
```

All output files are automatically placed in the `out/` directory.

### Testing the conversion interactively

1. Edit the `sample.md` file to add or modify markdown content
2. Run the demo tool to see the converted Typst output
3. Review the generated `.typ` file to see how markdown elements are converted

## Supported Markdown Features

The conversion supports:

- **Text formatting**: *emphasis*, **strong**, ~~strikethrough~~, `inline code`
- **Links**: `[text](url)` becomes `#link("url")[text]`
- **Lists**: Both bullet lists (- item) and numbered lists (1. item) with nesting
- **Paragraphs**: Proper paragraph separation
- **Text escaping**: Typst special characters are properly escaped

## Example Conversion

**Markdown input:**
```markdown
This is **bold** and *italic* text with a [link](https://example.com).

- First item
  - Nested item
- Second item
```

**Typst output:**
```typst
This is *bold* and _italic_ text with a #link("https://example.com")[link].

+ First item
  + Nested item
+ Second item
```

## Development

You can use this example to:

1. Test modifications to the conversion logic
2. Verify that new markdown features are converted correctly
3. Ensure that Typst special characters are properly escaped
4. Check that the output is valid Typst syntax