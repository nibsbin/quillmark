# Plate Data Injection

> **Implementation**: `crates/backends/typst/src/`

## TL;DR

Plates get document data through a backend-injected virtual Typst package, not a template engine. Data flows in two stages: `Quill::compile_data()` produces validated, zero-filled JSON in which content fields are canonical `Content` objects; `Backend::open()` generates the helper's `lib.typ`, lowering each to Typst markup at codegen (and dates to click-to-edit value-objects wrapping `datetime(..)`), no per-field markdown re-parse.

## Overview

1. `Quill::compile_data()` coerces, validates, normalizes, and **zero-fills** the
   root-block fields, and each composable card's fields against its `card_kind`
   schema, into a plain JSON object: every absent schema field resolves to its
   authored value, else the schema `default:`, else its type-empty zero value.
   Content fields cross as canonical `Content` objects (coercion imports an
   authored markdown string to the `Content` and re-canonicalizes an
   editor-supplied one). An incomplete document still renders: an absent or
   present-null field zero-fills, and a `!must_fill` marker uses its suggested
   value or zero-fills. Only a malformed value: one that won't coerce or
   validate to its type: errors.
2. `Backend::open()` receives that JSON and generates the helper package. Content fields lower to Typst markup at codegen via `emit::emit_content`, dates to value-objects wrapping `datetime(..)`; a direct `apply` path revalidates dates. There is no markdown-string transform.

### Data Shape

- Document-level metadata uses `$`-prefixed keys: `$quill` (quill ref string), `$body` (root prose body, a canonical `Content` object, present when the main enables a body), `$cards` (array of card objects)
- Each card object carries its user fields flat, a `$kind` discriminator when the card authors one, and a `$body` (card prose body, a content object) when the card's kind enables a body
- **`$`-metadata is present exactly where the schema defines it** ("absent on
  undefined"). Which definition gates the key splits the rule:
  - `$kind` is *document-defined*: present iff the card authors one, absent for
    a kindless card.
  - `$body` is *schema-defined*: present iff a declared kind enables a body,
    absent for a body-disabled or unknown kind. A present `$body` is always a
    content object, never a raw object needing a type check.

  Absence is the signal. Read `$`-metadata with a total accessor:
  `card.at("$kind", default: none)`, `card.at("$body", default: "")`: never a
  bare `card.$body`
- User payload fields sit flat at the root next to the `$` keys; field names match `[a-z_][a-z0-9_]*` and therefore never collide with `$` metadata

## Typst Helper Package

The Typst backend injects a virtual package `@local/quillmark-helper:<version>` that exposes the JSON to plates and provides helpers.

```typst
#import "@local/quillmark-helper:0.1.0": data

#data.title                  // plain field access
#data.at("$body")            // root $body: a content object when the main enables a body
#(data.date.display)("…")    // date/datetime fields are value-objects; .value is the native datetime
#for card in data.at("$cards") {
  if card.at("$kind", default: none) == "indorsement" {
    // per-kind handling; $kind/$body are present only where the schema
    // defines them, so read them totally: card.<field>, card.at("$body", default: "")
  }
}
```

The `$`-prefixed keys must be accessed via `.at("$...")` because Typst identifiers do not include `$`.

Helper contents (generated in `backends/typst/helper.rs` from `lib.typ.template`):

- `data`: a backend-generated Typst dictionary **literal** of all fields, no runtime processing, no `__meta__` sentinel. The backend classified and lowered every field at generation time, reading classification from the session's cached `SchemaMeta`.
- Content fields lower to Typst content: each field's `Content` value lowered
  by `emit::emit_content`. Two schema shapes qualify (see
  `content_field_names`), both classified on `contentMediaType:
  application/quillmark-content+json`:
  - a scalar richtext field (`{type: object, contentMediaType:
    application/quillmark-content+json}`): one `Content` object, lowered in
    place.
  - `array<richtext>` (`{type: array, items: {type: object, contentMediaType:
    application/quillmark-content+json}}`): each array element lowered
    individually.

  Each non-blank content is emitted as a markup **block** binding (`#let _qm_cN
  = [ .. ]`) that `data` references; a blank content stays an empty string
  literal.

  A `richtext(inline)` field (`quillmark:inline: true`, classified by
  `inline_field_names`) instead lowers via `emit::emit_content_inline` to **pure
  inline** markup: the single `Para`'s content with no block terminator, so no
  `parbreak`. The value therefore composes in an inline slot (`par(..)`, a grid
  cell) without Typst's "parbreak may not occur inside of a paragraph" warning.

  A `plaintext` field rides the *same* media type (plus an editor-only
  `quillmark:plain: true`), so `content_field_names` classifies it identically
  and it lowers through this exact path. The codec differs only at
  authoring/coercion (literal `from_plaintext`), never at codegen.
- `date` / `datetime` fields (`format: date` / `format: date-time`) lower to a
  **value-object**, a `#let _qm_dN = { let v = datetime(..); (value: v, display:
  (..args) => text(v.display(..args))) }` block the data cell references (blank ⇒
  `none`). `v` is the three-component `datetime(year:, month:, day:)` for a
  `date`, the six-component `datetime(year:, .., second:)` (authored wall-clock,
  seconds zero-filled) for a `datetime`: the distinct transform-schema `format`
  stamps the backend keys its per-type lowering on. The object exposes two
  projections and is the date sibling of the content block
  ([`date_object`](../../crates/backends/typst/src/helper.rs)):
  - `value`: the native `datetime` `v`, for arithmetic, comparison, `.year()`/`.weekday()`/… components, and datetime-consuming packages. `.value.display(..)` is the native `str`.
  - `display`: a closure `(..args) => text(v.display(..args))` called as
    `(data.<field>.display)(..)` (the paren form; Typst reserves dict-key method
    sugar). It returns *content*, so its glyphs are born at the generated
    `text(..)` node's site, inside a recorded **segment-less window** keyed by
    the field's schema path → one whole-placement **region** per emitted cell.

    This is what makes a date the first click-to-edit target. A closure's ink is
    born at its lexical definition site, not the reference, so the region
    survives the value being laundered (`#let d = card.on`) or handed into a
    vendored package that formats it internally. Emitting one `text(..)` node
    per cell manufactures the per-instance identity a shared loop variable
    (`card.<field>`) lacks: the case `span_scan`'s "Not chased" note describes,
    so each card's date surfaces its own region. Wrapping `v.display` (not a
    re-literalized date) inherits `v`'s type, so a `date`-only field's `display`
    throws Typst's native error on an `[hour]` pattern.

    Two Typst-language facts the paren form rests on, verified against **Typst
    0.15** and worth re-checking on a major bump: dict-key method sugar
    (`d.display(..)`) is a hard error, and grabbing `.display` off a *native*
    `datetime` without calling it is too: which is why `display-date`
    dispatches on `type(date)` instead of parenthesizing uniformly. Neither is
    pinned by a test: a test on Typst's error wording fails on a reworded
    message, which says nothing about a Quillmark regression.
