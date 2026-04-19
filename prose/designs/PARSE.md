# Quillmark Parser Implementation

Implementation notes for `quillmark-core/src/parse.rs`.

> **Specification**: See [EXTENDED_MARKDOWN.md](./EXTENDED_MARKDOWN.md) for the authoritative syntax standard.

## Architecture

### ParsedDocument

Stores fields and body in a single `HashMap<String, QuillValue>`.

- Body stored under `BODY_FIELD = "BODY"`
- Quill reference is required from top-level `QUILL` frontmatter
- Access via `body()`, `get_field()`, `fields()`, `quill_reference()`

### Parsing Flow

1. Scan for `---` delimiters
2. Classify as metadata block or horizontal rule (per disambiguation rules)
3. Parse YAML, extract CARD/QUILL keys
4. Extract body content between blocks
5. Aggregate card blocks into `CARDS` array
6. Validate and assemble result

## Design Decisions

### Error Handling

Fail-fast on malformed YAML. Top-level frontmatter must include `QUILL`; missing `QUILL` returns `ParseError::InvalidStructure`.

### Line Endings

Supports both `\n` and `\r\n`.

### YAML Parsing

Uses `serde-saphyr` for YAML → `serde_json::Value` conversion, then converts to `QuillValue`.
