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
    workflow = engine.workflow("my-quill")
    result = workflow.render(parsed, OutputFormat.PDF)

    # Access the generated PDF
    pdf_bytes = result.artifacts[0].bytes
    with open("output.pdf", \"wb\") as f:
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

        // Load quill
        let quill = Quill::from_path("path/to/quill")?;
        engine.register_quill(quill)?;

        // Parse markdown
        let markdown = r#"---
    title: Example Document
    ---

    # Hello World

    This is a simple example.
    "#;
        let parsed = ParsedDocument::from_markdown(markdown)?;

        // Render to PDF
        let workflow = engine.workflow("my-quill")?;
        let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;

        // Access PDF bytes
        std::fs::write("output.pdf", &result.artifacts[0].bytes)?;
        Ok(())
    }
    ```

=== "JavaScript"

    ## Installation

    ```bash
    npm install @quillmark-test/wasm
    ```

    ## Basic Usage

    ```javascript
    import { Quillmark } from "@quillmark-test/wasm";

    const engine = new Quillmark();

    // Register a quill (JSON)
    const quill = {
      files: {
        "Quill.yaml": { contents: "Quill:\n  name: my-quill\n  backend: typst\n  description: Demo\n" },
        "plate.typ": { contents: "#import \"@local/quillmark-helper:0.1.0\": data, eval-markup\n#eval-markup(data.BODY)\n" }
      }
    };
    engine.registerQuill(quill);

    const markdown = `---
    title: Example Document
    ---

    # Hello World
    `;

    const parsed = Quillmark.parseMarkdown(markdown);
    const result = engine.render(parsed, { format: "pdf" });

    const pdfBytes = result.artifacts[0].bytes;
    ```
