# Diagnostic-Based Error System Recommendation

**Status**: Phase 1 ✓ Complete | Phase 2 ✓ Complete | Phase 3 Pending  
**Created**: 2025-10-21  
**Related**: [ERROR_SYSTEM_EVALUATION.md](ERROR_SYSTEM_EVALUATION.md), [ERROR.md](ERROR.md)

---

## Executive Summary

This document proposes a **unified, Diagnostic-centric error system** for Quillmark that addresses the critical gaps identified in the error system evaluation while maintaining simplicity and elegance. The design embraces Diagnostic as the foundation for all error reporting while preserving error context through explicit source chains.

**Key Recommendations:**
1. **Unified Error Type**: Keep `RenderError` as the mono-error enum with Diagnostic payloads
2. **Source Field Addition**: Add optional `source` field to Diagnostic for error chaining
3. **Trait-Based Approach**: Do NOT use Diagnostic as a trait - keep it as a concrete struct
4. **Error Conversion**: Standardize conversion from external errors to Diagnostic
5. **Backwards Compatibility**: Maintain existing error variants with enhanced Diagnostic support

---

## Problem Statement

The ERROR_SYSTEM_EVALUATION.md identifies several issues with the current error system:

1. **Python Diagnostic Loss**: Compilation errors lose all diagnostic details at the Python boundary
2. **Inconsistent Error Types**: Some errors use Diagnostic, others use generic boxed errors
3. **Source Information Loss**: Error chains lose context when converted to strings
4. **Parse Error Locations**: YAML parsing errors lack line/column information
5. **Limited Error Context**: Some errors lack file/line location information

### Central Questions

1. **Can Diagnostic cover every error use case?**
2. **Should we add a `source` field within Diagnostic?**
3. **Should we allow custom errors that implement Diagnostic as a trait or enum?**

---

## Design Principles

### 1. Diagnostic as Foundation, Not Abstraction

**Decision**: Keep `Diagnostic` as a **concrete struct**, not a trait.

**Rationale:**
- Simple to use and understand
- Serializable out of the box (JSON, WASM)
- No trait object complexity or dynamic dispatch overhead
- Easy to pass across FFI boundaries (Python, WASM)
- Consistent structure across all error types

### 2. RenderError as Mono-Error Enum

**Decision**: Maintain `RenderError` enum but ensure **every variant carries Diagnostic**.

**Rationale:**
- Type-safe error handling with pattern matching
- Callers can handle specific error scenarios
- Backwards compatible with existing error handling code
- Allows for variant-specific additional data
- Works well with `thiserror` for Display and source chains

### 3. Source Chain Preservation

**Decision**: Add optional **`source` field** to Diagnostic, type `Option<Box<dyn Error>>`.

**Rationale:**
- Preserves full error context and causality
- Compatible with Rust's Error::source() pattern
- Can be omitted for leaf errors (no underlying cause)
- Enables error chain traversal for debugging
- Doesn't break serialization (source field can be skipped)

---

## Proposed Architecture

### Enhanced Diagnostic Structure

```rust
/// Structured diagnostic information with source chain support
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Error severity level
    pub severity: Severity,
    
    /// Optional error code (e.g., "E001", "typst::syntax")
    pub code: Option<String>,
    
    /// Human-readable error message
    pub message: String,
    
    /// Primary source location
    pub primary: Option<Location>,
    
    /// Related source locations for context
    pub related: Vec<Location>,
    
    /// Optional hint for fixing the error
    pub hint: Option<String>,
    
    /// Source error that caused this diagnostic (for error chaining)
    /// Note: This field is excluded from serialization as Error trait
    /// objects cannot be serialized
    #[serde(skip)]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(severity: Severity, message: String) -> Self { /* ... */ }
    
    /// Set error source (chainable)
    pub fn with_source(mut self, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        self.source = Some(source);
        self
    }
    
    /// Get the source chain as a list of error messages
    pub fn source_chain(&self) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current_source = self.source.as_ref();
        while let Some(err) = current_source {
            chain.push(err.to_string());
            current_source = err.source();
        }
        chain
    }
    
    /// Format diagnostic with source chain for debugging
    pub fn fmt_pretty_with_source(&self) -> String {
        let mut result = self.fmt_pretty();
        
        for (i, cause) in self.source_chain().iter().enumerate() {
            result.push_str(&format!("\n  cause {}: {}", i + 1, cause));
        }
        
        result
    }
}
```

