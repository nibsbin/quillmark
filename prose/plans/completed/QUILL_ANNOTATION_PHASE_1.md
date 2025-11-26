# Quill Annotation Revamp - Phase 1 Plan

**Status**: Design Phase
**Goal**: Simplify orchestration logic and consolidate workflow creation to prepare for metadata changes
**Related**: [`prose/QUILL_ANNOTATION.md`](../QUILL_ANNOTATION.md), [`prose/plans/CASCADES.md`](CASCADES.md)

---

## Overview

Phase 1 prepares the codebase for the Quill Annotation metadata enhancements (Phase 2 and 3) by:

1. **Simplifying workflow creation** - Reducing API surface from 3 methods to 1
2. **Centralizing the quill registry** - Eliminating duplicate storage in WASM bindings
3. **Improving orchestration structure** - Making the codebase easier to navigate and maintain

This plan follows the architecture principle: **consolidate before extending**. By cleaning up the orchestration layer first, Phase 2's metadata schema changes will have a simpler, more maintainable foundation.

---

## Current State Analysis

### Workflow Creation APIs

The `Quillmark` engine currently provides three workflow creation methods:

```
workflow_from_quill(QuillRef)      // Canonical implementation
workflow(&str)     // Thin wrapper → workflow_from_quill
workflow(ParsedDocument) // Thin wrapper → workflow
```

**Problem**: Multiple entry points create confusion about which method to use. The thin wrappers provide minimal value but increase API surface area and maintenance burden.

**Location**: `crates/quillmark/src/orchestration.rs:374-454`

### WASM Registry Duplication

The WASM bindings maintain a duplicate quill registry:

- `Quillmark.inner: quillmark::Quillmark` (has internal registry)
- `Quillmark.quills: HashMap<String, Quill>` (duplicate registry)

**Problem**:
- Memory overhead from storing every quill twice
- Risk of inconsistency between registries
- Manual synchronization required on register/unregister operations
- The duplicate registry is only used for `get_quill_info()` and `list_quills()` - both could query the core engine

**Location**: `crates/bindings/wasm/src/engine.rs:30-43`

### Orchestration File Structure

The `orchestration.rs` file (~758 lines) contains:
- `QuillRef` enum and conversions (lines 172-202)
- `Quillmark` engine struct and impl (lines 204-471)
- `Workflow` struct and impl (lines 473-757)

**Assessment**: The current structure is reasonably organized. The main complexity comes from:
- Dynamic asset/font management duplication (addressed in [`CASCADES.md`](CASCADES.md) Cascade 2)
- Multiple workflow creation entry points (addressed in this phase)

**No major file splitting needed** - the current organization is appropriate for the scope.

---

## Desired State

### 1. Unified Workflow Creation

**Single Entry Point**: `workflow_from_quill(impl Into<QuillRef>)`

