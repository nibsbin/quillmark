# Quill Versioning System Completion Plan
## Addressing Implementation Gaps and Code Quality Issues

**Date:** 2026-01-22
**Status:** Active
**Context:** Complete the Quill versioning system implementation by addressing missing features, test coverage gaps, and code quality issues identified in the post-implementation review.
**Related Documents:**
- [VERSIONING.md](../designs/VERSIONING.md) - Design specification
- [QUILL_VERSIONING_IMPLEMENTATION.md](completed/QUILL_VERSIONING_IMPLEMENTATION.md) - Original implementation plan

---

## Executive Summary

The core versioning system (`Version`, `VersionSelector`, `QuillReference`) is well-implemented with proper types, parsing, and resolution logic. However, the implementation is **incomplete** and has **critical gaps** that prevent it from being production-ready:

**Missing Completeness:**
- ❌ No CLI commands for version management (versions, resolve, pin, upgrade)
- ❌ No integration tests validating the end-to-end system
- ❌ No fixture examples demonstrating version usage
- ❌ Documentation contains incorrect file paths

**Code Quality Issues:**
- ⚠️ Silent error fallback in parser can hide user mistakes
- ⚠️ Ambiguous error types (VersionNotFound used for two different cases)
- ⚠️ Missing edge case validation (empty version sets)

**Impact:** Users cannot discover versions, manage document pinning, or learn by example. The system lacks validation that it works as designed.

---

## Problem Areas

### 1. Missing CLI Commands

**Current State:** No CLI commands exist for version management despite being specified in the original design.

**Impact:**
- Users cannot list available versions for a template
- No way to test version resolution without rendering
- Cannot programmatically pin documents to specific versions
- No upgrade assistance when moving to newer template versions

**Evidence:**
```bash
# These commands were specified but don't exist:
$ quillmark versions resume_template        # Not implemented
$ quillmark resolve resume_template@2       # Not implemented
$ quillmark pin document.md                 # Not implemented
$ quillmark upgrade document.md             # Not implemented
```

Current CLI only has: `render`, `schema`, `validate` (see `crates/bindings/cli/src/main.rs:18-27`)

### 2. Missing Integration Tests

**Current State:** Only unit tests exist in `version.rs`. No tests validate the system working end-to-end.

**Impact:**
- No validation that version resolution works in practice
- No validation that documents with version syntax parse correctly
- No validation that multiple versions can coexist
- No validation that error messages are helpful

**Evidence:**
- `version.rs` has 8 unit tests (parsing, validation)
- `engine.rs` has 0 tests for versioning
- `quill_engine_test.rs` doesn't test multi-version scenarios
- No tests use `@version` syntax in QUILL tags

**Missing Test Scenarios:**
- Registering multiple versions of same quill (e.g., 1.0, 1.1, 2.0)
- Resolving `@2` to latest 2.x version
- Parsing documents with `QUILL: resume@2.1` syntax
- Version collision detection (same name+version registered twice)
- Helpful error messages when version not found
- Workflow creation from versioned document

### 3. Missing Fixture Examples

**Current State:** All fixtures use unversioned QUILL tags.

**Impact:**
- No reference implementations for users to learn from
- No examples in documentation work without modification
- Cannot visually validate version syntax in real documents

**Evidence:**
```markdown
# All fixture documents use this pattern:
QUILL: cmu_letter
QUILL: taro
QUILL: classic_resume

# None use versioned syntax:
QUILL: cmu_letter@0.1
QUILL: taro@0
```

### 4. Documentation Issues

**Current State:** `prose/designs/VERSIONING.md` references incorrect file paths.

**Impact:** Developers looking at implementation find wrong files.

**Errors:**
- Says `quillmark-core/src/version.rs` → should be `crates/core/src/version.rs`
- Says `quillmark/src/orchestration/engine.rs` → should be `crates/quillmark/src/orchestration/engine.rs`