### Serializable Diagnostic for Cross-Language Boundaries

For Python and WASM, we need a serializable version without the `source` field:

```rust
/// Serializable diagnostic for cross-language boundaries
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableDiagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub primary: Option<Location>,
    pub related: Vec<Location>,
    pub hint: Option<String>,
    /// Source chain as list of strings (for display purposes)
    pub source_chain: Vec<String>,
}

impl From<Diagnostic> for SerializableDiagnostic {
    fn from(diag: Diagnostic) -> Self {
        Self {
            severity: diag.severity,
            code: diag.code,
            message: diag.message,
            primary: diag.primary,
            related: diag.related,
            hint: diag.hint,
            source_chain: diag.source_chain(),
        }
    }
}
```

### Updated RenderError Variants

Every variant should carry a Diagnostic:

```rust
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    /// Failed to create rendering engine
    #[error("{diag}")]
    EngineCreation { 
        diag: Diagnostic,
    },

    /// Invalid YAML frontmatter in markdown document
    #[error("{diag}")]
    InvalidFrontmatter { 
        diag: Diagnostic,
    },

    /// Template rendering failed
    #[error("{diag}")]
    TemplateFailed { 
        diag: Diagnostic,
    },

    /// Backend compilation failed with one or more errors
    #[error("Backend compilation failed with {} error(s)", diags.len())]
    CompilationFailed { 
        diags: Vec<Diagnostic>,
    },

    /// Requested output format not supported by backend
    #[error("{diag}")]
    FormatNotSupported { 
        diag: Diagnostic,
    },

    /// Backend not registered with engine
    #[error("{diag}")]
    UnsupportedBackend { 
        diag: Diagnostic,
    },

    /// Dynamic asset filename collision
    #[error("{diag}")]
    DynamicAssetCollision { 
        diag: Diagnostic,
    },

    /// Dynamic font filename collision
    #[error("{diag}")]
    DynamicFontCollision { 
        diag: Diagnostic,
    },

    /// Input size limits exceeded
    #[error("{diag}")]
    InputTooLarge { 
        diag: Diagnostic,
    },

    /// YAML size exceeded maximum allowed
    #[error("{diag}")]
    YamlTooLarge { 
        diag: Diagnostic,
    },

    /// Nesting depth exceeded maximum allowed
    #[error("{diag}")]
    NestingTooDeep { 
        diag: Diagnostic,
    },

    /// Template output exceeded maximum size
    #[error("{diag}")]
    OutputTooLarge { 
        diag: Diagnostic,
    },
}

impl RenderError {
    /// Extract all diagnostics from this error
    pub fn diagnostics(&self) -> Vec<&Diagnostic> {
        match self {
            RenderError::CompilationFailed { diags } => {
                diags.iter().collect()
            }
            RenderError::EngineCreation { diag }
            | RenderError::InvalidFrontmatter { diag }
            | RenderError::TemplateFailed { diag }
            | RenderError::FormatNotSupported { diag }
            | RenderError::UnsupportedBackend { diag }
            | RenderError::DynamicAssetCollision { diag }
            | RenderError::DynamicFontCollision { diag }
            | RenderError::InputTooLarge { diag }
            | RenderError::YamlTooLarge { diag }
            | RenderError::NestingTooDeep { diag }
            | RenderError::OutputTooLarge { diag } => vec![diag],
        }
    }
}
```

---

## Error Conversion Patterns

### Pattern 1: External Library Errors

When converting errors from external libraries (MiniJinja, Typst, serde_yaml):

```rust
impl From<minijinja::Error> for RenderError {
    fn from(e: minijinja::Error) -> Self {
        let loc = e.line().map(|line| Location {
            file: e.name().unwrap_or("template").to_string(),
            line: line as u32,
            col: e.range().map(|r| r.start as u32).unwrap_or(0),
        });

        let hint = generate_minijinja_hint(&e);

        let diag = Diagnostic {
            severity: Severity::Error,
            code: Some(format!("minijinja::{:?}", e.kind())),
            message: e.to_string(),
            primary: loc,
            related: vec![],
            hint,
            source: Some(Box::new(e.clone())), // Preserve source!
        };

        RenderError::TemplateFailed { diag }
    }
}
```

