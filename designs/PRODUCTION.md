# Quillmark Production Readiness Plan

> **Purpose**: This document outlines what is needed before `quillmark-core`, `quillmark-typst`, and `quillmark` are ready for production release as open source packages on crates.io.
>
> **Status**: Planning Phase - Implementation Not Yet Started

---

## Table of Contents

1. [Overview](#overview)
2. [Production Readiness Criteria](#production-readiness-criteria)
3. [Crate-Specific Requirements](#crate-specific-requirements)
4. [Cross-Cutting Requirements](#cross-cutting-requirements)
5. [CI/CD Infrastructure](#cicd-infrastructure)
6. [Release Process](#release-process)
7. [Post-Release Requirements](#post-release-requirements)
8. [Timeline and Phasing](#timeline-and-phasing)

---

## Overview

### Scope

This plan covers the three crates that will be published to crates.io:

1. **quillmark-core** - Core types, parsing, templating, and interfaces
2. **quillmark-typst** - Typst backend implementation for PDF/SVG rendering
3. **quillmark** - High-level sealed engine API (main entry point)

The `quillmark-fixtures` crate is internal-only and will not be published.

### Publication Order

Crates must be published in dependency order:

1. **quillmark-core** (no workspace dependencies)
2. **quillmark-typst** (depends on quillmark-core)
3. **quillmark** (depends on quillmark-core and quillmark-typst)

### Version Strategy

All workspace crates share the same version number (e.g., 0.1.0) to:
- Simplify the release process
- Ensure compatibility across workspace
- Reduce user confusion
- Align with common practice for tightly-coupled crates

---

## Production Readiness Criteria

### Code Quality

- [ ] **Code compiles** without warnings on stable Rust
- [ ] **All tests pass** on Linux, macOS, and Windows
- [ ] **Code coverage** ≥80% line coverage for all published crates
- [ ] **Clippy** passes with no warnings using recommended lints
- [ ] **Rustfmt** formatting is consistent across all code
- [ ] **No compiler warnings** in any configuration

### Documentation

- [ ] **All public APIs documented** with rustdoc comments
- [ ] **Module-level documentation** for all modules
- [ ] **Usage examples** in documentation
- [ ] **Doc tests** compile and pass
- [ ] **README.md** for each published crate
- [ ] **Crate-level documentation** (lib.rs) with overview

### API Stability

- [ ] **Public API reviewed** and approved
- [ ] **No planned breaking changes** before 0.1.0
- [ ] **API follows Rust guidelines** (naming, patterns, idioms)
- [ ] **Error types** are well-designed and actionable
- [ ] **Feature flags** are documented and tested

### Security

- [ ] **No known vulnerabilities** in dependencies
- [ ] **Dependency audit** passes
- [ ] **Security policy** documented
- [ ] **No secrets** in source code or examples

### Legal

- [ ] **License** clearly specified (Apache-2.0)
- [ ] **LICENSE file** present
- [ ] **Copyright notices** where appropriate
- [ ] **Third-party licenses** acknowledged

---

## Crate-Specific Requirements

### quillmark-core

#### Functionality
- [ ] **Parsing** - YAML frontmatter and markdown decomposition working
- [ ] **Templating** - MiniJinja-based Glue system functional
- [ ] **Filter API** - Stable interface for backend filters
- [ ] **Quill model** - Quill.toml parsing and validation
- [ ] **Error handling** - Structured diagnostics with locations
- [ ] **TOML/YAML conversion** - Utility functions tested

#### Documentation
- [ ] **README.md** with:
  - Overview of quillmark-core
  - Installation instructions
  - Basic usage example
  - Link to full documentation
  - License information
- [ ] **API documentation** for:
  - `Backend` trait
  - `Artifact` type
  - `OutputFormat` enum
  - `decompose` function
  - `ParsedDocument` type
  - `Glue` type and filter_api
  - `Quill` type
  - Error types (`RenderError`, `TemplateError`, `Diagnostic`)
- [ ] **Examples** showing:
  - How to implement a backend
  - How to use the parsing API
  - How to work with templates

#### Testing
- [ ] **Unit tests** for parsing logic
- [ ] **Unit tests** for templating
- [ ] **Integration tests** for end-to-end parsing
- [ ] **Doc tests** for all examples
- [ ] **Error cases** covered

#### Cargo.toml Metadata
- [ ] `name = "quillmark-core"`
- [ ] `version = "0.1.0"`
- [ ] `edition = "2021"`
- [ ] `authors` filled in
- [ ] `license = "Apache-2.0"`
- [ ] `description` (compelling, < 140 chars)
- [ ] `documentation = "https://docs.rs/quillmark-core"`
- [ ] `homepage = "https://github.com/nibsbin/quillmark"`
- [ ] `repository = "https://github.com/nibsbin/quillmark"`
- [ ] `keywords` (up to 5: e.g., "markdown", "pdf", "typst", "rendering", "templates")
- [ ] `categories` (e.g., "text-processing", "template-engine")
- [ ] `readme = "README.md"`
- [ ] `[package.metadata.docs.rs]` configured

### quillmark-typst

#### Functionality
- [ ] **Backend trait** implementation complete
- [ ] **Markdown→Typst** conversion working (`mark_to_typst`)
- [ ] **Filters** implemented and tested:
  - String filter
  - Lines filter
  - Date filter
  - Dict filter
  - Body filter
  - YAML→TOML injector
- [ ] **QuillWorld** - Typst World trait implementation
- [ ] **Font loading** - Dynamic font discovery and resolution
- [ ] **Asset resolution** - Images and other assets
- [ ] **Package loading** - typst.toml package management
- [ ] **Diagnostics** - Typst errors mapped to structured diagnostics

#### Documentation
- [ ] **README.md** with:
  - Overview of quillmark-typst
  - Installation instructions
  - Basic usage example (standalone backend usage)
  - Link to full documentation
  - License information
- [ ] **API documentation** for:
  - `TypstBackend` type
  - `mark_to_typst` function
  - All filters
  - `QuillWorld` type
  - Diagnostic mapping
- [ ] **Examples** showing:
  - Standalone Typst backend usage
  - Custom filter registration
  - Asset handling

#### Testing
- [ ] **Unit tests** for markdown→typst conversion
- [ ] **Unit tests** for each filter
- [ ] **Integration tests** with real documents
- [ ] **Font loading** tests
- [ ] **Package resolution** tests
- [ ] **Error case** tests

#### Cargo.toml Metadata
- [ ] `name = "quillmark-typst"`
- [ ] `version = "0.1.0"`
- [ ] `edition = "2021"`
- [ ] `authors` filled in
- [ ] `license = "Apache-2.0"`
- [ ] `description` (compelling, < 140 chars)
- [ ] `documentation = "https://docs.rs/quillmark-typst"`
- [ ] `homepage = "https://github.com/nibsbin/quillmark"`
- [ ] `repository = "https://github.com/nibsbin/quillmark"`
- [ ] `keywords` (e.g., "typst", "pdf", "svg", "markdown", "backend")
- [ ] `categories` (e.g., "text-processing", "rendering")
- [ ] `readme = "README.md"`
- [ ] `[package.metadata.docs.rs]` configured

### quillmark

#### Functionality
- [ ] **Quillmark engine** - High-level API for managing backends and quills
- [ ] **Workflow** - Sealed rendering API
- [ ] **Orchestration** - Parse → compose → compile pipeline
- [ ] **Validation** - Input validation and error propagation
- [ ] **QuillRef** - Ergonomic quill references
- [ ] **Auto-registration** - Typst backend registered by default
- [ ] **Feature flags** - `typst` feature working correctly

#### Documentation
- [ ] **README.md** with:
  - Overview of quillmark (main entry point)
  - Installation instructions
  - Quick start example
  - Common use cases
  - Link to full documentation
  - License information
- [ ] **API documentation** for:
  - `Quillmark` engine type
  - `Workflow` type
  - `QuillRef` type
  - Orchestration functions
  - Feature flags
- [ ] **Examples** showing:
  - Basic usage (engine API)
  - Direct workflow API usage
  - Dynamic asset handling
  - Error handling

#### Testing
- [ ] **Unit tests** for engine API
- [ ] **Unit tests** for workflow
- [ ] **Integration tests** for end-to-end rendering
- [ ] **Feature flag** tests (with/without typst)
- [ ] **Example code** tests

#### Cargo.toml Metadata
- [ ] `name = "quillmark"`
- [ ] `version = "0.1.0"`
- [ ] `edition = "2021"`
- [ ] `authors` filled in
- [ ] `license = "Apache-2.0"`
- [ ] `description` (compelling, < 140 chars - primary public API)
- [ ] `documentation = "https://docs.rs/quillmark"`
- [ ] `homepage = "https://github.com/nibsbin/quillmark"`
- [ ] `repository = "https://github.com/nibsbin/quillmark"`
- [ ] `keywords` (e.g., "markdown", "pdf", "typst", "rendering", "templates")
- [ ] `categories` (e.g., "text-processing", "template-engine")
- [ ] `readme = "README.md"`
- [ ] `[package.metadata.docs.rs]` configured

---

## Cross-Cutting Requirements

### Code Quality Standards

#### Linting (Clippy)

**Configuration** (`.clippy.toml`):
```toml
# Clippy configuration
avoid-breaking-exported-api = true
msrv = "1.70.0"
```

**Denied Lints** (add to each crate's lib.rs):
```rust
#![deny(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub,
)]
```

- [ ] `.clippy.toml` created
- [ ] Lint attributes added to all crates
- [ ] All clippy warnings resolved

#### Formatting (rustfmt)

**Configuration** (`rustfmt.toml`):
```toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

- [ ] `rustfmt.toml` created
- [ ] All code formatted consistently

#### Testing Standards

- [ ] **Coverage target** ≥80% for all published crates
- [ ] **Test organization**:
  - Unit tests in same file as implementation
  - Integration tests in `tests/` directory
  - Doc tests in documentation
  - Fixtures using `quillmark-fixtures` crate
- [ ] **Test naming** using descriptive snake_case
- [ ] **Cross-platform testing** on Linux, macOS, Windows

#### Documentation Standards

- [ ] **All public items** have doc comments
- [ ] **Examples** in doc comments using ````rust` blocks
- [ ] **Doc tests** compile and pass
- [ ] **Module docs** with high-level overview
- [ ] **Error documentation** with "# Errors" sections
- [ ] **Panic documentation** with "# Panics" sections where applicable
- [ ] **Safety documentation** with "# Safety" sections for unsafe code

### Performance

- [ ] **Benchmarks** implemented using Criterion for:
  - Document parsing
  - Template rendering
  - Typst compilation
  - Full end-to-end workflow
- [ ] **Performance regressions** identified and fixed
- [ ] **Memory usage** reasonable for typical documents

### Security

- [ ] **Dependency audit** (`cargo audit`) passes
- [ ] **No known vulnerabilities** in any dependencies
- [ ] **Minimal dependencies** - only what's necessary
- [ ] **Security policy** (SECURITY.md) created
- [ ] **Dependabot** configured for automated updates

### Compatibility

- [ ] **MSRV** (Minimum Supported Rust Version) documented (1.70.0)
- [ ] **Platform support** verified on:
  - Linux (Ubuntu latest)
  - macOS (latest)
  - Windows (latest)
- [ ] **Feature combinations** tested (all-features, no-default-features)

---

## CI/CD Infrastructure

### Required Workflows

#### 1. Continuous Integration (`ci.yml`)

- [ ] **check** job:
  - Runs `cargo check --workspace --all-features`
  - Runs `cargo check --workspace --no-default-features`
- [ ] **test** job:
  - Tests on Linux, macOS, Windows
  - Runs all tests with all features
  - Runs all tests with no default features
  - Runs doc tests
  - Generates coverage (Ubuntu only)
- [ ] **fmt** job:
  - Checks formatting with `cargo fmt -- --check`
- [ ] **clippy** job:
  - Runs clippy with all features
  - Runs clippy with no default features
  - Treats warnings as errors
- [ ] **docs** job:
  - Builds documentation with nightly
  - Checks for broken links
- [ ] **msrv** job:
  - Verifies builds on Rust 1.70.0

#### 2. Security Audit (`security.yml`)

- [ ] **Scheduled audit** (daily)
- [ ] **Runs `cargo audit`**
- [ ] **Creates security advisories** for vulnerabilities
- [ ] **Dependabot** configured for:
  - Cargo dependencies
  - GitHub Actions

#### 3. Documentation (`docs.yml`)

- [ ] **Builds docs** for GitHub Pages
- [ ] **Broken link checker**
- [ ] **Deploys to GitHub Pages** on main branch

#### 4. Publish to crates.io (`publish-crates.yml`)

- [ ] **Triggered by** git tags (v*.*.*)
- [ ] **Version verification** - ensures all crates have matching versions
- [ ] **Dry-run publish** for each crate
- [ ] **Actual publish** in dependency order:
  1. quillmark-core
  2. quillmark-typst
  3. quillmark
- [ ] **Release creation** on success
- [ ] **Rollback** on failure (document manual steps)

### GitHub Configuration

- [ ] **.github/workflows/** directory created
- [ ] **Branch protection** rules configured:
  - Require CI passing before merge
  - Require docs build before merge
  - Require review (recommended)
- [ ] **GitHub secrets** configured:
  - `CARGO_REGISTRY_TOKEN` for crates.io publishing
- [ ] **Required checks** configured in repository settings

### Configuration Files

- [ ] `rustfmt.toml` - formatting configuration
- [ ] `.clippy.toml` - clippy configuration
- [ ] `.gitignore` - excludes target/, Cargo.lock for libraries (keep for workspace root)
- [ ] `SECURITY.md` - security policy and vulnerability reporting

---

## Release Process

### Pre-Release Checklist

Before releasing v0.1.0:

#### Code Quality
- [ ] All CI checks pass on main branch
- [ ] No compiler warnings
- [ ] Clippy passes with no warnings
- [ ] Code is formatted consistently
- [ ] Tests pass on all platforms

#### Documentation
- [ ] All crate READMEs written
- [ ] All public APIs documented
- [ ] Doc tests pass
- [ ] Examples compile and run
- [ ] CHANGELOG.md created with v0.1.0 entry

#### Package Metadata
- [ ] All Cargo.toml metadata complete
- [ ] Version numbers synchronized (0.1.0)
- [ ] Keywords and categories appropriate
- [ ] docs.rs configuration added

#### Security
- [ ] No security advisories
- [ ] Dependency audit passes
- [ ] No secrets in code

#### Verification
- [ ] `cargo publish --dry-run` succeeds for all crates
- [ ] Package contents verified
- [ ] Installation tested in isolated environment

### Release Steps

#### 1. Version Update

```bash
# Update all Cargo.toml files to version 0.1.0
sed -i 's/^version = ".*"/version = "0.1.0"/' quillmark-core/Cargo.toml
sed -i 's/^version = ".*"/version = "0.1.0"/' quillmark-typst/Cargo.toml
sed -i 's/^version = ".*"/version = "0.1.0"/' quillmark/Cargo.toml

# Or create a script: ./scripts/bump-version.sh 0.1.0
```

#### 2. Update CHANGELOG.md

- [ ] Move [Unreleased] changes to [0.1.0] section
- [ ] Add release date
- [ ] Update comparison links

#### 3. Commit and Tag

```bash
git add Cargo.toml quillmark-*/Cargo.toml CHANGELOG.md
git commit -m "Release v0.1.0"
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin main
git push origin v0.1.0
```

#### 4. GitHub Release

- [ ] Go to https://github.com/nibsbin/quillmark/releases/new
- [ ] Select tag: v0.1.0
- [ ] Release title: v0.1.0
- [ ] Description: Copy from CHANGELOG.md
- [ ] Click "Publish release"

#### 5. Automated Publishing

The `publish-crates.yml` workflow will:
- [ ] Verify version consistency
- [ ] Publish quillmark-core
- [ ] Wait for indexing
- [ ] Publish quillmark-typst
- [ ] Wait for indexing
- [ ] Publish quillmark

#### 6. Verification

```bash
# Check crates.io
open https://crates.io/crates/quillmark-core
open https://crates.io/crates/quillmark-typst
open https://crates.io/crates/quillmark

# Wait for docs.rs to build
open https://docs.rs/quillmark-core
open https://docs.rs/quillmark-typst
open https://docs.rs/quillmark

# Test installation
cargo new --bin test-quillmark
cd test-quillmark
cargo add quillmark
cargo build
```

### Post-Release

- [ ] Verify all crates published successfully
- [ ] Check docs.rs builds completed
- [ ] Monitor for issues or bug reports
- [ ] Update README badges (optional)
- [ ] Announce release (optional)

### CHANGELOG.md Format

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security

## [0.1.0] - 2024-XX-XX

### Added
- Initial release
- Core parsing and templating functionality
- Typst backend implementation
- High-level Quillmark engine API

[Unreleased]: https://github.com/nibsbin/quillmark/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nibsbin/quillmark/releases/tag/v0.1.0
```

---

## Post-Release Requirements

### Monitoring

- [ ] **Download counts** - track on crates.io
- [ ] **Issue reports** - monitor GitHub issues
- [ ] **User feedback** - gather from community
- [ ] **Docs.rs builds** - ensure they succeed

### Maintenance

- [ ] **Dependency updates** - review Dependabot PRs weekly
- [ ] **Security advisories** - respond within 7 days
- [ ] **Bug fixes** - triage and prioritize
- [ ] **Feature requests** - collect and plan

### Community

- [ ] **CONTRIBUTING.md** - guide for contributors
- [ ] **CODE_OF_CONDUCT.md** - community guidelines
- [ ] **Issue templates** - for bugs and features
- [ ] **PR template** - for contributions

### Future Releases

After 0.1.0, for subsequent releases:

- [ ] Follow semantic versioning strictly
- [ ] Update CHANGELOG.md for each release
- [ ] Keep versions synchronized across workspace
- [ ] Coordinate with Python and Web library releases (when available)

---

## Timeline and Phasing

### Phase 1: Foundation (Weeks 1-2)

**Goal**: Set up infrastructure and quality gates

- [ ] Create all CI/CD workflows
- [ ] Configure code quality tools (rustfmt, clippy)
- [ ] Set up security auditing
- [ ] Verify builds on all platforms
- [ ] Fix any warnings or errors

**Deliverable**: Green CI pipeline on all PRs

### Phase 2: Documentation (Weeks 2-3)

**Goal**: Complete all documentation

- [ ] Write README.md for each crate
- [ ] Add module-level documentation
- [ ] Add API documentation with examples
- [ ] Write and test doc tests
- [ ] Set up docs workflow and GitHub Pages

**Deliverable**: Comprehensive documentation for all public APIs

### Phase 3: Pre-Publishing (Weeks 3-4)

**Goal**: Prepare for publication

- [ ] Complete all Cargo.toml metadata
- [ ] Run `cargo publish --dry-run` for all crates
- [ ] Create CHANGELOG.md
- [ ] Verify version consistency
- [ ] Test in isolated environment

**Deliverable**: Crates ready for publication

### Phase 4: Publishing (Week 4-5)

**Goal**: Publish to crates.io

- [ ] Set up crates.io account and API token
- [ ] Create and test publish workflow
- [ ] Perform initial release (v0.1.0)
- [ ] Verify publication and docs.rs builds
- [ ] Monitor for initial issues

**Deliverable**: Quillmark v0.1.0 on crates.io

### Phase 5: Stabilization (Week 5-6)

**Goal**: Ensure stable release

- [ ] Fix any reported issues
- [ ] Gather user feedback
- [ ] Plan next release (if needed)
- [ ] Update documentation based on feedback

**Deliverable**: Stable, production-ready v0.1.0

### Phase 6: Ecosystem Expansion (Future)

**Goal**: Extend to other languages (after Rust stabilization)

- [ ] Python bindings (PyPI) - see PYTHON.md
- [ ] Web/WASM library (NPM) - see WEB_LIB.md
- [ ] Version synchronization across ecosystems
- [ ] Cross-language integration testing

**Deliverable**: Multi-language Quillmark ecosystem

---

## Appendix: Quick Reference Checklists

### Ready for v0.1.0?

Quick checklist to verify readiness:

**Code Quality**
- [ ] ✅ All tests pass on all platforms
- [ ] ✅ No compiler warnings
- [ ] ✅ Clippy clean
- [ ] ✅ Code formatted
- [ ] ✅ Coverage ≥80%

**Documentation**
- [ ] ✅ READMEs for all crates
- [ ] ✅ All public APIs documented
- [ ] ✅ Doc tests pass
- [ ] ✅ Examples work

**Metadata**
- [ ] ✅ Cargo.toml complete for all crates
- [ ] ✅ Versions synchronized
- [ ] ✅ CHANGELOG.md created
- [ ] ✅ License files present

**Security**
- [ ] ✅ Audit passes
- [ ] ✅ No vulnerabilities
- [ ] ✅ Dependabot configured

**CI/CD**
- [ ] ✅ All workflows created
- [ ] ✅ CI passing
- [ ] ✅ Publish workflow tested
- [ ] ✅ Secrets configured

### Cargo Publish Dry-Run

Before publishing, run:

```bash
# In workspace root
cargo publish --dry-run -p quillmark-core
cargo publish --dry-run -p quillmark-typst
cargo publish --dry-run -p quillmark
```

All three must succeed before proceeding.

---

## Resources

- [The Cargo Book](https://doc.rust-lang.org/cargo/)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [docs.rs Documentation](https://docs.rs/about)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)

---

**Document Version**: 1.0  
**Last Updated**: 2024-01-15  
**Status**: Planning Phase - Ready for Implementation

**Related Documents**:
- [CI_CD.md](CI_CD.md) - Detailed CI/CD workflows and automation
- [DESIGN.md](DESIGN.md) - Architecture and design decisions
- [PYTHON.md](PYTHON.md) - Python library design (future)
- [WEB_LIB.md](WEB_LIB.md) - Web/WASM library design (future)
