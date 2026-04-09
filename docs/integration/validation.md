# Document Validation

Validate documents without full compilation for faster feedback.

## Overview

Quillmark provides validation with language-appropriate APIs:

- **Python**: `dry_run()` validates inputs without backend compilation.
- **JavaScript**: use parse + render with error handling for validation feedback.

## Dry Run Validation

Validate parsing and schema without backend compilation:

=== "Python"

    ```python
    from quillmark import Quillmark, Quill, ParsedDocument, QuillmarkError

    engine = Quillmark()
    quill = Quill.from_path("./my-quill")
    workflow = engine.workflow(quill)

    parsed = ParsedDocument.from_markdown(markdown)

    try:
        workflow.dry_run(parsed)
        print("✓ Document valid")
    except QuillmarkError as e:
        print(f"✗ Validation error: {e}")
    ```

=== "JavaScript"

    ```javascript
    import { Quillmark } from "@quillmark-test/wasm";

    const engine = new Quillmark();
    engine.registerQuill(quillBundle);
    const parsed = Quillmark.parseMarkdown(markdown);

    // Rendering performs parse/schema/backend validation in the JS API
    const result = engine.render(parsed, { format: "pdf", quillName: "my-quill" });
    ```

**Use cases:**
- Fast feedback in editors
- Batch document validation
- LLM-driven document generation pipelines

## Validation Workflow

Use dry_run for efficient pipelines:

=== "Python"

    ```python
    from quillmark import Quillmark, Quill, ParsedDocument, OutputFormat, QuillmarkError

    def validate_and_render(markdown: str, quill_path: str):
        engine = Quillmark()
        quill = Quill.from_path(quill_path)
        workflow = engine.workflow(quill)
        parsed = ParsedDocument.from_markdown(markdown)

        try:
            workflow.dry_run(parsed)
        except QuillmarkError as e:
            return {"error": f"Validation failed: {e}", "stage": "validation"}

        try:
            result = workflow.render(parsed, OutputFormat.PDF)
            return {"success": True, "result": result}
        except Exception as e:
            return {"error": str(e), "stage": "compilation"}
    ```

=== "JavaScript"

    ```javascript
    function validateAndRender(engine, quillRef, markdown) {
      const parsed = Quillmark.parseMarkdown(markdown);

      return engine.render(parsed, { format: "pdf", quillName: quillRef });
    }
    ```

## Performance Comparison

| Method | Speed | Checks |
|--------|-------|--------|
| `dry_run()` | Fast | Parsing, schema validation |
| `render()` | Slower | Full compilation and rendering |

## LLM Document Generation

Use validation for fast iteration with language models:

```python
def generate_document_with_llm(prompt: str, quill_path: str):
    """Generate document using LLM with validation loop."""
    engine = Quillmark()
    quill = Quill.from_path(quill_path)
    workflow = engine.workflow(quill)

    for attempt in range(3):
        # Get markdown from LLM
        markdown = call_llm(prompt)
        parsed = ParsedDocument.from_markdown(markdown)

        # Fast validation
        try:
            workflow.dry_run(parsed)
            # Valid - proceed to render
            return workflow.render(parsed, OutputFormat.PDF)
        except Exception as e:
            # Invalid - retry with error feedback
            prompt = f"{prompt}\n\nPrevious error: {e}"

    raise Exception("Failed to generate valid document")
```

## Error Handling

```python
from quillmark import QuillmarkError, ParseError

try:
    parsed = ParsedDocument.from_markdown(markdown)
except ParseError as e:
    print(f"Parse error: {e}")

try:
    workflow.dry_run(parsed)
except QuillmarkError as e:
    print(f"Validation failed: {e}")
```

## Best Practices

1. **Validate early**: Use `dry_run()` for quick feedback before rendering
2. **Stage validation**: Run cheap checks before expensive operations
3. **Cache workflows**: Reuse workflow instances for batch validation
4. **Handle errors gracefully**: Provide clear feedback on validation failures