This method accepts:
- `&str` - Looks up registered quill by name
- `&String` - Looks up registered quill by name
- `&Quill` - Uses quill directly (doesn't need to be registered)
- `&ParsedDocument` - Extracts quill tag and looks up by name

**Benefits**:
- Single, flexible API covers all use cases
- Fewer methods to document and maintain
- Clearer mental model for users
- Better IDE autocomplete experience

### 2. Centralized WASM Registry Access

**Remove**: `Quillmark.quills` HashMap from WASM bindings

**Replace with**: Query methods that delegate to the core engine

**Benefits**:
- Single source of truth for registered quills
- Reduced memory footprint
- No synchronization overhead
- Impossible for registries to drift

### 3. Maintained Orchestration Structure

**Keep**: Current file organization in `orchestration.rs`

**Rationale**:
- The code is well-organized with clear module-level documentation
- Splitting into multiple files would fragment related logic
- Real complexity reduction comes from Cascade 2 (Dynamic Collections)

---

## Migration Strategy

### Step 1: Extend QuillRef Enum

**Add**: Support for `ParsedDocument` in the `QuillRef` enum

```
QuillRef::Name(&str)
QuillRef::Object(&Quill)
QuillRef::Parsed(&ParsedDocument)  // NEW
```

**Rationale**: This allows `workflow_from_quill()` to handle all existing use cases.

**Impact**:
- Add new `From<&ParsedDocument>` implementation for `QuillRef`
- Update `workflow_from_quill()` match arm to handle `QuillRef::Parsed` variant
- Extract quill tag from parsed document and look up by name

### Step 2: Deprecate Wrapper Methods

**Mark deprecated**:
- `workflow()`
- `workflow()`

**Update** all internal callers to use `workflow_from_quill()`

**Rationale**: Gradual deprecation allows external users time to migrate.

**Impact**:
- Add `#[deprecated]` attributes with migration guidance
- Update rustdoc examples to show new API
- Update all examples in `examples/` directory
- Update WASM bindings to call `workflow_from_quill()` directly

### Step 3: Add Core Engine Query Methods

**Add** to `Quillmark` engine:
- `get_quill(&self, name: &str) -> Option<&Quill>`
- `get_quill_metadata(&self, name: &str) -> Option<&HashMap<String, QuillValue>>`

**Rationale**: These methods expose the core registry without requiring WASM to maintain a duplicate.

**Impact**:
- WASM `get_quill_info()` can query the core engine instead of local HashMap
- WASM `list_quills()` can delegate to `engine.registered_quills()`

### Step 4: Remove WASM Registry Duplication

**Remove**: `quills: HashMap<String, Quill>` field from WASM `Quillmark` struct

**Update**:
- `register_quill()` - Remove `self.quills.insert()` line
- `get_quill_info()` - Query via `self.inner.get_quill(name)`
- `list_quills()` - Delegate to `self.inner.registered_quills()`
- `unregister_quill()` - Remove (no unregister in core engine, or add it)

**Consideration**: The core engine currently doesn't support unregistering quills. Options:
- **Option A**: Add `unregister_quill()` to core engine
- **Option B**: Remove `unregister_quill()` from WASM (breaking change)
- **Option C**: Keep a minimal WASM-layer tracking for unregister only

**Recommendation**: Option A - Add unregister support to core engine for consistency.

### Step 5: Update Documentation

**Update**:
- `prose/designs/WASM.md` - Document removal of duplicate registry
- `crates/quillmark/src/orchestration.rs` - Update module docs and examples
- `crates/bindings/wasm/src/engine.rs` - Update struct-level documentation

**Add**:
- Migration guide in CHANGELOG for deprecated methods
- Update README examples if they reference old methods

---

## Cross-Cutting Concerns

### Backward Compatibility

**Breaking Changes**: None in Phase 1
- Deprecated methods remain functional
- WASM API surface unchanged (internal implementation only)

**Timeline**:
- Phase 1: Deprecate old methods, implement new unified API
- Phase 2-3: Coexist with deprecation warnings
- Future: Remove deprecated methods in major version bump

### Testing Strategy

**Update** existing tests to use `workflow_from_quill()`

**Add** tests for new `QuillRef::Parsed` variant

**Verify** WASM bindings work correctly with centralized registry

**Regression** testing:
- Ensure all deprecated methods still work
- Verify WASM `get_quill_info()` returns identical results
- Confirm `list_quills()` output unchanged

### Performance Impact

**Expected**: Negligible to positive

- Removing WASM duplicate registry reduces memory usage
- Fewer method dispatch paths may improve compile times
- Registry lookups are HashMap O(1) operations

**Measurement**: Not required - changes are purely structural

---

## Implementation Checklist

Phase 1 implementation follows this sequence:

- [ ] **Step 1**: Extend `QuillRef` to support `ParsedDocument`
  - Add `QuillRef::Parsed(&ParsedDocument)` variant
  - Implement `From<&ParsedDocument>` for `QuillRef`
  - Update `workflow_from_quill()` match logic

- [ ] **Step 2**: Deprecate wrapper methods
  - Add `#[deprecated]` to `workflow()`
  - Add `#[deprecated]` to `workflow()`
  - Update internal callers to use `workflow_from_quill()`

- [ ] **Step 3**: Add core engine query methods
  - Implement `Quillmark::get_quill()`
  - Implement `Quillmark::get_quill_metadata()`
  - Add `Quillmark::unregister_quill()` for WASM support

- [ ] **Step 4**: Remove WASM registry duplication
  - Remove `quills` HashMap field from WASM `Quillmark`
  - Update `get_quill_info()` to query core engine
  - Update `list_quills()` to delegate to core
  - Update `unregister_quill()` to call core method

- [ ] **Step 5**: Update documentation
  - Update `prose/designs/WASM.md`
  - Update orchestration module docs
  - Add deprecation migration guide
  - Update examples and README

- [ ] **Testing**: Run full test suite
  - Unit tests for new `QuillRef` variant
  - Integration tests for workflow creation
  - WASM bindings tests
  - Regression tests for deprecated methods

---

## Dependencies

**Blocks**: Phase 2 (Metadata Schema Strategy)
- Phase 2 will add `Section` and `Tooltip` fields to quill metadata
- Cleaner workflow creation and centralized registry make Phase 2 changes simpler

**Blocked by**: None
- Phase 1 can proceed immediately

**Related**: [`prose/plans/CASCADES.md`](CASCADES.md) Cascade 2
- Dynamic asset/font consolidation is complementary but independent
- Can be implemented in parallel or sequence

---

## Success Criteria

Phase 1 is complete when:

1. ✅ Single `workflow_from_quill()` method handles all workflow creation use cases
2. ✅ WASM bindings use core engine registry (no duplicate HashMap)
3. ✅ All existing tests pass with deprecation warnings only
4. ✅ Documentation updated to reflect new API patterns
5. ✅ No breaking changes to public APIs

**Verification**:
- Run `cargo test --all-features`
- Run `cargo clippy -- -D warnings`
- Build WASM bindings: `wasm-pack build crates/bindings/wasm`
- Check documentation: `cargo doc --no-deps --open`

---

## Notes

### Why Not Split orchestration.rs?

The initial Phase 1 goal mentioned "split orchestration.rs into a cleaner file structure." After analysis:

**Current state**: 758 lines containing two main structs (`Quillmark` and `Workflow`)
- Well-documented with clear module-level rustdoc
- Logical grouping of engine and workflow concerns
- No single "problem area" requiring isolation

**Alternative considered**: Split into `engine.rs` and `workflow.rs`
- **Downside**: Fragments closely-related orchestration logic
- **Downside**: Forces reader to navigate multiple files
- **Upside**: Slightly smaller files (~400 lines each)

**Decision**: Keep unified `orchestration.rs`
- The real complexity is duplicate code patterns (dynamic collections)
- File splitting doesn't address the root complexity
- See [`CASCADES.md`](CASCADES.md) for where actual simplification happens

### WASM Unregister Decision

The `unregister_quill()` method in WASM creates a design choice:

**Why it exists**: Free memory in long-lived browser sessions

**Current implementation**: Only removes from WASM duplicate registry

**Problem**: After WASM unregister, quill still exists in core engine

**Options**:
1. Add `unregister_quill()` to core engine (chosen approach)
2. Document current behavior as "WASM-layer only"
3. Remove `unregister_quill()` entirely

**Chosen**: Option 1 - Add to core engine
- Most consistent with user expectations
- Properly frees memory in both layers
- Enables core engine users (CLI, Python) to manage memory
