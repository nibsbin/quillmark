# Proposal: Typst Date Field Auto-Conversion Rework

## Problem

Typst plates currently parse date values manually:

```typst
#import "@local/quillmark-helper:0.1.0": data, parse-date
#parse-date(data.date)
```

This is inconsistent with markdown field handling, where conversion is automatic via `__meta__` and plate authors just use `data.<field>`.

Current drawbacks:

- Boilerplate in every plate that uses date fields
- Public helper API surface (`parse-date`) that can be avoided
- Uneven data model (`data.BODY` is ready-to-use content, but `data.date` is still a raw string)

## Decisions

### 1. Date conversion becomes automatic inside `data`

For Typst backend output, fields declared as JSON Schema `"format": "date"` are converted to Typst `datetime` values in `quillmark-helper` during `data` construction.

- Plate authors use `data.date` directly
- Raw JSON payload remains unchanged (ISO string); conversion stays in Typst helper layer
- `format: "date-time"` remains out of scope for this change

### 2. Extend `__meta__` with date field annotations

In `crates/backends/typst/src/lib.rs` (`transform_markdown_fields`), collect date fields from:

- top-level schema `properties`
- card schemas under `$defs` (`*_card`)

Inject alongside existing markdown metadata:

```json
{
  "content_fields": [...],
  "card_content_fields": {...},
  "date_fields": ["date", "effective_date"],
  "card_date_fields": { "indorsement": ["date"] }
}
```

### 3. Keep date parser internal to helper package

In `crates/backends/typst/src/lib.typ.template`:

- keep a private date parser helper (for internal conversion)
- auto-convert `meta.date_fields` on top-level `d`
- auto-convert `meta.card_date_fields` for each `CARDS` item
- stop exporting `parse-date` as a public symbol

End state: only `data` is exported for normal plate usage.

### 4. Update first-party plates and docs

Update fixture plates and docs that currently import/call `parse-date`:

- remove `parse-date` from imports
- replace `parse-date(data.date)` with direct `data.date` usage
- keep existing card wiring unchanged (date fields remain available on card objects)

## Scope

### In scope

- Typst backend metadata enrichment for date fields
- Typst helper template auto-conversion for date fields
- Removal of public `parse-date` export from helper package
- Fixture plate updates to new usage
- Documentation updates that currently recommend `parse-date`
- Tests covering new metadata and helper behavior

### Out of scope

- New coercion/validation semantics in core field parsing
- Automatic conversion for `format: "date-time"`
- Changes to non-Typst backends

## Test plan

- **Unit (`lib.rs`)**: top-level `format: "date"` populates `date_fields`
- **Unit (`lib.rs`)**: card `$defs` `format: "date"` populates `card_date_fields`
- **Unit (`lib.rs`)**: `format: "date-time"` is excluded
- **Template/helper tests**: generated helper no longer exports `parse-date` and still renders valid data conversion logic
- **Integration/fixtures**: rendered output from updated plates remains semantically equivalent for date display

## Files affected

| File | Change |
|------|--------|
| `crates/backends/typst/src/lib.rs` | collect/inject `date_fields` + `card_date_fields` in `__meta__` |
| `crates/backends/typst/src/lib.typ.template` | auto-convert date fields in `data`, keep parser internal |
| `crates/backends/typst/src/helper.rs` | update helper docs/tests to match exported API |
| `crates/fixtures/resources/quills/**/plate.typ` | remove `parse-date` imports/call sites |
| `docs/format-designer/typst-backend.md` | update user guidance to direct date field usage |
