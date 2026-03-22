# Quillmark Parser

Status: **Implemented** (2026-03-22)  
Source: `crates/core/src/parse.rs`

Canonical contract for turning markdown with frontmatter into a `ParsedDocument`.

## Responsibilities
- Detect metadata blocks delimited by `---`.
- Disambiguate metadata vs. horizontal rules (`---` with blank lines above **and** below is body content).
- Parse YAML via `serde_saphyr`, convert to `QuillValue`.
- Enforce reserved keys and tag rules, then assemble the final field map.
- Attach a quill reference (defaults to `__default__@latest`).

## Rules
- **Reserved keys:** `BODY`, `CARDS` are rejected in any block. `QUILL` and `CARD` cannot appear together in one block.
- **Tag names:** `CARD` values must match `[a-z_][a-z0-9_]*`.
- **Block types:** First block may be global or a card; every subsequent block must be a card block.
- **Delimiters:** Only lines with exactly `---` start/end metadata. Inside fenced code blocks (strict ``` only), `---` is ignored.
- **Horizontal rules:** `---` with blank lines above and below is treated as body text, not metadata.
- **Quill selection:** If no `QUILL` key is present, `ParsedDocument` stores `QuillReference::latest("__default__")`.
- **Output shape:** Always emits `BODY` (string) and `CARDS` (array, possibly empty) in the returned field map.
- **Limits:** Enforced via `error.rs` constants – 10 MB input, 1 MB YAML per block, max YAML depth 100, max 1000 cards/fields.

## Flow
1. Scan for `---` delimiters, skipping ones inside strict fences.
2. Classify delimiter as metadata or horizontal rule.
3. Parse YAML for each metadata block; strip empty/whitespace-only blocks.
4. Validate tags/reserved keys; collect card blocks into unified `CARDS`.
5. Capture body text between blocks unchanged.
6. Build `ParsedDocument { fields, quill_ref }`.

See [EXTENDED_MARKDOWN.md](./EXTENDED_MARKDOWN.md) for the full surface syntax and [CARDS.md](./CARDS.md) for card semantics.