### 5. Silent Error Fallback (Code Quality)

**Current State:** `ParsedDocument::with_quill_tag()` silently treats parse errors as valid quill names.

**Location:** `crates/core/src/parse.rs:74-81`

```rust
pub fn with_quill_tag(fields: HashMap<String, QuillValue>, quill_tag: String) -> Self {
    let quill_ref = quill_tag.parse().unwrap_or_else(|_| {
        // Silent fallback - treats parse error as quill name
        QuillReference::latest(quill_tag)
    });
    Self { fields, quill_ref }
}
```

**Impact:**
- User writes `QUILL: resume@2.x` (invalid)
- System silently treats "resume@2.x" as quill name with Latest selector
- User gets confusing "Quill 'resume@2.x' not registered" error instead of "invalid version syntax"

**Risk:** Medium - confusing error messages, masks user mistakes

### 6. Ambiguous Error Semantics

**Current State:** `RenderError::VersionNotFound` is used for two distinct error cases.

**Location:** `crates/quillmark/src/orchestration/engine.rs:258-270`

```rust
let version_set = self.quills.get(&quill_ref.name)
    .ok_or_else(|| RenderError::VersionNotFound { ... })?;  // Case 1: Quill doesn't exist

version_set.resolve(&quill_ref.selector)
    .ok_or_else(|| RenderError::VersionNotFound { ... })    // Case 2: Version doesn't match
```

**Impact:**
- Error messages say "Version not found" when quill itself doesn't exist
- Semantically incorrect - "version not found" implies the quill exists
- Harder to provide targeted error messages

**Risk:** Low - error messages are still helpful, just semantically imprecise

### 7. Missing Edge Case Validation

**Current State:** No validation prevents empty `VersionedQuillSet` with Latest selector.

**Location:** `crates/quillmark/src/orchestration/engine.rs:42-46`

```rust
VersionSelector::Latest => {
    self.versions.iter().next_back().map(|(_, quill)| quill)
    // Returns None if versions is empty - but when can this happen?
}
```

**Impact:**
- Theoretically could have quill name with zero versions
- Latest selector would silently fail
- Unclear if this is actually possible given registration logic

**Risk:** Very Low - likely impossible given current code, but lacks assertion

---

## Proposed Solutions

### Phase 1: Critical Completeness (High Priority)

**Goal:** Complete the minimum viable versioning system that users can actually use.

#### 1.1 Add Integration Tests

**Scope:** Comprehensive test coverage validating the system works end-to-end.

**Test Files to Create:**
- `crates/quillmark/tests/versioning_test.rs` - Main integration test suite

**Test Scenarios:**
```rust
#[test]
fn test_register_multiple_versions_same_quill() {
    // Register versions 1.0, 1.1, 2.0 of same quill
    // Verify all are registered
}

#[test]
fn test_resolve_major_version_selector() {
    // Register 2.0, 2.1, 2.2, 3.0
    // Resolve @2 -> should get 2.2
    // Resolve @3 -> should get 3.0
}

#[test]
fn test_resolve_exact_version_selector() {
    // Resolve @2.1 -> should get exactly 2.1
    // Resolve @2.5 (not registered) -> should error
}

#[test]
fn test_parse_document_with_version_syntax() {
    // Parse document with QUILL: resume@2.1
    // Verify ParsedDocument.quill_reference() is correct
}

#[test]
fn test_workflow_from_versioned_document() {
    // Create workflow from document with versioned QUILL tag
    // Verify correct version is selected
}

#[test]
fn test_version_collision_error() {
    // Try to register same name+version twice
    // Verify proper error
}

#[test]
fn test_version_not_found_error_message() {
    // Request nonexistent version
    // Verify helpful error with suggestions
}

#[test]
fn test_latest_selector_with_multiple_versions() {
    // Register 1.0, 2.0, 3.0
    // Resolve @latest or no selector -> should get 3.0
}
```

