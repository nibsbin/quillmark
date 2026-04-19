# Dynamic Assets and Fonts

Add images, data files, and fonts to your workflow at runtime.

## Overview

Dynamic assets let you inject content that isn't bundled with your quill:

- **Assets**: Images, data files, or any binary content referenced in formats
- **Fonts**: Custom font files for typography

Use a `Workflow` for this path.

## Python

```python
from quillmark import Quillmark, ParsedDocument, OutputFormat

engine = Quillmark()
quill = engine.quill_from_path("./my-quill")
workflow = engine.workflow(quill)

with open("logo.png", "rb") as f:
    workflow.add_asset("logo.png", f.read())

with open("CustomFont-Regular.ttf", "rb") as f:
    workflow.add_font("CustomFont-Regular.ttf", f.read())

parsed = ParsedDocument.from_markdown(markdown)
result = workflow.render(parsed, OutputFormat.PDF)
```

## Multiple Assets

```python
assets = []
for filename in ["logo.png", "signature.jpg", "chart.svg"]:
    with open(filename, "rb") as f:
        assets.append((filename, f.read()))

workflow.add_assets(assets)
```

## Inspecting Dynamic Inputs

```python
print(workflow.dynamic_asset_names())
print(workflow.dynamic_font_names())
```

## Notes

- Add assets/fonts before `render()`.
- Asset names must match references inside the format.
- Fonts should be TTF or OTF.
- Dynamic assets/fonts are scoped to a workflow instance.
- WASM direct `quill.render(...)` currently does not expose dynamic asset/font injection; use Python workflow path when runtime injection is required.
