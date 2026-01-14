# Creating Quills

A Quill is a template bundle that defines how your Markdown content should be rendered. This guide will walk you through creating your own Quill.

## Quill Structure

A Quill is a directory containing:

```
my-quill/
├── Quill.toml          # Configuration and metadata
├── plate.typ           # Plate template (backend-specific)
├── example.md          # Optional example document
└── assets/             # Optional assets (fonts, images, etc.)
    ├── logo.png
    └── fonts/
        └── custom.ttf
```

## Quill.toml Configuration

The `Quill.toml` file defines your Quill's metadata and configuration:

```toml
[Quill]
name = "my-quill"
backend = "typst"
description = "A professional document template"
plate_file = "plate.typ"
example_file = "example.md"
version = "1.0.0"
author = "Your Name"

# Backend-specific configuration
[typst]
packages = ["@preview/appreciated-letter:0.1.0"]

# Field schemas for validation
[fields]
title = { description = "Document title", type = "str" }
author = { description = "Author name", type = "str", default = "Anonymous" }
date = { description = "Document date", type = "str" }
```

### Required Fields

- `name` - Unique identifier for your Quill
- `backend` - Backend to use (`"typst"` or `"acroform"`)
- `description` - Human-readable description

### Optional Fields

- `plate_file` - Path to plate template (defaults to auto-generated plate)
- `example_file` - Path to example markdown file
- `version` - Semantic version of your Quill
- `author` - Creator of the Quill

### Field Schemas

Define expected frontmatter fields in the `[fields]` section:

```toml
[fields]
title = { description = "Document title", type = "str" }
count = { description = "Number of items", type = "number", default = 10 }
enabled = { description = "Enable feature", type = "boolean", default = false }
```

Each field can specify:
- `title` - Short label for the field
- `description` - Detailed description of the field
- `type` - Data type
- `default` - Default value if not provided
- `examples` - Array of example values
- `required` - Whether the field must be present
- `enum` - Restrict string fields to specific values

Supported `type` values:
- `string` or `str`
- `number`
- `boolean`
- `array`
- `dict`
- `date`
- `datetime`

### UI Configuration

You can provide additional metadata for UI generators (like wizards or form builders) using the `ui` table within a field definition.

```toml
[fields.sender]
title = "Sender Name"
description = "The full name of the person sending the letter, including any titles or suffixes."
type = "string"

[fields.sender.ui]
group = "Sender Information"
```

Supported UI properties:
- `group` - Group name for organizing fields in the UI

## Plate Templates

Plate templates are pure backend-specific code (e.g., Typst) that access document data via a helper package.

### Basic Example (Typst)

```typst
#import "@local/quillmark-helper:0.1.0": data, eval-markup
#import "@preview/appreciated-letter:0.1.0": letter

#show: letter.with(
  sender: data.sender,
  recipient: data.recipient,
  date: data.date,
  subject: data.subject,
)

#eval-markup(data.at("body", default: ""))
```

### Data Access

Quillmark injects your document's frontmatter as JSON data via the `@local/quillmark-helper` virtual package:

```typst
#import "@local/quillmark-helper:0.1.0": data, eval-markup, parse-date
```

The helper provides:
- `data` - Dictionary containing all frontmatter fields
- `eval-markup(content)` - Render Markdown content as Typst markup
- `parse-date(str)` - Parse date strings into Typst datetime objects

### Accessing Frontmatter

Access YAML frontmatter fields from the `data` dictionary:

```yaml
---
title: My Document
author: John Doe
tags: ["important", "draft"]
---
```

```typst
Title: #data.title
Author: #data.author
Tags: #data.tags.join(", ")
```

Use `data.at()` for safe access with defaults:

```typst
#data.at("title", default: "Untitled")
#data.at("author", default: "Anonymous")
```

### Rendering Body Content

The document body is stored in `data.body`. Use `eval-markup()` to render it:

```typst
#eval-markup(data.at("body", default: ""))
```

### Working with Optional Fields

Check for optional fields using Typst's `in` operator:

```typst
#if "subtitle" in data {
  [Subtitle: #data.subtitle]
}
```

## Backend-Specific Configuration

### Typst Backend

Configure Typst packages and settings in the `[typst]` section:

```toml
[typst]
packages = [
    "@preview/appreciated-letter:0.1.0",
    "@preview/bubble:0.2.2"
]
```

See the [Typst Backend Guide](typst-backend.md) for more details.

### AcroForm Backend

> The AcroForm backend is experimental and currently not recommended for use.

## Assets and Resources

### Adding Fonts

Place font files in an `assets/fonts/` directory:

```
my-quill/
└── assets/
    └── fonts/
        ├── MyFont-Regular.ttf
        └── MyFont-Bold.ttf
```

Reference them in your Typst plate:

```typst
#set text(font: "MyFont")
```

### Adding Images

Place images in your Quill directory and reference them:

```
my-quill/
└── assets/
    └── logo.png
```

```typst
#image("assets/logo.png")
```

## Example Markdown

Provide an example markdown file to show users how to use your Quill:

```markdown
---
title: Example Document
author: John Doe
date: 2025-01-15
subject: Template Demonstration
---

# Introduction

This is an example document using the template.

## Features

- Professional styling
- Automatic layout
- Custom fonts
```

## Loading and Using Quills

### From Filesystem

```python
from quillmark import Quillmark, Quill

engine = Quillmark()
quill = Quill.from_path("path/to/my-quill")
engine.register_quill(quill)
```

### From JSON

```python
import json
from quillmark import Quillmark, Quill

quill_data = {
    "files": {
        "Quill.toml": {"contents": "..."},
        "plate.typ": {"contents": "..."}
    }
}

quill = Quill.from_json(json.dumps(quill_data))
engine = Quillmark()
engine.register_quill(quill)
```

## Best Practices

1. **Keep it simple** - Start with basic templates and add complexity only when needed
2. **Use examples** - Provide clear example markdown files
3. **Document fields** - Write good descriptions for all fields in `[fields]`
4. **Test thoroughly** - Try various input combinations
5. **Version carefully** - Use semantic versioning for your Quills

## Next Steps

- [Learn about Quill Markdown syntax](quill-markdown.md)
- [Explore the Typst backend](typst-backend.md)
