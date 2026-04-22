# Integration Overview

Use this page to understand the shared Quillmark flow before diving into language-specific API references.

## Core Workflow

Most integrations follow this flow:

1. **Load** a render-ready quill (backend attached).
2. **Parse** markdown into a `ParsedDocument`.
3. **Render** to the target output format.

=== "Python"

    ```python
    from quillmark import Quillmark, ParsedDocument, OutputFormat

    engine = Quillmark()
    quill = engine.quill_from_path("path/to/my-quill")

    parsed = ParsedDocument.from_markdown(markdown_text)
    result = quill.render(parsed, OutputFormat.PDF)
    ```

=== "JavaScript"

    ```javascript
    import { ParsedDocument, Quillmark } from "@quillmark-test/wasm";

    const engine = new Quillmark();
    const quill = engine.quill(tree);

    const parsed = ParsedDocument.fromMarkdown(markdownText);
    const result = quill.render(parsed, { format: "pdf" });
    ```

## Output Formats

Quillmark can produce one or more artifacts depending on backend + format:

- `pdf` for documents and print workflows
- `svg` for vector output
- `png` for raster output (use `render_with_options` / `ppi` to control resolution)
- `txt` for plain-text output

## Error Handling Philosophy

Quillmark returns structured diagnostics with source context so parse, validation, and render failures are actionable.

Recommended pattern:

1. Validate early (`ParsedDocument.from_markdown`, `quill.dry_run`).
2. Surface diagnostics directly to users/authoring UIs.
3. Fail fast on render errors in automated jobs.

## Where to Go Next

- [Python API Reference](python/api.md)
- [JavaScript/WASM API Reference](javascript/api.md)
- [Validation](validation.md)
