# Plate Data Injection

> **Status**: Implemented  
> **Scope**: How parsed document data reaches plates/backends

## Overview

Quillmark no longer runs a template engine for plates. Instead, `Workflow::compile_data()` produces JSON after coercion, defaults, normalization, and backend `transform_fields`, then passes it alongside the raw plate content to the backend `compile` call.

### Data Shape

- Keys mirror normalized frontmatter fields (including `BODY` and `CARDS`)
- Defaults from the Quill schema are applied before serialization
- Backend `transform_fields` may reshape values (e.g., Typst markdown → Typst markup strings)

## Typst Helper Package

The Typst backend injects a virtual package `@local/quillmark-helper:<version>` that exposes the JSON to plates and provides helpers.

```typst
#import "@local/quillmark-helper:0.1.0": data

#data.title          // plain field access
#data.BODY           // BODY is automatically converted to content
#data.date           // date fields are auto-converted to datetime
```

Helper contents (generated in `backends/typst/helper.rs`):
- `data`: parsed JSON dictionary of all fields, with markdown fields converted to Typst content objects and date fields (`format: date`) converted to Typst `datetime`

## Guarantees

- No `__metadata__` shadow fields; JSON matches normalized document keys
- Dynamic assets/fonts are injected into the quill file tree before compilation
- Backends receive the exact JSON used for compilation (also exposed via `Workflow::compile_data`)