### Pattern 2: Parsing Errors with Location Extraction

For serde_yaml errors that have location information:

```rust
impl From<serde_yaml::Error> for RenderError {
    fn from(e: serde_yaml::Error) -> Self {
        let loc = e.location().map(|l| Location {
            file: "frontmatter".to_string(),
            line: l.line() as u32,
            col: l.column() as u32,
        });

        let diag = Diagnostic::new(Severity::Error, e.to_string())
            .with_code("yaml::parse".to_string())
            .with_source(Box::new(e))
            .with_hint("Check YAML syntax - common issues include incorrect indentation, unclosed quotes, or invalid characters".to_string());
        
        if let Some(loc) = loc {
            diag = diag.with_location(loc);
        }

        RenderError::InvalidFrontmatter { diag }
    }
}
```

### Pattern 3: Creating New Diagnostics

When creating errors from scratch:

```rust
// Simple error
RenderError::UnsupportedBackend {
    diag: Diagnostic::new(Severity::Error, format!("Backend '{}' not registered", name))
        .with_code("engine::backend_not_found".to_string())
        .with_hint(format!("Available backends: {}", available.join(", "))),
}

// Error with location
RenderError::DynamicAssetCollision {
    diag: Diagnostic::new(
        Severity::Error,
        format!("Asset filename collision: '{}'", filename),
    )
    .with_code("engine::asset_collision".to_string())
    .with_location(Location {
        file: "dynamic_assets".to_string(),
        line: 0,
        col: 0,
    })
    .with_hint("Use unique filenames for dynamic assets".to_string()),
}
```

---

## Python Bindings Enhancement

### Current Problem

Python exceptions lose diagnostic details:

```rust
// Current implementation (BAD)
RenderError::CompilationFailed(count, _diags) => 
    CompilationError::new_err(format!("Compilation failed with {} error(s)", count))
```

### Solution: Attach Diagnostics to Exceptions

```rust
// quillmark-python/src/types.rs

/// Python-exposed diagnostic type
#[pyclass(name = "Diagnostic")]
#[derive(Clone)]
pub struct PyDiagnostic {
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub code: Option<String>,
    #[pyo3(get)]
    pub message: String,
    #[pyo3(get)]
    pub primary: Option<PyLocation>,
    #[pyo3(get)]
    pub related: Vec<PyLocation>,
    #[pyo3(get)]
    pub hint: Option<String>,
    #[pyo3(get)]
    pub source_chain: Vec<String>,
}

impl From<Diagnostic> for PyDiagnostic {
    fn from(diag: Diagnostic) -> Self {
        Self {
            severity: format!("{:?}", diag.severity),
            code: diag.code,
            message: diag.message,
            primary: diag.primary.map(Into::into),
            related: diag.related.into_iter().map(Into::into).collect(),
            hint: diag.hint,
            source_chain: diag.source_chain(),
        }
    }
}
```

```rust
// quillmark-python/src/errors.rs

pub fn convert_render_error(err: RenderError) -> PyErr {
    Python::with_gil(|py| {
        match err {
            RenderError::InvalidFrontmatter { diag } => {
                let py_err = ParseError::new_err(diag.message.clone());
                let exc = py_err.value_bound(py);
                let py_diag = PyDiagnostic::from(diag);
                let _ = exc.setattr("diagnostic", py_diag);
                py_err
            }
            RenderError::TemplateFailed { diag } => {
                let py_err = TemplateError::new_err(diag.message.clone());
                let exc = py_err.value_bound(py);
                let py_diag = PyDiagnostic::from(diag);
                let _ = exc.setattr("diagnostic", py_diag);
                py_err
            }
            RenderError::CompilationFailed { diags } => {
                let py_err = CompilationError::new_err(
                    format!("Compilation failed with {} error(s)", diags.len())
                );
                let exc = py_err.value_bound(py);
                let py_diags: Vec<PyDiagnostic> = diags
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let _ = exc.setattr("diagnostics", py_diags);
                py_err
            }
            // ... handle other variants
            _ => QuillmarkError::new_err(err.to_string()),
        }
    })
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
            print(f"  [{diag.severity}] {diag.message}")
            if diag.primary:
                print(f"    --> {diag.primary.file}:{diag.primary.line}:{diag.primary.col}")
            if diag.hint:
                print(f"    hint: {diag.hint}")
            # Source chain for debugging
            for i, cause in enumerate(diag.source_chain, 1):
                print(f"    cause {i}: {cause}")
```

