# Quillmark

Schema-driven document engine: Markdown + YAML card metadata → rendered PDF/SVG/PNG via a Typst or PDF-form backend.

Crates: `core` · `quillmark` · `content` · `backends/{typst,pdfform}` · `quillmark-pdf` · `bindings/{python,wasm,cli}` · `fixtures` · `fuzz`. What each carries: [`ARCHITECTURE.md`](prose/canon/ARCHITECTURE.md) §"Crate Structure".

Design docs: [`prose/canon/INDEX.md`](prose/canon/INDEX.md). Comments and docs follow the `dense-prose` skill.

- Released guides in [`docs/migrations/`](docs/migrations/) are immutable; edit only the unreleased one.
- The `Cargo.toml` version is the last *released* one; CI bumps it on release.
- Commit early and often; CI gates every push.
- Don't run `cargo fmt`.

## Tests

- `cargo test --workspace`
- WASM: `./scripts/build-wasm.sh && cd crates/bindings/wasm && npm test`
- Python: `cd crates/bindings/python && uv run maturin develop && uv run pytest`
- Binding surfaces build slowly; defer `bindings/{python,wasm}` to PR CI.
