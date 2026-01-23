# Versioning Implementation: Downstream Impact Analysis

**Date:** 2026-01-23
**Status:** Analysis Complete
**Related Documents:**
- [VERSIONING.md](../designs/VERSIONING.md) - Design specification
- [VERSIONING_COMPLETION_SUMMARY.md](completed/VERSIONING_COMPLETION_SUMMARY.md) - Implementation summary

---

## Executive Summary

The versioning implementation introduces **backward-compatible changes** for most consumers but requires **updates to language bindings** (Python and WASM) to handle new error variants. Core functionality remains compatible, but bindings need error handler updates to avoid runtime panics.

### Impact Severity
- **Rust API**: ✅ Fully compatible (minor breaking changes, easy migration)
- **Python Bindings**: ⚠️ **Requires updates** - Missing error handlers will panic
- **WASM Bindings**: ⚠️ **Requires updates** - Missing error handlers will panic
- **End Users**: ✅ Fully compatible - Documents work with or without version syntax

---

## Core Changes Affecting Consumers

### 1. New Error Variants (Breaking for Bindings)

Three new `RenderError` variants were added:

```rust
pub enum RenderError {
    // Existing variants...

    /// Version selector matched no versions
    VersionNotFound { diag: Box<Diagnostic> },

    /// Quill name not registered in engine
    QuillNotFound { diag: Box<Diagnostic> },

    /// Invalid version format in QUILL tag or API call
    InvalidVersion { diag: Box<Diagnostic> },
}
```

**Impact**: Language bindings with exhaustive `match` statements on `RenderError` will have compilation errors or runtime panics if these variants aren't handled.

### 2. ParsedDocument API Changes (Breaking)

**Before:**
```rust
pub fn with_quill_tag(
    fields: HashMap<String, QuillValue>,
    quill_tag: String
) -> Self
```

**After:**
```rust
pub fn with_quill_tag(
    fields: HashMap<String, QuillValue>,
    quill_tag: String
) -> Result<Self, ParseError>
```

**Reason**: Version syntax validation now happens at parse time. Invalid version syntax (e.g., `template@2.x`) produces clear error instead of silent fallback.

**Impact**: Any code calling `with_quill_tag()` must handle `Result`.

### 3. normalize_document() Signature (Breaking)

**Before:**
```rust
pub fn normalize_document(doc: ParsedDocument) -> ParsedDocument
```

**After:**
```rust
pub fn normalize_document(doc: ParsedDocument) -> Result<ParsedDocument, ParseError>
```

**Impact**: Call sites must propagate or handle error.

### 4. Version Syntax Support (Additive, Compatible)

Documents and API calls now support version selectors:

```yaml
---
QUILL: "template@2.1"      # Exact version
QUILL: "template@2"        # Latest 2.x
QUILL: "template@latest"   # Explicit latest
QUILL: "template"          # Implicit latest (unchanged)
---
```

**API:**
```rust
// All of these now work:
engine.workflow("template")?;          // Latest (unchanged)
engine.workflow("template@2.1")?;      // Exact version (new)
engine.workflow("template@2")?;        // Major version (new)
engine.workflow("template@latest")?;   // Explicit latest (new)
```

**Impact**: ✅ **Fully backward compatible** - Existing unversioned calls continue working.

### 5. Quill.toml Version Requirement (Breaking)

**Before:** `version` field was optional/ignored

**After:** `version` field is **required** (two-segment format: `MAJOR.MINOR`)

```toml
[Quill]
name = "my_template"
version = "1.0"        # Required
backend = "typst"
```

**Impact**: All Quill templates must declare a version. Registration fails with clear error if missing.

---

## Python Bindings Impact

### Current State

**File:** `crates/bindings/python/src/errors.rs`

The `convert_render_error()` function handles `RenderError → PyErr` conversion but is **missing handlers for the three new variants**:
- `VersionNotFound`
- `QuillNotFound`
- `InvalidVersion`

### Problem

Python code will trigger a **panic** when these errors occur:

```python
engine = Quillmark()
# ... register quill with version 1.0 ...

try:
    workflow = engine.workflow("template@2.0")  # Version doesn't exist
except Exception as e:
    # This will NEVER execute - Rust panics instead
    print(f"Error: {e}")
```

**Result:** Thread panic, Python interpreter crash.

### Required Changes

Add handlers in `convert_render_error()`:

```rust
// Add to match statement in errors.rs:

RenderError::VersionNotFound { diag } => {
    let py_err = QuillmarkError::new_err(format!(
        "Version not found: {}",
        diag.message
    ));
    if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
        let py_diag = crate::types::PyDiagnostic {
            inner: (*diag).into(),
        };
        let _ = exc.setattr("diagnostic", py_diag);
    }
    py_err
}

RenderError::QuillNotFound { diag } => {
    let py_err = QuillmarkError::new_err(format!(
        "Quill not found: {}",
        diag.message
    ));
    if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
        let py_diag = crate::types::PyDiagnostic {
            inner: (*diag).into(),
        };
        let _ = exc.setattr("diagnostic", py_diag);
    }
    py_err
}

RenderError::InvalidVersion { diag } => {
    let py_err = QuillmarkError::new_err(format!(
        "Invalid version: {}",
        diag.message
    ));
    if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
        let py_diag = crate::types::PyDiagnostic {
            inner: (*diag).into(),
        };
        let _ = exc.setattr("diagnostic", py_diag);
    }
    py_err
}
```

