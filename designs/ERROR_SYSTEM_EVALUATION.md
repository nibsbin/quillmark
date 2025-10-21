# Error System Evaluation

**Evaluation Date**: 2025-10-20  
**Evaluator**: AI Agent  
**Scope**: Error visibility from Markdown parsing through Python consumer

## Executive Summary

The Quillmark error system demonstrates **strong foundational design** with structured diagnostics and consistent error propagation. However, there are **critical gaps in the Python bindings** that prevent full error visibility for Python consumers. The system properly captures syntax, parsing, and conversion errors from Markdown to backends, but the Python layer **loses diagnostic detail** during error conversion.

**Overall Assessment**: ⚠️ **Partially Robust** - Works well at Rust level, needs improvement at Python level.

---

## Architecture Overview

### Error Flow Path

```
Markdown Input
    ↓
[1] ParsedDocument (parsing errors)
    ↓
[2] Template Rendering (template errors)
    ↓
[3] Backend Compilation (backend-specific errors)
    ↓
[4] Python Bindings (error conversion)
    ↓
Python Consumer
```

### Core Components

1. **quillmark-core** - Foundation layer with `Diagnostic` and `RenderError`
2. **quillmark-typst** - Typst backend with error mapping
3. **quillmark-acroform** - PDF form backend with minimal error handling
4. **quillmark-python** - Python bindings with error conversion
5. **quillmark-wasm** - WASM bindings (comparison reference)

---

## Layer-by-Layer Analysis

### 1. Core Error System (quillmark-core)

**Location**: `quillmark-core/src/error.rs`

#### Strengths ✅

1. **Comprehensive Diagnostic Structure**
   - `Diagnostic` type with severity, code, message, location, hints
   - Serializable to JSON via serde
   - Pretty-printing support for human-readable output

2. **Rich RenderError Variants**
   ```rust
   pub enum RenderError {
       InvalidFrontmatter { diag: Diagnostic, source: Option<anyhow::Error> },
       TemplateFailed { source: minijinja::Error, diag: Diagnostic },
       CompilationFailed(usize, Vec<Diagnostic>),
       FormatNotSupported { backend: String, format: OutputFormat },
       // ... 10+ variants total
   }
   ```

3. **Location Tracking**
   - File, line, column information
   - Consistent across all error types
   - Mapped from source errors (MiniJinja, Typst)

4. **Helpful Hints**
   - Context-aware hints generated from MiniJinja errors
   - Examples: "Check variable spelling", "Look for unclosed tags"

5. **Safety Limits**
   - Input size limits (10MB markdown, 1MB YAML)
   - Nesting depth limits (100 levels)
   - Template output limits (50MB)

#### Weaknesses ⚠️

1. **No Warning Propagation from Parsing**
   - `decompose()` returns `Result<ParsedDocument, Box<dyn Error>>`
   - No mechanism to capture non-fatal parsing warnings
   - YAML warnings are lost

2. **Generic Error Boxing**
   - Several `RenderError` variants use `Box<dyn Error + Send + Sync>`
   - Loses structured diagnostic information for some error paths

3. **Limited Context in Some Errors**
   - `DynamicAssetCollision` and `DynamicFontCollision` are plain errors
   - No file/line location for these issues

---

### 2. Parsing Layer (quillmark-core/parse.rs)

**Location**: `quillmark-core/src/parse.rs`

#### Strengths ✅

1. **Comprehensive Validation**
   - Multiple global frontmatter detection
   - Tag name validation (`[a-z_][a-z0-9_]*`)
   - YAML size limits
   - Name collision detection

2. **Error Messages**
   ```rust
   "Invalid YAML frontmatter: {}"
   "Multiple global frontmatter blocks found: only one untagged block allowed"
   "Name collision: global field '{}' conflicts with tagged attribute"
   ```

3. **ParseError Type**
   - Separate from RenderError for clarity
   - Wraps serde_yaml errors
   - Input size validation

#### Weaknesses ⚠️

1. **No Location Information**
   - `decompose()` errors lack line/column numbers
   - YAML errors from `serde_yaml` don't preserve location
   - Makes debugging difficult for users

