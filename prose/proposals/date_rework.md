---

**Feature: Auto-convert `date` fields to Typst `datetime` in quillmark-helper**

**Goal:** `date` fields arrive in `data` as Typst `datetime` objects. Plate authors use `data.date` directly — no helper function to import or call. Mirror exactly how markdown fields are auto-evaluated via `__meta__`.

---

### 1. Annotate date fields in `__meta__` — `crates/backends/typst/src/lib.rs`

In `transform_markdown_fields`, after collecting `content_field_names`, walk schema properties and collect fields where `"format" == "date"` into a `date_fields` vec. Do the same for card fields via `$defs`, producing `card_date_fields: { "<card_type>": [...] }` — same shape as `card_content_fields`.

Inject into `__meta__`:
```json
{
  "content_fields": [...],
  "card_content_fields": {...},
  "date_fields": ["date", "effective_date"],
  "card_date_fields": { "indorsement": ["date"] }
}
```

Date values remain ISO strings in JSON. No Rust-side parsing.

---

### 2. Rework `lib.typ.template` — `crates/backends/typst/src/lib.typ.template`

**Make `parse-date` private** (remove from exports, keep as internal `let`):
```typst
// internal — not exported
#let _parse-date(s) = {
  if s == none { return none }
  let parts = str(s).split("T").at(0).split("-")
  if parts.len() < 3 { return none }
  datetime(year: int(parts.at(0)), month: int(parts.at(1)), day: int(parts.at(2).slice(0, 2)))
}
```

**Add auto-conversion in the `data` block**, after the existing content-field eval loop:
```typst
for key in meta.date_fields {
  if key in d and d.at(key) != none { d.insert(key, _parse-date(str(d.at(key)))) }
}
```

**In the CARDS loop**, after the existing card content-field eval:
```typst
let date-fields = meta.card_date_fields.at(card-type, default: ())
for key in date-fields { 
  if key in card and card.at(key) != none { card.insert(key, _parse-date(str(card.at(key)))) }
}
```

**Update the export line** — `parse-date` is no longer public:
```typst
// Only `data` is exported. Date fields are already datetime objects inside data.
```

---

### 3. Update fixture plates

In all plates (`usaf_memo`, `cmu_letter`, etc.):
- Remove `parse-date` from the import line
- Remove `parse-date(...)` call sites — `data.date` is already a `datetime`
- Card date spreads (`..if "date" in card { (date: card.date) }`) work unchanged

---

### 4. Tests

- **Unit — `lib.rs`:** schema with `format: "date"` fields (top-level and in card `$defs`) produces correct `date_fields` / `card_date_fields` in `__meta__`
- **Unit — `lib.rs`:** `format: "date-time"` fields are **not** included (explicitly out of scope)
- **Integration:** existing render fixture output is byte-identical after rework — the auto-converted `datetime` formats the same way as the old manual `parse-date` result
- **Compile error:** confirm a plate importing `parse-date` fails — validates the symbol is no longer exported