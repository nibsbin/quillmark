# Bulk Generation (Merge)

> **Implementation**: `crates/merge/` (the interpreter), `crates/bindings/cli/` (the `merge` verb)
> **Related**: [SCHEMAS.md](SCHEMAS.md) § "The values form", [PROGRAMMATIC.md](PROGRAMMATIC.md), [ERROR.md](ERROR.md)

## TL;DR

Bulk generation is `(quill, rows, MergeSpec) → N documents + a report`: mail
merge over the schema model, with the engine's own coercion and validation as
the quality story. `quillmark-merge` lowers each row through the spec into the
values form, builds the document through the typed writer, validates it, and
anchors every refusal to its row and column. Rendering is the surface's loop
over the plan. Everything before render is engine-free, so a batch is judged
before any compilation is paid.

## Layering

```
tabular bytes (CSV / TSV / JSON)        ← edge: the CLI (csv crate), a GUI (paste, SheetJS)
  ↓ parse, header-key
rows: JSON objects keyed by header
  ↓ MergeSpec map / group_by            ← interpreter (quillmark-merge)
DocumentValues per document             ← the structured lane: the values form
  ↓ Document::new → writer.set_values   ← the typed writer: coercion, per-cell refusals
Document + RowDiagnostics               ← plan: engine-free
  ↓ engine per document                 ← surface-owned loop (rayon in the CLI)
artifacts + manifest
```

Edge parsing stays out of core: the interpreter sees JSON rows and never a
byte of CSV. Semantics live once, in the interpreter; the CLI, a GUI and an
HTTP consumer execute the same spec.

## The spec

One serde model; YAML and JSON both deserialize (`MergeSpec::from_yaml` /
`from_json`). It pins `$quill: name@selector` the way a document does, and the
same pairing check applies (`merge::quill_mismatch`; see
[VERSIONING.md](VERSIONING.md)).

```yaml
$quill: certificate@1.2.0
mode: document                   # or cards
key: "Employee ID"               # document mode; absent, the output name keys the row
map:
  recipient:   { column: Name }
  event:       { value: "Rustconf 2026" }
  awarded_on:  { column: Date, format: "%m/%d/%Y" }
  tags:        { column: Tags, split: "," }
  cohort.lead: { column: Instructor }
  $body:       { column: Citation }
output: "{recipient}-certificate"
```

```yaml
$quill: invoice@2.0.0
mode: cards
group_by: "Invoice #"
map:                             # main fields: constant within a group
  customer:   { column: Customer }
  invoice_no: { column: "Invoice #" }
cards:
  line_item:                     # exactly one kind: what each row becomes
    map:
      desc: { column: Description }
      qty:  { column: Qty }
output: "invoice-{invoice_no}"
```

| Key | Meaning |
|---|---|
| `$quill` | The pairing assertion, checked against the loaded quill's name and version |
| `mode` | `document` (one row, one document; the default) or `cards` (rows sharing `group_by`, one document, one card per row) |
| `key` | `document` mode only: the column whose value keys a row. Absent, the output name is the key. Duplicates are `merge::duplicate_key` |
| `map` | Main-card mappings keyed by target |
| `group_by` | `cards` mode: the grouping column. Its value is the document's key; a row with none is `merge::missing_group_key` |
| `cards` | `cards` mode: one card kind and its `map`. Two kinds are `merge::spec_mode`: a row becomes one card, and nothing says which kind |
| `output` | The output *stem*; `{field}` interpolates a main field. The surface appends the format's extension |

A mapping is `{column}` or `{value}`, exactly one, plus `split` and `format`.
`value` is a constant, verbatim: `value: ""` authors the blank. A row that is
empty in every cell is skipped and counted, not planned.

### Targets

