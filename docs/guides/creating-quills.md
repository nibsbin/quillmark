# Creating Quills

A Quill is a template bundle that defines how your Markdown content should be rendered. This guide will walk you through creating your own Quill.

## Quill Structure

A Quill is a directory containing:

```
my-quill/
├── Quill.toml          # Configuration and metadata
├── glue.typ            # MiniJinja template (backend-specific)
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
glue_file = "glue.typ"
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

- `glue_file` - Path to glue template (defaults to auto-generated glue)
- `example_file` - Path to example markdown file
- `version` - Semantic version of your Quill
- `author` - Creator of the Quill
- `json_schema_file` - Path to JSON schema file (overrides `[fields]`)

### Field Schemas

Define expected frontmatter fields in the `[fields]` section:

```toml
[fields]
title = { description = "Document title", type = "str" }
count = { description = "Number of items", type = "int", default = 10 }
enabled = { description = "Enable feature", type = "bool", default = false }
```

Each field can specify:
- `description` - Human-readable description
- `type` - Data type (`"str"`, `"int"`, `"bool"`, etc.)
- `default` - Default value if not provided
- `examples` - Array of example values

### UI Configuration

You can provide additional metadata for UI generators (like wizards or form builders) using the `ui` table within a field definition.

```toml
[fields.sender]
type = "string"

[fields.sender.ui]
group = "Sender Information"
tooltip = "The name of the person sending the letter"
extra = { placeholder = "John Doe" }
```

Supported UI properties:
- `group` - Group name for organizing fields
- `tooltip` - Help text to display on hover
- `extra` - Arbitrary key-value pairs for custom UI logic

## Glue Templates

Glue templates use MiniJinja syntax to compose backend-specific code. They have access to frontmatter data and special filters.

### Basic Example (Typst)

```jinja
#import "@preview/appreciated-letter:0.1.0": letter

#show: letter.with(
  sender: {{ sender | String }},
  recipient: {{ recipient | String }},
  date: {{ date | String }},
  subject: {{ subject | String }},
)

#{{ body | Content }}
```

### Available Filters

Filters convert data to backend-specific formats. For the Typst backend:

- `String` - Convert to Typst string
- `Lines` - Convert to Typst line array
- `Date` - Convert to Typst datetime
- `Dict` - Convert to Typst dictionary
- `Content` - Convert Markdown body to Typst content
- `Asset` - Reference asset files

Example using multiple filters:

```jinja
#let metadata = {{ frontmatter | Dict }}
#let authors = {{ authors | Lines }}

= {{ title | String }}

#{{ body | Content }}
```

### Accessing Frontmatter

Access YAML frontmatter fields directly in your template:

```yaml
---
title: My Document
author: John Doe
tags: ["important", "draft"]
---
```

```jinja
Title: {{ title | String }}
Author: {{ author | String }}
Tags: {{ tags | Lines }}
```

### Using the Metadata Object

Quillmark provides a special `__metadata__` field that contains all frontmatter fields except `body`. This is useful for iterating over metadata:

```jinja
{% for key, value in __metadata__ %}
  #set document({{ key }}: {{ value | String }})
{% endfor %}

{# Body content separately #}
#{{ body | Content }}
```

The `__metadata__` field is automatically created and includes all fields from frontmatter (including SCOPE-based collections), but excludes the `body` field. You can still access individual fields at the top level (e.g., `{{ title }}`), but `__metadata__` provides convenient metadata-only access.

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

The AcroForm backend fills PDF forms. Place a `form.pdf` file in your Quill directory:

```
my-form-quill/
├── Quill.toml
├── glue.jinja       # Template for field values
└── form.pdf         # PDF form to fill
```

See the [AcroForm Backend Guide](acroform-backend.md) for more details.

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

Reference them in your Typst glue:

```jinja
#set text(font: "MyFont")
```

### Adding Images

Place images in your Quill directory and reference them:

```
my-quill/
└── assets/
    └── logo.png
```

```jinja
#image({{ "assets/logo.png" | Asset }})
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
        "glue.typ": {"contents": "..."}
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
- [Understand the AcroForm backend](acroform-backend.md)
