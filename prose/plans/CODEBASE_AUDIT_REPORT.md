# Quillmark Codebase Audit Report

**Audit Date**: January 2025  
**Repository**: nibsbin/quillmark  
**Scope**: Full codebase audit for redundant, legacy, and leftover code

---

## Executive Summary

| Category | Issues Found | Severity |
|----------|-------------|----------|
| Dead Code | 8 items | Low-Medium |
| Unused Dependencies | 13 dependencies | Medium |
| Legacy Patterns | 5 patterns | Low |
| Orphaned Files/Fixtures | 3 items | Low |
| Documentation Issues | 12 items | Medium |
| Unused Exports | 21+ items | Low |

**Overall Assessment**: The quillmark codebase is well-maintained with no critical issues. The main areas requiring attention are:
1. Outdated documentation referencing removed MiniJinja templating
2. Unused Cargo dependencies increasing compile time
3. Several public exports that could be `pub(crate)`

---

## Phase 1: Dead Code Analysis

### 1.1 Unused Functions

**Findings**: 8 functions identified with limited or test-only usage

| Function | Location | Status | Recommendation |
|----------|----------|--------|----------------|
| `FieldType::from_str()` | quill.rs:102 | Test-only | Implement `FromStr` trait instead |
| `FieldType::as_str()` | quill.rs:117 | Test-only | Keep for completeness |
| `QuillValue::as_mapping()` | value.rs:101 | Unused | Remove YAML alias |
| `build_schema_from_fields()` | schema.rs:348 | Backwards-compat | Mark `#[doc(hidden)]` |
| `extract_card_item_defaults()` | schema.rs:451 | Test-only | Document or integrate |
| `apply_card_item_defaults()` | schema.rs:514 | Test-only | Document or integrate |
| `Quill::dir_exists()` | quill.rs:1156 | Test-only | Keep for testing |
| `Quill::list_subdirectories()` | quill.rs:1166 | Test-only | Keep for testing |

### 1.2 Unreachable Code

**Findings**: No issues found
- 1 `unreachable!()` macro in CLI output.rs - appropriate defensive programming
- 1 `todo!()` in documentation example - appropriate for docs
- All `#[cfg]` feature flags are properly defined

### 1.3 Unused Dependencies

**Findings**: 13 unused dependencies identified

| Crate | Unused Dependencies |
|-------|---------------------|
| quillmark | `anyhow`, `serde`, `serde-saphyr`, `thiserror` |
| quillmark-core | `anyhow` |
| quillmark-typst | `anyhow` |
| quillmark-acroform | `thiserror` |
| quillmark-wasm | `serde-saphyr`, `toml`, `web-sys` |
| quillmark-cli | `anyhow` |
| quillmark-python | `anyhow` |
| quillmark-fixtures | `quillmark`, `quillmark-typst` |

**Note**: `quillmark-typst` in bindings crates should be kept for linking.

---

## Phase 2: Legacy Code Detection

### 2.1 TODO/FIXME Comments

**Findings**: Excellent code hygiene
- No active TODO/FIXME/HACK comments in production code
- 1 ignored test (`dry_run_test.rs:123`) - obsolete after MiniJinja removal
- 5 design documents still reference removed MiniJinja architecture

### 2.2 Deprecated Functions

**Findings**: No issues in project code
- No `#[deprecated]` attributes defined
- 2 deprecated transitive dependencies from typst ecosystem (outside project control)
  - `serde_yaml` (deprecated in favor of `serde_yml`)
  - `proc-macro-hack` (no longer needed in modern Rust)

### 2.3 Outdated Patterns

**Findings**: 5 patterns could be modernized

| Pattern | Location | Recommendation |
|---------|----------|----------------|
| Pre-thiserror error handling | cli/errors.rs | Migrate CliError to thiserror |
| `Box<dyn Error>` returns | world.rs (10+ methods) | Create concrete WorldError type |
| Unnecessary clones | schema.rs, parse.rs | Review for ownership transfer |
| Verbose match patterns | normalize.rs, schema.rs | Use let-else or map_or |
| `.to_string()` on Cow | world.rs | Use `.into_owned()` |

---

## Phase 3: Leftover/Orphan Code Discovery

### 3.1 Orphaned Source Files

**Findings**: No orphaned source files
- All 53 Rust source files are properly included in module trees
- Minor issue: `tests/common.rs` compiled as test binary instead of utility module

### 3.2 Unused Test Fixtures

**Findings**: 2 fixtures with limited usage

| Fixture | Status | Notes |
|---------|--------|-------|
| `frontmatter_demo.md` | Documentation only | Not used in any test |
| `sample.md` | Documentation only | Not used in any test |
| `classic_resume` quill | Limited | No Rust test references |
| `cmu_letter` quill | Limited | No Rust test references |

### 3.3 Unused Assets

**Findings**: 1 broken fixture, 1 orphaned file

| Issue | Location | Details |
|-------|----------|---------|
| **Broken fixture** | `appreciated_letter/` | `Quill.toml` references non-existent `plate.typ` |
| **Orphaned file** | `appreciated_letter/glue.typ` | Not referenced anywhere |
| Undocumented script | `scripts/update-fixtures.sh` | No documentation |

---

## Phase 4: Documentation Audit

### 4.1 Stale References

**Findings**: 1 broken link, 2 stale references

| Issue | Location | Details |
|-------|----------|---------|
| **Broken link** | prose/designs/INDEX.md | References non-existent `TYPST_GUILLEMET_CONVERSION.md` |
| Stale reference | GLUE_METADATA.md | References non-existent `templating.rs` |
| Conceptual only | ARCHITECTURE.md | References `filter_api` module that doesn't exist |

### 4.2 Outdated Examples

