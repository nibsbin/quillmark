# Concepts

Understanding the core concepts behind Quillmark will help you use it effectively.

## What is Markdown?

Markdown is a lightweight markup language that uses plain text formatting syntax. It's commonly used for documentation, README files, and content authoring. Quillmark extends standard Markdown with YAML frontmatter for structured metadata.

## The Template-First Philosophy

Quillmark is built around a **template-first** design philosophy:

- **Templates control structure and styling** - Quill templates define how documents are laid out and styled
- **Markdown provides content** - Your markdown files contain the actual content that fills the templates
- **Separation of concerns** - Content authors can focus on writing without worrying about layout

This approach differs from traditional Markdown renderers where styling is an afterthought.

## Core Components

### Quill Templates

A **Quill** is a template bundle that defines how Markdown content should be rendered. It contains:

- **Metadata** (`Quill.yaml`) - Configuration including name, backend, and field schemas
- **Plate template** - Backend-specific template that receives document data as JSON
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

This metadata is accessible in templates and can be validated against JSON schemas defined in the Quill.

### Backends

Backends compile raw plate content with injected JSON data into final artifacts:

- **Typst Backend** - Generates PDF and SVG files using the Typst typesetting system. It transforms markdown fields (annotated with `contentMediaType = "text/markdown"`) into Typst markup before serialization.
- **AcroForm Backend** - Fills PDF forms using MiniJinja templates embedded in form fields/tooltips (plate-less).

Each backend has its own compilation process and error mapping.

### Default Quill System

Quillmark includes a **default quill system** that allows rendering documents without explicitly specifying a Quill template. When no `QUILL` field is present in your frontmatter, Quillmark uses the `__default__` template provided by the backend (if available).

For example, the Typst backend provides a default Quill that renders simple documents with minimal styling. This means you can get started quickly:

```markdown
---
title: My First Document
author: Jane Doe
---

# Introduction

Content here.
```

This document will render using the default Typst quill without requiring you to create or register a custom Quill template. When you're ready for more customization, you can:

1. Specify a custom Quill in frontmatter: `QUILL: my-custom-template`
2. Create and register your own Quill templates

## The Rendering Pipeline

Quillmark follows a multi-stage pipeline:

1. **Parse & Normalize** - Extract YAML frontmatter/body, apply schema coercion/defaults, normalize bidi/HTML fences
2. **Transform Fields** - Backend-specific shaping (e.g., markdown→Typst markup) before JSON serialization
3. **Compile** - Backend processes plate with injected JSON data into final artifacts (PDF, SVG, etc.)
4. **Output** - Return artifacts with metadata

```
Markdown + YAML → Parse/Normalize → Transform Fields → Compile (Backend) → Artifacts
```

## Mental Model

Think of Quillmark as a factory:

- **Input**: Raw materials (Markdown content + metadata)
- **Quill**: The mold/template that shapes the output
- **Backend**: The manufacturing process
- **Output**: Finished products (PDF, SVG, filled forms)

Different Quills can produce completely different outputs from the same input, just as different molds produce different shapes.

## Key Design Principles

1. **Zero-Config Defaults** - Basic projects work without configuration files
2. **Dynamic Resource Loading** - Assets, fonts, and packages are discovered at runtime
3. **Structured Error Handling** - Clear diagnostics with source locations
4. **Thread-Safe** - Backends are thread-safe with no global state
5. **Language-Agnostic** - Core concepts apply across all language bindings

## Next Steps

- [Create your first Quill](../guides/creating-quills.md)
- [Learn about Quill Markdown syntax](../guides/quill-markdown.md)
- [Explore the Typst backend](../guides/typst-backend.md)