### Python User Experience After Fix

```python
from quillmark import Quillmark, QuillmarkError

engine = Quillmark()
# ... register template version 1.0 ...

# Version syntax works seamlessly
try:
    workflow = engine.workflow("template@2.0")  # Not registered
except QuillmarkError as e:
    print(f"Error: {e}")  # "Quill 'template' version 2.0 not found"
    print(f"Available: {e.diagnostic.hint}")  # Helpful suggestions

# Backward compatibility maintained
workflow = engine.workflow("template")  # Uses latest (1.0)
```

### Validation Test

Add test to ensure error handling works:

```python
def test_version_not_found_error():
    """Ensure VersionNotFound produces QuillmarkError, not panic"""
    engine = Quillmark()
    # Register template v1.0 only

    with pytest.raises(QuillmarkError) as exc_info:
        engine.workflow("template@2.0")

    assert "Version not found" in str(exc_info.value)
    assert exc_info.value.diagnostic is not None
```

---

## WASM Bindings Impact

### Current State

**File:** `crates/bindings/wasm/src/engine.rs`

The WASM bindings already handle the `ParsedDocument::with_quill_tag()` signature change correctly (line 290-291):

```rust
let parsed = quillmark_core::ParsedDocument::with_quill_tag(fields, quill_tag)
    .map_err(|e| JsValue::from_str(&format!("Failed to parse QUILL tag: {}", e)))?;
```

✅ **This is correct and ready.**

### Problem

The `WasmError` conversion likely uses a similar pattern to Python and may panic on new error variants. Need to verify `WasmError::from(RenderError)` implementation.

**File to check:** `crates/bindings/wasm/src/error.rs`

### Expected Changes

If `WasmError` has exhaustive matching on `RenderError`, add handlers:

```rust
impl From<RenderError> for WasmError {
    fn from(err: RenderError) -> Self {
        match err {
            // Existing handlers...

            RenderError::VersionNotFound { diag } => {
                WasmError::new(&format!("Version not found: {}", diag.message))
                    .with_diagnostic(diag)
            }

            RenderError::QuillNotFound { diag } => {
                WasmError::new(&format!("Quill not found: {}", diag.message))
                    .with_diagnostic(diag)
            }

            RenderError::InvalidVersion { diag } => {
                WasmError::new(&format!("Invalid version: {}", diag.message))
                    .with_diagnostic(diag)
            }
        }
    }
}
```

### JavaScript User Experience After Fix

```javascript
import { Quillmark } from '@quillmark/wasm';

const engine = new Quillmark();
// ... register template version 1.0 ...

try {
  // Version syntax works seamlessly
  const workflow = engine.workflow("template@2.0");  // Not registered
} catch (e) {
  console.error(e.message);  // "Version not found: ..."
  console.error(e.diagnostic.hint);  // Helpful suggestions
}

// Backward compatibility maintained
const workflow = engine.workflow("template");  // Uses latest (1.0)
```

---

## Migration Guide for Binding Maintainers

### Step 1: Update Error Handlers

**Priority:** 🔴 **CRITICAL** - Prevents runtime panics

**Files:**
- `crates/bindings/python/src/errors.rs`
- `crates/bindings/wasm/src/error.rs`

Add match arms for:
- `RenderError::VersionNotFound`
- `RenderError::QuillNotFound`
- `RenderError::InvalidVersion`

### Step 2: Verify ParsedDocument Usage

**Priority:** 🟡 Medium

Check all call sites of:
- `ParsedDocument::with_quill_tag()` - Now returns `Result`
- `normalize_document()` - Now returns `Result`

**Search:**
```bash
grep -r "with_quill_tag" crates/bindings/
grep -r "normalize_document" crates/bindings/
```

Add error handling:
```rust
// Before
let doc = ParsedDocument::with_quill_tag(fields, tag);

// After
let doc = ParsedDocument::with_quill_tag(fields, tag)
    .map_err(|e| /* convert to binding error */)?;
```

### Step 3: Add Tests

**Priority:** 🟡 Medium

Add integration tests covering:
1. **Version syntax parsing** - Valid and invalid formats
2. **Version resolution** - Exact, major, latest selectors
3. **Error handling** - All three new error variants
4. **Backward compatibility** - Unversioned documents still work

### Step 4: Update Documentation

**Priority:** 🟢 Low

Add version syntax examples to binding docs:

**Python:**
```python
# Pin to exact version
workflow = engine.workflow("template@2.1")

# Latest 2.x version
workflow = engine.workflow("template@2")

# Latest overall
workflow = engine.workflow("template@latest")
```

**JavaScript:**
```javascript
// Pin to exact version
const workflow = engine.workflow("template@2.1");

// Latest 2.x version
const workflow = engine.workflow("template@2");

// Latest overall
const workflow = engine.workflow("template@latest");
```

