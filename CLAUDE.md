# Quillmark

Schema-driven document engine: Markdown + YAML card metadata → rendered PDF/SVG/PNG via a Typst or PDF-form backend.

Crate layout and what each crate carries: [`ARCHITECTURE.md`](prose/canon/ARCHITECTURE.md) §"Crate Structure".

Design docs: [`prose/canon/INDEX.md`](prose/canon/INDEX.md).

Comments default to none, and one earns its place only where the code cannot carry the fact itself. What survives states what is: present tense, unsold, no history. The `dense-prose` skill is the whole policy.

- The `Cargo.toml` version is the last *released* one; CI bumps it on release.
- A `CHANGELOG.md` conflict resolves itself: `.gitattributes` marks it `merge=union`, so merge `main` with local git rather than pressing "Update branch", which does not read it.
- Don't run `cargo fmt`.

## Tests

`cargo test --workspace` is the working loop; run it freely. The binding surfaces below compile in minutes and PR CI runs both on every push: build one locally only to reproduce a red CI job, or for a change in that binding's own code no Rust test reaches.

- WASM: `./scripts/build-wasm.sh --ci && cd crates/bindings/wasm && npm test`. `--ci` is the fast-compile profile; bare `build-wasm.sh` is the publish build.
- Python: `cd crates/bindings/python && uv venv && source .venv/bin/activate && uv pip install maturin pytest && maturin develop && pytest`. `maturin develop` builds debug.
- `uv run` in that directory syncs the project first, and the sync is maturin's PEP 517 backend building release: `uv run maturin --version` compiles the extension to print a version string. The flow above is CI's (`.github/workflows/ci.yml`) and never syncs.

A test earns its place by exercising logic that can break: parsing, assembly, emission, diffing, resolution, validation, error classification, round-trips, rendering. It earns nothing by restating a rule the implementation or canon already carries, and deleting a stale test is maintenance.

- Diagnostic codes, wire-format keys, and public API names are contract; human-readable error prose, derive output, and constant literals are not.
- One strong test beats three angles on the same behavior.
- Assert the property, not the artifact: byte counts, hashes, and full-string snapshots of rendered output rot.

## Commits and releases

The changelog is seeded from commit subjects, so the `!` marker is where a break is first recorded and the last place to catch a missing one.

- `!` marks an observable-contract shift, not only a type change. A region address, a wire token, a diagnostic code, a rendered value — anything a working consumer reads that stops meaning what it meant. A break a type checker cannot report is the one that most needs the marker.
- A release carrying a `!` ships `docs/migrations/<prev>-to-<next>.md`, and a row in `docs/migrations/index.md` and `mkdocs.yml`'s `not_in_nav`. Lead the guide with the breaks no type checker reports; the changelog is organized by feature, which is the wrong index for an upgrade.
