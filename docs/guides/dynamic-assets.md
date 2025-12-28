# Dynamic Assets and Fonts

Add images, data files, and fonts to your workflow at runtime.

## Overview

Dynamic assets let you inject content that isn't bundled with your quill:

- **Assets**: Images, data files, or any binary content referenced in templates
- **Fonts**: Custom font files for typography

This is useful for:
- User-uploaded images
- Database-sourced logos or signatures
- Dynamically generated charts
- Custom branding per-tenant

## Adding Assets

### Single Asset

```python
from quillmark import Quillmark, ParsedDocument

engine = Quillmark()
workflow = engine.workflow("my-quill")

# Load image data
with open("logo.png", "rb") as f:
    logo_data = f.read()

# Add to workflow
workflow.add_asset("logo.png", logo_data)

# Render (template can now reference logo.png)
parsed = ParsedDocument.from_markdown(markdown)
result = workflow.render(parsed)
```

### Multiple Assets

```python
assets = {}
for filename in ["logo.png", "signature.jpg", "chart.svg"]:
    with open(filename, "rb") as f:
        assets[filename] = f.read()

workflow.add_assets(assets)
```

## Adding Fonts

```python
# Load font file
with open("CustomFont-Bold.ttf", "rb") as f:
    font_data = f.read()

# Add to workflow
workflow.add_font("CustomFont-Bold.ttf", font_data)

# Or add multiple fonts
fonts = {
    "CustomFont-Regular.ttf": regular_data,
    "CustomFont-Bold.ttf": bold_data,
}
workflow.add_fonts(fonts)
```

## Using in Templates

### Typst Templates

Reference assets by name:

```typst
#image("logo.png", width: 100pt)

#set text(font: "CustomFont")
```

### AcroForm Templates

Use base64-encoded assets in form fields or annotations.

## Checking Dynamic Assets

```python
# List dynamically added assets
print(workflow.dynamic_asset_names())  # ['logo.png', 'signature.jpg']

# List dynamically added fonts
print(workflow.dynamic_font_names())  # ['CustomFont-Bold.ttf']
```

## Complete Example

```python
from quillmark import Quillmark, ParsedDocument, OutputFormat

def render_invoice(customer_name: str, logo_path: str):
    # Setup
    engine = Quillmark()
    workflow = engine.workflow("invoice")

    # Add customer logo dynamically
    with open(logo_path, "rb") as f:
        workflow.add_asset("customer-logo.png", f.read())

    # Prepare markdown
    markdown = f"""---
title: Invoice
customer: {customer_name}
---

# Invoice

Customer logo: {{{{ customer-logo.png }}}}
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
- Asset names should match references in your template
- Fonts must be in TTF or OTF format
- Dynamic assets don't persist between workflow instances
