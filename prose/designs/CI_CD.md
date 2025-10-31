# Quillmark Rust Workspace — CI/CD

**Status**: Implemented
**Scope**: Build, test, and publish the following crates to crates.io:

* `quillmark-core` (publish ✅)
* `backends/quillmark-typst` (publish ✅; depends on core)
* `backends/quillmark-acroform` (publish ✅; depends on core)
* `quillmark` (publish ✅; depends on core, typst, and acroform)

`quillmark-fixtures`, `quillmark-fuzz`, `bindings/quillmark-python`, and `bindings/quillmark-wasm` are internal/bindings (not published to crates.io).

**Publication order**: Automated via `cargo publish` which handles dependency order: `quillmark-core` → `backends/quillmark-typst` and `backends/quillmark-acroform` → `quillmark`.

---

## 1) Continuous Integration (CI)

**Goal**: Fast feedback on PRs and pushes to `main`, with minimal surface area.

* **Trigger**: pull requests and pushes to `main`.
* **Environment**: Single Linux runner.
* **Steps**:

  1. **Checkout & toolchain**: Use stable Rust.
  2. **Dependency cache**: Cache Cargo artifacts for speed.
  3. **Check**: `cargo check` (with all features, then with no-default-features).
  4. **Test**: `cargo test` (workspace, all features; include doctests).
  5. **Format**: `cargo fmt -- --check`.
  6. **Docs build**: `cargo doc --no-deps` (ensure docs compile).

> Excluded by design: Clippy linting, multi-OS test matrix, MSRV verification, security scanners, coverage, and benchmarks.

---

## 2) Continuous Delivery (CD) — Publishing

**Goal**: Publish crates to crates.io, Python packages to PyPI, and WASM packages to npm.

### Rust Crates

* **Trigger**: One of:

  * Manual dispatch (`workflow_dispatch`), **or**
  * A pushed tag like `vX.Y.Z`
* **Auth**: `CARGO_REGISTRY_TOKEN` stored as a repository secret.
* **Prechecks**:

  * All crates share the same version in workspace `Cargo.toml`.
  * CI on tag/main is green.
  * Tests pass with default features, all features, and no default features.
* **Publish sequence**:

  * Uses `cargo publish` which automatically handles dependency order
  * Publishes: `quillmark-core`, then `backends/quillmark-typst` and `backends/quillmark-acroform`, then `quillmark`

### Python Bindings

* **Workflow**: `.github/workflows/publish-python.yml`
* **Trigger**: Tag push `vX.Y.Z` or manual dispatch
* **Platform**: Builds wheels for Linux, macOS, and Windows
* **Publish**: PyPI via `maturin publish`

### WASM Bindings

* **Workflow**: `.github/workflows/publish-wasm.yml`
* **Trigger**: Tag push `vX.Y.Z` or manual dispatch
* **Build**: Uses `wasm-pack` to build for bundler, nodejs, and web targets
* **Publish**: npm via `wasm-pack publish`

---

## 3) Versioning

* **SemVer** across all workspace crates and bindings.
* **Lockstep versions**: bump `quillmark-core`, `backends/quillmark-typst`, `backends/quillmark-acroform`, and `quillmark` together in one commit.
* **Bindings**: Python and WASM bindings follow the same version as the Rust workspace.
* **Tagging**: Required for publishing (`vX.Y.Z` triggers automated workflows).

---

## 4) Release Steps

1. **Prepare**

   * Ensure `main` is green.
   * Bump versions in workspace `Cargo.toml` to `X.Y.Z`.
   * Commit and tag `vX.Y.Z`.
   * Push tag to trigger automated workflows.

2. **Automated Publishing**

   * Rust crates published to crates.io
   * Python wheels published to PyPI
   * WASM packages published to npm

3. **Verify**

   * Confirm crate pages on crates.io for all published crates.
   * Confirm docs.rs builds complete.
   * Verify PyPI package availability.
   * Verify npm package availability.

---

## 5) Readiness Checklist (per release)

* [ ] CI checks pass on `main`.
* [ ] Versions synchronized across workspace crates.
* [ ] Tests pass with default, all, and no-default features.
* [ ] README and crate metadata are accurate.
* [ ] Tag `vX.Y.Z` created and pushed.

---

## 6) Out of Scope (intentionally omitted)

* Clippy/lint gates
* Multi-OS testing for Rust crates (Python and WASM test on multiple platforms)
* MSRV gates
* Security auditing and dependency automation
* Coverage, benchmarks, and performance tracking

---

This CI/CD setup provides reliable builds and safe, ordered publishing to crates.io, PyPI, and npm.

---

## Cross-References

**Related Design Documents:**
- [ARCHITECTURE.md](ARCHITECTURE.md) - Overall architecture
- [PYTHON.md](PYTHON.md) - Python build and distribution
- [WASM.md](WASM.md) - WebAssembly build and distribution

**Implementation:**
- `.github/workflows/` - GitHub Actions workflows
- `scripts/` - Build and release automation scripts