2. **String-Based Errors**
   - Returns `Box<dyn Error + Send + Sync>` (loses structure)
   - Difficult to programmatically handle specific error cases

3. **No Diagnostic Integration**
   - Doesn't use the `Diagnostic` type
   - Inconsistent with rest of system

---

### 3. Template Layer (quillmark-core/templating.rs)

**Location**: `quillmark-core/src/templating.rs`

#### Strengths ✅

1. **MiniJinja Error Conversion**
   - Automatic conversion from `minijinja::Error` to `RenderError::TemplateFailed`
   - Extracts line/column from MiniJinja
   - Generates contextual hints

2. **Template Error Types**
   ```rust
   pub enum TemplateError {
       RenderError(minijinja::Error),
       InvalidTemplate(String, Box<dyn StdError>),
       FilterError(String),
   }
   ```

3. **Filter API Abstraction**
   - Stable filter API that doesn't expose MiniJinja directly
   - Good for backend implementations

#### Weaknesses ⚠️

1. **TemplateError Not Well Integrated**
   - Separate from main error flow
   - `RenderError::Template(TemplateError)` variant exists but underutilized

2. **Filter Errors Lose Context**
   - Filter errors become strings
   - No location information from filters

---

### 4. Typst Backend (quillmark-typst)

**Location**: `quillmark-typst/src/error_mapping.rs`, `quillmark-typst/src/compile.rs`

#### Strengths ✅

1. **Excellent Error Mapping**
   ```rust
   pub fn map_typst_errors(errors: &[SourceDiagnostic], world: &QuillWorld) -> Vec<Diagnostic>
   ```
   - Maps Typst severity to Quillmark severity
   - Extracts file/line/column from Typst spans
   - Preserves hints from Typst

2. **Span Resolution**
   - Resolves Typst spans to source locations
   - Handles line/column calculation correctly

3. **Comprehensive Error Coverage**
   - All Typst compilation errors are captured
   - Multiple errors collected and reported together

4. **Code Prefixing**
   ```rust
   code: Some(format!("typst::{}", error.message.split(':').next().unwrap_or("error")))
   ```

#### Weaknesses ⚠️

1. **Warning Handling**
   - Typst warnings are compiled but ignored:
   ```rust
   let Warned { output, warnings: _ } = typst::compile::<PagedDocument>(world);
   ```
   - No way to surface warnings to user

2. **World Creation Errors**
   - World creation failures become generic `Internal` errors
   - Lose specific error information

---

### 5. AcroForm Backend (quillmark-acroform)

**Location**: `quillmark-acroform/src/lib.rs`

#### Strengths ✅

1. **Basic Error Handling**
   - Wraps acroform library errors
   - Clear error messages

2. **Validation**
   - Checks for form.pdf existence
   - Validates format support

#### Weaknesses ⚠️

1. **No Diagnostic Integration**
   - All errors become `RenderError::Other(String)`
   - No location information
   - No structured diagnostics

2. **Silent Failures**
   - Template rendering errors are caught but ignored:
   ```rust
   if let Ok(rendered_value) = env.render_str(&source, &context) {
       // Success path
   }
   // Error path: silently skipped, field not filled
   ```

3. **No Error Feedback for Templates**
   - Field template errors are invisible to users
   - Could lead to confusing results

---

### 6. Python Bindings (quillmark-python)

**Location**: `quillmark-python/src/errors.rs`, `quillmark-python/src/types.rs`

#### Strengths ✅

1. **Exception Hierarchy**
   ```python
   QuillmarkError (base)
   ├── ParseError
   ├── TemplateError
   └── CompilationError
   ```

2. **Diagnostic Exposure**
   - `PyDiagnostic` class exposes: severity, message, code, primary location, hint
   - `PyLocation` class exposes: file, line, col
   - Available on `RenderResult.warnings`

3. **Result Warnings**
   - Non-fatal warnings are accessible via `result.warnings`

#### Weaknesses ⚠️

