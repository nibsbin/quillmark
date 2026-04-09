# Integration Overview

Use this page to understand the shared Quillmark workflow before diving into the language-specific API references.

## Core Workflow

Most integrations follow the same three-step flow:

1. **Parse** Markdown into a structured document.
2. **Register** a Quill template with the engine.
3. **Render** using a workflow and select an output format.

=== "Python"

    ```python
    from quillmark import Quillmark, ParsedDocument, Quill, OutputFormat

    engine = Quillmark()
    engine.register_quill(Quill.from_path("path/to/my-quill"))

    parsed = ParsedDocument.from_markdown(markdown_text)
    workflow = engine.workflow("my-quill")
    result = workflow.render(parsed, OutputFormat.PDF)
    ```

=== "JavaScript"

    ```javascript
    import { Quillmark } from "@quillmark-test/wasm";

    const engine = new Quillmark();
    engine.registerQuill(quillBundle);

    const parsed = Quillmark.parseMarkdown(markdownText);
    const result = engine.render(parsed, { format: "pdf" });
    ```

## Output Formats

Quillmark can produce one or more artifacts depending on the backend and format:

- `pdf` for document delivery and print workflows
- `svg` for scalable vector output
- `png` for raster previews and page images

For advanced format options (for example PNG `ppi`), see the backend and API reference pages.

## Error Handling Philosophy

Quillmark returns structured diagnostics with source context so validation and rendering failures are actionable.

Recommended pattern:

1. Validate documents early in your pipeline.
2. Surface parse/validation messages directly to the user or authoring UI.
3. Fail fast on render errors in automated or batch jobs.

## Common Integration Patterns

- **Template-driven rendering service**: keep Quills versioned and register at startup.
- **Authoring loop**: parse/validate on save, render only after validation passes.
- **Batch rendering**: process many documents with one initialized engine.

## Where to Go Next

- [Python API Reference](python/api.md)
- [JavaScript/WASM API Reference](javascript/api.md)
- [Dynamic Assets](dynamic-assets.md)
- [Validation](validation.md)
