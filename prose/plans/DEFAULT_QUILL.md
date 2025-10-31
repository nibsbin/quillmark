# Default Quill System Implementation Plan

> **Status**: Implementation Plan
>
> This plan outlines the steps to implement the default Quill system as designed in [../designs/DEFAULT_QUILL.md](../designs/DEFAULT_QUILL.md).

---

## Overview

This plan implements a default Quill system that allows backends to provide fallback Quill templates for documents that don't specify a `QUILL:` tag in their frontmatter.

---

## Implementation Steps

### Step 1: Extend Backend Trait

**File:** `quillmark-core/src/backend.rs`

**Changes:**
- Add `default_quill()` method to `Backend` trait with default implementation returning `None`
- Update trait documentation to explain default Quill behavior
- Update module documentation with examples

**Code:**
```rust
pub trait Backend: Send + Sync {
    // ... existing methods ...
    
    /// Provide an embedded default Quill for this backend.
    /// 
    /// Returns `None` if the backend does not provide a default Quill.
    /// The returned Quill will be registered with the name `__default__`
    /// during backend registration if no default Quill already exists.
    ///
    /// # Example
    ///
    /// ```rust
    /// fn default_quill(&self) -> Option<Quill> {
    ///     // Load embedded default Quill
    ///     Some(create_embedded_default_quill())
    /// }
    /// ```
    fn default_quill(&self) -> Option<Quill> {
        None
    }
}
```

**Testing:** No immediate tests needed (default implementation)

---

### Step 2: Update Backend Registration Logic

**File:** `quillmark/src/orchestration.rs`

**Changes:**
- Modify `Quillmark::register_backend()` to check for and register default Quills
- Only register default Quill if `__default__` doesn't already exist
- Log warning if default Quill registration fails (don't fail backend registration)

**Pseudo-code:**
```rust
pub fn register_backend(&mut self, backend: Box<dyn Backend>) {
    let id = backend.id().to_string();
    self.backends.insert(id, Arc::from(backend.clone()));
    
    // Register default Quill if available and not already registered
    if !self.quills.contains_key("__default__") {
        if let Some(default_quill) = backend.default_quill() {
            if let Err(e) = self.register_quill(default_quill) {
                eprintln!("Warning: Failed to register default Quill: {}", e);
            }
        }
    }
}
```

**Testing:**
- Test that default Quill is registered when backend provides one
- Test that default Quill is NOT registered if one already exists
- Test that backend registration succeeds even if default Quill registration fails

---

### Step 3: Enhance Workflow Loading

**File:** `quillmark/src/orchestration.rs`

**Changes:**
- Update `Quillmark::workflow_from_parsed()` to use `__default__` as fallback
- Update error message when neither Quill tag nor default is available

**Pseudo-code:**
```rust
pub fn workflow_from_parsed(&self, parsed: &ParsedDocument) -> Result<Workflow, RenderError> {
    let quill_name = parsed.quill_tag().unwrap_or("__default__");
    
    // Try to load the Quill
    self.workflow_from_quill_name(quill_name).map_err(|e| {
        // If we fell back to __default__ and it doesn't exist, provide better error
        if quill_name == "__default__" && parsed.quill_tag().is_none() {
            RenderError::UnsupportedBackend {
                diag: Diagnostic::new(
                    Severity::Error,
                    "No QUILL field found in parsed document and no default Quill is registered.".to_string(),
                )
                .with_code("engine::missing_quill_tag".to_string())
                .with_hint("Add 'QUILL: <name>' to specify which Quill template to use, or ensure a backend with a default Quill is registered.".to_string()),
            }
        } else {
            e
        }
    })
}
```

**Testing:**
- Test that `__default__` is used when no Quill tag is present
- Test that explicit Quill tag takes precedence over default
- Test error message when neither Quill tag nor default is available

---

### Step 4: Implement Typst Default Quill

**File:** `backends/quillmark-typst/src/lib.rs`

**Changes:**
- Create helper module `default_quill` to embed the default Quill
- Implement `default_quill()` method for `TypstBackend`
- Embed files from `default_quill/` directory at compile time

**Approach:**
Use `include_str!` and `include_bytes!` to embed files:

```rust
mod embedded {
    pub const QUILL_TOML: &str = include_str!("../default_quill/Quill.toml");
    pub const GLUE_TYP: &str = include_str!("../default_quill/glue.typ");
    pub const EXAMPLE_MD: &str = include_str!("../default_quill/example.md");
}

