# Quillmark Parser Implementation

Implementation notes for `quillmark-core/src/parse.rs`.

> **Specification**: See [EXTENDED_MARKDOWN.md](./EXTENDED_MARKDOWN.md) for the authoritative syntax standard.
> **Cards**: See [CARDS.md](./CARDS.md) for card block semantics.

## Architecture

### ParsedDocument

```rust
pub struct ParsedDocument {
    fields: HashMap<String, QuillValue>,
    quill_ref: QuillReference,
}
```

- `fields` holds all frontmatter key/value pairs plus two reserved entries: `BODY` (global document body as a string) and `CARDS` (array of card objects, always present, may be empty).
- `quill_ref` is stored separately as a `QuillReference` (name + `VersionSelector`); it is never included in `fields`.
- Access via `body()`, `get_field()`, `fields()`, `quill_reference()`.
- `with_defaults()` returns a new `ParsedDocument` with default values applied for missing fields; existing fields are preserved.

### Parsing Flow

1. Check input size (max 10 MB; each YAML block max 1 MB).
2. Scan for `---\n` / `---\r\n` delimiters that are at the start of a line and not inside a fenced code block (backtick or tilde, CommonMark rules).
3. For each found block, parse YAML content via `serde-saphyr` with a depth budget (max 100 levels) and extract special keys:
   - `QUILL` — Quill name + optional version selector; valid only in the first block.
   - `CARD` — card type discriminator; required in all blocks after the first.
   - `BODY` and `CARDS` are reserved and rejected if found in any YAML block.
4. Validate block roles:
   - Block 0: may carry `QUILL` (and optional other fields), or be plain global frontmatter (no `QUILL`, no `CARD`). Cannot have both `QUILL` and `CARD`.
   - Blocks 1+: must carry `CARD`. `QUILL` is forbidden here.
5. Assemble global frontmatter fields from block 0 (or the `QUILL` block if they overlap).
6. For each `CARD` block, build an item object with its YAML fields, a `CARD` discriminator, and a `BODY` field containing the Markdown between this block's closing `---` and the next block's opening `---` (or EOF). Append to `cards_array`.
7. Extract global body: text between the end of the first non-card block and the start of the first card block (or EOF).
8. Insert `BODY` and `CARDS` into `fields`; check total field count (max 1 000).
9. Require `QUILL` to have been found; parse it as `QuillReference`; return `ParsedDocument`.

### CARD name validation

CARD field names must match `[a-z_][a-z0-9_]*`.

### QUILL name validation

Quill names must match `[a-z_][a-z0-9_]*`. Version selectors follow `@MAJOR`, `@MAJOR.MINOR`, `@MAJOR.MINOR.PATCH`, or `@latest`.

## Design Decisions

### Error Handling

The parser returns `ParseError` variants; these are converted to `RenderError::InvalidFrontmatter` upstream.

| Situation | Error |
|---|---|
| Missing `QUILL` | `InvalidStructure` |
| Malformed YAML | `YamlErrorWithLocation` (includes line + block index) |
| `QUILL` + `CARD` in same block | `InvalidStructure` |
| `QUILL` in non-first block | `InvalidStructure` |
| Inline block without `CARD` | `MissingCardDirective` (with hint) |
| Reserved field name used | `InvalidStructure` |
| Field name collision | `InvalidStructure` |
| Input/block too large | `InputTooLarge` |
| Invalid card name | `InvalidStructure` |

### Line Endings

Supports both `\n` and `\r\n`.

### YAML Parsing

Uses `serde-saphyr` for YAML → `serde_json::Value` conversion, then wraps in `QuillValue`. A `serde_saphyr::Budget` limits nesting depth at the parser level to prevent stack overflow.

### Security Limits

| Limit | Value |
|---|---|
| Max document size | 10 MB |
| Max single YAML block | 1 MB |
| Max YAML nesting depth | 100 |
| Max CARD blocks | 1 000 |
| Max field count | 1 000 |
