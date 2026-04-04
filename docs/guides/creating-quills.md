# Creating Quills

A Quill is a template bundle that defines how your Markdown content should be rendered. This guide will walk you through creating your own Quill.

## Quill Structure

A Quill is a directory containing:

```
my-quill/
├── Quill.yaml          # Configuration and metadata
├── plate.typ           # Plate template (backend-specific)
├── example.md          # Optional example document
└── assets/             # Optional assets (fonts, images, etc.)
    ├── logo.png
    └── fonts/
        └── custom.ttf
```

## Quill.yaml Configuration

The `Quill.yaml` file defines your Quill's metadata and configuration:

```yaml
Quill:
  name: my-quill
  backend: typst
  description: A professional document template
  plate_file: plate.typ
  example_file: example.md
  version: "1.0.0"
  author: Your Name

# Backend-specific configuration
typst:
  packages:
    - "@preview/appreciated-letter:0.1.0"

# Field schemas for validation
fields:
  title:
    description: Document title
    type: string
  author:
    description: Author name
    type: string
    default: Anonymous
  date:
    description: Document date
    type: string
```

### Required Fields

- `name` - Unique identifier for your Quill
- `backend` - Backend to use (`"typst"` or `"acroform"`)
- `description` - Human-readable description
- `version` - Semantic version (`MAJOR.MINOR` or `MAJOR.MINOR.PATCH`)

### Optional Fields

- `plate_file` - Path to plate template (defaults to auto-generated plate)
- `example_file` - Path to example markdown file (defaults to `example.md` if present)
- `author` - Creator of the Quill

### Field Schemas

Define expected frontmatter fields in the `fields` section:

```yaml
fields:
  title:
    description: Document title
    type: string
  count:
    description: Number of items
    type: number
    default: 10
  enabled:
    description: Enable feature
    type: boolean
    default: false
```

Each field can specify:
- `title` - Short label for the field
- `description` - Detailed description of the field
- `type` - Data type
- `default` - Default value if not provided
- `examples` - Array of example values
- `required` - Whether the field must be present
- `enum` - Restrict string fields to specific values
- `items` - Item schema (for `array` type)

Supported `type` values:
- `string` or `str`
- `number`
- `boolean`
- `array`
- `date`
- `datetime`
- `markdown`

### Arrays of Structured Objects

Combine `array` with an `items` schema of `type: object` for lists of structured records:

```yaml
fields:
  recipients:
    description: List of recipients
    type: array
    items:
      type: object
      properties:
        name:
          type: string
          required: true
        email:
          type: string
```

The `type: object` keyword is only valid inside `items`. Standalone `type: object` fields are not supported — use separate fields with `ui: { group: ... }` instead.

### Markdown Fields

Use `type: markdown` for fields that contain rich text. Backends convert the markdown content (e.g., to Typst markup) before rendering:

```yaml
fields:
  summary:
    description: Executive summary
    type: markdown

  notes:
    description: Detailed notes
    type: markdown
    ui:
      multiline: true   # start as a larger text box
```

The `multiline: true` UI hint is for presentation only — it has no effect on validation or backend processing.

### UI Configuration

You can provide additional metadata for UI generators (like wizards or form builders) using the `ui` property within a field definition.

```yaml
fields:
  sender:
    title: Sender Name
    description: The full name of the person sending the letter, including any titles or suffixes.
    type: string
    ui:
      group: Sender Information
```

Supported UI properties:

- `group` - Group name for organizing fields in the UI
- `visible_when` - Conditionally show/hide based on sibling field values
- `hide_body` - Disable the markdown body editor for the document (document-level `ui` only)

#### Disabling the Body Editor

Some Quills are purely metadata-driven and don't use a markdown body at all. Set `hide_body: true` in the document-level `ui` block to signal to consumers (form builders, UI wizards) that the body editor should not be shown:

```yaml
Quill:
  name: my-quill
  backend: typst
  ui:
    hide_body: true
```

This is a UI hint only — it does not remove the `BODY` field from the schema or prevent the backend from receiving body content.

#### Conditional Visibility

Use `visible_when` to show fields only when they're relevant:

```yaml
fields:
  format:
    type: string
    enum: [standard, informal]
    default: standard

  from:
    type: string
    ui:
      group: Addressing
      visible_when:
        format: [standard]
```

The `from` field only appears when `format` is `"standard"`. See the [Conditional Fields](conditional-fields.md) guide for the full specification.

For a complete reference of all YAML properties, see the [Quill.yaml Reference](quill-yaml-reference.md).

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

The document body is stored in `data.BODY` (also exposed as `body` in some bindings). For Typst, `transform_fields` converts markdown to Typst markup, so render with `eval-markup(data.BODY)`:

```typst
#eval-markup(data.at("BODY", default: ""))
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

Configure Typst packages and settings in the `typst` section:

```yaml
typst:
  packages:
    - "@preview/appreciated-letter:0.1.0"
    - "@preview/bubble:0.2.2"
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

### From a Quill Object

You can also pass a `Quill` object directly to `workflow()` without registering it first:

```python
from quillmark import Quillmark, Quill

quill = Quill.from_path("path/to/my-quill")
engine = Quillmark()
workflow = engine.workflow(quill)
```

## Best Practices

1. **Keep it simple** - Start with basic templates and add complexity only when needed
2. **Use examples** - Provide clear example markdown files
3. **Document fields** - Write good descriptions for all fields in `fields`
4. **Test thoroughly** - Try various input combinations
5. **Version carefully** - Use semantic versioning for your Quills

## Next Steps

- [Quill.yaml Reference](quill-yaml-reference.md) - Complete YAML property reference
- [Conditional Fields](conditional-fields.md) - Show/hide fields with `visible_when`
- [Learn about Quill Markdown syntax](quill-markdown.md)
- [Explore the Typst backend](typst-backend.md)