impl Backend for TypstBackend {
    fn default_quill(&self) -> Option<Quill> {
        // Build Quill from embedded files
        let mut files = HashMap::new();
        files.insert("Quill.toml".to_string(), FileTreeNode::File { contents: embedded::QUILL_TOML.as_bytes().to_vec() });
        files.insert("glue.typ".to_string(), FileTreeNode::File { contents: embedded::GLUE_TYP.as_bytes().to_vec() });
        files.insert("example.md".to_string(), FileTreeNode::File { contents: embedded::EXAMPLE_MD.as_bytes().to_vec() });
        
        let root = FileTreeNode::Directory { files };
        Quill::from_tree(root, "__default__").ok()
    }
}
```

**Testing:**
- Test that `TypstBackend::default_quill()` returns a valid Quill
- Test that the default Quill can render a simple document
- Test that the default Quill name is `__default__`

---

### Step 5: Add Name Validation

**File:** `quillmark/src/orchestration.rs`

**Changes:**
- Add validation to `Quillmark::register_quill()` to warn about `__default__` name usage
- Allow registration if called internally (from `register_backend`)

**Implementation Note:**
This is a nice-to-have but not critical. Can be implemented as:
- Emit warning if user registers Quill named `__default__` via public API
- Track whether we're in "internal registration mode"

**Decision:** Skip this for initial implementation (low priority)

---

### Step 6: Retrofit Tests

**Files:** Various test files in `quillmark/tests/`

**Strategy:**
1. Identify tests that use minimal/generic Quills
2. Remove explicit Quill registration where default Quill suffices
3. Remove `QUILL:` tags from markdown in simple tests
4. Keep explicit Quills for feature-specific tests

**Example Test Files to Review:**
- `auto_glue_test.rs` - May not need explicit Quill
- `default_values_test.rs` - May benefit from default Quill
- `dynamic_assets_test.rs` - May need explicit Quill for assets
- `dynamic_fonts_test.rs` - May need explicit Quill for fonts

**Testing:**
- Ensure all existing tests still pass
- Add new test specifically for default Quill behavior

---

## Testing Strategy

### Unit Tests

1. **Backend Trait Default Implementation**
   - Test that backends without `default_quill()` override work correctly

2. **Backend Registration**
   - Test default Quill registration on backend registration
   - Test that existing `__default__` prevents new registration
   - Test that failed default Quill registration doesn't break backend registration

3. **Workflow Loading**
   - Test fallback to `__default__` when no Quill tag
   - Test explicit Quill tag precedence
   - Test error when neither available

4. **Typst Default Quill**
   - Test that embedded default Quill is valid
   - Test rendering with default Quill
   - Test that default Quill supports basic markdown

### Integration Tests

1. **End-to-End Default Quill Usage**
   - Create engine, register Typst backend, render without Quill tag
   - Verify output is valid PDF/SVG

2. **Multiple Backend Scenario**
   - Register multiple backends with default Quills
   - Ensure first one wins for `__default__` name

3. **Backward Compatibility**
   - Ensure existing code with explicit Quill tags still works
   - Ensure manual Quill registration still works

---

## File Changes Summary

### New Files
- None (all changes to existing files)

### Modified Files
1. `quillmark-core/src/backend.rs` - Add `default_quill()` method
2. `quillmark/src/orchestration.rs` - Update registration and workflow loading
3. `backends/quillmark-typst/src/lib.rs` - Implement default Quill embedding
4. Test files - Retrofit to use default Quill where appropriate

---

## Migration Path

### For Backend Implementers

Backends can optionally implement `default_quill()`:

```rust
impl Backend for MyBackend {
    fn default_quill(&self) -> Option<Quill> {
        // Return embedded default Quill if desired
        Some(my_embedded_quill())
    }
}
```

### For Library Users

**Before:**
```markdown
---
QUILL: my-quill
title: Hello
---
Content
```

**After (optional):**
```markdown
---
title: Hello
---
Content
```
(If default Quill is sufficient)

**Backward Compatibility:** All existing code continues to work unchanged.

---

## Implementation Order

1. Step 1: Extend Backend trait
2. Step 4: Implement Typst default Quill (to have something to test with)
3. Step 2: Update backend registration
4. Step 3: Enhance workflow loading
5. Step 6: Retrofit tests (verify everything works)

---

## Success Criteria

- [ ] Backend trait has `default_quill()` method
- [ ] Typst backend provides embedded default Quill
- [ ] Engine registers default Quill during backend registration
- [ ] Engine uses `__default__` when no Quill tag is present
- [ ] Clear error message when neither Quill tag nor default available
- [ ] All existing tests pass
- [ ] New tests cover default Quill behavior
- [ ] At least some existing tests retrofitted to use default Quill
- [ ] Documentation updated (inline docs, examples)

---

## Non-Goals

- Default Quills for backends other than Typst (can be added later)
- Configuration of default Quill name (hardcoded to `__default__`)
- Multiple default Quills per backend
- Default Quill discovery from filesystem

---

## Cross-References

- **Design:** [../designs/DEFAULT_QUILL.md](../designs/DEFAULT_QUILL.md)
- **Backend Trait:** [ARCHITECTURE.md](../designs/ARCHITECTURE.md#backend-architecture)
- **Quill Structure:** [QUILL.md](../designs/QUILL.md)
- **Error Handling:** [ERROR.md](../designs/ERROR.md)