A target is a schema address ([PLATE_DATA.md](PLATE_DATA.md) § "Schema
addresses") without the element step: a field, a typed dictionary's property,
a variant's cell or its `value` discriminant, at whatever depth the schema
nests, plus `$body` for the card's body. A dotted target assembles its
container, so `classification.value` and `classification.poc` together build
the variant shape a document authors. Every target is resolved against the
schema before any row is read (`merge::spec_unknown_target`).

A repeated shape reaches a document through `split:` (an `array` of scalars)
or through `cards` mode, never through a typed table: a flat row has no
element axis, and that is the deliberate ceiling.

### Identity mapping

A header that is itself a target maps with no entry: `qty` fills `qty`,
`cohort.lead` fills the property, `$body` fills the body. An entry in `map`
overrides the identity for its target, and the header it displaces warns
`merge::unmapped_column` like any header nothing reads. So the simplest merge
is a spreadsheet whose headers are field names and a spec carrying `$quill`
and `output`, and `map` is for renames, constants and transforms. In `cards`
mode a header both the main card and the card kind declare is
`merge::ambiguous_column`: it maps explicitly or not at all.

### Cells

- **An empty column cell is absent.** Null, empty and whitespace-only cells
  drop the key, so the field falls to `default:` › blank and an obliged cell
  warns `validation::must_fill` at its column. A spreadsheet's blank means
  "nothing here"; the writer would otherwise refuse `""` on an `integer` and
  record it as an authored answer on a `string` (which discharges the
  obligation). A deliberate blank is spelled `value: ""`.
- **Cells are trimmed**, and the typed writer judges them: `"3"` into an
  `integer` is `3`, `"TRUE"` into a `boolean` is `true`, `"abc"` into an
  `integer` is `edit::field_coercion_failed` at that row and column. Native
  JSON cells (from a JSON input or an API caller) ride through as they are.
- **`split`** turns a string cell into an array on the separator, pieces
  trimmed, empty pieces dropped.
- **`format`** is the strftime pattern a date cell is written in; the cell is
  re-spelled `YYYY-MM-DD`. Padding is lenient whatever the pattern says: a
  spreadsheet exports `3/1/2026` and `03/01/2026` from the same column, and
  `%m` reads both. A cell that does not parse is `merge::date_format`.

### Constant within a group

In `cards` mode a main-mapped target must lower to the same value on every row
of its group; a row that differs is `merge::group_conflict` at that row and
column, and the group is not planned. First row wins silently is how a wrong
invoice ships.

## Two lanes, one convergence

`Input::Rows` is the tabular lane, lowered through the mappings above.
`Input::Documents` is the structured lane: a list of `DocumentValues`, the
values form ([SCHEMAS.md](SCHEMAS.md) § "The values form"), handed to the
writer as is: no mapping, keys are field names, arrays, objects and ISO dates
native. It is the API and skill lane, and the shape a GUI grid lowers to.
Both converge before construction: one `Document::new(ref)` and one
`set_values` per document, whichever lane built the values.

## The report

Every diagnostic is a `RowDiagnostic { row, column, diagnostic }` wrapping the
engine's `Diagnostic` ([ERROR.md](ERROR.md)). `row` is the 0-based data row
(the documents lane's index), `None` for a spec-level diagnostic; `column` is
the input column that fed the anchored cell, reverse-mapped from the
diagnostic's `path` through the mapping table. The surface presents rows
however its input numbers them: the CLI prints spreadsheet numbering for a
tabular input (the header is row 1) and the index for JSON, and its `--json`
carries the 0-based row.

Three producers feed it, in order:

1. **Spec-level**: reference pairing, mode shape, mapping shape, unresolved
   targets, an unreadable output pattern, columns the input lacks. Any error
   here ends the plan with no documents.
2. **Lowering**: date parsing, a missing key or group key, a group conflict,
   an unresolvable or colliding output name.
3. **Construction**: every `(DocPath, EditError)` the writer refuses, then
   every `Quill::validate` diagnostic. A card's diagnostic anchors to the
   card's own source row; a main-field diagnostic in `cards` mode anchors to
   the group's first row.

A warning never blocks. `MergePlan::is_clean` is "no error";
`clean_documents()` is the documents no error anchors to, which is what a
forced render renders.

`merge::*` codes carry no `args`: their reader is a spec author, and the
message is the deliverable, so a consumer template falls back to `message`
on them ([ERROR.md](ERROR.md) § "Diagnostic args").

## Plan, then render

`plan(&Quill, &MergeSpec, &Input) → MergePlan { documents, report,
skipped_empty }`. The plan holds every built `Document`; the surface owns the
render loop. A render session's compile persists across `update`, so the loop
opens one `LiveSession` per worker and updates it per document rather than
opening per document, which pays the quill's font and package load once per
worker instead of once per row. `LiveSession` and `Quill` are `Send + Sync`,
so the workers are threads.

The CLI's `merge` verb ([CLI.md](CLI.md)) is plan first, always: it prints
the report, `--dry-run` stops there, a clean plan renders, and `--force`
renders the clean documents and reports the rest.

## Provenance

Each document is stamped `$ext.merge: { row_key, spec_hash }`
([PROGRAMMATIC.md](PROGRAMMATIC.md) § "Addressing cards for re-render").
`row_key` is the planned document's key; `spec_hash` is SHA-256 over the
spec's canonical JSON, so a respelled spec hashes the same and a changed
mapping does not. Each planned document also carries `input_hash`: SHA-256
over the spec hash, the quill reference and the values that built it, which
is what an incremental re-run compares. The CLI writes both into
`manifest.json` beside the output: `{ key, rows, filename, input_hash,
status, files }` per document. Skipping unchanged rows is a later feature of
that manifest, not a redesign.

## Out of scope

A value template language beyond `split` and `format` (a computed column is
the spreadsheet's job); typed tables as targets; multi-sheet joins
(`group_by` on one sheet covers the invoice shape); concatenated single-PDF
output; delivery; write-back from an edited document to its row. Where data →
document is a pure function, rebuild.