**Findings**: 4 priority-1 issues with user-facing documentation

| File | Issue |
|------|-------|
| docs/getting-started/quickstart.md | Old MiniJinja plate template syntax in JavaScript example |
| docs/guides/quill-markdown.md | MiniJinja template examples instead of Typst helper syntax |
| crates/bindings/wasm/README.md | Old MiniJinja plate template syntax |
| README.md | References 3 non-existent examples |

### 4.3 Design Documents

**Findings**: 2 documents have significant obsolete content

| Document | Issues |
|----------|--------|
| ARCHITECTURE.md | Filter API section describes removed MiniJinja architecture |
| GLUE_METADATA.md | Entire document based on removed templating approach |

**Current**: 12 of 18 design documents are fully accurate

---

## Phase 5: Cross-Reference Analysis

### 5.1 Unused Exports

**Findings**: 21+ public items could be `pub(crate)`

**quillmark-core exports unused externally**:
- normalize module: `normalize_markdown`, `strip_bidi_formatting`, `normalize_fields`, `NormalizationError`
- quill module: `QuillIgnore`, `field_key`, `ui_key`, `UiFieldSchema`, `UiContainerSchema`
- schema module: `coerce_document`, `extract_defaults_from_schema`, `extract_examples_from_schema`, `build_schema`, `build_schema_from_fields`, `extract_card_item_defaults`, `apply_card_item_defaults`
- error constants: `MAX_INPUT_SIZE`, `MAX_YAML_SIZE`, `MAX_YAML_DEPTH`, `MAX_CARD_COUNT`, `MAX_FIELD_COUNT`

### 5.2 Feature Flags

**Findings**: All features properly defined and used
- 5 features across 2 crates
- No orphaned feature flags
- All cfg-gated code references defined features

---

## Recommended Actions

### Immediate (High Priority)

1. **Remove unused dependencies** - Reduces compile time and binary size
   ```bash
   # Run cargo-machete periodically
   cargo machete --fix
   ```

2. **Fix `appreciated_letter` fixture** - Rename `glue.typ` to `plate.typ` or update `Quill.toml`

3. **Update user-facing documentation** - Replace MiniJinja syntax with Typst helper syntax

### Short-Term (Medium Priority)

4. **Remove ignored test** in `dry_run_test.rs` or convert to test new architecture

5. **Update design documents** - Add deprecation notes to `ARCHITECTURE.md` filter section

6. **Remove broken link** in `prose/designs/INDEX.md` to `TYPST_GUILLEMET_CONVERSION.md`

7. **Migrate CliError to thiserror** for consistency with rest of codebase

### Long-Term (Low Priority)

8. **Reduce public API surface** - Change unused exports to `pub(crate)`

9. **Create concrete WorldError type** - Replace `Box<dyn Error>` returns

10. **Add tests for unused fixtures** - Or remove `frontmatter_demo.md` and `sample.md`

---

## Procedural Remediation Plan (Divide & Conquer)

1. **Dependency Hygiene (Owner: Core Maintainer, Sprint: Current)**
   - Run `cargo machete --fix` per crate; commit lockfile changes.
   - Add CI check to fail on newly introduced unused dependencies.
2. **Fixture Repair (Owner: Docs/Fixtures, Sprint: Current)**
   - Update `appreciated_letter/Quill.toml` to point to an existing `.typ` file or rename `glue.typ` to `plate.typ`.
   - Validate fixtures via existing fixture update script; document outcome in changelog.
3. **Documentation Refresh (Owner: Docs, Sprint: Current)**
   - Replace MiniJinja syntax with Typst helpers in the four listed user-facing docs; verify examples render or lint where applicable.
   - Remove broken link in `prose/designs/INDEX.md`; add deprecation note in `ARCHITECTURE.md` filter section and rewrite `GLUE_METADATA.md` to current flow.
4. **Test & Design Cleanup (Owner: Core Maintainer, Sprint: Next)**
   - Remove or rewrite ignored `dry_run_test.rs` to exercise current architecture.
   - Decide on `frontmatter_demo.md`/`sample.md`: either add coverage or delete fixtures.
5. **API Surface Hardening (Owner: Core Maintainer, Sprint: Next)**
   - Narrow listed `pub` items in `quillmark-core` to `pub(crate)` where safe; ensure downstream crates still compile.
   - Introduce `WorldError` type and migrate `Box<dyn Error>` returns; migrate `CliError` to `thiserror` with minimal breakage.

---

## Appendix: Files Analyzed

### Crates Audited
- crates/core (9 source files)
- crates/quillmark (3 source files + 10 test files)
- crates/backends/typst (6 source files)
- crates/backends/acroform (1 source file)
- crates/bindings/cli (6 source files)
- crates/bindings/python (4 source files)
- crates/bindings/wasm (4 source files)
- crates/fixtures (1 source file)
- crates/fuzz (4 source files)

### Documentation Audited
- 14 files in docs/
- 18 files in prose/designs/
- 2 files in prose/plans/
- 3 files in prose/proposals/
- Root README.md and CONTRIBUTING.md

### Tools Used
- cargo clippy (dead code warnings)
- cargo machete (unused dependencies)
- grep/ripgrep (pattern searching)
- Manual code review

---

## Audit Methodology

This audit was conducted using a divide-and-conquer approach with 5 phases:

1. **Dead Code Analysis**: Ran Rust compiler warnings and manual code review
2. **Legacy Code Detection**: Searched for TODO comments, deprecated patterns
3. **Leftover Discovery**: Verified all files are included in module trees
4. **Documentation Audit**: Cross-referenced docs against actual code
5. **Cross-Reference Analysis**: Analyzed exports, imports, and feature flags

Each phase was executed by a general-purpose agent with specialized prompts, and findings were consolidated into this report.
