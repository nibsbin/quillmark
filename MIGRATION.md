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

## Compatibility note (transition only)

Current parser behavior still accepts legacy root-level `fields:` for backward compatibility, but that is a temporary transition aid. Treat it as deprecated and migrate immediately.

If both are present, prefer using only `main.fields` to avoid ambiguity.

---

## Opinionated migration checklist (do all of it)

- [ ] Add a `main:` section.
- [ ] Move root `fields:` into `main.fields:`.
- [ ] Move root/Quill container UI metadata to `main.ui:`.
- [ ] Keep named reusable cards under `cards:`.
- [ ] Re-run validation (`quillmark validate <quill-dir>` or your existing CI checks).
