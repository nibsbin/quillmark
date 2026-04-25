# Concepts

Understanding the core concepts behind Quillmark will help you use it effectively.

## What is Markdown?

Markdown is a lightweight markup language that uses plain text formatting syntax. It's commonly used for documentation, README files, and content authoring. Quillmark extends standard Markdown with YAML frontmatter for structured metadata.

## The Format-First Philosophy

Quillmark is built around a **format-first** design philosophy:

- **Formats control structure and styling** - Quill formats define how documents are laid out and styled
- **Markdown provides content** - Your markdown files contain the actual content that fills the format
- **Separation of concerns** - Content authors can focus on writing without worrying about layout

This approach differs from traditional Markdown renderers where styling is an afterthought.

## Core Components

### Quill Formats

A **Quill** is a format bundle that defines how Markdown content should be rendered. It contains:

- **Metadata** (`Quill.yaml`) - Configuration including name, backend, and field schemas
- **Plate file** - Backend-specific plate that receives document data as JSON
- **Assets** - Fonts, images, and other resources needed for rendering
- **Packages** - Backend-specific packages (e.g., Typst packages)

### YAML Frontmatter

Quillmark documents use YAML frontmatter to provide structured metadata:

```markdown
---
title: My Document
author: John Doe
date: 2025-01-15
---

# Content starts here
```

This metadata is accessible in formats and is validated against native schema rules defined in the Quill.

### Backends

Backends compile plate content with injected JSON data into final artifacts:

- **Typst Backend** - Generates PDF, SVG, and PNG files using the Typst typesetting system. Markdown fields (annotated with `contentMediaType = "text/markdown"`) are converted to Typst markup during compilation.

Each backend has its own compilation process and error mapping.

### Required `QUILL` Reference

Each document must declare its target format in top-level frontmatter using `QUILL`.

```markdown
---
QUILL: my_custom_format
title: My First Document
author: Jane Doe
---
```

If `QUILL` is missing, parsing fails. Quill names must be `snake_case` (`[a-z][a-z0-9_]*`); hyphens are not allowed.

## The Rendering Pipeline

Quillmark follows a three-stage pipeline:

1. **Parse & Normalize** - Extract YAML frontmatter/body, apply schema coercion/defaults, strip bidi characters, fix HTML fences
2. **Compile** - Backend receives plate content + JSON data and converts them into final artifacts (PDF, SVG, PNG, etc.)
3. **Output** - Return artifacts with metadata

```
Markdown + YAML → Parse/Normalize → Compile (Backend) → Artifacts
```

## Mental Model

Think of Quillmark as a factory:

- **Input**: Raw materials (Markdown content + metadata)
- **Quill**: The format that shapes the output
- **Backend**: The manufacturing process
- **Output**: Finished artifacts (PDF, SVG, PNG)

Different Quills can produce completely different outputs from the same input, just as different molds produce different shapes.

## Key Design Principles

1. **Explicit Format Selection** - Documents declare their format with required `QUILL`
2. **Dynamic Resource Loading** - Assets, fonts, and packages are discovered at runtime
3. **Structured Error Handling** - Clear diagnostics with source locations
4. **Thread-Safe** - Backends are thread-safe with no global state
5. **Language-Agnostic** - Core concepts apply across all language bindings

## Next Steps

- [Create your first Quill](../format-designer/creating-quills.md)
- [Learn Quill versioning](../format-designer/versioning.md)
- [Learn about Markdown syntax](../authoring/markdown-syntax.md)
- [Explore the Typst backend](../format-designer/typst-backend.md)
