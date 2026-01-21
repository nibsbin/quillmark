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
#import "@local/quillmark-helper:0.1.0": data, eval-markup, parse-date

#data.title          // plain field access
#eval-markup(data.BODY) // BODY is pre-converted markdown
#parse-date(data.date)  // ISO 8601 → datetime
```

Helper contents (generated in `backends/typst/helper.rs`):
- `data`: parsed JSON dictionary of all fields
- `eval-markup(s)`: evaluate pre-converted Typst markup strings
- `parse-date(s)`: ISO 8601 date parsing helper

## AcroForm Backend

AcroForm ignores plates and uses MiniJinja to render PDF field values from the same JSON data. Tooltips (`description__{{template}}`) and existing field values can contain templates.

## Guarantees

- No `__metadata__` shadow fields; JSON matches normalized document keys
- Dynamic assets/fonts are injected into the quill file tree before compilation
- Backends receive the exact JSON used for compilation (also exposed via `Workflow::compile_data`)
