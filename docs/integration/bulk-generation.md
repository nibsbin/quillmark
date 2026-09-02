# Bulk Generation

Generate one document per spreadsheet row, or one per group of rows, through a
quill: certificates from an attendee list, invoices from a line-item export,
letters from a contact table. The engine's own coercion and validation judge
every cell, and the report names the row and column of anything it refuses
before a single page is typeset.

Full model: [MERGE.md](https://github.com/borb-sh/quillmark/blob/main/prose/canon/MERGE.md).

## The shortest merge

If the spreadsheet's headers are the quill's field names, the spec is two
lines:

```yaml
# certificates.yaml
$quill: certificate@1.2.0
output: "{recipient}-certificate"
```

```bash
quillmark merge ./quills/certificate certificates.yaml attendees.csv --out ./certificates
```

Every header that names a field (or a nested address such as `cohort.lead`,
or `$body` for the document body) fills that field. A header nothing reads
warns once and is ignored. `output` is the file stem; the format's extension
is appended, so the same spec serves `-f pdf` and `-f svg`.

## Mapping columns

When headers and fields differ, or a value needs a transform, add a `map`:

```yaml
$quill: certificate@1.2.0
key: "Employee ID"                # keys each row; absent, the output name does
map:
  recipient:   { column: Name }
  event:       { value: "Rustconf 2026" }        # a constant, on every row
  awarded_on:  { column: Date, format: "%m/%d/%Y" }
  tags:        { column: Tags, split: "," }      # "a, b" → ["a", "b"]
  cohort.lead: { column: Instructor }            # a property of a dictionary field
  $body:       { column: Citation }              # the document body, as markdown
output: "{recipient}-certificate"
```

- A mapping is `column` or `value`, one of the two.
- `format` is a strftime pattern for a date column. Padding is lenient:
  `3/1/2026` and `03/01/2026` both read under `%m/%d/%Y`. The cell is stored
  as `YYYY-MM-DD`.
- `split` turns a cell into an array on the separator.
- An empty cell means "nothing here": the field falls to its `default:` (or
  its blank) and, where the quill obliges a value, the report warns at that
  row and column. To author an empty string on purpose, use `value: ""`.
- Cells are judged by the field's type: `"3"` into an `integer` is `3`,
  `"abc"` is an error naming the row and the column.

## One document from several rows

An invoice is one document with one card per line: group rows on a column
and name the card kind each row becomes.

```yaml
$quill: invoice@2.0.0
mode: cards
group_by: "Invoice #"
map:                              # main fields: the same on every row of a group
  customer:   { column: Customer }
  invoice_no: { column: "Invoice #" }
cards:
  line_item:
    map:
      desc: { column: Description }
      qty:  { column: Qty }
output: "invoice-{invoice_no}"
```

Groups come out in first-appearance order and cards in row order. A main
column that differs within a group is an error: the report names the row,
and nothing is guessed.

## Plan first

Every run plans the whole batch before rendering anything and prints the
report:

```
[error] row 17 column 'Qty' main.qty edit::field_coercion_failed: field 'qty' could not be coerced to its schema type: string is not a valid integer
[warning] (312 rows, first at row 2) column 'Instructor' main.cohort.lead validation::must_fill: Field `main.cohort.lead` must be filled in: nobody has authored a value.
Planned 311 document(s): 1 error(s), 312 warning(s), 0 empty row(s) skipped. Nothing rendered: fix the errors, or pass --force to render the clean rows.
```

Rows are numbered as the spreadsheet numbers them (the header is row 1).
Warnings never block; an error does, until `--force` renders the clean rows
and reports the rest.

- `--dry-run` stops after the report.
- `--json` puts the report and the manifest on stdout as one object, with
  0-based rows, for CI.
- `--jobs N` bounds the render threads; the default is every core.

## What lands in `--out`

One artifact per document, named from `output` (a multi-page SVG or PNG
render numbers its pages: `stem-1.svg`, `stem-2.svg`), and `manifest.json`:

```json
[
  { "key": "Ada-certificate", "rows": [0], "filename": "Ada-certificate",
    "input_hash": "…", "status": "rendered", "files": ["Ada-certificate.pdf"] }
]
```

`status` is `rendered`, `failed` (the backend refused it; the report says
why), or `skipped` (an error on its row under `--force`). `input_hash` covers
the row's values, the spec and the quill version, so a later run can tell an
unchanged row from a changed one.

## Feeding it from a program

The tabular lane is one of two. A JSON file whose top level is an array of
objects is read as rows with native values (no string coercion). A JSON file
carrying a `documents` array skips the mapping entirely: each entry is a
document in the values form, the shape `reader.values()` reads and
`writer.set_values()` writes
([Programmatic Construction](programmatic.md)).

```json
{ "documents": [
  { "fields": { "recipient": "Ada", "awarded_on": "2026-03-01", "tags": ["a", "b"] },
    "body": "For *outstanding* contributions." }
] }
```

Every document is stamped `$ext.merge: { row_key, spec_hash }` so a program
holding it later can tell which row and which spec produced it.