---

## Addressing Specific Use Cases

### Use Case 1: YAML Parsing Errors

**Current Issue**: No location information.

**Solution**:

```rust
pub fn decompose(markdown: &str) -> Result<ParsedDocument, RenderError> {
    // ... parsing logic ...
    
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_str)
        .map_err(|e| {
            let loc = e.location().map(|l| Location {
                file: "frontmatter".to_string(),
                line: l.line() as u32,
                col: l.column() as u32,
            });
            
            let mut diag = Diagnostic::new(Severity::Error, "Invalid YAML frontmatter".to_string())
                .with_code("parse::yaml".to_string())
                .with_source(Box::new(e))
                .with_hint("Check YAML syntax and indentation".to_string());
            
            if let Some(loc) = loc {
                diag = diag.with_location(loc);
            }
            
            RenderError::InvalidFrontmatter { diag }
        })?;
    
    // ...
}
```

### Use Case 2: Template Rendering Errors

**Current Implementation**: Already good, but can preserve source.

**Enhancement**:

```rust
impl From<minijinja::Error> for RenderError {
    fn from(e: minijinja::Error) -> Self {
        let loc = e.line().map(|line| Location {
            file: e.name().unwrap_or("template").to_string(),
            line: line as u32,
            col: e.range().map(|r| r.start as u32).unwrap_or(0),
        });

        let hint = generate_minijinja_hint(&e);
        let source = Box::new(e.clone()) as Box<dyn std::error::Error + Send + Sync>;

        let diag = Diagnostic::new(Severity::Error, e.to_string())
            .with_code(format!("minijinja::{:?}", e.kind()))
            .with_source(source)
            .with_hint_opt(hint);
        
        if let Some(loc) = loc {
            diag = diag.with_location(loc);
        }

        RenderError::TemplateFailed { diag }
    }
}
```

### Use Case 3: Backend Compilation Errors

**Current Implementation**: Good structure with Vec<Diagnostic>.

**Enhancement**: Ensure all backend errors are mapped properly.

```rust
// In quillmark-typst/src/error_mapping.rs
pub fn map_typst_errors(
    errors: &[SourceDiagnostic],
    world: &QuillWorld,
) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| {
            let severity = match error.severity {
                typst::diag::Severity::Error => Severity::Error,
                typst::diag::Severity::Warning => Severity::Warning,
            };

            let primary = resolve_span_to_location(error.span, world);
            
            let related = error
                .trace
                .iter()
                .filter_map(|t| resolve_span_to_location(t.span, world))
                .collect();

            let code = format!(
                "typst::{}",
                error.message.split(':').next().unwrap_or("error")
            );

            let hint = error.hints.first().map(|h| h.to_string());

            Diagnostic {
                severity,
                code: Some(code),
                message: error.message.clone(),
                primary,
                related,
                hint,
                source: None, // Typst errors are leaf errors
            }
        })
        .collect()
}
```

### Use Case 4: AcroForm Silent Failures

**Current Issue**: Template errors silently ignored.

**Solution**:

```rust
// In quillmark-acroform/src/lib.rs
let rendered_value = env.render_str(&source, &context).map_err(|e| {
    let diag = Diagnostic::new(
        Severity::Error,
        format!("Failed to render template for field '{}'", field.name),
    )
    .with_code("acroform::template".to_string())
    .with_source(Box::new(e))
    .with_hint(format!("Template: {}", source));
    
    RenderError::TemplateFailed { diag }
})?;
```

---

## Migration Strategy

### Phase 1: Foundation (Non-Breaking)

1. **Add source field to Diagnostic** ✓
   - Add optional `source` field
   - Add `with_source()` builder method
   - Add `source_chain()` helper
   - Add `fmt_pretty_with_source()` method

2. **Create SerializableDiagnostic** ✓
   - For Python/WASM boundaries
   - Includes flattened source_chain

3. **Update existing From implementations** ✓
   - Add source preservation to MiniJinja conversion
   - Add location extraction to serde_yaml conversion
   - Keep existing error variants unchanged

### Phase 2: Standardization

4. **Update all RenderError variants** ✓
   - Ensure every variant has a Diagnostic
   - Remove redundant fields (backend, format, etc.) from variants
   - Move all context into Diagnostic fields

