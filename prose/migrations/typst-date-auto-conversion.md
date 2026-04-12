# Migration: Typst date auto-conversion & `parse-date` removal

**Audience:** Maintainers updating existing Quill Typst plates after the date helper rework.

## What changed

- **JSON Schema** fields with `type: string` and `format: "date"` are converted to Typst `datetime` inside the generated `data` dictionary (top-level and per card type).
- **`parse-date` is no longer part of the public helper API.** Only `data` should be imported for normal plates.
- **`format: "date-time"`** is unchanged: it is **not** auto-converted; keep any manual handling you already use.

## Plate edits (checklist)

1. **Imports:** use only `data`.

   ```typst
   // Before
   #import "@local/quillmark-helper:0.1.0": data, parse-date

   // After
   #import "@local/quillmark-helper:0.1.0": data
   ```

2. **Top-level dates:** drop `parse-date(...)`; use the field from `data`.

   ```typst
   // Before
   #parse-date(data.effective_date)

   // After
   #data.effective_date
   ```

3. **Card dates:** same idea—read the key on each card object after `data` is built (e.g. `card.date`); do not call a helper parser.

4. **Optional / bad values:** if a date string is missing or not parseable as an ISO date, the helper leaves a **`none`** (or skips conversion where appropriate). Keep `#if field != none { ... }` or `.at(..., default: ...)` patterns as needed.

## Schema reminder

Auto-conversion applies only when the field is declared in the Quill JSON Schema as a **`date`** (`format: "date"` on a string). If a field should stay a raw string, do not mark it as `format: "date"`.

## Internal detail (usually ignore)

The Typst backend injects `date_fields` and `card_date_fields` into `__meta__`; the helper consumes `__meta__` and never exposes it to plates. No plate changes are required for that metadata beyond fixing imports and removing `parse-date` call sites.
