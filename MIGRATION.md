# Quill.yaml Migration Guide (`fields:` → `main:`)

This guide helps Quill.yaml authors migrate to the new main-card configuration model.

## Why this changed

Quill now treats the primary document schema as an explicit card (`main`) rather than a special root-level `fields:` block.

- **Before:** top-level `fields:` and separate `cards:` map.
- **Now:** explicit `main:` section plus optional `cards:`.

This unifies schema structure and keeps the primary document and card definitions conceptually aligned.

---

## ✅ Required change

Move root-level `fields:` under `main.fields`.

### Before

```yaml
Quill:
  name: my_quill
  version: 0.1.0
  backend: typst
  description: Example quill

fields:
  sender:
    type: string
  date:
    type: date

cards:
  indorsement:
    fields:
      from:
        type: string
```

### After

```yaml
Quill:
  name: my_quill
  version: 0.1.0
  backend: typst
  description: Example quill

main:
  fields:
    sender:
      type: string
    date:
      type: date

cards:
  indorsement:
    fields:
      from:
        type: string
```

Do **not** keep document UI container settings in `Quill.ui`; canonical location is `main.ui`.

---

## ✅ Required: move document UI settings under `main.ui`

If you previously used container UI settings under `Quill.ui`, move them to `main.ui` as part of this migration.

### Before

```yaml
Quill:
  name: my_quill
  version: 0.1.0
  backend: typst
  description: Example quill
  ui:
    hide_body: true

fields:
  title:
    type: string
```

### After

```yaml
Quill:
  name: my_quill
  version: 0.1.0
  backend: typst
  description: Example quill

main:
  ui:
    hide_body: true
  fields:
    title:
      type: string
```

---

## What does **not** change

- Markdown authoring format is unchanged.
  - The first `---` block containing `QUILL:` remains the main document block.
  - Additional `---` blocks with `CARD:` remain card instances.
- Parsed output shape is unchanged.
  - Main fields remain top-level values.
  - Card instances still appear in `CARDS`.

---

## Parser behavior

Root-level `fields:` is **rejected**. Loading `Quill.yaml` fails with an error directing you to use `main.fields`.

---

## Opinionated migration checklist (do all of it)

- [ ] Add a `main:` section.
- [ ] Move root `fields:` into `main.fields:`.
- [ ] Move root/Quill container UI metadata to `main.ui:`.
- [ ] Keep named reusable cards under `cards:`.
- [ ] Re-run validation (`quillmark validate <quill-dir>` or your existing CI checks).

---

# API Migration — `Workflow` removed; `Quill.render()` replaces it

> Applies to consumers of the Rust `quillmark` crate and the Python/WASM bindings.

The two parallel render pipelines (core `Quill` methods and `Workflow`) have
collapsed into one. The renderable type is now `quillmark::Quill`, and it
exposes `render`, `open`, `dry_run`, and `compile_data` directly. The
`Workflow` wrapper is removed; `engine.workflow(...)` no longer exists.

Core's `Quill` has been renamed to `QuillSource`, holds only source data, and
is a Rust-internal type. Bindings expose only the orchestration `Quill`.

## Rust

### Before

```rust
let engine = Quillmark::new();
let quill = engine.quill_from_path(path)?;
let workflow = engine.workflow(&quill)?;
let result = workflow.render(&doc, Some(OutputFormat::Pdf))?;
```

### After

```rust
let engine = Quillmark::new();
let quill = engine.quill_from_path(path)?;
let result = quill.render(&doc, Some(OutputFormat::Pdf))?;
```

### Backend trait

`Backend::open` now takes `&QuillSource` instead of `&Quill`. Field access is
identical (`source.find_files`, `source.get_file`, `source.metadata`, …).

### Other removals

- `Quill::from_path` (filesystem walking) moved to the engine;
  `QuillSource::from_tree` is the core entry point.
- `Quill::with_backend` / `Quill::backend()` — gone; the engine attaches the
  backend.
- `RenderError::NoBackend` — gone; rendering goes through `Quill`, which
  always has a backend.
- `Quill::typst_packages()` — private to `quillmark-typst`.
- `Quill::build_transform_schema()` — now
  `quillmark_core::quill::build_transform_schema(&config)`.

## Python

```python
# Before
workflow = engine.workflow(quill)
result = workflow.render(doc, OutputFormat.PDF)

# After
result = quill.render(doc, OutputFormat.PDF)
```

`quill.name`, `quill.backend`, `quill.plate`, `quill.example`,
`quill.metadata`, `quill.schema`, `quill.defaults`, `quill.examples`,
`quill.supported_formats()`, `quill.render(...)`, `quill.open(...)`,
`quill.dry_run(...)`, and `quill.project_form(...)` all remain.

## JavaScript / WASM

```js
// Before and after are the same — `quill.render(doc, opts)` and
// `quill.open(doc)` already lived on the wasm `Quill` handle.
const quill = engine.quill(tree);
const result = quill.render(doc, opts);
```

`quill.backendId` and `quill.projectForm(doc)` are available as delegating
methods; `QuillSource` is not exposed across the FFI boundary.
