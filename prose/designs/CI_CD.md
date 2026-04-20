# Quillmark Rust Workspace — CI/CD

**Status**: Implemented

Published crates: `quillmark-core`, `backends/quillmark-typst`, `quillmark`.

Not published: `quillmark-fixtures`, `quillmark-fuzz`, `bindings/quillmark-python`, `bindings/quillmark-wasm`.

---

## 1) Continuous Integration (CI)

**Trigger**: pull requests and pushes to any branch except version tags.
**Jobs** (all Linux, run in parallel; `ci-success` gate aggregates results):

| Job | What it does |
|-----|-------------|
| `lint` | `cargo fmt --all -- --check` (Clippy commented out, not yet enforced) |
| `test` | `cargo test --locked` in a matrix: default features and `--all-features` |
| `docs` | `cargo doc --no-deps --locked` with `-Dwarnings` |
| `wasm` | `cargo check --package quillmark-wasm --target wasm32-unknown-unknown --locked` |

Excluded: multi-OS matrix, MSRV, security scanners, coverage, benchmarks.

---

## 2) Continuous Delivery (CD)

### Rust Crates (`publish.yml`)

**Trigger**: tag `vX.Y.Z` or manual dispatch.

1. Runs `cargo test` matrix (same as CI, fail-fast enabled).
2. Runs `cargo publish --locked --no-verify` to publish all publishable workspace crates.
**Auth**: `CARGO_REGISTRY_TOKEN` secret (via `Publish` environment).

### Python Bindings (`publish-python.yml`)

**Trigger**: tag `vX.Y.Z` or `py-vX.Y.Z`, or manual dispatch.

1. Runs pytest via `uv`.
2. Builds wheels via `maturin-action` for Linux (x86_64, aarch64), Windows (x64), macOS (aarch64) — Python 3.10–3.12 — plus sdist.
3. Publishes all artifacts to PyPI via `maturin upload`.
**Auth**: `MATURIN_PYPI_TOKEN` secret (via `Publish` environment).

### WASM Bindings (`publish-wasm.yml`)

**Trigger**: tag `vX.Y.Z` or `wasm-vX.Y.Z`, or manual dispatch.

1. Builds via `./scripts/build-wasm.sh` and runs `npm test`.
2. Publishes `@quillmark/wasm` to npm via `npm publish --provenance` (OIDC Trusted Publisher).
**Auth**: OIDC `id-token: write` permission; no token secret needed.

---

## 3) Versioning

- SemVer across all workspace crates and bindings.
- Version bumps managed locally via `cargo-release` (`release.toml`): bumps, commits (`chore: release X.Y.Z`), and pushes tag `vX.Y.Z`; publishing is delegated entirely to CI.
- Python and WASM bindings can also be released independently via `py-vX.Y.Z` / `wasm-vX.Y.Z` tags.
