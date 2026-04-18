# Dynamic Assets and Fonts

Add images, data files, and fonts to your workflow at runtime.

## Overview

Dynamic assets let you inject content that isn't bundled with your quill:

- **Assets**: Images, data files, or any binary content referenced in formats
- **Fonts**: Custom font files for typography

This is useful for:
- User-uploaded images
- Database-sourced logos or signatures
- Dynamically generated charts
- Custom branding per-tenant

## Adding Assets

### Single Asset

=== "Python"

    ```python
    from quillmark import Quillmark, Quill, ParsedDocument

    engine = Quillmark()
    quill = Quill.from_path("./my-quill")
    workflow = engine.workflow(quill)

    with open("logo.png", "rb") as f:
        workflow.add_asset("logo.png", f.read())

    parsed = ParsedDocument.from_markdown(markdown)
    result = workflow.render(parsed)
    ```

=== "JavaScript"

    ```javascript
    import { Quill, Quillmark } from "@quillmark-test/wasm";

    const engine = new Quillmark();
    const quill = Quill.fromJson(quillBundle);
    engine.registerQuill(quill);

    const logoBytes = await fetch("logo.png").then((r) => r.arrayBuffer());
    const parsed = Quillmark.parseMarkdown(markdown);
    const result = engine.render(parsed, {
      format: "pdf",
      assets: {
        "logo.png": new Uint8Array(logoBytes)
      }
    });
    ```

### Multiple Assets

```python
assets = []
for filename in ["logo.png", "signature.jpg", "chart.svg"]:
    with open(filename, "rb") as f:
        assets.append((filename, f.read()))

workflow.add_assets(assets)
```

## Adding Fonts

=== "Python"

    ```python
    with open("CustomFont-Bold.ttf", "rb") as f:
        workflow.add_font("CustomFont-Bold.ttf", f.read())

    fonts = [
        ("CustomFont-Regular.ttf", regular_data),
        ("CustomFont-Bold.ttf", bold_data),
    ]
    workflow.add_fonts(fonts)
    ```

=== "JavaScript"

    ```javascript
    const result = engine.render(parsed, {
      format: "pdf",
      fonts: {
        "CustomFont-Regular.ttf": regularFontBytes,
        "CustomFont-Bold.ttf": boldFontBytes
      }
    });
    ```

## Using in Formats

### Typst Formats

Reference assets by name:

```typst
#image("logo.png", width: 100pt)

#set text(font: "CustomFont")
```

## Checking Dynamic Assets

```python
# List dynamically added assets
print(workflow.dynamic_asset_names())  # ['logo.png', 'signature.jpg']

# List dynamically added fonts
print(workflow.dynamic_font_names())  # ['CustomFont-Bold.ttf']
```

## Complete Example

```python
from quillmark import Quillmark, Quill, ParsedDocument, OutputFormat

def render_invoice(customer_name: str, logo_path: str):
    # Setup
    engine = Quillmark()
    quill = Quill.from_path("./invoice")
    workflow = engine.workflow(quill)

    # Add customer logo dynamically (accessible as assets/DYNAMIC_ASSET__customer-logo.png)
    with open(logo_path, "rb") as f:
        workflow.add_asset("customer-logo.png", f.read())

    # Prepare markdown
    markdown = f"""---
    title: Invoice
    customer: {customer_name}
    ---

    # Invoice
    """

    # Render
    parsed = ParsedDocument.from_markdown(markdown)
    result = workflow.render(parsed, OutputFormat.PDF)
    result.artifacts[0].save("invoice.pdf")

# Usage
render_invoice("Acme Corp", "acme-logo.png")
render_invoice("TechStart Inc", "techstart-logo.png")
```

## Notes

- Assets must be added before calling `render()`
- Asset names should match references in your format
- Fonts must be in TTF or OTF format
- Dynamic assets don't persist between workflow instances
