# Error Handling Evaluation - Executive Summary

**Full Analysis:** See [ERROR_PROPOSAL.md](./ERROR_PROPOSAL.md)

---

## TL;DR

**Status:** 🟡 Functional but incomplete

**Key Issues:**
1. ❌ **Typst errors** → converted to strings, lose structure
2. ❌ **Backend panics** → `.unwrap()` in production code path
3. ⚠️ **MiniJinja mapping** → misses column info and context
4. ⚠️ **Typst warnings** → completely ignored

**Recommendation:** Implement Phase 1 fixes (8-12 hours) to achieve production-grade error handling.

---

## Current State Report Card

| Component | Grade | Status |
|-----------|-------|--------|
| **Core `Diagnostic` Design** | A | ✅ Excellent structure |
| **MiniJinja Mapping** | B | ⚠️ Good but incomplete |
| **Typst Mapping** | D | ❌ Barely implemented |
| **Backend Integration** | D | ❌ Has `.unwrap()` panic |
| **Error Printing** | C | ⚠️ Basic, inconsistent |
| **Overall Robustness** | C | ⚠️ Works but fragile |

---

## Critical Findings

### 1. Typst Error Handling is Broken

**Problem:**
```rust
// Current: Returns string, loses all structure
fn format_compilation_errors(errors: &[SourceDiagnostic], ...) -> String {
    let mut formatted = format!("Compilation failed...");
    // ... string concatenation
}

// Backend: PANICS on PDF error!
let bytes = compile::compile_to_pdf(quill, glued_content).unwrap(); // ❌
```

**Impact:**
- Users get unhelpful string errors
- Production crashes on compilation errors
- Can't provide IDE integration or tooling

**Solution:**
```rust
// Proposed: Return structured diagnostics
pub fn map_typst_errors(
    errors: &[SourceDiagnostic],
    world: &QuillWorld,
) -> Vec<Diagnostic> {
    errors.iter().map(|e| {
        Diagnostic {
            severity: map_severity(e.severity),
            code: Some(format!("typst::{}", e.message.split(':').next().unwrap_or("error"))),
            message: e.message.clone(),
            primary: resolve_span_to_location(&e.span, world),
            related: e.trace.iter().filter_map(...).collect(),
            hint: e.hints.first().map(|h| h.to_string()),
        }
    }).collect()
}
```

### 2. MiniJinja Errors Lose Information

**Problem:**
```rust
col: 0, // ❌ Comment says "MiniJinja doesn't provide column info" - WRONG!
```

**Reality:** MiniJinja provides:
- ✅ `error.line()` - we use this
- ❌ `error.range()` - we ignore this (contains column!)
- ❌ `error.detail()` - we ignore this
- ❌ `error.kind()` - we use in code, but don't generate hints

**Solution:** Extract actual column from range, generate context-aware hints.

### 3. Design vs Reality Gap

**DESIGN.md says:**
```rust
fn map_typst(errors: &[SourceDiagnostic], world: &QuillWorld) -> Vec<Diagnostic> {
    // ... proper mapping
}
```

**Reality:**
```rust
// Function doesn't exist!
// Errors are stringified instead
```

**Multiple gaps:**
- ❌ `map_typst()` not implemented
- ❌ Source mapping (`@origin:` comments) not implemented
- ⚠️ Warning propagation not implemented

---

## Proposed Fix Priority

### 🔴 **P0 - Critical** (Fix Immediately)

1. **Implement Typst Error Mapping**
   - Create `error_mapping.rs` module
   - Map `SourceDiagnostic` → `Diagnostic`
   - Use `RenderError::CompilationFailed` in backend
   - **Eliminates:** String errors, provides structure
   - **Effort:** 4-6 hours

2. **Remove Backend Panics**
   - Replace `.unwrap()` with proper error propagation
   - Return `Result<Vec<u8>, RenderError>` from compile functions
   - **Eliminates:** Production crashes
   - **Effort:** 2-3 hours

### 🟡 **P1 - High** (Next Sprint)

