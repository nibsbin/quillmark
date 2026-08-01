# Quillmark Rust Workspace: CI/CD

> **Implementation**: `.github/workflows/`

## TL;DR

Four workflows. `ci.yml` gates docs, the workspace tests, both binding surfaces, and the release tooling on every PR and non-tag push. `release-prepare.yml` computes the next version, bumps the workspace, and opens a release PR. `release.yml` tags and publishes to crates.io, npm, and PyPI when that PR merges. `docs.yml` builds MkDocs and deploys to GitHub Pages on stable releases.

Published crates, in dependency order: `quillmark-content`, `quillmark-core`, `quillmark-pdf`, `quillmark-pdfform`, `quillmark-typst`, `quillmark`, `quillmark-cli`. Not published: `quillmark-fixtures`, `quillmark-fuzz`, `quillmark-python`, `quillmark-wasm`.

---

## 1) CI (`ci.yml`)

**Trigger**: pull requests, and pushes to `main`. A PR branch push would
otherwise run the whole suite twice for one commit, and the PR event is the one
carrying the merge result. Tags cannot match `branches`, so release tags stay out.
**Jobs** (parallel, all Linux):

| Job | What it does |
|-----|-------------|
| `lint` | `node scripts/check-canon.mjs` (the canon spine and link gate; see [prose/README.md](../README.md)), then `cargo doc --no-deps --locked --workspace` with `RUSTDOCFLAGS=-Dwarnings`: the standing lint gate; clippy is deliberately not gated. `--workspace` because a bare `cargo doc` walks default-members and never lints a crate outside it, which is how intra-doc links rotted in `quillmark-wasm` and `quillmark-content`; every member documents on the host toolchain, so no `--exclude` is needed. Also asserts `scripts/strip-seed-comment.sh` strips a seed block and no-ops without one: `release.yml` runs it unattended against the notes it publishes and `release-prepare.yml` against the section it seeds, so a regression corrupts a GitHub Release body or a changelog entry. It rides here rather than in its own job because the assertions take 0s and a runner for them was pure overhead |
| `test` | `cargo test --workspace --all-features --locked`, then `cargo test -p quillmark --no-default-features --locked`: `--all-features` forces `typst` on, so the zero-backend branch compiles only in the second run (the native counterpart of the `wasm` job's `core` variant) |
| `package` | `cargo package --workspace --locked`, then asserts every resulting `.crate` contains a `LICENSE`. `release.yml` publishes with `--no-verify`, so this is the only place a crate is built from its own archive, and a crates.io version cannot be replaced, so an `include` gap is fixable only before the tag |
| `audit` | bare `cargo audit` over the lockfile's 400+ resolved packages, which fails on vulnerabilities and prints warnings. Not `rustsec/audit-check`: that action fails on warnings too, and `unmaintained`/`yanked` fire on crates Typst pins that no change here can move, a check red no matter what the PR does gets clicked past. The two out-of-reach vulnerabilities are listed with reasons in `.cargo/audit.toml` |
| `msrv` | reads `rust-version` from `cargo metadata` and runs `cargo check --workspace --all-features --locked` on exactly that toolchain, so a dependency bump cannot raise the real floor silently |
| `wasm` | first asserts the no-default-features core graph excludes Typst (`cargo tree -i quillmark-typst` must fail), then builds via `./scripts/build-wasm.sh --ci`, then `npx vitest run` |
| `python` | `maturin develop` into a `uv` venv (Python 3.12, debug profile), then `pytest -q` |

The `wasm` job caches `target/wasm32-unknown-unknown/wasm-ci` under key `wasm-ci-${os}-${hashFiles('Cargo.lock')}` (restore-prefix `wasm-ci-${os}-`), so a lockfile change takes a fresh key while source-only edits restore the prefix and rebuild incrementally. The `wasm-ci-` namespace is deliberately disjoint from `release.yml`'s `wasm-release-` cache so a CI build (debug `wasm-ci` profile) can never be restored into a release job and published to npm.

Excluded: coverage, benchmarks. Dependency bumps arrive by dependabot (`.github/dependabot.yml`): cargo, the wasm binding's npm tree, and the actions themselves, grouped weekly so a routine week is one PR per ecosystem; security updates come ungrouped.

---

## 2) Release prepare (`release-prepare.yml`)

**Trigger**: `workflow_dispatch` with `bump` (`patch`/`minor`) and a `release_candidate` boolean.

1. **Compute next version** (custom bash, not `cargo-release`): reads the current `quillmark` version via `cargo metadata`.
   - If current is `X.Y.Z-rc.N`: the base is fixed and `bump` is ignored. `release_candidate=true` → `X.Y.Z-rc.(N+1)`; false → promote to `X.Y.Z`.
   - Otherwise apply `bump` (`minor` → `X.(Y+1).0`, `patch` → `X.Y.(Z+1)`), appending `-rc.1` when `release_candidate=true`.
2. `cargo release version <computed> --workspace --no-confirm --execute` writes the literal computed string into every `Cargo.toml` and updates intra-workspace deps.
3. Seeds a `CHANGELOG.md` section from `git log` since the last stable tag, then pushes `release/vX.Y.Z` and opens a PR into `main`.

The PR uses a GitHub App token (`TAGGER_APP_ID`/`TAGGER_PRIVATE_KEY`) so CI runs on it and so its merge fires `release.yml`: PRs opened with the default `GITHUB_TOKEN` do not trigger workflow events.

---

## 3) Release & publish (`release.yml`)

**Trigger**: a `release/v*` PR merged into `main` (`pull_request: closed` + `merged == true`).

**`prepare` job** (App token; the tag-creation ruleset blocks `GITHUB_TOKEN`): reads the workspace version, tags `vX.Y.Z`, extracts the matching `CHANGELOG.md` section, and creates a GitHub Release, `--prerelease` for versions containing `-`, else `--latest`.

**Publish jobs** (parallel, `needs: prepare`, OIDC `id-token: write`):

| Target | Registry | Command |
|--------|----------|---------|
| Rust crates | crates.io | `cargo publish --workspace --locked --no-verify` via `rust-lang/crates-io-auth-action` |
| WASM | npm | `npm publish --access public --provenance` (Trusted Publisher) |
| Python | PyPI | `pypa/gh-action-pypi-publish` over prebuilt wheels |

- **Rust crates**: `--workspace` reaches every publishable member, including `quillmark-content` (the leaf the default-members exclude) and skips the `publish = false` members (fixtures, fuzz, bindings). Cargo orders the rest by dependency and skips any version already on the registry with a warning, so re-running resumes a partially-uploaded release instead of erroring.
- **WASM**: restores the `wasm-release-` cache (`wasm-release` profile), builds via `./scripts/build-wasm.sh`, runs `npx vitest run`, publishes `@quillmark/wasm`. Pre-release versions (containing `-`) publish with `--tag next` so they land on the `next` dist-tag instead of `latest`.
- **Python**: `maturin-action` builds wheels for Linux (x86_64, aarch64), Windows (x64), macOS (aarch64) across Python 3.10–3.12, plus an sdist; artifacts are gathered and uploaded with `skip-existing`.

### Trusted Publishing scope (crates.io)

`crates-io-auth-action` mints an OIDC token scoped to exactly the crates
carrying a matching Trusted Publisher config: repo `borb-sh/quillmark`,
workflow `release.yml`, environment `Publish`. That config is **per crate**.

A publishable crate without one draws `403 … the provided access token is not
valid for crate <name>` the moment the dependency-ordered publish reaches it,
after earlier crates have already gone up (crates.io versions are immutable, so
those uploads stand). Every crate in the published list carries its own config.
A new publishable crate needs one added on crates.io before its first release,
and the partial-upload skip above makes the re-run that finishes the batch safe.

---

## 4) Docs (`docs.yml`)

**Triggers**: published GitHub Releases, PRs touching `docs/**`/`mkdocs.yml`/the workflow, and `workflow_dispatch`.

- `build`: `mkdocs build --strict`; uploads the Pages artifact except on PRs (PRs are build-only validation).
- `deploy`: runs only for `workflow_dispatch` or a published **non-prerelease** release (RCs are skipped), deploying to GitHub Pages. Serialized via `concurrency: pages` with `cancel-in-progress: false`.

---

## Versioning

- SemVer across all crates and bindings; one workspace version drives everything.
- MSRV is `rust-version` in `[workspace.package]`, inherited by every crate via
  `rust-version.workspace = true` (the field does *not* apply without that
  opt-in) and held by CI's `msrv` job. It tracks the floor the Typst toolchain
  forces, so for the default build it is measured rather than chosen.
- Edition is 2024, set in `[workspace.package]` and inherited the same way. It
  requires 1.85 (seven releases under the MSRV floor) so it constrains no
  consumer the floor does not already. `resolver` stays explicit at `"2"`;
  edition 2024 implies `"3"` only where the resolver is unset.
- WASM npm version is derived from the workspace version at build time (`scripts/build-wasm.sh`); Python version comes from the workspace `Cargo.toml` via maturin `dynamic = ["version"]`.