5. **Fix Python bindings** ✓
   - Update convert_render_error to attach diagnostics
   - Add diagnostic attribute to all exception types
   - Test Python error visibility

6. **Fix YAML parsing** ✓
   - Extract location from serde_yaml::Error
   - Create proper Diagnostic with location
   - Add helpful hints

7. **Fix AcroForm errors** ✓
   - Propagate template rendering errors
   - Create diagnostics with proper context
   - Remove silent failures

### Phase 3: Enhancement

8. **Add warning system**
   - Expose Typst warnings
   - Populate RenderResult.warnings
   - Test warning propagation

9. **Improve error messages**
   - Add context-aware hints
   - Improve error codes
   - Add documentation links

10. **Add tests (KISS; just test core functionality)**
    - Test error conversion
    - Test diagnostic preservation
    - Test Python visibility
    - Test WASM serialization
---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_with_source_chain() {
        let root_err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        );
        let mid_err = std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to load template",
        );
        
        let diag = Diagnostic::new(Severity::Error, "Rendering failed".to_string())
            .with_source(Box::new(root_err));
        
        let chain = diag.source_chain();
        assert_eq!(chain.len(), 1);
        assert!(chain[0].contains("File not found"));
    }

    #[test]
    fn diagnostic_serialization() {
        let diag = Diagnostic::new(Severity::Error, "Test error".to_string())
            .with_code("E001".to_string())
            .with_location(Location {
                file: "test.typ".to_string(),
                line: 10,
                col: 5,
            });
        
        let serializable: SerializableDiagnostic = diag.clone().into();
        let json = serde_json::to_string(&serializable).unwrap();
        assert!(json.contains("Test error"));
        assert!(json.contains("E001"));
    }

    #[test]
    fn render_error_diagnostics_extraction() {
        let diag1 = Diagnostic::new(Severity::Error, "Error 1".to_string());
        let diag2 = Diagnostic::new(Severity::Error, "Error 2".to_string());
        
        let err = RenderError::CompilationFailed {
            diags: vec![diag1, diag2],
        };
        
        let diags = err.diagnostics();
        assert_eq!(diags.len(), 2);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_yaml_error_with_location() {
    let markdown = r#"---
title: [unclosed
---
# Content
"#;
    
    let result = ParsedDocument::from_markdown(markdown);
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    if let RenderError::InvalidFrontmatter { diag } = err {
        assert!(diag.primary.is_some());
        let loc = diag.primary.unwrap();
        assert_eq!(loc.file, "frontmatter");
        assert!(loc.line > 0);
        assert!(!diag.source_chain().is_empty());
    } else {
        panic!("Expected InvalidFrontmatter error");
    }
}
```

### Python Integration Tests

```python
def test_compilation_error_diagnostics():
    """Test that Python exceptions expose diagnostic details"""
    markdown = """---
QUILL: test-quill
---
# Content
"""
    # Inject bad Typst code into quill
    
    try:
        result = workflow.render(parsed, OutputFormat.PDF)
        assert False, "Should have raised CompilationError"
    except CompilationError as e:
        assert hasattr(e, 'diagnostics')
        assert len(e.diagnostics) > 0
        
        diag = e.diagnostics[0]
        assert diag.severity in ['Error', 'Warning']
        assert diag.message
        assert diag.primary is not None
        assert diag.primary.file
        assert diag.primary.line > 0
```

---

## Answers to Key Questions

### Q1: Can Diagnostic cover every error use case?

**Answer: YES**, with the source field addition.

**Coverage:**
- ✅ Syntax errors (with location)
- ✅ Parsing errors (with location from serde_yaml)
- ✅ Template errors (with location from MiniJinja)
- ✅ Compilation errors (with location from Typst)
- ✅ Validation errors (with custom location)
- ✅ Runtime errors (with source chain)
- ✅ External library errors (via source field)
- ✅ Multiple errors (CompilationFailed uses Vec<Diagnostic>)
- ✅ Warnings (via Severity::Warning)

The Diagnostic struct is flexible enough to represent all error types while maintaining structure.

### Q2: Should we add a `source` field within Diagnostic?

**Answer: YES.**

**Benefits:**
- Preserves full error context and causality
- Enables proper error chain traversal
- Compatible with Rust's Error trait pattern
- Can be excluded from serialization when needed
- Doesn't break existing code (optional field)

**Implementation:**
```rust
pub struct Diagnostic {
    // ... existing fields ...
    #[serde(skip)]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
```

### Q3: Should we allow custom errors that implement Diagnostic as a trait or enum?

**Answer: NO - Keep Diagnostic as a concrete struct.**

**Rationale:**
1. **Simplicity**: Concrete struct is easier to use and understand
2. **Serialization**: Trait objects can't be serialized directly
3. **FFI Compatibility**: Concrete types work better with Python/WASM
4. **No Need**: RenderError enum provides type-safe variant discrimination
5. **Performance**: No dynamic dispatch overhead

**Alternative**: Use RenderError enum for variant discrimination, Diagnostic for payload.

This gives us:
- Type safety through RenderError variants
- Structured data through Diagnostic
- Pattern matching on error types
- Serialization of diagnostic data
- Best of both worlds!

---

## Comparison with Alternatives

### Alternative 1: Diagnostic as Trait

```rust
pub trait Diagnostic {
    fn severity(&self) -> Severity;
    fn message(&self) -> &str;
    fn location(&self) -> Option<&Location>;
    // ...
}
```

**Pros:**
- Extensible by external crates
- Can have different implementations

**Cons:**
- Can't serialize trait objects directly
- Requires Box<dyn Diagnostic> (heap allocation)
- Complex for cross-language boundaries
- Loses type information
- Not needed given RenderError variants

**Verdict: REJECTED** - Complexity outweighs benefits.

### Alternative 2: Result-Style Objects (No Exceptions)

```rust
pub struct RenderOutcome {
    success: bool,
    artifacts: Option<Vec<Artifact>>,
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
}
```

**Pros:**
- All information in one place
- No exception throwing

**Cons:**
- Breaks Rust conventions (Result is idiomatic)
- Breaking API change
- Less type-safe (success flag instead of Result)
- Awkward error handling in Rust

**Verdict: REJECTED** - Not idiomatic Rust.

### Alternative 3: Current System (Mixed Approach)

**Status Quo:**
- Some errors use Diagnostic
- Some use generic boxed errors
- Python loses diagnostic details

**Verdict: NEEDS IMPROVEMENT** - This proposal addresses the gaps.

---

## Implementation Checklist

### Immediate (Must Have)

- [ ] Add `source` field to Diagnostic
- [ ] Add `source_chain()` and `fmt_pretty_with_source()` methods
- [ ] Create SerializableDiagnostic type
- [ ] Update MiniJinja error conversion to preserve source
- [ ] Update serde_yaml error conversion to extract location
- [ ] Fix Python exception diagnostic exposure
- [ ] Fix AcroForm silent template failures
- [ ] Add comprehensive error conversion tests

### Soon (Should Have)

- [ ] Standardize all RenderError variants to use Diagnostic
- [ ] Remove redundant fields from error variants
- [ ] Expose Typst warnings
- [ ] Add context-aware error hints
- [ ] Improve error codes across all backends
- [ ] Add Python integration tests for diagnostics

### Later (Nice to Have)

- [ ] Add source code snippets to fmt_pretty output
- [ ] Implement @origin comment system for source mapping
- [ ] Add JSON output mode for errors
- [ ] Create error code documentation registry
- [ ] Add IDE integration support
- [ ] Color-coded terminal output

---

## Conclusion

This proposal provides a **clean, elegant, and practical solution** to unify Quillmark's error system around Diagnostic while maintaining backwards compatibility and simplicity.

**Key Decisions:**
1. ✅ Keep Diagnostic as concrete struct (not trait)
2. ✅ Add optional source field for error chaining
3. ✅ Keep RenderError enum for variant discrimination
4. ✅ Ensure every error path has structured diagnostics
5. ✅ Fix Python bindings to expose diagnostics on exceptions
6. ✅ Standardize error conversion from external libraries

**Benefits:**
- Consistent error reporting across all components
- Full error context preservation through source chains
- Python/WASM can access rich diagnostic information
- Backwards compatible with existing error handling
- Simple to use and understand
- Serializable for cross-language boundaries

The design strikes the right balance between **power and simplicity**, addressing all identified gaps while maintaining Quillmark's design principles of clarity, elegance, and zero surprises.