3. **Improve MiniJinja Mapping**
   - Extract column from `error.range()`
   - Add hint generation based on `error.kind()`
   - **Improves:** Error messages, debugging
   - **Effort:** 2-3 hours

4. **Enhance Error Printing**
   - Add source code context snippets
   - Add color coding
   - Consolidate print functions
   - **Improves:** User experience
   - **Effort:** 3-4 hours

### 🟢 **P2 - Medium** (Future)

5. **Propagate Warnings**
   - Capture Typst warnings
   - Add to `RenderResult.warnings`
   - **Adds:** Non-fatal diagnostics
   - **Effort:** 2-3 hours

6. **Implement Source Mapping**
   - Add `@origin:` comment injection
   - Map Typst errors back to Markdown
   - **Adds:** Advanced debugging
   - **Effort:** 4-6 hours

---

## Code Examples

### Before (Current)
```rust
// Backend panics on error
let bytes = compile::compile_to_pdf(quill, glued_content).unwrap();

// Typst errors become strings
output.map_err(|errors| format_compilation_errors(&errors, world).into())

// MiniJinja missing info
col: 0, // "doesn't provide column info" - INCORRECT!
```

### After (Proposed)
```rust
// Backend handles errors gracefully
let bytes = compile::compile_to_pdf(quill, glued_content)?;

// Typst errors are structured
let diagnostics = map_typst_errors(&errors, world);
Err(RenderError::CompilationFailed(diagnostics.len(), diagnostics))

// MiniJinja captures full info
col: e.range().map(|r| r.start as u32).unwrap_or(0),
hint: generate_minijinja_hint(&e),
```

---

## Testing Requirements

### Must Add Tests For:

1. **Typst Error Mapping**
   - ✅ Syntax errors map to Diagnostic
   - ✅ Trace entries map to related locations
   - ✅ Severity mapping (Error/Warning)
   - ✅ Span resolution to line/col

2. **Backend Error Handling**
   - ✅ PDF compilation errors don't panic
   - ✅ SVG compilation errors preserve structure
   - ✅ CompilationFailed contains diagnostics

3. **MiniJinja Improvements**
   - ✅ Column information captured
   - ✅ Hints generated for common errors
   - ✅ Error detail preserved

### Coverage Target
- Error mapping code: **90%+**
- Backend error paths: **85%+**

---

## Impact Assessment

### User Experience
- **Before:** "Compilation failed with 1 error(s): Error #1: ..."
- **After:** 
  ```
  [ERROR] Unknown variable: foo (typst::undefined)
    --> glue.typ:12:5
   10 | let title = data.title
   11 | 
   12 | let foo = data.foo
      |           ^
   13 |
    hint: Check variable spelling and ensure it's defined in frontmatter
  ```

### Developer Experience
- Can programmatically access error details
- Can build IDE integrations
- Can write better tests

### Production Stability
- **Before:** Crashes on PDF compilation errors
- **After:** Graceful error reporting

---

## Risks & Mitigation

| Risk | Severity | Mitigation |
|------|----------|------------|
| Breaking API changes | Low | Phase approach, deprecation |
| Performance impact | Very Low | Only on error path |
| Regression bugs | Medium | Comprehensive test suite |
| Incomplete migration | Medium | Keep old code paths temporarily |

---

## Success Metrics

After implementing Phase 1:

- ✅ Zero panics in error paths
- ✅ 100% of Typst errors structured
- ✅ MiniJinja column info captured
- ✅ Test coverage >85% on error paths
- ✅ User-facing error messages improved

---

## Next Steps

1. **Review this proposal** with team
2. **Prioritize** P0 fixes for immediate implementation
3. **Create tasks** for each phase
4. **Implement** with test-driven approach
5. **Document** new error handling patterns

---

## Questions for Discussion

1. Should we make Backend.compile() return `RenderResult` instead of `Vec<Artifact>` to propagate warnings?
2. What color scheme for error output? (Currently proposing ANSI colors)
3. Should we add error codes documentation (like rustc error codes)?
4. Timeline for breaking changes (if any needed)?

---

**For full details, code examples, and implementation guide, see [ERROR_PROPOSAL.md](./ERROR_PROPOSAL.md)**
