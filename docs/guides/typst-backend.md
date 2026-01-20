# Typst Backend

The Typst backend generates professional PDF and SVG documents using the [Typst](https://typst.app/) typesetting system.

## Overview

Typst is a modern typesetting system designed as a better alternative to LaTeX. The Quillmark Typst backend:

- Converts Markdown to Typst markup via backend `transform_fields`
- Compiles Typst code to PDF or SVG
- Supports dynamic package loading
- Handles fonts and assets automatically (including dynamic assets/fonts)
- Provides JSON data injection via helper package

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

Typst plate templates are pure Typst code that access document data via a helper package:

```typst
#import "@local/quillmark-helper:0.1.0": data, eval-markup

#set document(title: data.at("title", default: "Untitled"))

#eval-markup(data.at("body", default: ""))
```

## Data Access

Quillmark injects your document's frontmatter as JSON data via the `@local/quillmark-helper` virtual package.

### Importing the Helper

```typst
#import "@local/quillmark-helper:0.1.0": data, eval-markup, parse-date
```

The helper provides:
- `data` - Dictionary containing all frontmatter fields
- `eval-markup(content)` - Render Markdown content as Typst markup
- `parse-date(str)` - Parse date strings into Typst datetime objects

### Accessing Fields

Access frontmatter fields directly from the `data` dictionary:

```typst
// Direct access (may error if field missing)
#data.title
#data.author

// Safe access with defaults (recommended)
#data.at("title", default: "Untitled")
#data.at("author", default: "Anonymous")
```

### Checking for Optional Fields

Use Typst's `in` operator to check for optional fields:

```typst
#if "subtitle" in data {
  [Subtitle: #data.subtitle]
}

// Or use spread syntax for function arguments
#show: template.with(
  title: data.title,
  ..if "subtitle" in data {
    (subtitle: data.subtitle,)
  } else {
    (:)
  },
)
```

### Rendering Body Content

The document body (Markdown content after frontmatter) is stored in `data.BODY` (and accessible via `data.body`). Use `eval-markup()` to render Typst markup produced by `transform_fields`:

```typst
#eval-markup(data.at("body", default: ""))
```

### Parsing Dates

Use `parse-date()` to convert date strings to Typst datetime objects:

```typst
// Input: date: "2025-01-15" in frontmatter
#parse-date(data.date)  // Returns datetime(year: 2025, month: 1, day: 15)
```

### Working with Arrays

Arrays from YAML frontmatter are accessible as Typst arrays:

```yaml
authors:
  - Alice
  - Bob
  - Charlie
```

```typst
#for author in data.authors {
  [- #author]
}
```

### Working with CARDS

If your document uses the CARDS feature, access them via `data.CARDS`:

```typst
#for card in data.at("CARDS", default: ()) {
  if card.CARD == "product" {
    [Product: #card.name - #eval-markup(card.BODY)]
  }
}
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

```typst
#import "@local/quillmark-helper:0.1.0": data, eval-markup
#import "@preview/appreciated-letter:0.1.0": letter
#import "@preview/fontawesome:0.5.0": fa-icon

#show: letter.with(
  sender: data.sender,
  recipient: data.recipient,
)

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

```typst
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

```typst
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

```typst
#import "@local/quillmark-helper:0.1.0": data, eval-markup, parse-date

#set page(margin: 1in)
#set text(font: "Arial", size: 11pt)

#parse-date(data.date).display("[month repr:long] [day], [year]")

#data.recipient

Dear #data.recipient,

#eval-markup(data.at("body", default: ""))

Sincerely,

#data.sender
```

### Academic Paper

```typst
#import "@local/quillmark-helper:0.1.0": data, eval-markup

#set page(paper: "a4", margin: 1in)
#set text(font: "Linux Libertine", size: 12pt)
#set par(justify: true)

#align(center)[
  #text(size: 18pt, weight: "bold")[#data.title]

  #text(size: 12pt)[#data.author]
]

#eval-markup(data.at("body", default: ""))
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