1. **❌ CRITICAL: Diagnostic Detail Loss**
   ```rust
   // Current implementation loses diagnostic details!
   RenderError::InvalidFrontmatter { diag, .. } => 
       ParseError::new_err(diag.message.clone()),  // Only message!
   
   RenderError::TemplateFailed { diag, .. } => 
       TemplateError::new_err(diag.message.clone()),  // Only message!
   
   RenderError::CompilationFailed(count, _diags) => 
       CompilationError::new_err(format!("Compilation failed with {} error(s)", count))
       // Diagnostics completely discarded! ❌
   ```

2. **No Diagnostic Access on Exceptions**
   - Python exceptions only contain message string
   - Location, hints, and traces are lost
   - Cannot programmatically inspect error details

3. **Inconsistent Error Information**
   - `result.warnings` has full diagnostic info
   - Exceptions have minimal info
   - No way to get diagnostics from caught exceptions

4. **Type Conversion Issues**
   - Line/col exposed as `usize` instead of `u32`
   - Minor inconsistency with Rust types

#### Comparison with WASM Bindings

The WASM bindings (quillmark-wasm) handle this **much better**:

```rust
// WASM preserves diagnostics! ✅
RenderError::CompilationFailed(count, diags) => QuillmarkError {
    message: format!("Compilation failed with {} error(s)", count),
    diagnostics: Some(diags.into_iter().map(|d| d.into()).collect()),
    // ...
}
```

The WASM error structure includes an optional `diagnostics` field that preserves the full diagnostic list.

---

### 7. Orchestration Layer (quillmark)

**Location**: `quillmark/src/orchestration.rs`

#### Strengths ✅

1. **Error Propagation**
   - Correctly propagates errors from backends
   - Preserves `RenderResult` with warnings

2. **Workflow Validation**
   - Validates backend registration
   - Checks format support

#### Weaknesses ⚠️

1. **No Additional Error Context**
   - Missing "which step failed" information

---

## Critical Issues Summary

### 🔴 High Priority

1. **Python Diagnostic Loss** (quillmark-python)
   - Compilation errors lose all diagnostic details
   - Template/parse errors lose location, hints, codes
   - **Impact**: Python users cannot get actionable error information

2. **AcroForm Silent Failures** (quillmark-acroform)
   - Template errors in PDF fields are silently ignored
   - **Impact**: Mysterious missing field values

3. **Parse Error Locations** (quillmark-core)
   - YAML parsing errors lack line/column information
   - **Impact**: Hard to debug frontmatter issues

### 🟡 Medium Priority

4. **Warning Suppression** (quillmark-typst)
   - Typst warnings are compiled but discarded
   - **Impact**: Missing helpful non-fatal information

5. **Generic Error Boxing** (quillmark-core)
   - Some errors use generic boxed errors
   - **Impact**: Loss of structure for programmatic handling

### 🟢 Low Priority

7. **Inconsistent Error Types**
   - Parse errors don't use Diagnostic
   - Some backends use ad-hoc error handling

---

## Recommendations

### 1. Fix Python Exception Diagnostic Exposure (Critical)

**Problem**: Python exceptions lose all diagnostic information.

**Solution**: Add diagnostic details to Python exceptions.

#### Option A: Add Custom Exception Attributes (Recommended)

```rust
// In quillmark-python/src/errors.rs
pub fn convert_render_error(err: RenderError) -> PyErr {
    match err {
        RenderError::InvalidFrontmatter { diag, .. } => {
            let py_err = ParseError::new_err(diag.message.clone());
            // Add diagnostic as exception attribute
            Python::with_gil(|py| {
                if let Ok(exc) = py_err.value(py).downcast::<PyAny>() {
                    let py_diag = PyDiagnostic { inner: diag };
                    let _ = exc.setattr("diagnostic", py_diag);
                }
            });
            py_err
        }
        RenderError::CompilationFailed(count, diags) => {
            let py_err = CompilationError::new_err(
                format!("Compilation failed with {} error(s)", count)
            );
            // Add diagnostics list as exception attribute
            Python::with_gil(|py| {
                if let Ok(exc) = py_err.value(py).downcast::<PyAny>() {
                    let py_diags: Vec<PyDiagnostic> = diags
                        .into_iter()
                        .map(|d| PyDiagnostic { inner: d })
                        .collect();
                    let _ = exc.setattr("diagnostics", py_diags);
                }
            });
            py_err
        }
        // ... similar for other cases
    }
}
```

