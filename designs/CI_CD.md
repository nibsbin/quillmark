# Quillmark Rust Workspace — Basic CI/CD

**Status**: Implemented
**Scope**: Build, test, and publish the following crates to crates.io:

* `quillmark-core` (publish ✅)
* `quillmark-typst` (publish ✅; depends on core)
* `quillmark` (publish ✅; depends on core & typst)

`quillmark-fixtures` is internal (not published).

**Publication order**: `quillmark-core` → `quillmark-typst` → `quillmark`.

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

**Goal**: Publish the three crates to crates.io in dependency order.

* **Trigger**: One of:

  * Manual dispatch with a `version` input (recommended for simplicity), **or**
  * A pushed tag like `vX.Y.Z` (optional).
* **Auth**: `CARGO_REGISTRY_TOKEN` stored as a repository secret.
* **Prechecks**:

  * All three crates share the same version in `Cargo.toml`.
  * CI on `main` is green.
  * Dry-run publishing succeeds for each crate.
* **Publish sequence**:

  1. `quillmark-core`
  2. Wait briefly for crates.io indexing.
  3. `quillmark-typst`
  4. Wait briefly for crates.io indexing.
  5. `quillmark`
* **Post-publish**: Verify crates appear on crates.io and docs.rs builds succeed.

---

## 3) Versioning (Minimal)

* **SemVer** across all three crates.
* **Lockstep versions**: bump `quillmark-core`, `quillmark-typst`, and `quillmark` together in one commit.
* **Tagging**: Optional but recommended (`vX.Y.Z`) if you want tag-based triggers.

---

## 4) Minimal Release Steps

1. **Prepare**

   * Ensure `main` is green.
   * Bump versions in all three `Cargo.toml` files to `X.Y.Z`.
   * Commit (and optionally tag `vX.Y.Z`).

2. **Publish**

   * Run the Publish workflow:

     * Either dispatch manually with `version = X.Y.Z`, or
     * Push tag `vX.Y.Z` if using tag triggers.

3. **Verify**

   * Confirm crate pages on crates.io for all three.
   * Confirm docs.rs builds complete.

---

## 5) Readiness Checklist (per release)

* [ ] CI checks pass on `main`.
* [ ] Versions synchronized across `quillmark-core`, `quillmark-typst`, `quillmark`.
* [ ] Dry-run `cargo publish` succeeds for all three.
* [ ] README and crate metadata are accurate.
* [ ] Publish completed in the correct order.

---

## 6) Out of Scope (intentionally omitted)

* Clippy/lint gates
* Multi-OS testing or MSRV gates
* Security auditing and dependency automation
* Coverage, benchmarks, and performance tracking
* Python/NPM workflows and cross-ecosystem orchestration

---

This is the leanest possible plan that still gives you reliable builds and a safe, ordered publish to crates.io.
