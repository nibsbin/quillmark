# Quillmark Fuzzing Tests

Property-based fuzz tests over Quillmark's escaping functions, parsers, and JSON decode lanes, built on `proptest` rather than `cargo-fuzz`.

**Note:** This crate is not published to crates.io and is only used for internal testing.

## Quickstart

Run all property-based fuzzing tests:

```bash
cargo test --package quillmark-fuzz
```

Or from the `quillmark-fuzz` directory:

```bash
cd crates/fuzz
cargo test
```

Run a specific test module:

```bash
cargo test --package quillmark-fuzz coerce_fuzz
cargo test --package quillmark-fuzz convert_fuzz
cargo test --package quillmark-fuzz decode_fuzz
cargo test --package quillmark-fuzz emit_roundtrip_fuzz
cargo test --package quillmark-fuzz parse_fuzz
cargo test --package quillmark-fuzz pdf_fuzz
```

**Note:** This crate is excluded from `default-members` so expensive fuzzing does not run on every `cargo test`. Use `cargo test --workspace` to include it.

## Modules

| Module | Target |
|---|---|
| `coerce_fuzz.rs` | `QuillConfig::coerce_payload`: no panic on arbitrary `(FieldSchema, Value)` pairs, well-formed error paths, idempotent successful coercions |
| `convert_fuzz.rs` | Markdown → Typst: `escape_string` / `escape_markup` in `quillmark-typst`, and the import-then-lower render path |
| `decode_fuzz.rs` | The four JSON decode lanes (storage DTO, card wire, canonical content, op wire) where arbitrary JSON yields `Err`, never a panic |
| `emit_roundtrip_fuzz.rs` | `parse → emit → re-parse` stability, and idempotence on the canonical form |
| `parse_fuzz.rs` | card-yaml payloads: malformed YAML, composable card kinds, nested structures, Unicode and special characters |
| `pdf_fuzz.rs` | `quillmark-pdf`'s byte-level PDF reads (`page_media_boxes`, `PdfUpdate::begin`, `stamp`): arbitrary bytes, and a real AcroForm truncated, single-byte-corrupted, or spliced, yield `Err`, never a panic |

## Security properties

1. **No injection**: quotes are always escaped in string contexts, so nothing breaks out; including `\"); eval(...)`-shaped payloads.
2. **Escaping completeness**: every Typst special character is escaped in markup context.
3. **Control-character safety**: null bytes and ASCII control characters escape as `\u{...}`.
4. **Backslash handling**: backslashes escape first, so nothing double-escapes.
5. **DoS resistance**: deeply nested input (blockquotes and lists to 20 levels) and large input (to 10,000 characters) parse without panicking.
6. **Unicode safety**: arbitrary Unicode input does not crash.
7. **Binary-input safety**: the hand-rolled PDF reader refuses corrupt, truncated, and non-PDF bytes rather than panicking. Nothing in the workspace catches unwind, so a panic there kills the CLI and the Python extension and poisons the WASM module.

## Contributing

A new escaping, parsing, or decoding surface gets a fuzz target here.

## References

- [proptest documentation](https://docs.rs/proptest/) for property-based testing guidelines