Python usage:
```python
try:
    result = workflow.render(parsed, OutputFormat.PDF)
except CompilationError as e:
    print(f"Error: {e}")
    if hasattr(e, 'diagnostics'):
        for diag in e.diagnostics:
            print(f"  {diag.severity}: {diag.message}")
            if diag.primary:
                print(f"    at {diag.primary.file}:{diag.primary.line}:{diag.primary.col}")
```

#### Option B: Return Result-Style Objects

Create a `RenderResult` that can represent success or failure with diagnostics:

```python
class RenderOutcome:
    success: bool
    artifacts: Optional[List[Artifact]]
    errors: List[Diagnostic]
    warnings: List[Diagnostic]
```

This is more idiomatic for typed Python but breaks existing API.

### 2. Fix AcroForm Template Error Handling

**Current code** (silent failure):
```rust
if let Ok(rendered_value) = env.render_str(&source, &context) {
    // process
}
// Error silently ignored!
```

**Fixed code**:
```rust
let rendered_value = env.render_str(&source, &context)
    .map_err(|e| RenderError::TemplateFailed {
        source: e.clone(),
        diag: Diagnostic::new(
            Severity::Error,
            format!("Failed to render template for field '{}'", field.name),
        )
        .with_hint(format!("Template: {}", source))
        .with_code("acroform::template".to_string()),
    })?;
```

### 3. Add Location to Parse Errors

Enhance `ParseError` to include location:

```rust
// In quillmark-core/src/error.rs
pub enum ParseError {
    YamlError {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    // ...
}
```

Extract location from `serde_yaml::Error`:
```rust
let yaml_err: serde_yaml::Error = ...;
if let Some(loc) = yaml_err.location() {
    ParseError::YamlError {
        message: yaml_err.to_string(),
        line: Some(loc.line()),
        column: Some(loc.column()),
    }
}
```

### 4. Expose Typst Warnings

```rust
// In quillmark-typst/src/compile.rs
fn compile_document(world: &QuillWorld) -> Result<(PagedDocument, Vec<Diagnostic>), RenderError> {
    let Warned { output, warnings } = typst::compile::<PagedDocument>(world);
    
    let warning_diags = map_typst_errors(&warnings, world);
    
    match output {
        Ok(doc) => Ok((doc, warning_diags)),
        Err(errors) => {
            let diagnostics = map_typst_errors(&errors, world);
            Err(RenderError::CompilationFailed(diagnostics.len(), diagnostics))
        }
    }
}
```

Then add warnings to `RenderResult`:
```rust
let (doc, warnings) = compile_document(&world)?;
// ... generate PDF/SVG
let mut result = RenderResult::new(artifacts, format);
for warning in warnings {
    result = result.with_warning(warning);
}
```

### 5. Standardize on Diagnostic Type

Convert all error sources to use `Diagnostic`:

```rust
// In parse.rs
pub fn decompose(markdown: &str) -> Result<ParsedDocument, Diagnostic> {
    // Use Diagnostic for all errors
}
```

This ensures consistency across the entire system.

---

## Testing Recommendations

### 1. Error Flow Tests

Create integration tests that verify error propagation:

```rust
#[test]
fn test_python_compilation_error_has_diagnostics() {
    let bad_typst = "#unknown_function()";
    // Verify Python exception has .diagnostics attribute
}

#[test]
fn test_parse_error_has_location() {
    let bad_yaml = "---\ntitle: [unclosed\n---";
    // Verify error includes line/column
}
```

### 2. Documentation Examples

Add error handling examples to documentation:

```python
# Example: Handling compilation errors
try:
    result = workflow.render(parsed, OutputFormat.PDF)
except CompilationError as e:
    print(f"Compilation failed: {e}")
    if hasattr(e, 'diagnostics'):
        for diag in e.diagnostics:
            print(f"  [{diag.severity}] {diag.message}")
            if diag.primary:
                loc = diag.primary
                print(f"    --> {loc.file}:{loc.line}:{loc.col}")
            if diag.hint:
                print(f"    hint: {diag.hint}")
```

