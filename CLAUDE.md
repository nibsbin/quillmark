# Quillmark

Schema-driven document engine: Markdown + YAML card metadata → rendered PDF/SVG/PNG via a Typst or PDF-form backend.

Crate layout and what each crate carries: [`ARCHITECTURE.md`](prose/canon/ARCHITECTURE.md) §"Crate Structure".

Design docs: [`prose/canon/INDEX.md`](prose/canon/INDEX.md).

What earns a test a place, and what the `!` commit marker obliges: [`CONTRIBUTING.md`](CONTRIBUTING.md).

Comments default to none, and one earns its place only where the code cannot carry the fact itself. What survives states what is: present tense, unsold, no history. The `dense-prose` skill is the whole policy.

- The `Cargo.toml` version is the last *released* one; CI bumps it on release.
- A `CHANGELOG.md` conflict resolves itself: `.gitattributes` marks it `merge=union`, so merge `main` with local git rather than pressing "Update branch", which does not read it.
- Don't run `cargo fmt`.

## Tests

`cargo test --workspace` is the working loop; run it freely. The binding surfaces below compile in minutes and PR CI runs both on every push: build one locally only to reproduce a red CI job, or for a change in that binding's own code no Rust test reaches.

- WASM: `./scripts/build-wasm.sh --ci && cd crates/bindings/wasm && npm test`. `--ci` is the fast-compile profile; bare `build-wasm.sh` is the publish build.
- Python: `cd crates/bindings/python && uv venv && source .venv/bin/activate && uv pip install maturin pytest && maturin develop && pytest`. `maturin develop` builds debug.
- `uv run` in that directory syncs the project first, and the sync is maturin's PEP 517 backend building release: `uv run maturin --version` compiles the extension to print a version string. The flow above is CI's (`.github/workflows/ci.yml`) and never syncs.
