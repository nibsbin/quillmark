# Quillmark Rust Workspace — CI/CD

**Status**: Implemented

Published crates: `quillmark-core`, `backends/quillmark-typst`, `backends/quillmark-acroform`, `quillmark`.

Not published: `quillmark-fixtures`, `quillmark-fuzz`, `bindings/quillmark-python`, `bindings/quillmark-wasm`.

---

## 1) Continuous Integration (CI)

**Trigger**: pull requests and pushes to `main`.
**Environment**: single Linux runner.

Steps:
1. Checkout & stable Rust toolchain
2. Cache Cargo artifacts
3. `cargo check` (all features, then no-default-features)
4. `cargo test` (workspace, all features; doctests included)
5. `cargo fmt -- --check`
6. `cargo doc --no-deps`

Excluded: Clippy, multi-OS matrix, MSRV, security scanners, coverage, benchmarks.

---

## 2) Continuous Delivery (CD)

### Rust Crates

**Trigger**: manual dispatch (`workflow_dispatch`) or pushed tag `vX.Y.Z`.
**Auth**: `CARGO_REGISTRY_TOKEN` repository secret.

Publish sequence (via `cargo publish`): `quillmark-core` → `backends/quillmark-typst` and `backends/quillmark-acroform` → `quillmark`.

### Python Bindings

**Workflow**: `.github/workflows/publish-python.yml`
**Trigger**: tag push `vX.Y.Z` or manual dispatch
**Publish**: PyPI via `maturin publish` (Linux, macOS, Windows wheels)

### WASM Bindings

**Workflow**: `.github/workflows/publish-wasm.yml`
**Trigger**: tag push `vX.Y.Z` or manual dispatch
**Publish**: npm via `wasm-pack publish` (bundler, nodejs, web targets)

---

## 3) Versioning

- SemVer across all workspace crates and bindings
- Bump `quillmark-core`, `backends/quillmark-typst`, `backends/quillmark-acroform`, and `quillmark` together
- Python and WASM bindings follow the same version as the Rust workspace
- Tag `vX.Y.Z` required for publishing
