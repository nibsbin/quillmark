# Quillmark Fuzzing Tests

Property-based fuzz tests over Quillmark's escaping functions, parsers, and JSON decode lanes, built on `proptest` rather than `cargo-fuzz`. Unpublished; internal testing only.

```bash
cargo test --package quillmark-fuzz            # everything
cargo test --package quillmark-fuzz pdf_fuzz   # one module
```

The crate is excluded from `default-members`, so a bare `cargo test` skips it; `cargo test --workspace` includes it.

## Modules

| Module | Target |
|---|---|
| `coerce_fuzz.rs` | `QuillConfig::coerce_payload`: no panic on arbitrary `(FieldSchema, Value)` pairs, well-formed error paths, idempotent successful coercions |
| `convert_fuzz.rs` | Markdown → Typst: `escape_string` / `escape_markup` in `quillmark-typst`, and the import-then-lower render path |
| `decode_fuzz.rs` | The four JSON decode lanes (storage DTO, card wire, canonical content, op wire) where arbitrary JSON yields `Err`, never a panic |
| `emit_roundtrip_fuzz.rs` | `parse → emit → re-parse` stability, and idempotence on the canonical form |
| `parse_fuzz.rs` | card-yaml payloads: malformed YAML, composable card kinds, nested structures, Unicode and special characters |
| `pdf_fuzz.rs` | `quillmark-pdf`'s byte-level PDF reads (`page_media_boxes`, `PdfUpdate::begin`, `stamp`): arbitrary bytes, and a real AcroForm truncated, single-byte-corrupted, or spliced, yield `Err`, never a panic |

The properties these hold: escaped output never breaks out of its Typst string or markup context, deeply nested and oversize input parses without panicking, arbitrary Unicode survives, and the hand-rolled PDF reader refuses corrupt bytes rather than panicking.

A new escaping, parsing, or decoding surface gets a fuzz target here.
