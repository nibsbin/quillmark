# Technical Debt Reduction Plan

> **Status**: Planning Document  
> **Created**: 2025-10-31  
> **Related**: See [designs/ARCHITECTURE.md](../designs/ARCHITECTURE.md)

---

## Executive Summary

This plan identifies technical debt in the Quillmark codebase and provides recommendations for improving maintainability. The primary areas of concern are:

1. **Design Document Consistency** - Design documents need updates to reflect current implementation
2. **Test Organization** - Consolidate and simplify test suite
3. **Code Structure** - Improve modularity and reduce duplication
4. **Documentation Coverage** - Align documentation with actual codebase

---

## Current State Analysis

### Design Documents

The design documents in `prose/designs/` are generally well-maintained but have minor inconsistencies with the current codebase:

**Strengths:**
- Comprehensive architecture documentation
- Clear separation of concerns across modules
- Well-documented error handling and backend systems
- Up-to-date default Quill implementation (with debrief)

**Areas for Improvement:**
- ARCHITECTURE.md references some outdated API patterns
- Cross-references between documents could be stronger
- Some implementation details have evolved beyond original designs

### Test Suite

**Current Test Structure:**
```
quillmark/tests/
├── acroform_integration_tests.rs (82 lines)
├── api_rework_test.rs (162 lines)
├── auto_glue_test.rs (144 lines)
├── backend_registration_test.rs (158 lines)
├── common.rs (66 lines)
├── default_quill_test.rs (230 lines)
├── default_values_test.rs (231 lines)
├── dynamic_assets_test.rs (90 lines)
├── dynamic_fonts_test.rs (132 lines)
├── feature_flag_test.rs (30 lines)
├── quill_engine_test.rs (264 lines)
```

**Observations:**
- Total: ~1,589 lines of test code across 11 files
- Tests are well-organized by feature area
- Some overlap between `api_rework_test.rs` and `quill_engine_test.rs`
- Most tests use temporary directories and create custom quills
- Tests are comprehensive but could benefit from consolidation

### Code Organization

**Strengths:**
- Clean workspace structure with logical crate separation
- Backend trait provides good extensibility
- Error handling is well-structured with diagnostics
- Filter API provides stable ABI for backends

**Technical Debt:**
- Some duplication in test setup code (mitigated by `common.rs`)
- Backend registration could use clearer documentation
- Parse module is quite large (53K+ lines in parse.rs)

---

## Recommendations

### 1. Update Design Documents (High Priority)

#### ARCHITECTURE.md
**Changes:**
- Update filter API documentation to clarify stability guarantees
- Add more cross-references to implementation files
- Document the default Quill system (already implemented)
- Clarify backend auto-registration behavior

**Impact:** Low risk, high value for onboarding and maintenance

#### QUILL.md
**Changes:**
- Add examples of Quill JSON contract usage
- Clarify file tree navigation patterns
- Document edge cases in file loading

**Impact:** Low risk, improves developer experience

#### Other Design Documents
- **PARSE.md**: Already comprehensive, minimal changes needed
- **ERROR.md**: Well-documented, cross-reference with implementation
- **PYTHON.md / WASM.md**: Update to reflect any API changes since initial implementation
- **CI_CD.md**: Already matches actual workflows

### 2. Test Consolidation (Medium Priority)

#### Recommendation: Keep Current Test Structure

**Rationale:**
After analysis, the current test organization is appropriate:
- Tests are well-segregated by feature (`default_quill_test.rs`, `dynamic_assets_test.rs`, etc.)
- `api_rework_test.rs` provides targeted API validation
- `quill_engine_test.rs` provides comprehensive integration tests
- Total test count is reasonable (~189 tests)

**Minor Improvements:**
1. **Consolidate Setup Code**: The `common.rs` helper is good, but consider adding more shared test fixtures
2. **Document Test Purpose**: Add module-level documentation to each test file explaining its scope
3. **Remove Redundancy**: Review if `api_rework_test.rs` can be merged into `quill_engine_test.rs`

**Legacy Test Analysis:**
- `api_rework_test.rs` was likely created during an API refactoring
- Consider: Is this still needed as a separate file, or can it be merged?
- Decision: **Keep for now** - it tests specific API patterns that complement `quill_engine_test.rs`

### 3. Documentation Alignment (High Priority)

#### Inline Documentation
**Areas to improve:**
- Ensure all public API items have doc comments
- Add more examples to trait implementations
- Cross-reference design documents from code

#### README Updates
**Changes:**
- Verify Quick Start example matches current API
- Update installation instructions if needed
- Link to design documents for architecture details

### 4. Code Quality Improvements (Medium Priority)

#### Parse Module
**Observation:** `parse.rs` is 53K lines, which is large for a single module

