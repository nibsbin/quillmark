# Quickstart

Get started with Quillmark in your preferred language.

=== "Python"

    ## Installation

    Install using `uv` (recommended):

    ```bash
    uv pip install quillmark
    ```

    Or using `pip`:

    ```bash
    pip install quillmark
    ```

    ## Basic Usage

    ```python
    from quillmark import Quillmark, ParsedDocument, OutputFormat, Quill

    # Create engine
    engine = Quillmark()

    # Load a quill template
    quill = Quill.from_path("path/to/quill")
    engine.register_quill(quill)

    # Parse markdown
    markdown = """---
    title: Example Document
    ---

    # Hello World

    This is a simple example.
    """
    parsed = ParsedDocument.from_markdown(markdown)

    # Create workflow and render
    workflow = engine.workflow("quill_name")
    result = workflow.render(parsed, OutputFormat.PDF)

    # Access the generated PDF
    pdf_bytes = result.artifacts[0].bytes
    with open("output.pdf", "wb") as f:
        f.write(pdf_bytes)
    ```

=== "Rust"

    ## Installation

    Add Quillmark to your `Cargo.toml`:

    ```bash
    cargo add quillmark
    ```

    Or add it manually:

    ```toml
    [dependencies]
    quillmark = "0.6"
    quillmark-core = "0.6"
    ```

    ## Basic Usage

    ```rust
    use quillmark::{Quillmark, OutputFormat, ParsedDocument};
    use quillmark_core::Quill;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        // Create engine
        let mut engine = Quillmark::new();

        // Load a quill template
        let quill = Quill::from_path("path/to/quill")?;
        engine.register_quill(quill);

        // Parse markdown
        let markdown = r#"---
    title: Example Document
    ---

    # Hello World

    This is a simple example.
    "#;
        let parsed = ParsedDocument::from_markdown(markdown)?;

        // Create workflow and render
        let workflow = engine.workflow("quill_name")?;
        let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;

        // Access the generated PDF
        let pdf_bytes = &result.artifacts[0].bytes;
        std::fs::write("output.pdf", pdf_bytes)?;

        Ok(())
    }
    ```

=== "JavaScript"

    ## Installation

    Install using `npm`:

    ```bash
    npm install @quillmark-test/wasm
    ```

    Or using `yarn`:

    ```bash
    yarn add @quillmark-test/wasm
    ```

    ## Basic Usage

    ```javascript
    import { Quillmark, ParsedDocument, OutputFormat } from '@quillmark-test/wasm';

    // Create engine
    const engine = new Quillmark();

    // Load a quill template (as JSON)
    const quillJson = {
        files: {
            "Quill.toml": {
                contents: `[Quill]
name = "my-quill"
backend = "typst"
description = "My template"
`
            },
            "plate.typ": { contents: "..." },
            // ... other files
        }
    };
    engine.registerQuill(JSON.stringify(quillJson));

    // Parse markdown
    const markdown = `---
    title: Example Document
    ---

    # Hello World

    This is a simple example.
    `;
    const parsed = ParsedDocument.from_markdown(markdown);

    // Create workflow and render
    const workflow = engine.workflow("my-quill");
    const result = workflow.render(parsed, OutputFormat.PDF);

    // Access the generated PDF
    const pdfBytes = result.artifacts[0].bytes;
    ```

## Next Steps

- Learn about [core concepts](concepts.md) in Quillmark
- Explore how to [create your own Quills](../guides/creating-quills.md)
- Check out the [Python API reference](../python/api.md)
- Read the [Rust API documentation](https://docs.rs/quillmark/latest/quillmark/)
