# 03 — Frontmatter-Only Parse Entry Point

**Status:** Draft
**Depends on:** 01, 02 (Frontmatter type shape)
**Blocks:** nothing

## Background

`MetadataWidget` and `WizardCore` consume bare YAML fragments (the
contents that would live between `---` fences, without the fences
themselves). `Document.fromMarkdown` requires a full markdown document
with an opening `---` fence containing `QUILL:` and a closing fence,
which makes it unusable for the fragment case. The consumers therefore
ship their own YAML parser.

The parsed frontmatter type from taskings 01/02 is exactly what those
consumers want. Exposing a parse entry point that takes a YAML string and
returns that type lets them drop their parser.

## Change

Add a static method on `Document`:

```rust
impl Document {
    /// Parse a YAML fragment as frontmatter.
    ///
    /// The fragment must be bare YAML — no `---` fences. `QUILL:` is
    /// **not** required in a fragment; callers using this entry are
    /// assumed to be operating outside a full document context (e.g.
    /// editing a card's fields in isolation).
    pub fn parse_frontmatter(yaml: &str) -> Result<Frontmatter, ParseError>;
}
```

The return type is the `Frontmatter` from tasking 01 (the ordered
`FrontmatterItem` vec with map-keyed accessors and fill markers).

### Semantics

- No `---` fence handling — the input is the fence body.
- `QUILL:` is neither required nor rejected. If present, it's a normal
  field.
- Reserved-key validation (`CARD`, `BODY`, `CARDS`) still applies —
  those keys are rejected per MARKDOWN.md §3 even in a fragment.
- Comments and `!fill` behave exactly as in 01/02.
- Errors bubble through the same `ParseError` as `fromMarkdown`.

### WASM surface

```ts
class Document {
    static parseFrontmatter(yaml: string): Frontmatter;
}
```

`Frontmatter` serializes to JS as an object with the map-keyed fields
plus an `items` array (same shape tasking 01 will expose on
`Document.frontmatterItems`).

## Non-goals

- A bare-body parser or bare-card parser. User direction:
  "we likely don't need a markdown body parser for our document." Body
  and cards stay entry-point-less; consumers that want them parse a full
  document.
- Writing back. `Frontmatter` has no `toYaml()` in this tasking.
  Consumers that want to emit a fragment can do so via the existing
  `Document.toMarkdown` path after constructing a full document, or a
  follow-on tasking can add a dedicated emitter if demand materializes.

## Done when

- `Document::parse_frontmatter("recipient: !fill")` returns a
  `Frontmatter` with a single field whose `fill` is true and value is
  `""`.
- Reserved-key rejection tested.
- WASM `Document.parseFrontmatter` has a basic.test.js case.
- `MetadataWidget` / `WizardCore` have a documented migration path off
  their bespoke YAML parser (covered in a followup to `WASM_MIGRATION.md`
  once this lands).