**Estimated Effort:** 4-6 hours

**Acceptance Criteria:**
- All scenarios above pass
- Code coverage for versioning paths >90%
- Tests use realistic fixture quills

#### 1.2 Create Fixture Examples

**Scope:** Add versioned documents and multi-version quills to fixtures.

**Files to Create/Modify:**

1. **Multi-version quill fixture:**
   - `crates/fixtures/resources/classic_resume/` (modify existing)
   - Add `v1.0/`, `v2.0/` subdirectories with different Quill.toml versions
   - Or: Create new `multi_version_test/` fixture specifically for this

2. **Versioned documents:**
   - `crates/fixtures/resources/examples/versioned_resume_exact.md`
     ```markdown
     ---
     QUILL: classic_resume@2.1
     name: John Doe
     ---
     # Resume content
     ```

   - `crates/fixtures/resources/examples/versioned_resume_major.md`
     ```markdown
     ---
     QUILL: classic_resume@2
     name: Jane Smith
     ---
     # Resume content
     ```

3. **Update existing fixtures:**
   - Modify 2-3 existing documents to use version syntax
   - Demonstrates real-world usage patterns

**Estimated Effort:** 2-3 hours

**Acceptance Criteria:**
- At least one quill exists in multiple versions
- At least 3 documents demonstrate version syntax (@exact, @major, @latest)
- Fixtures render successfully with versioned references

#### 1.3 Fix Documentation Paths

**Scope:** Correct file paths in VERSIONING.md design document.

**Changes:**
```markdown
# Before:
**Implementation**: `quillmark-core/src/version.rs`, `quillmark/src/orchestration/engine.rs`

# After:
**Implementation**: `crates/core/src/version.rs`, `crates/quillmark/src/orchestration/engine.rs`
```

**Estimated Effort:** 15 minutes

**Acceptance Criteria:**
- All file paths in VERSIONING.md are correct and reachable
- Paths follow actual crate structure

### Phase 2: Code Quality Fixes (High Priority)

**Goal:** Fix brittle code patterns that could cause confusion or hide errors.

#### 2.1 Fix Silent Parsing Fallback

**Scope:** Make version parse errors visible to users.

**Current Code:** `crates/core/src/parse.rs:74-81`

**Option A: Propagate Parse Error (Preferred)**
```rust
pub fn with_quill_tag(fields: HashMap<String, QuillValue>, quill_tag: String) -> Result<Self, ParseError> {
    let quill_ref = QuillReference::from_str(&quill_tag)
        .map_err(|e| ParseError::InvalidStructure(
            format!("Invalid QUILL tag '{}': {}", quill_tag, e)
        ))?;
    Ok(Self { fields, quill_ref })
}
```

**Option B: Log Warning + Fallback (Backward Compatible)**
```rust
pub fn with_quill_tag(fields: HashMap<String, QuillValue>, quill_tag: String) -> Self {
    let quill_ref = quill_tag.parse().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to parse QUILL tag '{}': {}. Treating as quill name.", quill_tag, e);
        QuillReference::latest(quill_tag)
    });
    Self { fields, quill_ref }
}
```

**Recommendation:** Option A (breaking but correct). This is pre-1.0, better to fail fast.

**Impact Analysis:**
- Breaking change: `with_quill_tag()` signature changes to return `Result`
- Call sites in `decompose()` need updating
- More correct behavior: invalid syntax is caught at parse time

**Estimated Effort:** 1-2 hours (includes updating call sites and tests)

**Acceptance Criteria:**
- Invalid version syntax produces clear parse error
- Error message shows what was invalid and why
- Tests validate error cases

#### 2.2 Add QuillNotFound Error Variant

**Scope:** Distinguish between "quill doesn't exist" and "version doesn't match".

**Changes:**

