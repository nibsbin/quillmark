# Typst JSON Data Delivery Proposal
## Replacing MiniJinja Templating with Direct JSON Injection

**Date:** 2025-01-13
**Context:** Simplify the Typst backend by removing the MiniJinja templating layer and delivering parsed document data directly to Typst as JSON via a virtual package.
**Design Focus:** Cleaner architecture, pure Typst plates, reduced dependencies

---

## Problem Statement

The current architecture uses MiniJinja as an intermediary templating layer between parsed document data and Typst compilation. This creates several pain points:

1. **Mixed syntax in plate files.** Plate authors must write a hybrid of MiniJinja template syntax and Typst code, leading to confusing constructs like `#{{ field | Filter }}` where the `#` is Typst and `{{ }}` is MiniJinja.

2. **Poor IDE support.** The Typst language server cannot understand MiniJinja interpolations, so plate authors lose autocompletion, error checking, and other tooling benefits.

3. **Filter complexity.** The backend must register filters (`String`, `Lines`, `Date`, `Content`, `Dict`, `Json`, `Asset`, `Number`) that transform values into Typst-compatible syntax. This adds a layer of abstraction that obscures what data the plate actually receives.

4. **Additional dependency.** MiniJinja adds to the dependency tree and introduces its own error surface area during template composition.

5. **Two mental models.** Plate authors must understand both MiniJinja's control flow (`{% if %}`, `{% for %}`) and Typst's native equivalents (`#if`, `#for`), when Typst's are more powerful and better integrated.

---

## Proposed Solution

Remove MiniJinja entirely and deliver document data to Typst as a JSON object via a virtual `@local/quillmark-helper` package. Plate files become pure Typst code that imports the helper and accesses data directly.

### Core Principles

1. **Schema-conformant JSON.** Rust serializes the parsed document to JSON that directly reflects the Quill.toml schema. Typst consumers receive clean, predictable data.

2. **Backend-specific transformations happen Rust-side.** Fields with `type = "markdown"` are pre-converted to Typst markup before JSON serialization. The Typst consumer receives ready-to-evaluate strings.

3. **No file artifacts.** The JSON data and helper package are injected directly into the Typst virtual filesystem. No intermediate files are written to disk.

4. **Pure Typst plates.** Plate files contain only valid Typst code. They explicitly import the helper package and use Typst's native control flow.

---

## Architecture Overview

### Current Flow

```
ParsedDocument
    ↓
Workflow: coerce, validate, normalize
    ↓
Backend: register MiniJinja filters
    ↓
Plate: MiniJinja compose (filters transform values)
    ↓
Plated content string (generated Typst source)
    ↓
QuillWorld: compile
```

### New Flow

```
ParsedDocument
    ↓
Workflow: coerce, validate, normalize
    ↓
Backend: transform fields (markdown → Typst markup)
    ↓
Serialize to JSON string
    ↓
QuillWorld: inject @local/quillmark-helper with embedded JSON
    ↓
Plate.typ: pure Typst, imports helper, accesses data
    ↓
Compile
```

---

## The `@local/quillmark-helper` Package

A virtual Typst package injected into the compilation world. It exports:

- **`data`** — A dictionary containing all document fields, serialized from the parsed document according to the Quill.toml schema.

- **`eval-markup(string)`** — A helper function that evaluates a pre-converted Typst markup string. Used for fields that were originally markdown (e.g., BODY).

- **`parse-date(string)`** — A helper function that parses an ISO 8601 date string into a Typst `datetime` value.

The package is generated dynamically with the JSON data embedded inline. No physical files are created.

---

## Plate File Transformation

### Before (MiniJinja + Typst)

```
#import "@preview/some-package:1.0.0": frontmatter, mainmatter

#show: frontmatter.with(
  title: {{ title | String(default="Untitled") }},
  date: {{ date | Date }},
  recipients: {{ memo_for | Lines(default=["recipient"]) }},
)

#mainmatter[
#{{ BODY | Content }}
]

{% for item in CARDS %}
{% if item.CARD == "indorsement" %}
// handle indorsement
{% endif %}
{% endfor %}
```

### After (Pure Typst)

