# Document Validation

Validate documents without full compilation for faster feedback and multi-stage processing.

## Overview

Quillmark provides validation methods for different stages:

- **`validate_schema()`**: Check document fields against quill schema
- **`dry_run()`**: Validate template composition without compilation
- **`process_plate()`**: Process template and return composed content

## Schema Validation

Check if document fields match the quill's schema requirements.

```python
from quillmark import Quillmark, ParsedDocument, QuillmarkError

engine = Quillmark()
workflow = engine.workflow("my-quill")

markdown = """---
title: My Document
author: Alice
---
# Content
"""

parsed = ParsedDocument.from_markdown(markdown)

# Validate schema only
try:
    workflow.validate_schema(parsed)
    print("✓ Schema valid")
except QuillmarkError as e:
    print(f"✗ Schema error: {e}")
```

**Use cases:**
- Form validation before rendering
- Quick feedback in editors
- Batch document validation

## Dry Run Validation

Validate template composition without backend compilation.

```python
# Validate template processing
try:
    workflow.dry_run(parsed)
    print("✓ Template valid")
except TemplateError as e:
    print(f"✗ Template error: {e}")
```

**Use cases:**
- Faster validation than full rendering
- Check template syntax before expensive compilation
- LLM-driven document generation pipelines

## Template Processing

Process the plate template and return composed content without compilation.

```python
# Get processed template content
content = workflow.process_plate(parsed)
print(content)  # Backend-specific template (Typst, etc.)
```

**Use cases:**
- Debugging template composition
- Two-stage rendering pipelines
- Custom post-processing

## Validation Workflow

Combine validation methods for efficient pipelines:

```python
from quillmark import (
    Quillmark,
    ParsedDocument,
    OutputFormat,
    QuillmarkError,
    TemplateError
)

def validate_and_render(markdown: str, quill_name: str):
    engine = Quillmark()
    workflow = engine.workflow(quill_name)
    parsed = ParsedDocument.from_markdown(markdown)

    # Stage 1: Schema validation (fastest)
    try:
        workflow.validate_schema(parsed)
    except QuillmarkError as e:
        return {"error": f"Invalid schema: {e}", "stage": "schema"}

    # Stage 2: Template validation (fast)
    try:
        workflow.dry_run(parsed)
    except TemplateError as e:
        return {"error": f"Invalid template: {e}", "stage": "template"}

    # Stage 3: Full render (slowest)
    try:
        result = workflow.render(parsed, OutputFormat.PDF)
        return {"success": True, "result": result}
    except Exception as e:
        return {"error": str(e), "stage": "compilation"}

# Usage
outcome = validate_and_render(markdown, "invoice")
if "error" in outcome:
    print(f"Failed at {outcome['stage']}: {outcome['error']}")
else:
    outcome['result'].artifacts[0].save("output.pdf")
```

## Performance Comparison

| Method | Speed | Checks |
|--------|-------|--------|
| `validate_schema()` | Fastest | Field types and requirements |
| `dry_run()` | Fast | Template syntax and composition |
| `process_plate()` | Fast | Template processing only |
| `render()` | Slowest | Full compilation and rendering |

## LLM Document Generation

Use validation for fast iteration with language models:

```python
def generate_document_with_llm(prompt: str, quill_name: str):
    """Generate document using LLM with validation loop."""
    engine = Quillmark()
    workflow = engine.workflow(quill_name)

    for attempt in range(3):
        # Get markdown from LLM
        markdown = call_llm(prompt)
        parsed = ParsedDocument.from_markdown(markdown)

        # Fast validation
        try:
            workflow.validate_schema(parsed)
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
from quillmark import QuillmarkError, TemplateError, ParseError

try:
    workflow.validate_schema(parsed)
except QuillmarkError as e:
    print(f"Schema validation failed: {e}")

try:
    workflow.dry_run(parsed)
except TemplateError as e:
    print(f"Template validation failed: {e}")
    # e may contain diagnostic info
```

## Best Practices

1. **Validate early**: Use `validate_schema()` first for quick feedback
2. **Stage validation**: Run cheap checks before expensive operations
3. **Debug with process_plate()**: Inspect composed templates when debugging
4. **Cache workflows**: Reuse workflow instances for batch validation
5. **Handle errors gracefully**: Provide clear feedback on validation failures