1. **Add error variant:** `crates/core/src/error.rs`
```rust
pub enum RenderError {
    // Existing
    VersionNotFound { diag: Box<Diagnostic> },

    // New
    QuillNotFound { diag: Box<Diagnostic> },

    // ...
}
```

2. **Update usage:** `crates/quillmark/src/orchestration/engine.rs:258`
```rust
let version_set = self.quills.get(&quill_ref.name)
    .ok_or_else(|| RenderError::QuillNotFound {  // Changed from VersionNotFound
        diag: Box::new(
            Diagnostic::new(
                Severity::Error,
                format!("Quill '{}' not registered", quill_ref.name),
            )
            .with_code("engine::quill_not_found".to_string())
            .with_hint(format!(
                "Available quills: {}",
                self.quills.keys().cloned().collect::<Vec<_>>().join(", ")
            )),
        ),
    })?;
```

**Estimated Effort:** 1 hour

**Acceptance Criteria:**
- QuillNotFound used when quill doesn't exist
- VersionNotFound used when quill exists but version doesn't match
- Error messages are distinct and clear
- Tests validate both error types

#### 2.3 Add Empty Version Set Validation

**Scope:** Add defensive assertion preventing impossible edge case.

**Changes:** `crates/quillmark/src/orchestration/engine.rs:42-46`

```rust
VersionSelector::Latest => {
    let result = self.versions.iter().next_back().map(|(_, quill)| quill);
    debug_assert!(result.is_some(), "VersionedQuillSet should never be empty");
    result
}
```

Or more robust:

```rust
impl VersionedQuillSet {
    fn insert(&mut self, version: Version, quill: Quill) {
        self.versions.insert(version, quill);
    }

    fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }
}

// In engine.rs register_quill():
let version_set = self.quills.entry(name.clone())
    .or_insert_with(VersionedQuillSet::new);
version_set.insert(version, quill);
debug_assert!(!version_set.is_empty(), "Just inserted version");
```

**Estimated Effort:** 30 minutes

**Acceptance Criteria:**
- Debug assertion catches impossible state in development
- No performance impact in release builds

### Phase 3: CLI Commands (Medium Priority)

**Goal:** Provide user-facing tools for version management.

#### 3.1 Implement `quillmark versions` Command

**Scope:** List all available versions for a template.

**Interface:**
```bash
$ quillmark versions resume_template
Available versions for resume_template:
  3.0
  2.2
  2.1
  2.0
  1.0
```

**Implementation:**
- Add `versions` subcommand to `crates/bindings/cli/src/main.rs`
- Create `crates/bindings/cli/src/commands/versions.rs`
- Add method to `Quillmark` engine: `pub fn list_versions(&self, name: &str) -> Vec<Version>`

**Error Handling:**
```bash
$ quillmark versions nonexistent
Error: Quill 'nonexistent' not registered

Available quills: resume_template, letter_template, __default__
```

**Estimated Effort:** 2-3 hours

**Acceptance Criteria:**
- Lists versions in descending order (newest first)
- Clear error if quill not registered
- Help text explains usage

#### 3.2 Implement `quillmark resolve` Command

**Scope:** Show what version a selector resolves to.

**Interface:**
```bash
$ quillmark resolve resume_template@2
resume_template@2 → 2.2

$ quillmark resolve resume_template@2.1
resume_template@2.1 → 2.1

$ quillmark resolve resume_template
resume_template → 3.0 (latest)
```

**Implementation:**
- Add `resolve` subcommand
- Create `crates/bindings/cli/src/commands/resolve.rs`
- Use existing `resolve_quill_reference()` logic

**Error Handling:**
```bash
$ quillmark resolve resume_template@2.5
Error: Version not found
  Template: resume_template
  Requested: @2.5
  Available: 3.0, 2.2, 2.1, 2.0, 1.0

Suggestion: Use @2 for latest 2.x (currently 2.2)
```

**Estimated Effort:** 2-3 hours