```
#import "@local/quillmark-helper:0.1.0": data, content, parse-date
#import "@preview/some-package:1.0.0": frontmatter, mainmatter

#show: frontmatter.with(
  title: data.at("title", default: "Untitled"),
  date: parse-date(data.date),
  recipients: data.at("memo_for", default: ("recipient",)),
)

#mainmatter[
  #eval-markup(data.BODY)
]

#for item in data.at("CARDS", default: ()) {
  if item.CARD == "indorsement" {
    // handle indorsement
  }
}
```

---

## Content Field Handling

Fields with `type = "markdown"` in Quill.toml require special handling. These fields contain markdown that must be rendered as Typst content.

### Transformation Pipeline

1. **During field transformation**, the backend identifies markdown-typed fields using the schema.

2. **The backend calls `mark_to_typst()`** to convert the markdown to Typst markup. This conversion already exists in the codebase.

3. **The converted string is stored in the JSON.** It contains valid Typst markup, ready to be evaluated.

4. **In the plate, the author calls `eval-markup(data.BODY)`**, which wraps the string in `eval(..., mode: "markup")` to produce Typst content.

### Nested Content Fields

Scopes (collections) may contain their own markdown fields, such as `CARDS.*.BODY`. The backend recursively transforms all markdown fields based on the schema, including those nested within arrays.

---

## Date Handling

JSON has no native date type. Dates remain as ISO 8601 strings in the serialized data. The `parse-date` helper function provided by `@local/quillmark-helper` parses these strings into Typst `datetime` values.

This keeps the JSON clean and schema-conformant while giving Typst consumers an easy path to proper date objects.

---

## Asset Handling

Asset paths are validated during the normalization phase, before JSON serialization. The JSON contains plain string paths. Typst consumers reference assets using standard Typst file operations.

---

## Backend Trait Changes

The `Backend` trait gains a new method for field transformation:

- **`transform_fields(fields, schema)`** — Transforms field values according to backend-specific rules. For the Typst backend, this converts markdown fields to Typst markup. Other backends may implement different transformations or pass through unchanged.

The existing filter registration method is removed, as filters are no longer used.

---

## Compile Function Changes

The compile entry point accepts the JSON data as a separate parameter alongside the plate content. This keeps the data flow explicit and maintains immutability of the Quill configuration object.

---

## QuillWorld Changes

The world constructor accepts the JSON data and injects the `@local/quillmark-helper` package into the virtual filesystem. The package's `lib.typ` is generated dynamically with the JSON embedded as a byte literal.

The plate file becomes the main entry point for compilation, rather than a generated intermediate file.

---

## What Gets Removed

- **MiniJinja dependency** — No longer needed.

- **`templating.rs`** — The entire template composition module, including `Plate`, `TemplatePlate`, `AutoPlate`, and related types.

- **Filter system** — All filter functions (`string_filter`, `date_filter`, `content_filter`, etc.) and the filter registration API.

- **`filter_api` module** — The stable filter API wrapper is no longer needed.

- **Mixed-syntax plate files** — All existing plates must be rewritten as pure Typst.

---

## Migration Approach

This is a breaking change with no backwards compatibility. All existing quills with plate files must be updated to the new pure-Typst format.

The transformation is mechanical:

1. Add the `@local/quillmark-helper` import.
2. Replace `{{ field | Filter }}` with `data.field` or the appropriate helper call.
3. Replace `{% if %}` with `#if`.
4. Replace `{% for %}` with `#for`.
5. Replace filter-specific patterns with their Typst equivalents.

---

## Benefits

1. **Simpler architecture.** One less layer between data and output.

2. **Better tooling.** Typst LSP works correctly in plate files.

3. **Smaller dependency footprint.** MiniJinja removed.

4. **Clearer data contract.** The JSON schema is explicit and predictable.

5. **More powerful plates.** Full access to Typst's programming capabilities without template language limitations.

6. **Easier debugging.** The JSON data can be inspected directly; no filter transformation mysteries.

---

## Open Considerations

### Package Versioning

The helper package uses version `0.1.0`. If the helper API changes in the future, we may need a versioning strategy to maintain compatibility with existing plates.

### Error Messages

Errors that previously surfaced during MiniJinja composition will now appear at different points: JSON serialization errors in Rust, content conversion errors in Rust, and Typst evaluation errors during compilation. Error messages should clearly indicate which phase failed.

### Schema Evolution

If the Quill.toml schema changes (e.g., a field is renamed), the JSON structure changes accordingly. Plates must be updated to match. This is the same situation as today, just more direct.
