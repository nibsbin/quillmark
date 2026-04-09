# Creating Quills

A Quill is a format bundle that defines how your Markdown content should be rendered. This guide will walk you through creating your own Quill.

## Quick Tutorial

Build a minimal Quill in four steps:

1. Create a new folder with `Quill.yaml` and `plate.typ`.
2. Set required metadata (`name`, `backend`, `description`, `version`) in `Quill.yaml`.
3. Define `main.fields` for the frontmatter your format expects.
4. Add an `example.md` and render it with the CLI or API.

Use the sections below as you complete each step.

## Quill Structure

A Quill is a directory containing:

```
my-quill/
├── Quill.yaml          # Configuration and metadata
├── plate.typ           # Plate file (backend-specific)
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
  description: A professional document format
  plate_file: plate.typ
  example_file: example.md
  version: "1.0.0"
  author: Your Name

# Backend-specific configuration
typst:
  packages:
    - "@preview/appreciated-letter:0.1.0"

# Field schemas for validation (document main card)
main:
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
- `backend` - Backend to use (e.g. `"typst"`)
- `description` - Human-readable description
- `version` - Semantic version (`MAJOR.MINOR` or `MAJOR.MINOR.PATCH`)

### Optional Fields

- `plate_file` - Path to plate file (defaults to auto-generated plate)
- `example_file` - Path to example markdown file (defaults to `example.md` if present)
- `author` - Creator of the Quill
- `ui` - Document-level UI hints (e.g. `hide_body`; see [Disabling the Body Editor](#disabling-the-body-editor))

### Field Schemas

Define expected frontmatter fields under `main.fields` (the main document card):

```yaml
main:
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
- `properties` - Nested field schemas for `object` rows inside `array` `items` (see [Typed tables](#typed-tables))

### Field types

| `type` in Quill.yaml | Role |
|----------------------|------|
| `string` or `str` | Plain text |
| `number` | Numeric values |
| `boolean` | `true` / `false` |
| `array` | Lists; use `items` to describe each element |
| `date` | `YYYY-MM-DD` (string with date format in JSON Schema) |
| `datetime` | ISO 8601 date-time string |
| `markdown` | Markdown source; see [Markdown fields](#markdown-fields) |
| `object` or `dict` | Used for typed table rows under `array.items` with `properties` (see [Typed tables](#typed-tables)) |

### Typed tables

A **typed table** is an `array` whose elements are objects with a fixed set of columns. Define it with `items: { type: object, properties: { ... } }` (you can write `type: dict` instead of `type: object` in `items`; both mean the same).

```yaml
main:
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

Quillmark **coerces** each row’s properties to the declared types (e.g. `"95"` → `95` for a `number` column) during document coercion and when loading Quill config.

### Markdown fields

Use `type: markdown` for frontmatter fields whose value is Markdown. The generated JSON Schema uses `type: string` with `contentMediaType: "text/markdown"`. The Typst backend converts these fields with the same markdown pipeline as `BODY`.

```yaml
main:
  fields:
    summary:
      description: Executive summary
      type: markdown

    notes:
      description: Detailed notes
      type: markdown
      ui:
        multiline: true   # optional: larger initial control in form UIs
```

Use `ui.multiline` when you want form builders to open a larger control by default; it is a UI hint only (serialized as `x-ui.multiline` in JSON Schema).

To hide the main body editor for metadata-only Quills, set `hide_body` on the `Quill` section — see [Disabling the Body Editor](#disabling-the-body-editor).

### UI Configuration

**Field-level `ui`** (on each entry under `main.fields`) is for form builders and wizards:

| Property | Purpose |
|----------|---------|
| `group` | Group related fields in the UI |
| `multiline` | Larger initial control for `markdown` fields ([Markdown fields](#markdown-fields)) |
| `compact` | Prefer a compact control where the UI supports it |

Field order in `Quill.yaml` sets `ui.order` automatically.

```yaml
main:
  fields:
    sender:
      title: Sender Name
      description: The full name of the person sending the letter, including any titles or suffixes.
      type: string
      ui:
        group: Sender Information
```

**Container `ui`** (`Quill.ui` or `cards.<name>.ui`) only supports `hide_body` — see below.

#### Disabling the Body Editor

For metadata-only documents, set `hide_body: true` on the **`Quill`** section so consumers can hide the main body editor:

```yaml
Quill:
  name: my-quill
  backend: typst
  ui:
    hide_body: true
```

The same flag exists on a **card** definition’s `ui` when a card has no body content. This does not remove `BODY` from the schema or block body content from reaching the backend.

For a complete reference of all YAML properties, see the [Quill.yaml Reference](quill-yaml-reference.md).

## Plate Files

Plate files are pure backend-specific code (e.g., Typst) that access document data via a helper package.

### Basic Example (Typst)

```typst
#import "@local/quillmark-helper:0.1.0": data
#import "@preview/appreciated-letter:0.1.0": letter

#show: letter.with(
  sender: data.sender,
  recipient: data.recipient,
  date: data.date,
  subject: data.subject,
)

#data.at("BODY", default: "")
```

### Data Access

Quillmark injects parsed document fields (frontmatter, `BODY`, `CARDS`, etc.) as JSON via the `@local/quillmark-helper` virtual package:

```typst
#import "@local/quillmark-helper:0.1.0": data, parse-date
```

The helper provides:
- `data` - Dictionary of all fields passed to the backend, with markdown fields automatically converted to Typst content objects
- `parse-date(str)` - Parse date strings into Typst datetime objects

### Accessing fields in Typst

Read frontmatter keys and other fields from `data` (for example `data.title` or `data.at("BODY", default: "")`):

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

In Typst plates, the document body is `data.BODY`. Markdown fields (including `BODY`) are automatically converted to Typst content objects by the helper package. Render it with:

```typst
#data.at("BODY", default: "")
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

Add dependencies under `typst:` in `Quill.yaml` (see the example at the start of this page). For compiler settings, package pins, and plate patterns, see the [Typst Backend Guide](typst-backend.md).

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
date: 2026-01-15
subject: Format Walkthrough
---

# Introduction

This is an example document using this format.

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

1. **Keep it simple** - Start with basic formats and add complexity only when needed
2. **Use examples** - Provide clear example markdown files
3. **Document fields** - Write good descriptions for all fields in `fields`
4. **Test thoroughly** - Try various input combinations
5. **Version carefully** - Use semantic versioning for your Quills (see [Quill Versioning](versioning.md))

## Next Steps

- [Quill.yaml Reference](quill-yaml-reference.md) - Complete YAML property reference
- [Quill Versioning](versioning.md) - Versioning and compatibility guidance
- [Learn about Markdown syntax](../authoring/markdown-syntax.md)
- [Explore the Typst backend](typst-backend.md)