**Acceptance Criteria:**
- Shows resolved version clearly
- Indicates when using latest (implicit or explicit)
- Helpful error messages with suggestions

#### 3.3 Implement `quillmark pin` Command

**Scope:** Add or update version in document's QUILL tag.

**Interface:**
```bash
# Pin to exact current version
$ quillmark pin document.md
Updated document.md: QUILL: "resume_template@2.1"

# Pin to major version
$ quillmark pin document.md --major
Updated document.md: QUILL: "resume_template@2"

# Show current version without changing
$ quillmark pin document.md --show
document.md uses: resume_template@2.1 (resolved from resume_template@2)
```

**Implementation:**
- Add `pin` subcommand
- Create `crates/bindings/cli/src/commands/pin.rs`
- Parse document, resolve current version, rewrite QUILL tag
- Preserve formatting where possible

**Safety:**
- Default: show what would change, require --yes to modify
- Or: default modifies, --dry-run to preview

**Estimated Effort:** 4-5 hours (file modification is tricky)

**Acceptance Criteria:**
- Correctly updates QUILL tag preserving document formatting
- Validates document parses before modification
- Clear output showing what changed

#### 3.4 Implement `quillmark upgrade` Command

**Scope:** Update document to newer template version with warnings.

**Interface:**
```bash
$ quillmark upgrade document.md
Current: resume_template@2.1
Latest:  resume_template@3.0

Warning: Major version change (2 → 3) may contain breaking changes.
Review changelog before proceeding.
Proceed? [y/N]

# Minor upgrades
$ quillmark upgrade document.md --minor-only
Current: resume_template@2.1
Latest:  resume_template@2.2 (within major version 2)
Proceed? [Y/n]
```

**Implementation:**
- Add `upgrade` subcommand
- Create `crates/bindings/cli/src/commands/upgrade.rs`
- Resolve current version, find latest, prompt for confirmation
- Special handling for major version changes

**Estimated Effort:** 3-4 hours

**Acceptance Criteria:**
- Warns on major version changes
- `--minor-only` flag restricts to same major version
- Interactive confirmation (skip with `--yes`)

**CLI Commands Total Estimated Effort:** 11-15 hours

### Phase 4: Documentation & Polish (Low Priority)

#### 4.1 Add Usage Examples to VERSIONING.md

**Scope:** Add practical examples section to design document.

**Content:**
- Example: Registering multiple versions
- Example: Parsing versioned documents
- Example: Using CLI commands
- Example: Migration workflow

**Estimated Effort:** 1-2 hours

#### 4.2 Add Migration Guide

**Scope:** Document how to add versions to existing quills.

**Content:**
- Step-by-step: adding version field
- Choosing initial version number
- Registering multiple versions
- Pinning existing documents

**Estimated Effort:** 1 hour

---

## Implementation Priority

### Must-Have (Blocking Production)
1. **Integration tests** (Phase 1.1) - Validates system works
2. **Fix silent parsing fallback** (Phase 2.1) - Prevents silent errors
3. **Fixture examples** (Phase 1.2) - Enables learning by example
4. **Fix documentation paths** (Phase 1.3) - Prevents confusion

**Estimated Total:** 8-12 hours

### Should-Have (Improves Quality)
5. **Add QuillNotFound error** (Phase 2.2) - Better error semantics
6. **Add empty version validation** (Phase 2.3) - Defensive programming
7. **CLI commands** (Phase 3) - User-facing tooling

**Estimated Total:** 12-17 hours

### Nice-to-Have (Polish)
8. **Usage examples** (Phase 4.1) - Better documentation
9. **Migration guide** (Phase 4.2) - Onboarding assistance

**Estimated Total:** 2-3 hours

---

## Testing Strategy

### Unit Tests (Already Exist)
- ✅ Version parsing and validation (`version.rs`)
- ✅ Version comparison and ordering
- ✅ QuillReference parsing

