# Quillmark Rust Workspace CI/CD Plan

> **Status**: Planning Phase - Implementation Not Yet Started
>
> This document formalizes the CI/CD strategy for publishing the Quillmark Rust workspace crates to crates.io, with a runway for Python (PyPI) and Web (NPM) library publishing efforts described in PYTHON.md and WEB_LIB.md.

---

## Table of Contents

1. [Overview](#overview)
2. [Publishing Strategy](#publishing-strategy)
3. [Workspace Structure](#workspace-structure)
4. [CI/CD Workflows](#cicd-workflows)
5. [Version Management](#version-management)
6. [Release Process](#release-process)
7. [Security and Quality](#security-and-quality)
8. [Documentation Requirements](#documentation-requirements)
9. [Integration with Python and Web Libraries](#integration-with-python-and-web-libraries)
10. [Implementation Roadmap](#implementation-roadmap)

---

## Overview

### Goals

- **crates.io Publishing**: Automate publishing of all workspace crates to the Rust package registry
- **Quality Assurance**: Comprehensive testing, linting, and formatting checks on all PRs and commits
- **Cross-Platform Testing**: Validate builds on Linux, macOS, and Windows
- **Security**: Regular dependency audits and vulnerability scanning
- **Documentation**: Automated docs.rs generation and validation
- **Unified Versioning**: Coordinate releases across the workspace with semantic versioning
- **Foundation for Ecosystem**: Establish CI/CD patterns that extend to Python and Web libraries

### Non-Goals

- Publishing nightly or preview builds (stick to stable releases initially)
- Complex branching strategies (start with main/develop, add as needed)
- Custom badge servers or metrics dashboards (use GitHub/crates.io built-ins)

### Workspace Crates

The Quillmark workspace consists of four crates:

1. **quillmark-core** - Core types, parsing, templating, and interfaces
2. **quillmark-typst** - Typst backend implementation for PDF/SVG rendering
3. **quillmark** - High-level sealed engine API (depends on core and typst)
4. **quillmark-fixtures** - Test utilities and resources (not published to crates.io)

---

## Publishing Strategy

### Crates to Publish

| Crate | Publish to crates.io | Reason |
|-------|---------------------|---------|
| `quillmark-core` | ✅ Yes | Public API - core types and interfaces |
| `quillmark-typst` | ✅ Yes | Public backend - users may want standalone Typst backend |
| `quillmark` | ✅ Yes | Primary public API - main entry point |
| `quillmark-fixtures` | ❌ No | Internal test utilities only |

### Publication Order

Crates must be published in dependency order:

1. **quillmark-core** (no workspace dependencies)
2. **quillmark-typst** (depends on quillmark-core)
3. **quillmark** (depends on quillmark-core and quillmark-typst)

### Visibility and Metadata

All published crates should include:

```toml
[package]
name = "quillmark-*"
version = "0.1.0"
edition = "2021"
authors = ["Quillmark Contributors"]
license = "Apache-2.0"
description = "Brief, compelling description"
documentation = "https://docs.rs/quillmark-*"
homepage = "https://github.com/nibsbin/quillmark"
repository = "https://github.com/nibsbin/quillmark"
keywords = ["markdown", "pdf", "typst", "rendering", "templates"]
categories = ["text-processing", "template-engine"]
readme = "../README.md"  # or crate-specific README
```

**README Requirements**: Each published crate needs a README with:
- Quick overview
- Installation instructions
- Basic usage example
- Link to full documentation
- License information

---

## Workspace Structure

### Current Structure

```
quillmark/
├── Cargo.toml                  # Workspace manifest
├── Cargo.lock                  # Locked dependencies
├── .github/
│   └── workflows/
│       ├── ci.yml              # Continuous integration
│       ├── publish-crates.yml  # Publish to crates.io
│       ├── docs.yml            # Documentation checks
│       └── security.yml        # Dependency audits
├── quillmark-core/
│   ├── Cargo.toml
│   ├── src/
│   └── README.md
├── quillmark-typst/
│   ├── Cargo.toml
│   ├── src/
│   └── README.md
├── quillmark/
│   ├── Cargo.toml
│   ├── src/
│   └── README.md
└── quillmark-fixtures/
    ├── Cargo.toml
    └── src/
```

### Configuration Files

**.gitignore**:
```gitignore
/target
**/*.rs.bk
Cargo.lock  # Remove this line - we want to commit Cargo.lock
.idea/
.vscode/
*.swp
.DS_Store
```

**Note**: Cargo.lock SHOULD be committed for applications and workspace roots to ensure reproducible builds.

---

## CI/CD Workflows

### 1. Continuous Integration Workflow

**File**: `.github/workflows/ci.yml`

**Trigger**: Push to main/develop, all pull requests

**Jobs**:

#### Job: `check`
- **Purpose**: Fast syntax and compilation check
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install Rust stable toolchain
  - Cache Cargo dependencies
  - Run `cargo check --workspace --all-features`
  - Run `cargo check --workspace --no-default-features`

#### Job: `test`
- **Purpose**: Run all workspace tests
- **Runs on**: Matrix of [ubuntu-latest, macos-latest, windows-latest]
- **Strategy**: fail-fast: false
- **Steps**:
  - Checkout code
  - Install Rust stable toolchain
  - Cache Cargo dependencies
  - Run `cargo test --workspace --all-features`
  - Run `cargo test --workspace --no-default-features`
  - Run `cargo test --workspace --doc` (doctest)
  - Upload coverage to Codecov (ubuntu only)

#### Job: `fmt`
- **Purpose**: Verify code formatting
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install Rust stable with rustfmt
  - Run `cargo fmt --all -- --check`

#### Job: `clippy`
- **Purpose**: Lint for common mistakes and improvements
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install Rust stable with clippy
  - Cache Cargo dependencies
  - Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - Run `cargo clippy --workspace --all-targets --no-default-features -- -D warnings`

#### Job: `docs`
- **Purpose**: Ensure documentation builds without warnings
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install Rust nightly (for better docs features)
  - Run `cargo doc --workspace --all-features --no-deps`
  - Check for broken links in generated docs

#### Job: `msrv`
- **Purpose**: Verify Minimum Supported Rust Version
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install Rust 1.70.0 (align with PyO3 MSRV)
  - Run `cargo check --workspace --all-features`

**Workflow Snippet**:

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache Cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache Cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache Cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Check all features
        run: cargo check --workspace --all-features
      
      - name: Check no default features
        run: cargo check --workspace --no-default-features

  test:
    name: Test - ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Run tests (all features)
        run: cargo test --workspace --all-features
      
      - name: Run tests (no default features)
        run: cargo test --workspace --no-default-features
      
      - name: Run doctests
        run: cargo test --workspace --doc
      
      - name: Install cargo-tarpaulin (Ubuntu only)
        if: matrix.os == 'ubuntu-latest'
        run: cargo install cargo-tarpaulin
      
      - name: Generate coverage (Ubuntu only)
        if: matrix.os == 'ubuntu-latest'
        run: cargo tarpaulin --workspace --all-features --out Xml --output-dir coverage
      
      - name: Upload coverage (Ubuntu only)
        if: matrix.os == 'ubuntu-latest'
        uses: codecov/codecov-action@v4
        with:
          files: ./coverage/cobertura.xml
          flags: rust

  fmt:
    name: Rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      
      - name: Check formatting
        run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Run clippy (all features)
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      
      - name: Run clippy (no default features)
        run: cargo clippy --workspace --all-targets --no-default-features -- -D warnings

  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust nightly
        uses: dtolnay/rust-toolchain@nightly
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Build documentation
        run: cargo doc --workspace --all-features --no-deps
        env:
          RUSTDOCFLAGS: -D warnings
      
      - name: Check for broken links
        run: |
          cargo install cargo-deadlinks || true
          cargo deadlinks --check-http --dir target/doc

  msrv:
    name: MSRV (1.70.0)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust 1.70.0
        uses: dtolnay/rust-toolchain@1.70.0
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Check MSRV compatibility
        run: cargo check --workspace --all-features
```

### 2. Security Audit Workflow

**File**: `.github/workflows/security.yml`

**Trigger**: 
- Daily schedule
- On push to main
- Manual workflow dispatch

**Jobs**:

#### Job: `audit`
- **Purpose**: Check dependencies for known security vulnerabilities
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install cargo-audit
  - Run `cargo audit --deny warnings`
  - Report findings as GitHub Security Advisories

#### Job: `outdated`
- **Purpose**: Check for outdated dependencies (informational)
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install cargo-outdated
  - Run `cargo outdated --workspace --depth 1`
  - Comment on PR with results (if applicable)

**Workflow Snippet**:

```yaml
name: Security Audit

on:
  push:
    branches: [main]
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight UTC
  workflow_dispatch:

jobs:
  audit:
    name: Dependency Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install cargo-audit
        run: cargo install cargo-audit
      
      - name: Run security audit
        run: cargo audit --deny warnings
      
      - name: Upload audit results
        if: failure()
        uses: github/codeql-action/upload-sarif@v2
        with:
          sarif_file: audit.sarif

  outdated:
    name: Check Outdated Dependencies
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install cargo-outdated
        run: cargo install cargo-outdated
      
      - name: Check for outdated dependencies
        run: cargo outdated --workspace --depth 1
```

### 3. Documentation Workflow

**File**: `.github/workflows/docs.yml`

**Trigger**: Push to main

**Jobs**:

#### Job: `build-and-deploy`
- **Purpose**: Build and publish documentation to GitHub Pages
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code
  - Install Rust nightly
  - Build docs with `cargo doc --workspace --all-features --no-deps`
  - Deploy to GitHub Pages (gh-pages branch)

**Note**: This is optional if relying solely on docs.rs. However, it's useful for:
- Preview documentation before publishing
- Custom documentation themes
- Additional guides and tutorials

**Workflow Snippet**:

```yaml
name: Documentation

on:
  push:
    branches: [main]

jobs:
  build-and-deploy:
    name: Build and Deploy Docs
    runs-on: ubuntu-latest
    permissions:
      contents: write
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust nightly
        uses: dtolnay/rust-toolchain@nightly
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Build documentation
        run: cargo doc --workspace --all-features --no-deps
        env:
          RUSTDOCFLAGS: --cfg docsrs
      
      - name: Add index redirect
        run: echo '<meta http-equiv="refresh" content="0; url=quillmark">' > target/doc/index.html
      
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc
          force_orphan: true
```

### 4. Publish to crates.io Workflow

**File**: `.github/workflows/publish-crates.yml`

**Trigger**: 
- GitHub release published
- Manual workflow dispatch with version input

**Jobs**:

#### Job: `publish`
- **Purpose**: Publish crates to crates.io in dependency order
- **Runs on**: ubuntu-latest
- **Steps**:
  - Checkout code at release tag
  - Install Rust stable
  - Verify version consistency across workspace
  - Dry-run publish to catch issues
  - Publish quillmark-core
  - Wait for crates.io to index
  - Publish quillmark-typst
  - Wait for crates.io to index
  - Publish quillmark
  - Create GitHub release summary

**Workflow Snippet**:

```yaml
name: Publish to crates.io

on:
  release:
    types: [published]
  workflow_dispatch:
    inputs:
      crate:
        description: 'Crate to publish (or "all" for all crates)'
        required: true
        default: 'all'
        type: choice
        options:
          - all
          - quillmark-core
          - quillmark-typst
          - quillmark

env:
  CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

jobs:
  publish:
    name: Publish Crates
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Verify version consistency
        run: |
          CORE_VERSION=$(grep '^version' quillmark-core/Cargo.toml | head -1 | cut -d'"' -f2)
          TYPST_VERSION=$(grep '^version' quillmark-typst/Cargo.toml | head -1 | cut -d'"' -f2)
          MAIN_VERSION=$(grep '^version' quillmark/Cargo.toml | head -1 | cut -d'"' -f2)
          
          echo "Core version: $CORE_VERSION"
          echo "Typst version: $TYPST_VERSION"
          echo "Main version: $MAIN_VERSION"
          
          if [ "$CORE_VERSION" != "$TYPST_VERSION" ] || [ "$CORE_VERSION" != "$MAIN_VERSION" ]; then
            echo "Error: Version mismatch across workspace crates"
            exit 1
          fi
          
          echo "All versions match: $CORE_VERSION"
      
      - name: Dry-run publish quillmark-core
        run: cargo publish --dry-run -p quillmark-core
      
      - name: Dry-run publish quillmark-typst
        run: cargo publish --dry-run -p quillmark-typst
      
      - name: Dry-run publish quillmark
        run: cargo publish --dry-run -p quillmark
      
      - name: Publish quillmark-core
        if: github.event.inputs.crate == 'all' || github.event.inputs.crate == 'quillmark-core' || github.event_name == 'release'
        run: |
          cargo publish -p quillmark-core
          echo "Published quillmark-core"
      
      - name: Wait for crates.io to index quillmark-core
        if: github.event.inputs.crate == 'all' || github.event_name == 'release'
        run: sleep 30
      
      - name: Publish quillmark-typst
        if: github.event.inputs.crate == 'all' || github.event.inputs.crate == 'quillmark-typst' || github.event_name == 'release'
        run: |
          cargo publish -p quillmark-typst
          echo "Published quillmark-typst"
      
      - name: Wait for crates.io to index quillmark-typst
        if: github.event.inputs.crate == 'all' || github.event_name == 'release'
        run: sleep 30
      
      - name: Publish quillmark
        if: github.event.inputs.crate == 'all' || github.event.inputs.crate == 'quillmark' || github.event_name == 'release'
        run: |
          cargo publish -p quillmark
          echo "Published quillmark"
      
      - name: Create release summary
        if: github.event_name == 'release'
        run: |
          VERSION=$(grep '^version' quillmark/Cargo.toml | head -1 | cut -d'"' -f2)
          echo "# Published Quillmark v$VERSION to crates.io" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          echo "Published crates:" >> $GITHUB_STEP_SUMMARY
          echo "- quillmark-core v$VERSION" >> $GITHUB_STEP_SUMMARY
          echo "- quillmark-typst v$VERSION" >> $GITHUB_STEP_SUMMARY
          echo "- quillmark v$VERSION" >> $GITHUB_STEP_SUMMARY
```

**Required Secret**: `CARGO_REGISTRY_TOKEN`
- Generate at https://crates.io/me
- Add to repository secrets in GitHub Settings

---

## Version Management

### Semantic Versioning

All crates follow [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR**: Breaking API changes
- **MINOR**: New features, backwards-compatible
- **PATCH**: Bug fixes, backwards-compatible

### Workspace Version Synchronization

**Strategy**: All workspace crates share the same version number.

**Rationale**:
- Simplifies release process
- Ensures compatibility across workspace
- Reduces user confusion
- Aligns with common practice for tightly-coupled crates

**Implementation**:
- Update all `Cargo.toml` version fields together
- Use workspace verification step in publish workflow
- Document version sync in CONTRIBUTING.md

### cargo-release Integration

[cargo-release](https://github.com/crate-ci/cargo-release) is a powerful tool that automates the entire release process, from version bumping to publishing. It's the **recommended** approach for releases.

#### Installation

```bash
cargo install cargo-release
```

#### Configuration

Create a `release.toml` file in the workspace root to configure cargo-release behavior:

```toml
# Workspace-wide release configuration
[workspace]
# Publish all crates together with the same version
consolidate-commits = true

# Git commit and tag configuration
pre-release-commit-message = "chore: release {{version}}"
tag-name = "v{{version}}"
tag-message = "Release {{version}}"

# Allow releases from main branch only
allow-branch = ["main"]

# Default settings for all crates
[workspace.metadata.release]
sign-commit = false
sign-tag = false
push = true
verify = true
publish = true

# Individual crate configuration
[[package]]
name = "quillmark-fixtures"
# Don't publish test fixtures
publish = false
```

#### Basic Usage

**Dry-run mode** (preview changes without executing - always use this first):

```bash
# Preview a patch release
cargo release patch

# Preview a minor release
cargo release minor

# Preview a major release
cargo release major
```

**Execute a release**:

```bash
# Execute a patch release (0.1.0 → 0.1.1)
cargo release patch --execute

# Execute a minor release (0.1.0 → 0.2.0)
cargo release minor --execute

# Execute a major release (0.1.0 → 1.0.0)
cargo release major --execute

# Release a specific version
cargo release 0.3.0 --execute
```

**Advanced options**:

```bash
# Don't push to remote (for testing)
cargo release patch --execute --no-push

# Don't create git tag
cargo release patch --execute --no-tag

# Don't publish to crates.io
cargo release patch --execute --no-publish

# Skip confirmation prompts
cargo release patch --execute --no-confirm
```

**Key Benefits of cargo-release**:
- ✅ Automatically bumps versions across all workspace crates
- ✅ Updates Cargo.lock
- ✅ Creates git commits and tags
- ✅ Publishes to crates.io in dependency order
- ✅ Handles inter-crate dependencies correctly
- ✅ Validates versions before publishing
- ✅ Supports pre-release versions (alpha, beta, rc)

### Manual Version Bumping Process

If not using cargo-release, follow these manual steps:

1. **Determine Version Bump Type**:
   - Breaking changes → MAJOR bump
   - New features → MINOR bump
   - Bug fixes only → PATCH bump

2. **Update Version Files**:
   ```bash
   # For version 0.2.0
   sed -i 's/^version = ".*"/version = "0.2.0"/' quillmark-core/Cargo.toml
   sed -i 's/^version = ".*"/version = "0.2.0"/' quillmark-typst/Cargo.toml
   sed -i 's/^version = ".*"/version = "0.2.0"/' quillmark/Cargo.toml
   ```

3. **Update Cargo.lock**:
   ```bash
   cargo update -p quillmark-core -p quillmark-typst -p quillmark
   ```

4. **Update CHANGELOG.md** (see below)

### CHANGELOG.md Format

Follow [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- New features go here

### Changed
- Changes in existing functionality

### Deprecated
- Soon-to-be removed features

### Removed
- Removed features

### Fixed
- Bug fixes

### Security
- Security fixes

## [0.1.0] - 2024-01-15

### Added
- Initial release
- Core parsing and templating
- Typst backend for PDF/SVG generation
- High-level Workflow API

[Unreleased]: https://github.com/nibsbin/quillmark/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nibsbin/quillmark/releases/tag/v0.1.0
```

---

## Release Process

### Pre-Release Checklist

- [ ] All CI checks pass on main branch
- [ ] No outstanding security advisories
- [ ] Documentation is up-to-date
- [ ] CHANGELOG.md is updated with all changes
- [ ] Version numbers are synchronized across workspace
- [ ] All examples compile and run
- [ ] README.md files are accurate for each published crate

### Release Steps

There are two approaches to releasing: **Using cargo-release (Recommended)** or **Manual Process**.

#### Option A: Using cargo-release (Recommended)

This is the streamlined, automated approach that handles all steps for you.

**1. Prepare for Release**

```bash
# Ensure you're on main branch and up-to-date
git checkout main
git pull origin main

# Ensure all tests pass
cargo test --workspace --all-features

# Update CHANGELOG.md with release notes
vim CHANGELOG.md
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for release"
```

**2. Preview the Release**

```bash
# Dry-run to preview all changes
cargo release minor  # or patch, major, or specific version like 0.2.0

# This will show:
# - Version changes
# - Git commits that will be created
# - Git tags that will be created
# - Crates that will be published
```

**3. Execute the Release**

```bash
# Execute the release
cargo release minor --execute

# This will:
# 1. Bump versions in all Cargo.toml files
# 2. Update Cargo.lock
# 3. Create a git commit
# 4. Create a git tag (v0.2.0)
# 5. Publish quillmark-core to crates.io
# 6. Wait for crates.io indexing
# 7. Publish quillmark-typst to crates.io
# 8. Wait for crates.io indexing
# 9. Publish quillmark to crates.io
# 10. Push commits and tags to GitHub
```

**4. Create GitHub Release**

After cargo-release completes:

- Go to https://github.com/nibsbin/quillmark/releases/new
- Select the tag that was just created (e.g., `v0.2.0`)
- Release title: `v0.2.0`
- Description: Copy relevant section from CHANGELOG.md
- Click "Publish release"

The `publish-crates.yml` workflow will run automatically when the GitHub release is published.

**5. Verify Publication**

```bash
# Check crates.io
open https://crates.io/crates/quillmark-core
open https://crates.io/crates/quillmark-typst
open https://crates.io/crates/quillmark

# Verify installation works
cargo install quillmark --version 0.2.0
```

#### Option B: Manual Release Process

If you prefer manual control or cargo-release is unavailable:

**1. Prepare Release Branch (Optional for Major/Minor)**

```bash
git checkout -b release/v0.2.0
```

**2. Update Versions**

```bash
# Update all Cargo.toml files
./scripts/bump-version.sh 0.2.0

# Or manually:
sed -i 's/^version = ".*"/version = "0.2.0"/' quillmark-core/Cargo.toml
sed -i 's/^version = ".*"/version = "0.2.0"/' quillmark-typst/Cargo.toml
sed -i 's/^version = ".*"/version = "0.2.0"/' quillmark/Cargo.toml
```

**3. Update CHANGELOG.md**

```bash
# Move [Unreleased] changes to new version section
# Update comparison links
vim CHANGELOG.md
```

**4. Commit and Tag**

```bash
# Commit version bump
git add Cargo.toml */Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"

# Create annotated tag
git tag -a v0.2.0 -m "Release version 0.2.0"

# Push to GitHub
git push origin main
git push origin v0.2.0
```

**5. Create GitHub Release**

- Go to https://github.com/nibsbin/quillmark/releases/new
- Select tag: `v0.2.0`
- Release title: `v0.2.0`
- Description: Copy relevant section from CHANGELOG.md
- Click "Publish release"

**6. Automatic Publishing**

The `publish-crates.yml` workflow will automatically:
- Verify version consistency
- Publish crates to crates.io in dependency order
- Create release summary

**7. Verify Publication**

```bash
# Check crates.io
open https://crates.io/crates/quillmark-core
open https://crates.io/crates/quillmark-typst
open https://crates.io/crates/quillmark

# Verify installation
cargo install quillmark --version 0.2.0
```

**8. Announce Release**

- Update documentation site
- Announce on social media (Twitter, Reddit, etc.)
- Update examples and tutorials
- Notify dependent projects

### Post-Release

- [ ] Verify all crates published successfully
- [ ] Check docs.rs builds completed
- [ ] Monitor for issues or bug reports
- [ ] Create milestone for next version

### Hotfix Process

For critical bugs in released versions:

1. Create hotfix branch from release tag:
   ```bash
   git checkout -b hotfix/v0.2.1 v0.2.0
   ```

2. Fix the bug and commit

3. Update version to 0.2.1 (PATCH bump)

4. Follow standard release process

5. Merge hotfix back to main:
   ```bash
   git checkout main
   git merge hotfix/v0.2.1
   git push origin main
   ```

---

## Security and Quality

### Dependency Management

#### Regular Audits

- **Frequency**: Daily via scheduled workflow
- **Tool**: `cargo audit`
- **Action**: Create GitHub Security Advisory for vulnerabilities
- **Review**: Security team reviews and patches within 7 days

#### Dependency Updates

- **Tool**: Dependabot or Renovate
- **Strategy**: 
  - Minor/patch updates: Auto-merge if CI passes
  - Major updates: Manual review required
- **Frequency**: Weekly check for updates

#### Minimal Dependencies

- **Principle**: Only add dependencies when necessary
- **Review**: All new dependencies require justification in PR
- **Alternatives**: Prefer workspace dependencies over duplicates

### Code Quality Standards

#### Linting (Clippy)

**Default Lint Level**: `warn`

**Denied Lints**:
```rust
#![deny(
    missing_docs,           // All public items must have documentation
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub,
)]
```

**Configuration** (.clippy.toml):
```toml
# Clippy configuration
avoid-breaking-exported-api = true
msrv = "1.70.0"
```

#### Formatting (rustfmt)

**Configuration** (rustfmt.toml):
```toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

#### Testing Standards

- **Coverage Target**: >80% line coverage
- **Test Organization**:
  - Unit tests: Same file as implementation
  - Integration tests: `tests/` directory
  - Doc tests: Documentation examples
  - Fixtures: Use `quillmark-fixtures` crate

- **Test Naming**: Descriptive names using snake_case
  ```rust
  #[test]
  fn parse_valid_frontmatter_returns_ok() { ... }
  ```

#### Documentation Standards

- **Public API**: All public items must have doc comments
- **Examples**: Include usage examples in doc comments
- **Doc Tests**: Use ````rust` blocks that compile and run
- **Module Docs**: High-level overview at module level

**Example**:
```rust
/// Parses a Markdown document with YAML frontmatter.
///
/// # Examples
///
/// ```rust
/// use quillmark_core::decompose;
///
/// let markdown = "---\ntitle: Hello\n---\n# Content";
/// let parsed = decompose(markdown)?;
/// assert_eq!(parsed.frontmatter.get("title"), Some("Hello"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns an error if the frontmatter is not valid YAML.
pub fn decompose(input: &str) -> Result<ParsedDocument, ParseError> {
    // ...
}
```

### Performance Benchmarking

**Tool**: `criterion`

**Benchmarks**: Track performance of:
- Document parsing
- Template rendering
- Typst compilation
- Full end-to-end workflow

**CI Integration**: Run benchmarks on release branches, store results for comparison

---

## Documentation Requirements

### Crate-Level Documentation

Each published crate needs:

1. **README.md**:
   - Overview
   - Installation
   - Quick start example
   - Link to full docs
   - License

2. **Cargo.toml metadata**:
   - Complete package metadata
   - Categories and keywords
   - Links to documentation, repository, homepage

3. **Crate root documentation** (lib.rs):
   - High-level overview
   - Architecture diagram (if applicable)
   - Usage examples
   - Feature flags documentation

### docs.rs Configuration

**Metadata** (in each crate's Cargo.toml):
```toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

This enables:
- Feature flags in documentation
- Platform-specific docs
- Better cross-linking

### User Guides and Tutorials

**Location**: GitHub Pages or dedicated documentation site

**Content**:
- Getting Started Guide
- Template Authoring Guide
- Backend Development Guide
- Migration Guides (for breaking changes)
- Cookbook (common recipes)

### API Documentation

**Generated by**: docs.rs automatically from doc comments

**Quality Checks**:
- No broken links
- All public items documented
- Examples compile and run
- Cross-references work

---

## Integration with Python and Web Libraries

### Alignment Strategy

The Rust CI/CD workflows form the foundation for Python and Web publishing:

```
Rust Core (crates.io)
    ↓
    ├─→ Python Bindings (PyPI)
    │   - Uses published Rust crates via git dependency initially
    │   - Switches to crates.io dependency after Rust 0.1.0 release
    │   - Maturin builds Python wheels
    │   - Version synced with Rust (e.g., 0.1.0)
    │
    └─→ Web/WASM Library (NPM)
        - Uses published Rust crates via git dependency initially
        - Switches to crates.io dependency after Rust 0.1.0 release
        - wasm-pack builds WASM modules
        - Version synced with Rust (e.g., 0.1.0)
```

### Shared CI Patterns

All three publishing targets share:

1. **Version Synchronization**: Same version number across Rust, Python, and Web
2. **Quality Gates**: Linting, testing, and security checks before release
3. **Cross-Platform Testing**: Linux, macOS, Windows where applicable
4. **Automated Publishing**: GitHub releases trigger automated publishing
5. **Documentation**: Comprehensive docs for each ecosystem
6. **Security Audits**: Regular dependency scanning

### Coordination Points

#### 1. Version Bumps

When bumping versions:
```bash
# Update all in one commit
./scripts/bump-version.sh 0.2.0
# Updates:
# - quillmark-*/Cargo.toml
# - python/pyproject.toml
# - typescript/package.json
```

#### 2. Breaking Changes

Breaking changes in Rust require updates in:
- Python bindings (PyO3 wrapper changes)
- Web bindings (wasm-bindgen wrapper changes)
- All three must release major version together

#### 3. Release Orchestration

**Sequence**:
1. Release Rust crates to crates.io (v0.2.0)
2. Wait for crates.io to index (~5-10 minutes)
3. Update Python/Web dependencies to use crates.io version
4. Release Python to PyPI (v0.2.0)
5. Release Web to NPM (v0.2.0)

**Automation**: Can be orchestrated via a single "Release All" workflow

### Dependency Management

#### Phase 1: Pre-Release (Git Dependencies)

```toml
# Python (Cargo.toml)
[dependencies]
quillmark = { git = "https://github.com/nibsbin/quillmark", branch = "main" }

# Web (Cargo.toml)
[dependencies]
quillmark = { git = "https://github.com/nibsbin/quillmark", branch = "main" }
```

#### Phase 2: Post-Release (crates.io)

```toml
# Python (Cargo.toml)
[dependencies]
quillmark = "0.1.0"

# Web (Cargo.toml)
[dependencies]
quillmark = "0.1.0"
```

### Testing Integration

**Integration Tests**: Verify Python and Web libraries work with Rust crates

```yaml
# .github/workflows/integration-tests.yml
jobs:
  test-python-integration:
    runs-on: ubuntu-latest
    steps:
      - Checkout code
      - Build Rust crates
      - Build Python bindings
      - Run Python integration tests
  
  test-web-integration:
    runs-on: ubuntu-latest
    steps:
      - Checkout code
      - Build Rust crates
      - Build WASM modules
      - Run Web integration tests
```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

- [ ] **1.1**: Set up GitHub Actions infrastructure
  - [ ] Create `.github/workflows/` directory
  - [ ] Configure branch protection rules
  - [ ] Set up required checks
  
- [ ] **1.2**: Implement CI workflow
  - [ ] Create `ci.yml` with check, test, fmt, clippy jobs
  - [ ] Configure caching for faster builds
  - [ ] Set up cross-platform testing matrix
  
- [ ] **1.3**: Configure code quality tools
  - [ ] Add `rustfmt.toml`
  - [ ] Add `.clippy.toml`
  - [ ] Enable clippy in CI
  - [ ] Enforce formatting in CI

- [ ] **1.4**: Set up security auditing
  - [ ] Create `security.yml` workflow
  - [ ] Configure Dependabot
  - [ ] Set up vulnerability scanning

**Deliverable**: Working CI pipeline that runs on all PRs

### Phase 2: Documentation (Week 2-3)

- [ ] **2.1**: Write crate READMEs
  - [ ] quillmark-core README.md
  - [ ] quillmark-typst README.md
  - [ ] quillmark README.md
  
- [ ] **2.2**: Enhance doc comments
  - [ ] Add module-level docs
  - [ ] Add examples to public APIs
  - [ ] Write usage guides in doc comments
  
- [ ] **2.3**: Set up docs workflow
  - [ ] Create `docs.yml` workflow
  - [ ] Configure GitHub Pages
  - [ ] Add broken link checker
  
- [ ] **2.4**: Documentation quality gates
  - [ ] Add docs job to CI
  - [ ] Require passing docs check for merge

**Deliverable**: Comprehensive documentation for all public APIs

### Phase 3: Preparation for Publishing (Week 3-4)

- [ ] **3.1**: Complete package metadata
  - [ ] Add all required Cargo.toml fields
  - [ ] Add keywords and categories
  - [ ] Set up docs.rs configuration
  
- [ ] **3.2**: Pre-publish checks
  - [ ] Run `cargo publish --dry-run` for all crates
  - [ ] Fix any warnings or errors
  - [ ] Verify package contents
  
- [ ] **3.3**: Create CHANGELOG.md
  - [ ] Document current state as v0.1.0
  - [ ] Set up changelog format
  - [ ] Document release process
  
- [ ] **3.4**: Version management
  - [ ] Verify version consistency
  - [ ] Create version bump script
  - [ ] Document versioning strategy

**Deliverable**: Crates ready for initial publication

### Phase 4: Publishing Setup (Week 4-5)

- [ ] **4.1**: Configure crates.io access
  - [ ] Create crates.io account
  - [ ] Generate API token
  - [ ] Add token to GitHub secrets
  
- [ ] **4.2**: Create publish workflow
  - [ ] Create `publish-crates.yml`
  - [ ] Implement dependency-order publishing
  - [ ] Add dry-run verification
  - [ ] Test with manual dispatch
  
- [ ] **4.3**: Test publish process
  - [ ] Do dry-run publishes
  - [ ] Verify package contents
  - [ ] Test in isolated environment
  
- [ ] **4.4**: Release documentation
  - [ ] Write RELEASING.md guide
  - [ ] Document manual steps
  - [ ] Create checklists

**Deliverable**: Automated publish workflow ready to use

### Phase 5: Initial Release (Week 5-6)

- [ ] **5.1**: Pre-release preparation
  - [ ] Complete pre-release checklist
  - [ ] Review all documentation
  - [ ] Final testing pass
  
- [ ] **5.2**: Publish v0.1.0
  - [ ] Create release tag
  - [ ] Trigger publish workflow
  - [ ] Monitor publication
  - [ ] Verify on crates.io
  
- [ ] **5.3**: Post-release validation
  - [ ] Test installation from crates.io
  - [ ] Verify docs.rs builds
  - [ ] Check all links
  
- [ ] **5.4**: Announcement
  - [ ] Write release announcement
  - [ ] Update README badges
  - [ ] Share on social media

**Deliverable**: Quillmark v0.1.0 published on crates.io

### Phase 6: Monitoring and Iteration (Ongoing)

- [ ] **6.1**: Monitor metrics
  - [ ] Track download counts
  - [ ] Monitor for issues
  - [ ] Gather user feedback
  
- [ ] **6.2**: Dependency maintenance
  - [ ] Review Dependabot PRs
  - [ ] Update dependencies regularly
  - [ ] Fix security advisories
  
- [ ] **6.3**: Process improvements
  - [ ] Refine release process
  - [ ] Optimize CI performance
  - [ ] Update documentation

**Deliverable**: Healthy, maintained crates

### Phase 7: Python and Web Integration (Week 6+)

- [ ] **7.1**: Update Python bindings
  - [ ] Switch from git to crates.io dependency
  - [ ] Align version numbers
  - [ ] Test PyPI publishing workflow
  
- [ ] **7.2**: Update Web bindings
  - [ ] Switch from git to crates.io dependency
  - [ ] Align version numbers
  - [ ] Test NPM publishing workflow
  
- [ ] **7.3**: Unified release process
  - [ ] Create orchestrated release workflow
  - [ ] Document multi-target release process
  - [ ] Test end-to-end release

**Deliverable**: Coordinated publishing across Rust, Python, and Web

---

## Appendix

### A. Useful Commands

#### Development Commands

```bash
# Check all crates
cargo check --workspace --all-features

# Test all crates
cargo test --workspace --all-features

# Format all code
cargo fmt --all

# Lint all code
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build documentation locally
cargo doc --workspace --all-features --no-deps --open

# Dry-run publish
cargo publish --dry-run -p quillmark-core

# Run security audit
cargo audit

# Check for outdated dependencies
cargo outdated --workspace

# Generate coverage report
cargo tarpaulin --workspace --all-features --out Html

# Benchmark performance
cargo bench
```

#### cargo-release Commands

```bash
# Install cargo-release
cargo install cargo-release

# Preview release changes (dry-run)
cargo release patch              # Preview patch release
cargo release minor              # Preview minor release
cargo release major              # Preview major release
cargo release 0.3.0              # Preview specific version

# Execute releases
cargo release patch --execute    # Execute patch release
cargo release minor --execute    # Execute minor release
cargo release major --execute    # Execute major release

# Release with options
cargo release patch --execute --no-push       # Don't push to remote
cargo release patch --execute --no-tag        # Don't create git tag
cargo release patch --execute --no-publish    # Don't publish to crates.io
cargo release patch --execute --no-confirm    # Skip confirmations

# Pre-release versions
cargo release alpha --execute    # Create alpha pre-release
cargo release beta --execute     # Create beta pre-release
cargo release rc --execute       # Create release candidate

# Show current config
cargo release config

# Show what changes since last release
cargo release changes
```

### B. Badge Examples

For README.md files:

```markdown
[![crates.io](https://img.shields.io/crates/v/quillmark.svg)](https://crates.io/crates/quillmark)
[![Documentation](https://docs.rs/quillmark/badge.svg)](https://docs.rs/quillmark)
[![CI](https://github.com/nibsbin/quillmark/workflows/CI/badge.svg)](https://github.com/nibsbin/quillmark/actions)
[![Coverage](https://codecov.io/gh/nibsbin/quillmark/branch/main/graph/badge.svg)](https://codecov.io/gh/nibsbin/quillmark)
[![License](https://img.shields.io/crates/l/quillmark.svg)](https://github.com/nibsbin/quillmark/blob/main/LICENSE)
```

### C. Example Version Bump Script

**Note**: This script is for manual version bumping. Consider using `cargo-release` instead for automated version management (see Version Management section).

**File**: `scripts/bump-version.sh`

```bash
#!/bin/bash
# Bump version across all workspace crates
# Note: cargo-release is the recommended tool for automated releases

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.2.0"
    echo ""
    echo "Recommended: Use cargo-release instead:"
    echo "  cargo release minor --execute"
    exit 1
fi

NEW_VERSION="$1"

echo "Bumping version to $NEW_VERSION..."

# Update Rust crates
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" quillmark-core/Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" quillmark-typst/Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" quillmark/Cargo.toml

# Update Cargo.lock
cargo update -p quillmark-core -p quillmark-typst -p quillmark

# Clean up backup files
rm quillmark-core/Cargo.toml.bak
rm quillmark-typst/Cargo.toml.bak
rm quillmark/Cargo.toml.bak

echo "Version bumped to $NEW_VERSION"
echo "Remember to:"
echo "  1. Update CHANGELOG.md"
echo "  2. Commit changes"
echo "  3. Create and push tag: git tag v$NEW_VERSION && git push origin v$NEW_VERSION"
```

### D. Resources

#### Rust and Cargo

- [The Cargo Book](https://doc.rust-lang.org/cargo/)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [docs.rs Documentation](https://docs.rs/about)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

#### Release Management

- [cargo-release](https://github.com/crate-ci/cargo-release) - Automated release tool
- [cargo-release Documentation](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Conventional Commits](https://www.conventionalcommits.org/)

#### CI/CD

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [GitHub Actions for Rust](https://github.com/actions-rs)

---

**Document Version**: 1.1  
**Last Updated**: 2025-01-15  
**Status**: Planning Phase - cargo-release Integration Complete