### 3. Error Scenario Tests

Test specific error scenarios:
- Invalid YAML syntax
- Undefined template variables
- Missing Typst functions
- Typst syntax errors
- Missing quill files
- Format not supported
- Backend not registered

---

## Comparison with Industry Standards

### Rust Error Handling (Good Example)

Quillmark follows Rust best practices:
- `thiserror` for error enums ✅
- Structured error types ✅
- Source chain preservation ✅

### Python Error Handling (Needs Improvement)

Industry standard (e.g., requests, sqlalchemy):
- Custom exception classes ✅
- Rich exception attributes ⚠️ (missing)
- Error context preservation ❌ (lost)

### TypeScript/WASM (Comparison)

The WASM bindings are better than Python:
- Preserves diagnostics ✅
- Structured error objects ✅
- Serializable to JSON ✅

**Recommendation**: Use WASM error handling as a model for Python.

---

## Summary Assessment

### What Works Well

1. ✅ **Core Diagnostic System** - Excellent structure with location, hints, codes
2. ✅ **Typst Error Mapping** - Comprehensive and accurate
3. ✅ **RenderError Variants** - Good coverage of error cases
4. ✅ **Pretty Printing** - Human-readable error output
5. ✅ **Warning System** - Foundation exists (though underutilized)
6. ✅ **WASM Bindings** - Good reference implementation

### What Needs Improvement

1. ❌ **Python Diagnostic Exposure** - Critical gap in error visibility
2. ⚠️ **Parse Error Locations** - Missing line/column information
3. ⚠️ **AcroForm Error Handling** - Silent failures on template errors
4. ⚠️ **Warning Propagation** - Typst warnings lost
5. ⚠️ **Consistency** - Parse errors don't use Diagnostic type

### Does It Meet the Goal?

**"Bubble up any syntax, parsing, or conversion errors from the markdown to the Python consumer"**

- **Syntax Errors**: ⚠️ Partial - Errors are detected but diagnostic details are lost at Python boundary
- **Parsing Errors**: ⚠️ Partial - Errors are detected but lack location information
- **Conversion Errors**: ⚠️ Partial - Typst errors are well-mapped but diagnostics are lost in Python

**Overall**: The system is **60-70% robust**. It properly detects and structures errors at the Rust level, but the Python bindings create a "diagnostic black hole" that prevents full error visibility.

---

## Priority Action Items

1. **Fix Python Exception Diagnostic Exposure** (1-2 days)
   - Add diagnostic attributes to Python exceptions
   - Update error conversion code
   - Add tests

2. **Fix AcroForm Silent Failures** (2-4 hours)
   - Propagate template rendering errors
   - Add proper error handling

3. **Add Parse Error Locations** (4-6 hours)
   - Extract location from serde_yaml errors
   - Update ParseError type
   - Add tests

4. **Expose Typst Warnings** (2-4 hours)
   - Capture warnings from Typst
   - Add to RenderResult
   - Test warning propagation

5. **Add Comprehensive Error Tests** (1-2 days)
   - Test each error scenario
   - Verify Python visibility
   - Document examples

**Total Estimated Effort**: 3-4 days

---

## Conclusion

The Quillmark error system has a **strong foundation** with excellent diagnostic structures and comprehensive error coverage at the Rust level. However, the **Python bindings create a critical gap** that prevents Python consumers from accessing the rich diagnostic information that the system generates.

The good news is that the fixes are straightforward:
1. The diagnostic information exists - it just needs to be exposed to Python
2. The WASM bindings provide a working model to follow
3. Most issues can be fixed with localized changes

**Recommendation**: Prioritize fixing the Python diagnostic exposure as it directly impacts the ability for Python consumers to debug their documents. The other improvements (parse locations, warnings) are valuable but less critical.

With these improvements, the error system will be **fully robust** and meet the goal of bubbling up all syntax, parsing, and conversion errors from Markdown to Python consumers with actionable diagnostic information.