---

## End User Impact

### Document Authors

✅ **Fully backward compatible** - No changes required

**Before (still works):**
```yaml
---
QUILL: resume_template
name: John Doe
---
```

**After (optional enhancement):**
```yaml
---
QUILL: resume_template@2.1   # Pin to specific version
name: John Doe
---
```

### Template Authors

⚠️ **Version field now required**

**Migration:**
1. Add `version = "1.0"` to `Quill.toml`
2. Increment as you make changes:
   - **MAJOR**: Breaking changes (layout, removed fields)
   - **MINOR**: Compatible changes (bug fixes, new features)

**Example:**
```toml
[Quill]
name = "resume_template"
version = "2.1"              # Add this
backend = "typst"
description = "Professional resume"
```

### API Users (Rust)

✅ **Mostly compatible** with easy migration path

**Breaking changes:**
1. Handle new error variants (compilation error if exhaustive match)
2. Handle `Result` from `with_quill_tag()` and `normalize_document()`

**New features:**
- Version syntax in `engine.workflow()` calls
- Multiple versions of same template coexist

---

## Testing Recommendations

### For Binding Maintainers

```rust
#[test]
fn test_version_not_found_error() {
    // Ensure error converts properly, doesn't panic
}

#[test]
fn test_quill_not_found_error() {
    // Ensure error converts properly, doesn't panic
}

#[test]
fn test_invalid_version_error() {
    // Ensure error converts properly, doesn't panic
}

#[test]
fn test_version_syntax_parsing() {
    // Valid: @2.1, @2, @latest
    // Invalid: @2.x, @2.1.0, @v2
}

#[test]
fn test_backward_compatibility() {
    // Unversioned QUILL tags still work
}
```

### For Integration Testing

**Scenarios to validate:**
1. Register multiple versions (1.0, 1.1, 2.0)
2. Resolve `@2` → gets 2.0 (latest 2.x)
3. Resolve `@1` → gets 1.1 (latest 1.x)
4. Request non-existent version → clear error
5. Request non-existent quill → clear error
6. Invalid syntax → parse error with hint
7. Unversioned documents → use latest

---

## Timeline and Priority

### Immediate (Before Next Release)

🔴 **CRITICAL**: Update error handlers in Python and WASM bindings
- Without this, production code will panic on version-related errors
- Estimated effort: 1-2 hours

### Short-term (Next Sprint)

🟡 **IMPORTANT**: Add integration tests for bindings
- Validates error handling works correctly
- Prevents regressions
- Estimated effort: 2-3 hours

### Medium-term (Next Release Cycle)

🟢 **NICE-TO-HAVE**: Update binding documentation
- Examples showing version syntax
- Migration guide for template authors
- Estimated effort: 1-2 hours

---

## Risk Assessment

### High Risk

⚠️ **Unhandled error variants in production bindings**
- **Likelihood:** High (if bindings aren't updated)
- **Impact:** Severe (runtime panics, crashes)
- **Mitigation:** Add error handlers immediately, add tests

### Medium Risk

⚠️ **Template authors don't add version fields**
- **Likelihood:** Medium (requires migration)
- **Impact:** Moderate (registration fails with clear error)
- **Mitigation:** Clear error messages, migration guide

### Low Risk

✅ **Breaking API changes affect Rust users**
- **Likelihood:** Low (pre-1.0 allows breaking changes)
- **Impact:** Minor (compilation errors, easy to fix)
- **Mitigation:** Clear migration guide, semantic versioning

---

## Success Criteria

Versioning is ready for downstream consumers when:

1. ✅ Python bindings handle all three new error variants
2. ✅ WASM bindings handle all three new error variants
3. ✅ Integration tests cover version resolution scenarios
4. ✅ Documentation includes version syntax examples
5. ✅ Migration guide for template authors exists
6. ✅ Backward compatibility validated with tests

---

## Appendix: Quick Reference

### New Error Variants

| Error | When | Example |
|-------|------|---------|
| `QuillNotFound` | Template name not registered | `engine.workflow("nonexistent")` |
| `VersionNotFound` | Version selector matches nothing | `engine.workflow("template@2.0")` when only 1.0 exists |
| `InvalidVersion` | Malformed version syntax | `QUILL: template@2.x` (invalid format) |

### Version Syntax

| Syntax | Meaning | Example Resolution |
|--------|---------|-------------------|
| `template` | Latest overall | `3.0` if available |
| `template@latest` | Latest overall (explicit) | `3.0` if available |
| `template@2` | Latest 2.x | `2.2` if 2.0, 2.1, 2.2 exist |
| `template@2.1` | Exactly 2.1 | `2.1` or error |

### Version Field Format

```toml
version = "MAJOR.MINOR"   # Two segments required
version = "2.1"           # ✅ Valid
version = "2.1.0"         # ❌ Invalid (three segments)
version = "v2.1"          # ❌ Invalid (no 'v' prefix)
version = "2"             # ❌ Invalid (must have minor)
```
