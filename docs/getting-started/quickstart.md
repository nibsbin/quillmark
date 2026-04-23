# Quickstart

Get started with Quillmark in Python or JavaScript.

=== "Python"

    ## Installation

    ```bash
    uv pip install quillmark
    ```

    ## Basic Usage

    ```python
    from quillmark import Document, Quillmark, OutputFormat

    engine = Quillmark()
    quill = engine.quill_from_path("path/to/quill")

    markdown = """---
    QUILL: my_quill
    title: Example Document
    ---

    # Hello World
    """

    doc = Document.from_markdown(markdown)
    result = quill.render(doc, OutputFormat.PDF)

    with open("output.pdf", "wb") as f:
        f.write(result.artifacts[0].bytes)
    ```

=== "JavaScript"

    ## Installation

    ```bash
    npm install @quillmark-test/wasm
    ```

    ## Basic Usage

    ```javascript
    import { Document, Quillmark } from "@quillmark-test/wasm";

    const engine = new Quillmark();
    const enc = new TextEncoder();

    const quill = engine.quill(new Map([
      ["Quill.yaml", enc.encode("quill:\n  name: my_quill\n  backend: typst\n  plate_file: plate.typ\n  description: Demo\n")],
      ["plate.typ", enc.encode("#import \"@local/quillmark-helper:0.1.0\": data\n#data.BODY\n")],
    ]));

    const markdown = `---
    QUILL: my_quill
    title: Example Document
    ---

    # Hello World`;

    const doc = Document.fromMarkdown(markdown);
    const result = quill.render(doc, { format: "pdf" });
    const pdfBytes = result.artifacts[0].bytes;
    ```
