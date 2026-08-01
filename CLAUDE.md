# Quillmark

Schema-driven document engine: Markdown + YAML card metadata → rendered PDF/SVG/PNG via a Typst or PDF-form backend.

Crates: `core` · `quillmark` · `content` · `backends/{typst,pdfform}` · `quillmark-pdf` · `bindings/{python,wasm,cli}` · `fixtures` · `fuzz`. What each carries: [`ARCHITECTURE.md`](prose/canon/ARCHITECTURE.md) §"Crate Structure".

Design docs: [`prose/canon/INDEX.md`](prose/canon/INDEX.md). Comments and docs follow the `dense-prose` skill.

- Released guides in [`docs/migrations/`](docs/migrations/) are immutable; edit only the unreleased one.
- The `Cargo.toml` version is the last *released* one; CI bumps it on release.
- Commit early and often; CI gates every push.
- Don't run `cargo fmt`.

## Tests

- `cargo test --workspace` is the working loop; run it freely.
- The binding surfaces cost minutes to compile and PR CI runs both on every push. Build them locally only to reproduce a red CI job, or when the change is in that binding's own code and no Rust test reaches it. Rust tests passing is not a reason to rebuild them.
- WASM: `./scripts/build-wasm.sh --ci && cd crates/bindings/wasm && npm test`. `--ci` selects the fast-compile profile. Bare `build-wasm.sh` is the publish build (opt-level=z, fat LTO) and belongs to release.
- Python: `cd crates/bindings/python && uv run maturin develop && uv run pytest`. `maturin develop` builds debug; keep it there. `--release` and `pip install -e .` (release by PEP 517 default) both spend minutes on an opt-level no test outcome depends on.
