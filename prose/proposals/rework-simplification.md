# Typst JSON Data Delivery Rework: Simplification Opportunities

**Date:** 2026-01-14  
**Context:** Analysis of redundant, legacy, or dead code following the JSON data delivery implementation  
**Status:** Review document for planned simplifications

---

## Summary

The Typst JSON Data Delivery rework successfully eliminated MiniJinja templating. This document identifies remaining opportunities for code cleanup and simplification.

---

## Findings

### 1. ✅ **MiniJinja Dependency: Successfully Removed**

- **`quillmark-core/Cargo.toml`**: No MiniJinja dependency.
- **`templating.rs`**: Module completely removed.
- **`register_filters()`**: Trait method removed from `Backend` trait.

The only remaining MiniJinja usage is in `quillmark-acroform` (AcroForm backend), which is appropriate since that backend uses its own templating approach.

---

### 2. ⚠️ **`filters.rs`: Consider Removal or Consolidation**

**Location:** `crates/backends/typst/src/filters.rs`

**Current State:** This file now contains only a single utility function:

```rust
pub fn inject_json(bytes: &str) -> String {
    format!("json(bytes(\"{}\"))", escape_string(bytes))
}
```

**Analysis:**
- The function is used by fuzzing tests (`crates/fuzz/src/filter_fuzz.rs`) via `fuzz_utils::inject_json`
- It's also used by the helper package generator (`helper.rs` via `escape_string`)

**Recommendation:** 
- **Move `inject_json` to `helper.rs`** since it's directly related to the JSON injection mechanism
- Update the `fuzz_utils` module to re-export from `helper.rs`
- Delete `filters.rs` to reduce module count

**Effort:** Low (single function move)

---

### 3. ⚠️ **`MAX_TEMPLATE_OUTPUT` Constant: Potentially Dead**

**Location:** `crates/core/src/error.rs:145`

```rust
pub const MAX_TEMPLATE_OUTPUT: usize = 50 * 1024 * 1024;
```

**Analysis:**
- This constant was used to limit MiniJinja template output size
- With MiniJinja removed, no code currently checks this limit
- The `OutputTooLarge` error variant still exists but may not be raised

**Recommendation:**
- **Search for usage** — if truly unused, remove the constant
- **If keeping for future use**, document that it's currently inactive but reserved

**Effort:** Low (verification and removal)

---

### 4. ⚠️ **`OutputTooLarge` Error Variant: Verify Usage**

**Location:** `crates/core/src/error.rs`

**Analysis:**
- `RenderError::OutputTooLarge` exists but may no longer be raised by any code path
- The error is handled in `print_errors()` and Python bindings

**Recommendation:**
- **Verify if any code path raises this error**
- If unused, mark as deprecated or remove (with documentation)

**Effort:** Low

---

### 5. ✅ **`compile.rs` Duplication: Acceptable**

**Location:** `crates/backends/typst/src/compile.rs`

**Current State:** Contains both legacy and new compile functions:
- `compile_to_pdf()` and `compile_to_svg()` — Legacy, no JSON injection
- `compile_to_pdf_with_data()` and `compile_to_svg_with_data()` — New, with JSON injection

**Analysis:**
The legacy functions (`compile_to_pdf`, `compile_to_svg`) are still used by:
- `Backend::compile()` trait method (for backward compatibility)
- Potentially external consumers of the public API

**Recommendation:**
- **Keep both** — the legacy functions provide backward compatibility
- Consider deprecation annotations in a future release if external usage is confirmed low
- Could consolidate by having legacy functions call `_with_data("")` internally, but the current separation is clear

**Effort:** None needed for now

---

### 6. ⚠️ **`allow_auto_plate()` Backend Method: Review Purpose**

**Location:** `crates/core/src/backend.rs`

**Analysis:**
- The `allow_auto_plate()` method exists for backends to indicate if they support automatic plate generation
- With the new architecture, "auto plate" behavior may have changed

**Current Usage:**
- Checked in `orchestration/engine.rs` during quill validation
- Returns `true` for TypstBackend

**Recommendation:**
- **Review if auto-plate is still meaningful** — in the new architecture, plates are now pure Typst files that import the helper
- The concept of "automatically generating a plate" may need updating
- Consider clarifying the semantics in documentation

**Effort:** Medium (design decision)

---

### 7. ⚠️ **`process_plate()` Workflow Method: Deprecated**

**Location:** `crates/quillmark/src/orchestration/workflow.rs:204-219`

**Current State:**

```rust
/// Process a parsed document (compatibility method).
///
/// NOTE: This method is deprecated. In the new architecture without MiniJinja,
/// plates are pure Typst files that receive data via JSON injection.
/// This method now only performs validation and returns serialized JSON data
/// for backwards compatibility with existing tests and examples.
pub fn process_plate(&self, parsed: &ParsedDocument) -> Result<String, RenderError> { ... }
```

**Analysis:**
- The method is marked as deprecated via doc comment
- It now returns JSON instead of composed template output
- Kept for backward compatibility with tests and examples

**Recommendation:**
- Add `#[deprecated]` attribute to make the deprecation official
- Update tests that rely on this method to use `render()` instead
- Plan removal in a future major version

**Effort:** Medium

---

### 8. ✅ **`QuillWorld` Methods: Clean Architecture**

**Location:** `crates/backends/typst/src/world.rs`

**Analysis:**
- `QuillWorld::new()` — Legacy constructor, no JSON injection
- `QuillWorld::new_with_data()` — New constructor with helper package injection

This parallel structure is intentional for backward compatibility and is acceptable.

---

### 9. ⚠️ **Documentation References to MiniJinja**

**Locations to check:**
- Doc comments in `workflow.rs` mention MiniJinja in historical context
- `prose/designs/*.md` may reference the old architecture

**Recommendation:**
- **Audit design documents** for outdated references
- Update `ARCHITECTURE.md` and related docs to reflect the new flow

**Effort:** Medium (documentation sweep)

---

## Priority Matrix

| Finding | Effort | Impact | Priority |
|---------|--------|--------|----------|
| Move `inject_json` to `helper.rs` | Low | Low | Low |
| Remove `MAX_TEMPLATE_OUTPUT` if unused | Low | Low | Low |
| Verify `OutputTooLarge` usage | Low | Low | Low |
| Add `#[deprecated]` to `process_plate()` | Low | Medium | Medium |
| Review `allow_auto_plate()` semantics | Medium | Medium | Medium |
| Update documentation | Medium | High | High |

---

## Recommendations Summary

### Immediate (Low Effort, High Clarity)
1. Verify if `MAX_TEMPLATE_OUTPUT` and `OutputTooLarge` are still needed
2. Consider consolidating `filters.rs` into `helper.rs`
3. Add `#[deprecated]` to `process_plate()`

### Near-Term (Medium Effort)
1. Update architecture documentation to remove MiniJinja references
2. Review `allow_auto_plate()` semantics for the new paradigm
3. Update tests using `process_plate()` to use `render()`

### Future Versions
1. Remove deprecated `process_plate()` method
2. Potentially consolidate legacy `compile_to_*` functions if backward compatibility concerns diminish

---

## Conclusion

The JSON Data Delivery rework was successful in achieving its primary goals:
- ✅ MiniJinja removed from Typst backend
- ✅ Pure Typst plates with helper package import
- ✅ Clean data flow with backend-side markdown transformation

The remaining simplification opportunities are minor and can be addressed incrementally without affecting the core architecture. The code is in a stable, maintainable state.