### Integration Tests (To Be Added)
- **Multi-version registration** - Verify versions coexist correctly
- **Version resolution** - All selector types work correctly
- **Document parsing** - Version syntax in QUILL tags
- **Workflow creation** - End-to-end with versioned documents
- **Error scenarios** - Helpful messages for common mistakes

### CLI Tests (To Be Added)
- **Command execution** - All commands work correctly
- **Error handling** - Proper exit codes and messages
- **File modification** - Pin/upgrade modify documents correctly

### Regression Tests
- **Error messages** - Validate helpful error text doesn't degrade
- **Backward compatibility** - Unversioned documents still work

---

## Risk Assessment

### Low Risk
- ✅ Integration tests - Pure addition, no breaking changes
- ✅ Fixture examples - Documentation improvement
- ✅ Documentation fixes - Text-only changes
- ✅ CLI commands - New features, don't affect existing code

### Medium Risk
- ⚠️ **Silent parsing fallback fix** - Breaking change to `with_quill_tag()` signature
  - **Mitigation:** Pre-1.0 allows breaking changes, fix is more correct
  - **Impact:** Call sites need updating, tests need updating

### High Risk
- None identified

---

## Success Criteria

The versioning system is complete when:

1. ✅ **Integration tests pass** - All version resolution scenarios work
2. ✅ **Fixtures demonstrate usage** - Users can learn by example
3. ✅ **Documentation is accurate** - File paths are correct
4. ✅ **Parse errors are visible** - Invalid syntax doesn't silently fall back
5. ✅ **CLI commands work** - Users can list, resolve, pin, upgrade versions
6. ✅ **Error semantics are clear** - QuillNotFound vs VersionNotFound distinct
7. ✅ **Code is defensive** - Edge cases have assertions

---

## Open Questions

### 1. Should `with_quill_tag()` breaking change be deferred?

**Options:**
- A) Fix now (pre-1.0, correct behavior)
- B) Add warning, fix in next breaking release
- C) Keep silent fallback forever

**Recommendation:** Fix now (Option A). Pre-1.0 allows breaking changes, and correctness is more important than backward compatibility at this stage.

### 2. Should CLI commands be in Phase 1?

**Current:** Phase 3 (medium priority)

**Argument for Phase 1:** Without CLI, users can't discover versions or manage pinning. This limits practical usability significantly.

**Argument for Phase 3:** Core library functionality works without CLI. API users can build their own tooling.

**Recommendation:** Keep in Phase 3. Library-first approach is fine. CLI is important but not blocking.

### 3. Should we add `list_versions()` public API method?

**Not currently exposed:** Users of the library have no way to list available versions programmatically.

**Proposal:**
```rust
impl Quillmark {
    pub fn list_versions(&self, name: &str) -> Option<Vec<Version>> {
        self.quills.get(name)
            .map(|vs| vs.available_versions())
    }
}
```

**Recommendation:** Yes, add this. Useful for CLI and for library users building version-aware tools.

---

## Next Steps

1. **Review this plan** - Validate priorities and approach
2. **Phase 1: Critical Completeness** - Integration tests, fixtures, docs
3. **Phase 2: Code Quality** - Fix silent fallback, add error variant
4. **Phase 3: CLI Commands** - Implement user-facing tooling
5. **Phase 4: Documentation** - Usage examples and migration guide

**Estimated Total Effort:** 22-32 hours for complete implementation

---

## Conclusion

The versioning system has a solid foundation but lacks completion in critical areas. This plan addresses the gaps in priority order, focusing first on validation (tests), discoverability (fixtures/docs), and correctness (parse errors), before adding user-facing tooling (CLI) and polish (documentation).

The "must-have" work (8-12 hours) brings the system to production-ready status. The "should-have" work (12-17 hours) makes it genuinely user-friendly. The "nice-to-have" work (2-3 hours) provides excellent documentation.
