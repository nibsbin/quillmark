# Quillmark

Schema-driven document engine: Markdown + YAML card metadata → rendered PDF/SVG/PNG via a Typst or PDF-form backend.

Crate layout and what each crate carries: [`ARCHITECTURE.md`](prose/canon/ARCHITECTURE.md) §"Crate Structure".

Design docs: [`prose/canon/INDEX.md`](prose/canon/INDEX.md). Comments and docs follow the `dense-prose` skill.

- Released guides in [`docs/migrations/`](docs/migrations/) are era-stamped: repair a false statement in place, never restate a true one in a later version's vocabulary.
- The `Cargo.toml` version is the last *released* one; CI bumps it on release.
- Commit early and often; CI gates every push.
- Don't run `cargo fmt`.

## Tests

- `cargo test --workspace`: the working loop, run freely.
- Binding surfaces compile in minutes and PR CI runs both on every push. Build one locally only to reproduce a red CI job, or when the change is in that binding's own code and no Rust test reaches it.
- WASM: `./scripts/build-wasm.sh --ci && cd crates/bindings/wasm && npm test`. `--ci` is the fast-compile profile; bare `build-wasm.sh` is the publish build.
- Python: `cd crates/bindings/python && uv venv && source .venv/bin/activate && uv pip install maturin pytest && maturin develop && pytest`. `maturin develop` builds debug.
- `uv run` in that directory syncs the project first, and the sync is maturin's PEP 517 backend building release: `uv run maturin --version` compiles the extension to print a version string. `--release` and `pip install -e .` reach the same build by hand. The flow above is CI's (`.github/workflows/ci.yml`) and never syncs.