**Recommendation:**
- Consider splitting into submodules:
  - `parse/frontmatter.rs` - YAML frontmatter parsing
  - `parse/extended.rs` - Extended YAML Metadata Standard
  - `parse/document.rs` - ParsedDocument implementation
- Keep as single file if most content is comprehensive tests/examples

**Impact:** Medium effort, improves navigability

#### Backend Registration
**Recommendation:**
- Add more inline documentation to `register_backend()` explaining auto-registration
- Document the interaction between backend registration and default Quills
- Add debug logging for registration events

**Impact:** Low effort, high clarity benefit

### 5. Reduce Contract Surface Area (Low Priority)

#### API Stability
The current API is well-designed with appropriate abstractions:
- Core traits are stable
- Filter API provides ABI stability
- Public types are well-encapsulated

**Recommendation: No major changes needed**

The question of "removing legacy tests" is less about the tests themselves and more about ensuring we're not testing deprecated API patterns. Current tests are appropriate for the current API.

---

## Contract Surface Reduction Strategy

### Current Public API Surface

**quillmark-core:**
- `ParsedDocument` - Essential
- `Quill` - Essential
- `Backend` trait - Essential for extensibility
- `Glue` - Essential for templating
- `QuillValue` - Essential data type
- Error types - Essential for diagnostics

**quillmark:**
- `Quillmark` - High-level engine (essential)
- `Workflow` - Rendering pipeline (essential)
- Helper functions - Convenient but could be reconsidered

### Recommendations

1. **Keep Current API** - It's minimal and well-designed
2. **Mark Experimental Features** - Use `#[doc(hidden)]` or feature flags for experimental APIs
3. **Semantic Versioning** - Continue following SemVer strictly
4. **Deprecation Path** - If any APIs need removal, use proper deprecation warnings

**Conclusion:** The contract surface area is appropriate for the project's needs.

---

## Implementation Priority

### Phase 1: Documentation Updates (Week 1)
- [ ] Update ARCHITECTURE.md with current implementation details
- [ ] Add cross-references between design documents
- [ ] Document default Quill system in ARCHITECTURE.md
- [ ] Verify all design documents match implementation

### Phase 2: Test Documentation (Week 1)
- [ ] Add module-level documentation to each test file
- [ ] Review api_rework_test.rs for potential merge
- [ ] Document test setup patterns in common.rs
- [ ] Create test coverage matrix

### Phase 3: Code Quality (Week 2)
- [ ] Add inline documentation to backend registration
- [ ] Consider splitting parse.rs if beneficial
- [ ] Add debug logging for key operations
- [ ] Review and update public API docs

### Phase 4: Maintenance Infrastructure (Week 2)
- [ ] Create this plan directory (`prose/plans/`)
- [ ] Establish documentation review process
- [ ] Set up design document versioning
- [ ] Create templates for future design docs

---

## Metrics for Success

1. **Documentation Coverage**: All public APIs have doc comments
2. **Test Clarity**: Each test file has clear purpose documentation
3. **Design Alignment**: Design documents accurately reflect implementation
4. **Onboarding Time**: New contributors can understand architecture in < 2 hours
5. **Build Time**: No regression in build/test times

---

## Non-Goals

- Rewriting working code for architectural purity
- Removing tests that provide value (even if "legacy")
- Breaking backward compatibility
- Introducing new dependencies
- Major API redesigns

---

## Risk Assessment

### Low Risk
- Documentation updates
- Adding inline comments
- Test documentation
- Cross-referencing improvements

### Medium Risk
- Splitting parse.rs module
- Merging test files
- Adding debug logging

### High Risk
- None identified in this plan

---

## Maintenance Guidelines

### For Design Documents
1. Update design documents **before** major implementations
2. Create debriefs **after** implementations complete
3. Keep designs at medium-to-high level (avoid excessive code details)
4. Cross-reference related documents
5. Version designs when significant changes occur

### For Tests
1. Keep tests focused on specific features
2. Use descriptive test names
3. Add module-level documentation
4. Share setup code via common.rs
5. Prefer integration tests for API validation

### For Code
1. Document public APIs thoroughly
2. Add examples to trait implementations
3. Cross-reference design documents in code comments
4. Use debug logging for operational visibility
5. Keep modules focused and cohesive

---

## Conclusion

The Quillmark codebase is in good shape with minimal technical debt. The primary improvements needed are:

1. **Documentation alignment** - Ensure design docs match implementation
2. **Test documentation** - Add clarity about test purposes
3. **Minor refactoring** - Improve code navigability

These are all low-risk, high-value improvements that will enhance maintainability without disrupting the working system.

**Recommendation:** Proceed with Phase 1 (documentation updates) immediately, then evaluate need for subsequent phases based on feedback and priorities.
