# Typst Backend

The Typst backend generates professional PDF and SVG documents using the [Typst](https://typst.app/) typesetting system.

## Overview

Typst is a modern typesetting system designed as a better alternative to LaTeX. The Quillmark Typst backend:

- Converts Markdown to Typst markup
- Compiles Typst code to PDF or SVG
- Supports dynamic package loading
- Handles fonts and assets automatically
- Provides rich template filters

## Basic Usage

Specify `backend = "typst"` in your `Quill.toml`:

```toml
[Quill]
name = "my-typst-quill"
backend = "typst"
description = "Document template using Typst"
plate_file = "plate.typ"

[typst]
packages = ["@preview/appreciated-letter:0.1.0"]
```

## Plate Templates

Typst plate templates use MiniJinja syntax to generate Typst code:

```jinja
#import "@preview/appreciated-letter:0.1.0": letter

#show: letter.with(
  sender: {{ sender | String }},
  recipient: {{ recipient | String }},
  date: {{ date | String }},
)

#{{ BODY | Content }}
```

## Template Filters

The Typst backend provides specialized filters to convert data into Typst format:

### String Filter

Converts values to Typst string literals:

```jinja
#let title = {{ title | String }}
#let author = {{ author | String }}
```

Input:
```yaml
title: My Document
author: John Doe
```

Output (Typst):
```typst
#let title = "My Document"
#let author = "John Doe"
```

### Lines Filter

Converts arrays to Typst arrays:

```jinja
#let authors = {{ authors | Lines }}
```

Input:
```yaml
authors:
  - Alice
  - Bob
  - Charlie
```

Output (Typst):
```typst
#let authors = ("Alice", "Bob", "Charlie")
```

### Date Filter

Converts date strings to Typst datetime objects:

```jinja
#let doc_date = {{ date | Date }}
```

Input:
```yaml
date: 2025-01-15
```

Output (Typst):
```typst
#let doc_date = datetime(year: 2025, month: 1, day: 15)
```

### Dict Filter

Converts objects to Typst dictionaries:

```jinja
#let metadata = {{ frontmatter | Dict }}
```

Input:
```yaml
title: My Doc
author: Alice
version: 1.0
```

Output (Typst):
```typst
#let metadata = (
  title: "My Doc",
  author: "Alice",
  version: 1.0
)
```

### Content Filter

Converts Markdown body to Typst markup:

```jinja
#{{ BODY | Content }}
```

Input (Markdown):
```markdown
# Introduction

This is **bold** and this is *italic*.

- List item 1
- List item 2
```

Output (Typst):
```typst
= Introduction

This is *bold* and this is _italic_.

- List item 1
- List item 2
```

### Asset Filter

References asset files from the Quill bundle:

```jinja
#image({{ "assets/logo.png" | Asset }})
```

Output (Typst):
```typst
#image("/path/to/quill/assets/logo.png")
```

## Typst Packages

Typst packages extend functionality with pre-built templates and utilities. Specify packages in `Quill.toml`:

```toml
[typst]
packages = [
    "@preview/appreciated-letter:0.1.0",
    "@preview/bubble:0.2.2",
    "@preview/fontawesome:0.5.0"
]
```

Then import and use them in your plate template:

```jinja
#import "@preview/appreciated-letter:0.1.0": letter
#import "@preview/fontawesome:0.5.0": fa-icon

#show: letter.with(...)

#fa-icon("envelope") Contact: info@example.com
```

### Popular Packages

- **appreciated-letter** - Professional letter templates
- **bubble** - Speech bubble and callout boxes
- **fontawesome** - Font Awesome icons
- **tablex** - Advanced table layouts
- **cetz** - Drawing and diagrams

Browse packages at [Typst Universe](https://typst.app/universe/).

## Fonts

### System Fonts

Typst can use system-installed fonts:

```typst
#set text(font: "Arial")
```

### Custom Fonts

Include custom fonts in your Quill's `assets/fonts/` directory:

```
my-quill/
└── assets/
    └── fonts/
        ├── CustomFont-Regular.ttf
        └── CustomFont-Bold.ttf
```

Reference them in your plate:

```jinja
#set text(font: "CustomFont")
```

## Output Formats

The Typst backend supports multiple output formats:

### PDF

```python
from quillmark import OutputFormat

result = workflow.render(parsed, OutputFormat.PDF)
pdf_bytes = result.artifacts[0].bytes
```

### SVG

```python
result = workflow.render(parsed, OutputFormat.SVG)
svg_bytes = result.artifacts[0].bytes
```

SVG output is useful for web applications and scalable graphics.

## Advanced Features

### Page Setup

Control page size, margins, and orientation:

```typst
#set page(
  paper: "us-letter",
  margin: (x: 1in, y: 1in),
  numbering: "1",
)
```

### Text Styling

Apply global text styles:

```typst
#set text(
  font: "Linux Libertine",
  size: 11pt,
  lang: "en",
)
```

### Paragraph Settings

Configure paragraph spacing and alignment:

```typst
#set par(
  justify: true,
  leading: 0.65em,
  first-line-indent: 1.8em,
)
```

### Custom Functions

Define reusable Typst functions:

```jinja
#let highlight(content) = {
  rect(fill: yellow, inset: 8pt)[#content]
}

#highlight[Important information]
```

## Error Handling

The Typst backend provides detailed error diagnostics:

```
Compilation error at line 12, column 5:
  unknown function: `invalidFunc`
```

Errors include:
- **Syntax errors** - Invalid Typst syntax
- **Type errors** - Type mismatches in function calls
- **Package errors** - Missing or incompatible packages
- **Resource errors** - Missing fonts or assets

## Examples

### Simple Letter

```jinja
#set page(margin: 1in)
#set text(font: "Arial", size: 11pt)

{{ date | String }}

{{ recipient | String }}

Dear {{ recipient | String }},

#{{ BODY | Content }}

Sincerely,

{{ sender | String }}
```

### Academic Paper

```jinja
#set page(paper: "a4", margin: 1in)
#set text(font: "Linux Libertine", size: 12pt)
#set par(justify: true)

#align(center)[
  #text(size: 18pt, weight: "bold")[{{ title | String }}]
  
  #text(size: 12pt)[{{ author | String }}]
]

#{{ BODY | Content }}
```

## Best Practices

1. **Test incrementally** - Build your template step-by-step
2. **Use packages** - Leverage existing Typst packages when possible
3. **Separate concerns** - Keep complex logic in Typst, data in frontmatter
4. **Validate inputs** - Define field schemas in `Quill.toml`
5. **Handle missing fields** - Use defaults for optional fields

## Resources

- [Typst Documentation](https://typst.app/docs/)
- [Typst Tutorial](https://typst.app/docs/tutorial/)
- [Typst Universe](https://typst.app/universe/) - Package directory
- [Typst Discord](https://discord.gg/2uDybryKPe) - Community support

## Next Steps

- [Create your own Typst Quill](creating-quills.md)
- [Learn about Quill Markdown](quill-markdown.md)
- [Explore the AcroForm backend](acroform-backend.md)
