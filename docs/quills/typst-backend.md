# Typst Backend

The Typst backend generates PDF, SVG, and PNG documents using the [Typst](https://typst.app/) typesetting system. It converts card-yaml payload fields to Typst markup, injects them into the plate as JSON via a helper package, and compiles to the requested format.

## Data Access

Plates are plain Typst code. Document metadata reaches the plate as a JSON dictionary exposed by the virtual `@local/quillmark-helper` package:

```typst
#import "@local/quillmark-helper:0.1.0": data

#data.title                                  // a declared field: always present
#data.at("logo", default: none)              // an undeclared key: may be absent
```

Fields declared `type: richtext` in `Quill.yaml` arrive as Typst content (their content lowered to markup, ready to render). `type: date` and `type: datetime` fields arrive as a **value-object** (see below). Everything else is a plain JSON-shaped value.

### Dates

A present `type: date` / `type: datetime` field arrives as a small value-object with two projections; a blank date is `none` (so `#if data.field != none` guards are unchanged):

```typst
#(data.issued.display)("[day padding:none] [month repr:long] [year]")  // rendered, click-to-edit
#(data.issued.display)()                                                // rendered, ISO default
#data.issued.value                                                      // native datetime (math, compare, pkgs)
#data.issued.value.display("[year]")                                    // native string (no region)
#data.issued.value.year()                                              // components: int
#if data.issued != none { .. }                                          // presence
```

- **`display` renders and is clickable.** It is a closure returning Typst *content*, so its glyphs carry a region keyed on the field's schema path: the atomic, picker-editable click-to-edit target. Note the **parentheses**: `(data.issued.display)(..)`, not `data.issued.display(..)`. Typst reserves dict-key method sugar for built-in dict methods, so the stored closure must be called through the parenthesized form. The date it wraps is the field's declared type, so a `date`-only field's `display` inherits Typst's native error on a `[hour]` pattern.
- **`.value` is the native escape hatch.** It is the underlying `datetime` for arithmetic, comparison, `.year()`/`.weekday()`/… components, and any datetime-consuming package. `data.issued.value.display("…")` returns a native `str` (no region).

One rule: **native anything → `.value`; region render → `(…display)(…)`.** A package that formats a date internally still needs `.value` when it does native `datetime` work, but placing `(data.issued.display)(..)` (even deep inside a package) keeps the region, because the closure's ink is born at its generated definition site, not the call site.

### Which accessor to reach for

A key's *declaration* decides whether it can be absent, and that decides the accessor. Three cases, no judgement calls:

| Key | Accessor | Why |
|---|---|---|
| A field declared in `Quill.yaml` | `data.subtitle` | Always present: compilation blank-fills every declared field with its authored value, else the schema `default:`, else the field's blank (`""`, `()`, `0`, the empty content). |
| A `$`-sigiled key (`$kind`, `$body`, `$cards`, `$path`) | `data.at("$body", default: "")` | Typst identifiers exclude `$`, *and* `$`-metadata is present only where it is defined: `$kind` only on a card that authors one, `$body` only where the kind enables a body. |
| An undeclared key, or any field of a card whose `$kind` is unknown | `data.at("logo", default: none)` | No schema fills it, so absence is real. |

So a `default:` on a declared field is dead code, and an `#if "field" in data` guard on one is always true. When a declared field is optional, guard its *value*, not its presence:

```typst
#if data.subtitle != "" {
  [Subtitle: #data.subtitle]
}
```

If a default belongs anywhere, it belongs in `Quill.yaml` — a `default:` restated in the plate is never read, and silently diverges when the schema's own default changes.

An `enum` needs this most: its blank is `""`, which is never one of its `values:`, so branch over `values ∪ blank` and never let an `else` swallow the blank into a variant nobody picked.

```typst
#if data.seal != "" { .. }        // the blank means "no seal", not the first value
```

### Body, arrays, and cards

The document body is exposed under the `$body` key, accessed via `data.at("$body")` because Typst identifiers exclude `$`. Arrays come through as Typst arrays. Cards live under the `$cards` key, each carrying its own `$kind` discriminator, fields, and `$body`:

```typst
#data.at("$body", default: "")

#for author in data.authors [- #author]

#for card in data.at("$cards", default: ()) {
  if card.at("$kind", default: none) == "product" {
    [Product: #card.name — #card.at("$body", default: "")]
  }
}
```

A card block with no `$kind:` line is a *kindless* card: it reaches the plate carrying its authored fields verbatim and no `$kind`, so a bare `card.at("$kind")` panics on it. Read the discriminator with a default and let unrecognized kinds fall through.

## Typst Packages

Declare packages in `Quill.yaml`, then `#import` them from the plate:

```yaml
typst:
  packages:
    - "@preview/appreciated-letter:0.1.0"
```

```typst
#import "@local/quillmark-helper:0.1.0": data
#import "@preview/appreciated-letter:0.1.0": letter

#show: letter.with(sender: data.sender, recipient: data.recipient)
```

Browse the full catalog at [Typst Universe](https://typst.app/universe/).

## Fonts

System-installed fonts are available directly (`#set text(font: "Arial")`). To bundle fonts with the Quill, drop them in `assets/fonts/`:

```
my-quill/
└── assets/
    └── fonts/
        ├── CustomFont-Regular.ttf
        └── CustomFont-Bold.ttf
```

Then reference them by family name (`#set text(font: "CustomFont")`).

## Typesetting

Plate authors style output with Typst's standard `#set` directives:

```typst
#set page(paper: "us-letter", margin: 1in, numbering: "1")
#set text(font: "Linux Libertine", size: 11pt, lang: "en")
#set par(justify: true, leading: 0.65em)
```

See the [Typst tutorial](https://typst.app/docs/tutorial/) for the full styling vocabulary. For worked plates that combine data access with real layout, see the `usaf_memo` and `taro` examples in `crates/quillmark/examples/`.

## Signature Fields

Import `signature-field` from the helper package to drop an unsigned PDF signature box anywhere in your plate:

```typst
#import "@local/quillmark-helper:0.1.0": signature-field

Approving authority:
#signature-field("approver")

Witness:
#signature-field("witness", width: 220pt, height: 60pt)
```

PDF output gains a clickable AcroForm SigField widget at each call site. Open the result in Acrobat (or any reader that supports form signing) and the widget presents a "Sign Here" affordance. SVG and PNG outputs reserve the same invisible layout space: useful for preview but no widget visual.

**Important:** the widget is **unsigned**. Quillmark does not perform any cryptography. To produce a signed PDF, run the output through pyHanko, Acrobat, endesive, or another signing tool.

### Positioning

`signature-field` is ordinary Typst inline content sized `width × height`. It participates in layout the same way `#rect(width: 200pt, height: 50pt)` would: content after it gets pushed by the box's dimensions. Two modes:

**In-flow (reserves layout space).** Drop the call where you want to claim that block of space and let the rest of the document flow around it:

```typst
Sign here:
#signature-field("approver")  // reserves 200×50pt below the label
The above signature acknowledges receipt.
```

**Overlay (no displacement).** Wrap in `#place(...)` to anchor the widget without consuming flow. This is what you want when the surrounding template *already* reserves space, for example, the four blank lines above a typed-name signature block in a USAF memo:

```typst
// At the cursor position where the typed-name signature block begins:
#place(dx: 0pt, dy: -3.5in,
       signature-field("approver", width: 3in, height: 0.5in))
```

`#place` without an alignment argument anchors the widget at the current cursor (then offsets by `dx`/`dy`); `#place(top + left, ...)` anchors to the containing block's top-left. Either way, the call consumes no flow space and the surrounding template stays put.

Inside `#box`, `#table`, `#figure`, `#footnote`, `#move`, `#pad`: `signature-field` tracks the layout system normally. Multi-page documents work; each field's `page` is the page it lays out on, not where it was written in source.

### Parameters

| Name | Type | Default | Notes |
|---|---|---|---|
| `name` | `str` | required (positional) | Field name: must be unique within the document and match `[A-Za-z0-9_.]+` (`.` allowed for fully-qualified names). Surfaces as the widget's `/T` entry. |
| `width` | `length` | `200pt` | Must be an absolute length (`pt`, `mm`, `cm`, `in`): relative lengths like `2em` or `50%` are rejected. |
| `height` | `length` | `50pt` | Same constraint as `width`. |

### Errors

- Two calls with the same `name` raise a compilation error (`typst::duplicate_form_field`). `signature-field` is a thin wrapper over the same `form-field` primitive that backs text/checkbox/choice widgets, so its names share one uniqueness domain with theirs.
- A non-absolute `width` or `height` raises a Typst assert pointing at `form-field`.
- Names violating `[A-Za-z0-9_.]+` raise a Typst assert.

The label `<__qm_field__>` and metadata `kind: "__qm_field__"` are reserved for this hand-off: don't use them for unrelated metadata in your plate.

> `signature-field` emits a document-global `metadata` element (standard Typst
> introspection). If your plate or its packages read config via
> `query(metadata)`, filter to your own elements rather than assuming a single
> or last metadata element.

## Form Fields

`signature-field` is a thin wrapper over the general `form-field` primitive, which backs all four widget kinds: text inputs, checkboxes, choice dropdowns, and signature boxes. Import it from the same helper package:

```typst
#import "@local/quillmark-helper:0.1.0": form-field
```

Each call drops an AcroForm widget at its call site (a clickable field in PDF; reserved invisible layout space in SVG/PNG, same as `signature-field`). Value binding is the plate author's job: pass `value:` straight from your data; there is no resolver on the Typst side.

### Parameters

| Name | Type | Default | Notes |
|---|---|---|---|
| `name` | `str` | required (positional) | Widget `/T` name: unique within the document, matching `[A-Za-z0-9_.]+`. Shares one uniqueness domain with `signature-field`. |
| `type` | `str` | `"text"` | One of `"text"`, `"checkbox"`, `"choice"`, `"signature"`. |
| `value` | per type | `none` | The delivered field value; interpretation depends on `type` (see below). |
| `options` | `array` of `str` | `()` | Display strings for `type: "choice"`; ignored otherwise. |
| `multiline` | `bool` | `false` | Toggles the multi-line flag for `type: "text"`; ignored otherwise. |
| `width` | `length` | `200pt` | Absolute length (`pt`/`mm`/`cm`/`in`); relative lengths (`2em`, `50%`) are rejected. |
| `height` | `length` | `20pt` | Same constraint as `width`. |
| `field` | `str` or `none` | `none` | Schema-field address this widget's region is keyed on (see "Binding to a schema field"). |
| `font` | `str` | `"helvetica"` | `"helvetica"`, `"times"`, or `"courier"`; `"text"`/`"choice"` only (see "Styling the value text"). |
| `size` | `length` or `auto` | `auto` | Absolute length, or `auto` for the viewer's fit-to-box. `"text"`/`"choice"` only. |
| `align` | `str` | `"left"` | `"left"`, `"center"`, or `"right"`. `"text"`/`"choice"` only. |

Positioning works exactly as for `signature-field` (in-flow reserves space; wrap in `#place(...)` to overlay without displacement); see the "Positioning" notes above.

### The four field types

`value:` is forwarded verbatim; the Rust adapter maps it to the AcroForm value per `type`:

**Text**: `value` is a string (numbers stringify). A blank value emits no `/V`. Set `multiline: true` for a multi-line box.

```typst
#form-field("full_name", type: "text", value: data.name)
#form-field("bio", type: "text", value: data.bio, multiline: true, height: 80pt)
```

**Checkbox**: `value` is a bool; `true` renders checked.

```typst
#form-field("agree", type: "checkbox", value: data.agree)
```

**Choice**: `value` is a string, bound only if it matches an entry in `options`.

```typst
#form-field("size", type: "choice", options: ("S", "M", "L"), value: data.size)
```

**Signature**: `value` is ignored; the widget is an unsigned SigField (Quillmark performs no cryptography: sign the output with pyHanko, Acrobat, endesive, etc.). `signature-field(name, ...)` is exactly `form-field(name, type: "signature", ...)`.

```typst
#form-field("approver", type: "signature", height: 50pt)
```

### Binding to a schema field

By default a widget's only identity is its `/T` name. Pass `field:` to additionally key the widget's region on a schema-field address, so it surfaces in the geometry sidecar (`session.regions()`) and resolves under `session.fieldAt(...)`:

```typst
#form-field("Signature", type: "signature", field: "signature_block")
```

`field:` is **region-only**: the `/T` widget name stays `name`; only the sidecar entry keys on `field:`. The address must be a real schema field: a bare field name, an array element like `"refs.2"`, or a card path built from the card's `$path` prefix (a bad address raises a Typst assert). Omit `field:` and the widget exposes no region: a click has no schema field to route to.

### Styling the value text

The widget itself draws nothing: a viewer synthesizes the value's appearance when someone fills the field. `font`, `size`, and `align` are what that synthesis reads. They apply to `"text"` and `"choice"` only, the other two kinds having no variable text, and passing a non-default on those raises an assert rather than silently doing nothing.

```typst
#form-field("memo_date", type: "text", field: "date",
            font: "times", size: 12pt, align: "right", width: 1.2in, height: 16pt)
```

**`size` is worth setting whenever the value has to match surrounding text.** The default `auto` is the AcroForm auto-size, which fits text to the box *and refits as the user types*, so a long value renders smaller than a short one in the same field. An explicit size is the only way to make the rendered size predictable.

**`align` is the only way to pin a value to an edge.** A fillable box has to be sized for the longest plausible value, not the value actually typed, so its width says nothing about where the text lands: under the default `"left"` the value starts at the box's left edge and the leftover space trails off to the right. Right-aligning the box in Typst does not help, because that moves the box, not the text inside it. Reach for `align: "right"` wherever a template calls for a right-aligned fill-in, as a USAF memo does for its date.

**`font` is limited to the three base-14 families** (`"helvetica"`, `"times"`, `"courier"`), which every PDF viewer is required to have. A widget cannot carry a font program, so a quill's own bundled fonts are not reachable here; pick the base-14 family closest to the surrounding type. `"times"` is a close match for the Times-alike faces most formal templates use.

These affect the PDF only. SVG and PNG reserve the same invisible layout space regardless.

### Errors

- Duplicate `name` across any `form-field`/`signature-field` calls → `typst::duplicate_form_field`.
- A non-absolute `width`/`height`/`size`, a `type` outside the four values, a `font`/`align` outside its set, a name violating `[A-Za-z0-9_.]+`, or a `field:` that is not a known schema address → a Typst assert pointing at `form-field`.
- `font`/`size`/`align` set to a non-default on a `"checkbox"` or `"signature"` field → a Typst assert.

The label `<__qm_field__>` and metadata `kind: "__qm_field__"` are reserved for this hand-off: the same `query(metadata)` caveat noted for `signature-field` applies.

## Tying Composed Content to a Field

A live preview routes a click back to the schema field that produced the ink under it, and it finds that field automatically for content it generated: a `richtext` field's markup, a `#data.subject` reference in your plate. Content your plate *composes* — a banner keyed off `data.classification`, an address block a vendored package lays out, a computed table — draws ink Quillmark cannot attribute to anything. `field-region` claims it:

```typst
#import "@local/quillmark-helper:0.1.0": data, field-region

#let banner(level) = box(stroke: 1pt, inset: 6pt)[#upper(level)]

#field-region("classification")[#banner(data.classification)]
```

The banner now appears in `session.regions()` under `classification` and a click on it resolves through `session.fieldAt(...)`, exactly as if the field had drawn it.

`body` is returned untouched, bracketed by two invisible `metadata` markers, so the wrapper changes nothing about layout or output bytes. Unlike a `form-field` widget it reserves no space and draws no click target of its own: it claims the ink that is already there.

### What it claims

A claim is a **fallback**, not an override. Ink already tracked to a field keeps that field, and the wrapper takes only what is left:

```typst
#field-region("recipient")[
  #line(length: 2in)          // no field of its own → claimed for `recipient`
  #data.body                  // a richtext field → stays `body`
  Prepared by #data.author    // a scalar reference → stays `author`
]
```

Nesting therefore reads as ordinary scoping, and wrapping never moves a region off the field that generated it. The flip side: you cannot use `field-region` to *retarget* ink that is already attributed. Ink Typst attributes to no source position at all — list bullets, underline rules — stays unclaimed here as it is everywhere else.

Each **call** claims independently, so `field` need not be a literal and a wrapper used once per card yields one region per card:

```typst
#for card in data.at("$cards", default: ()) {
  field-region(card.at("$path") + "$body", render-card(card))
}
```

That is the way to give a card's *scalar* fields regions: read from the loop variable, they carry no per-instance identity of their own.

### Parameters and errors

| Name | Type | Default | Meaning |
|------|------|---------|---------|
| `field` | `str` | required (positional) | Schema address: a field name, an array element like `"refs.2"`, or a card path built from the card's `$path` prefix. |
| `body` | any content | required (positional) | Returned unchanged; its ink is what gets claimed. |

- A `field` that is not a known schema address, or is not a string, raises a Typst assert pointing at `field-region`.
- A claim whose content Typst lays out somewhere else entirely (`#place`, a float) claims whatever ink lands between its markers instead; wrap the placed content rather than the `place` call.
- Emit the call's return value whole. Splitting it — passing `.children` through separately, say — can land the opening marker in a frame without its closing one, and a claim with no end would otherwise take every unattributed piece of ink to the end of the document. Such a claim is dropped entirely and reported as a `typst::unclosed_field_region` warning naming the field.

The label `<__qm_region__>` and metadata `kind: "__qm_region__"` are reserved for this hand-off: the same `query(metadata)` caveat applies.

## Output Formats

PDF and SVG render as a single artifact. PNG renders one artifact per page.

Python binding (rendering lives on the engine, not the quill):

```python
from quillmark import OutputFormat
result = engine.render(quill, doc, OutputFormat.PDF)   # or .SVG, .PNG
```

WASM/JS binding (rendering lives on the engine, not the quill):

```javascript
engine.render(quill, doc, { format: 'png' });           // 144 PPI
engine.render(quill, doc, { format: 'png', ppi: 300 });  // print quality
```

PNG resolution is set via the `ppi` option (default **144**, 2× at 72pt/inch, suitable for retina previews):

| PPI | Use case |
|-----|----------|
| 72  | Low-res web thumbnails |
| 144 | Retina screen preview (2×) |
| 192 | High-DPI screen display |
| 300 | Standard print quality |
| 600 | High-quality print / archival |

## Resources

- [Typst Documentation](https://typst.app/docs/)
- [Typst Universe](https://typst.app/universe/): package directory

## Next Steps

- [Create your own Typst Quill](creating-quills.md)
- [Learn about Markdown syntax](../authoring/markdown-syntax.md)