- A **card's** declared `string`/`number` fields lower to the same value-object
  shape, `#let _qm_sN = { let v = <literal>; (value: v, display: () =>
  text(str(v))) }`, one block per card *cell*
  ([`scalar_object`](../../crates/backends/typst/src/helper.rs)). The motive is
  the date's, isolated: a plate reads a card field through the shared loop
  variable, one expression site for every instance, so only a per-cell `text(..)`
  node can key a region on `$cards.<kind>.<n>.<field>`. `str(v)` because a
  number is not content and `str` is a no-op on the string case.

  The lowering is deliberately narrow, and each boundary is a place the identity
  problem does not arise or the address does not exist:
  - **Root fields keep their literals.** A root field is placed through its own
    plate expression, which `scalar_windows` already windows; there is no shared
    site to disambiguate, and converting them would break every plate to buy
    nothing.
  - **Declared fields only** (`card_field_names`), so every emitted window keys
    on an address `_qm-known-path` would accept. `$kind` is document-defined,
    not a schema property, so card dispatch is untouched.
  - **Array elements stay raw.** `$cards.<k>.<n>.<field>.<i>` is a real address,
    but the plate reaches an element through a *second* shared site (the index
    or `.map`), so per-element identity needs its own pass rather than falling
    out of this one.
  - Booleans stay literals (`#if card.flag` must keep working, and a boolean
    renders as chrome the plate draws, not as its own ink); `none` stays `none`,
    so presence guards are unchanged.

  Unlike a date, a *blank* scalar still lowers here rather than degrading to
  `none`: that keeps `.value` total over a declared field, at the cost of a
  direct `card.f != ""` comparison silently becoming always-true — the one break
  in the migration a type error does not catch. A blank cell draws no ink and so
  surfaces no region.
- `plaintext(field)`: the sanctioned content→`str` coercion. Where
  `data.<field>` is Typst **content**, `plaintext(field)` returns the content
  field's plain text: the content text with island slots stripped and marks
  dropped (the same projection pdfform lowers a richtext field to). It reads a
  generated `_qm-plaintext` literal keyed by schema address (`subject`,
  `refs.2`, `$cards.<kind>.<n>.<field>`); `""` for a blank field or an address
  with no content. Use it when a plate or package needs a string (string ops, an
  `assert(type(item) == str)` consumer) for any content field: `richtext` or
  `plaintext`.

  Note the name collision: this Typst helper is distinct from the `plaintext`
  **field type**. The helper projects *any* content to a `str`; the field type
  declares a field's content plain from the start.
