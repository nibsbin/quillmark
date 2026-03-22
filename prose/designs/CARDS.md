# Cards

Status: **Implemented** (2026-03-22)  
Scope: Typed, repeatable content blocks parsed into a unified `CARDS` array.

## Canonical Behavior
- Parser collects every `---` block with `CARD: <name>` into `CARDS`, preserving order. Name regex: `[a-z_][a-z0-9_]*`.
- `CARDS` is always present (possibly empty). Each card object includes `CARD` and `BODY` plus any block fields.
- Global and card field names may overlap; only `BODY`/`CARDS` are reserved.

## Defining Cards (Quill.yaml)
```yaml
cards:
  product:
    title: "Product"
    description: "Catalog item"
    ui: { hide_body: true }          # optional
    fields:
      name:    { type: "string", required: true }
      price:   { type: "number" }
      details: { type: "markdown" }
```
- Field syntax matches document fields; supports defaults, examples, enum, nested objects/arrays, and `x-ui` hints (`group/order/visible_when/compact`).

## JSON Schema Shape
- Each card becomes a `$defs.<card>_card` object containing:
  - `CARD` const discriminator.
  - Properties for card fields + required list.
  - Optional `x-ui` (`hide_body`).
- `CARDS` property uses `oneOf` + discriminator mapping to those defs.

## Pipeline
1. Parse → `CARDS`.
2. `coerce_document` recurses into cards using their `$defs`.
3. Validation uses generated schema; defaults/examples applied after backend `transform_fields`.

## Consumption
- Typst helper exposes `data.CARDS[*]` (markdown fields pre-converted to Typst markup).
- AcroForm receives the same JSON for MiniJinja templating.
- WASM/Python/Rust expose identical JSON from `Workflow::compile_data()`.

Related: [SCHEMAS.md](SCHEMAS.md), [EXTENDED_MARKDOWN.md](EXTENDED_MARKDOWN.md), [PARSE.md](PARSE.md).  
Legacy scopes are documented only in [SCOPES.md](SCOPES.md) as superseded.
