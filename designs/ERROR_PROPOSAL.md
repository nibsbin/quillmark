# Error Handling System Evaluation & Proposal

**Status:** Proposal  
**Date:** 2024  
**Scope:** quillmark, quillmark-core, quillmark-typst

---

## Executive Summary

This document evaluates the current error handling system across Quillmark's three main crates and proposes improvements for robustness, informativeness, and simplicity.

**Key Findings:**
- ✅ **Strong foundation:** Structured `Diagnostic` type with location tracking
- ✅ **Good separation:** Clear error enums avoid stringly-typed errors
- ⚠️ **Incomplete:** Typst error mapping not fully implemented
- ⚠️ **Inconsistent:** MiniJinja errors well-mapped, Typst errors lose structure
- ⚠️ **Duplication:** Multiple error formatting paths and print functions

**Proposed Improvements:**
1. Complete Typst error mapping to `Diagnostic` type
2. Consolidate error printing utilities
3. Add structured error context propagation
4. Implement source mapping for better debugging
5. Standardize hint generation across error sources

---

## Current State Analysis

### 1. Core Error Types (`quillmark-core/src/error.rs`)

**Strengths:**
- Well-designed `Diagnostic` structure with:
  - `Severity` enum (Error, Warning, Note)
  - `Location` with file, line, col
  - Optional error codes and hints
  - Related locations for traces
  - Serializable (JSON output ready)
- Builder pattern for diagnostic creation
- Clear `RenderError` enum variants

**Weaknesses:**
- `print_errors()` function incomplete (uses `_ => eprintln!("{}", err)` fallback)
- No structured context propagation (only `Option<anyhow::Error>`)
- Missing column information in many cases (especially MiniJinja)
- `Diagnostic.fmt_pretty()` is basic - no source code context

**Code Quality:**
```rust
// GOOD: Builder pattern
let diag = Diagnostic::new(Severity::Error, "message".to_string())
    .with_code("E001".to_string())
    .with_location(loc)
    .with_hint("try this".to_string());

// WEAK: Generic fallback loses structure
_ => eprintln!("{}", err)
```

### 2. MiniJinja Error Mapping (`quillmark-core/src/error.rs`)

**Strengths:**
- Automatic conversion via `impl From<minijinja::Error> for RenderError`
- Captures line number when available
- Creates structured `Diagnostic` with error kind in code field
- Preserves original error as source

**Weaknesses:**
- **No column information:** MiniJinja provides it but we set `col: 0`
- **Limited context:** No hint generation from common MiniJinja errors
- **Lost detail:** `message: e.to_string()` instead of structured fields

**Current Implementation:**
```rust
impl From<minijinja::Error> for RenderError {
    fn from(e: minijinja::Error) -> Self {
        let loc = e.line().map(|line| Location {
            file: e.name().unwrap_or("template").to_string(),
            line: line as u32,
            col: 0, // ❌ MiniJinja doesn't provide column info - INCORRECT!
        });

        let diag = Diagnostic {
            severity: Severity::Error,
            code: Some(format!("minijinja::{:?}", e.kind())),
            message: e.to_string(), // ❌ Loses structured information
            primary: loc,
            related: vec![],
            hint: None, // ❌ No hint generation
        };

        RenderError::TemplateFailed { source: e, diag }
    }
}
```

**MiniJinja Actually Provides:**
- `error.line()` - line number (we use this ✅)
- `error.range()` - character range (we ignore this ❌)
- `error.kind()` - error type (we use this ✅)
- `error.detail()` - additional context (we ignore this ❌)

### 3. Typst Error Handling (`quillmark-typst/src/compile.rs`)

**Strengths:**
- Attempts to extract location information from spans
- Shows hints and traces
- Multi-error reporting

**Critical Weaknesses:**
- **No `Diagnostic` mapping:** Uses string formatting instead of structured errors
- **Returns `Box<dyn std::error::Error>`:** Loses all structure before reaching caller
- **Inconsistent with design:** DESIGN.md describes `map_typst()` function that doesn't exist
- **Poor integration:** Backend catches errors as strings, can't provide structured diagnostics

**Current Implementation:**
```rust
fn format_compilation_errors(errors: &[SourceDiagnostic], world: &QuillWorld) -> String {
    // ❌ Returns String instead of Vec<Diagnostic>
    let mut formatted = format!("Compilation failed with {} error(s):", errors.len());
    // ... string concatenation
    formatted
}

fn compile_document(world: &QuillWorld) -> Result<PagedDocument, Box<dyn std::error::Error>> {
    let Warned { output, warnings: _ } = typst::compile::<PagedDocument>(world);
    output.map_err(|errors| format_compilation_errors(&errors, world).into())
    // ❌ Converts to string, loses structure
}
```

**Typst Actually Provides:**
- `SourceDiagnostic.span` - source location (we partially use this ✅)
- `SourceDiagnostic.severity` - error/warning/note (we ignore this ❌)
- `SourceDiagnostic.message` - error message (we use this ✅)
- `SourceDiagnostic.hints` - suggestions (we show but don't structure ⚠️)
- `SourceDiagnostic.trace` - stack trace (we show but don't structure ⚠️)

### 4. TemplateError Type (`quillmark-core/src/templating.rs`)

**Strengths:**
- Dedicated error type for template operations
- Preserves source errors

**Weaknesses:**
- Basic enum, no `Diagnostic` integration
- Converted to `RenderError::Template` which is less structured
- `InvalidTemplate` and `FilterError` variants lose context

**Current Implementation:**
```rust
pub enum TemplateError {
    #[error("{0}")]
    RenderError(#[from] minijinja::Error), // ✅ Auto-converts
    #[error("{0}")]
    InvalidTemplate(String, #[source] Box<dyn StdError + Send + Sync>), // ⚠️ String message
    #[error("{0}")]
    FilterError(String), // ❌ No structure at all
}
```

### 5. Backend Integration (`quillmark-typst/src/lib.rs`)

**Critical Issues:**
- Backend `compile()` method calls `compile_to_pdf()` with `.unwrap()` - **PANIC RISK**
- SVG compilation uses `.map_err(|e| RenderError::Other(...))` - loses Typst diagnostics
- No use of `RenderError::CompilationFailed` variant with structured diagnostics

**Current Implementation:**
```rust
match format {
    OutputFormat::Pdf => {
        let bytes = compile::compile_to_pdf(quill, glued_content).unwrap(); // ❌ PANIC!
        Ok(vec![Artifact { bytes, output_format: OutputFormat::Pdf }])
    }
    OutputFormat::Svg => {
        let svg_pages = compile::compile_to_svg(quill, glued_content).map_err(|e| {
            RenderError::Other(format!("SVG compilation failed: {}", e).into())
            // ❌ Loses all Typst diagnostics
        })?;
        // ...
    }
}
```

---

## Gap Analysis

### Critical Gaps

1. **❌ Typst Error Mapping Not Implemented**
   - DESIGN.md describes `map_typst()` function that doesn't exist
   - Typst errors converted to strings, losing all structure
   - No `RenderError::CompilationFailed` usage in backend
   - **Impact:** Users get string errors instead of actionable diagnostics

2. **❌ Unsafe Error Handling in Backend**
   - `.unwrap()` in PDF compilation path causes panics
   - No proper error propagation from Typst to RenderError
   - **Impact:** Production crashes instead of graceful error reporting

3. **❌ Incomplete MiniJinja Mapping**
   - Column information available but not captured
   - Error detail and range information ignored
   - No hint generation for common template errors
   - **Impact:** Less helpful error messages than possible

4. **❌ Inconsistent Error Formatting**
   - Multiple print functions (`print_errors()`, string formatting)
   - No source code context in error output
   - Generic fallback loses information
   - **Impact:** Inconsistent user experience, harder debugging

### Minor Gaps

5. **⚠️ No Warnings from Typst**
   - Typst provides warnings, we ignore them
   - `RenderResult.warnings` not populated from backend
   - **Impact:** Users miss non-fatal issues

6. **⚠️ Limited Error Context**
   - No propagation of operation context (which template, which filter, etc.)
   - Hints not generated based on error patterns
   - **Impact:** Harder to debug complex rendering pipelines

7. **⚠️ Source Mapping Not Implemented**
   - DESIGN.md describes comment anchor system (`@origin:`)
   - Not implemented in code
   - **Impact:** Can't trace Typst errors back to Markdown source

---

## Proposed Improvements

### Phase 1: Critical Fixes (High Priority)

#### 1.1 Implement Typst Error Mapping

**Create:** `quillmark-typst/src/error_mapping.rs`

```rust
use quillmark_core::{Diagnostic, Location, Severity};
use typst::diag::SourceDiagnostic;
use crate::world::QuillWorld;

/// Convert Typst SourceDiagnostic to structured Diagnostic
pub fn map_typst_errors(
    errors: &[SourceDiagnostic],
    world: &QuillWorld,
) -> Vec<Diagnostic> {
    errors.iter().map(|e| map_single_diagnostic(e, world)).collect()
}

fn map_single_diagnostic(
    error: &SourceDiagnostic,
    world: &QuillWorld,
) -> Diagnostic {
    // Map severity
    let severity = match error.severity {
        typst::diag::Severity::Error => Severity::Error,
        typst::diag::Severity::Warning => Severity::Warning,
    };

    // Extract location from span
    let location = resolve_span_to_location(&error.span, world);

    // Map trace to related locations
    let related = error.trace.iter()
        .filter_map(|span| resolve_span_to_location(span, world))
        .collect();

    // Get first hint if available
    let hint = error.hints.first().map(|h| h.to_string());

    Diagnostic {
        severity,
        code: Some(format!("typst::{}", error.message.split(':').next().unwrap_or("error"))),
        message: error.message.clone(),
        primary: location,
        related,
        hint,
    }
}

fn resolve_span_to_location(
    span: &typst::syntax::Span,
    world: &QuillWorld,
) -> Option<Location> {
    use typst::World;
    
    let source_id = world.main();
    let source = world.source(source_id).ok()?;
    let range = source.range(*span)?;
    
    let text = source.text();
    let line = text[..range.start].matches('\n').count() + 1;
    let col = range.start - text[..range.start].rfind('\n').map_or(0, |pos| pos + 1) + 1;
    
    Some(Location {
        file: source.id().vpath().as_rootless_path().display().to_string(),
        line: line as u32,
        col: col as u32,
    })
}
```

**Update:** `quillmark-typst/src/compile.rs`

```rust
use quillmark_core::RenderError;
use crate::error_mapping::map_typst_errors;

/// Compiles a Typst document to PDF format.
pub fn compile_to_pdf(
    quill: &Quill,
    glued_content: &str,
) -> Result<Vec<u8>, RenderError> {
    let world = QuillWorld::new(quill, glued_content)
        .map_err(|e| RenderError::Internal(anyhow::anyhow!(e)))?;
    
    let document = compile_document(&world)?;
    
    let pdf = typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("PDF generation failed: {:?}", e)))?;
    
    Ok(pdf)
}

fn compile_document(world: &QuillWorld) -> Result<PagedDocument, RenderError> {
    let Warned { output, warnings } = typst::compile::<PagedDocument>(world);
    
    match output {
        Ok(doc) => {
            // TODO: Add warnings to RenderResult
            Ok(doc)
        }
        Err(errors) => {
            let diagnostics = map_typst_errors(&errors, world);
            Err(RenderError::CompilationFailed(diagnostics.len(), diagnostics))
        }
    }
}
```

**Update:** `quillmark-typst/src/lib.rs`

```rust
impl Backend for TypstBackend {
    fn compile(
        &self,
        glued_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<Vec<Artifact>, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format,
            });
        }

        match format {
            OutputFormat::Pdf => {
                let bytes = compile::compile_to_pdf(quill, glued_content)?; // ✅ Proper error propagation
                Ok(vec![Artifact {
                    bytes,
                    output_format: OutputFormat::Pdf,
                }])
            }
            OutputFormat::Svg => {
                let svg_pages = compile::compile_to_svg(quill, glued_content)?; // ✅ Proper error propagation
                Ok(svg_pages
                    .into_iter()
                    .map(|bytes| Artifact {
                        bytes,
                        output_format: OutputFormat::Svg,
                    })
                    .collect())
            }
            OutputFormat::Txt => Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format: OutputFormat::Txt,
            }),
        }
    }
}
```

#### 1.2 Improve MiniJinja Error Mapping

**Update:** `quillmark-core/src/error.rs`

```rust
impl From<minijinja::Error> for RenderError {
    fn from(e: minijinja::Error) -> Self {
        // Extract location with proper range information
        let loc = e.line().and_then(|line| {
            Some(Location {
                file: e.name().unwrap_or("template").to_string(),
                line: line as u32,
                // MiniJinja provides range, we can extract approximate column
                col: e.range().map(|r| r.start as u32).unwrap_or(0),
            })
        });

        // Generate helpful hints based on error kind
        let hint = generate_minijinja_hint(&e);

        let diag = Diagnostic {
            severity: Severity::Error,
            code: Some(format!("minijinja::{:?}", e.kind())),
            message: format!("{}", e),
            primary: loc,
            related: vec![],
            hint,
        };

        RenderError::TemplateFailed { source: e, diag }
    }
}

/// Generate helpful hints for common MiniJinja errors
fn generate_minijinja_hint(e: &minijinja::Error) -> Option<String> {
    use minijinja::ErrorKind;
    
    match e.kind() {
        ErrorKind::UndefinedError => {
            Some("Check variable spelling and ensure it's defined in frontmatter".to_string())
        }
        ErrorKind::InvalidOperation => {
            Some("Check that you're using the correct filter or operator for this type".to_string())
        }
        ErrorKind::SyntaxError => {
            Some("Check template syntax - look for unclosed tags or invalid expressions".to_string())
        }
        _ => e.detail().map(|d| d.to_string()),
    }
}
```

#### 1.3 Consolidate Error Printing

**Update:** `quillmark-core/src/error.rs`

```rust
impl Diagnostic {
    /// Format diagnostic for pretty printing with optional source context
    pub fn fmt_pretty(&self) -> String {
        self.fmt_pretty_with_context(None)
    }

    /// Format diagnostic with source code context
    pub fn fmt_pretty_with_context(&self, source: Option<&str>) -> String {
        let mut result = format!(
            "{}[{}]{} {}",
            match self.severity {
                Severity::Error => "\x1b[31m",   // Red
                Severity::Warning => "\x1b[33m", // Yellow
                Severity::Note => "\x1b[36m",    // Cyan
            },
            match self.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARN",
                Severity::Note => "NOTE",
            },
            "\x1b[0m", // Reset
            self.message
        );

        if let Some(ref code) = self.code {
            result.push_str(&format!(" \x1b[90m({})\x1b[0m", code));
        }

        if let Some(ref loc) = self.primary {
            result.push_str(&format!(
                "\n  \x1b[90m-->\x1b[0m {}:{}:{}",
                loc.file, loc.line, loc.col
            ));

            // Add source context if available
            if let Some(src) = source {
                if let Some(context) = extract_source_context(src, loc.line, loc.col) {
                    result.push_str(&format!("\n{}", context));
                }
            }
        }

        // Add related locations
        for (i, related) in self.related.iter().enumerate() {
            result.push_str(&format!(
                "\n  \x1b[90m{}:\x1b[0m {}:{}:{}",
                if i == 0 { "trace" } else { "     " },
                related.file,
                related.line,
                related.col
            ));
        }

        if let Some(ref hint) = self.hint {
            result.push_str(&format!("\n  \x1b[36mhint:\x1b[0m {}", hint));
        }

        result
    }
}

/// Extract source code context around error location
fn extract_source_context(source: &str, line: u32, col: u32) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = (line as usize).saturating_sub(1);
    
    if line_idx >= lines.len() {
        return None;
    }

    let mut context = String::new();
    
    // Show line before (if exists)
    if line_idx > 0 {
        context.push_str(&format!("\n{:5} | {}", line - 1, lines[line_idx - 1]));
    }
    
    // Show error line with pointer
    context.push_str(&format!("\n{:5} | {}", line, lines[line_idx]));
    let pointer_offset = col as usize;
    context.push_str(&format!(
        "\n      | {}{}",
        " ".repeat(pointer_offset.saturating_sub(1)),
        "\x1b[31m^\x1b[0m"
    ));
    
    // Show line after (if exists)
    if line_idx + 1 < lines.len() {
        context.push_str(&format!("\n{:5} | {}", line + 1, lines[line_idx + 1]));
    }
    
    Some(context)
}

/// Enhanced error printing with source context where available
pub fn print_errors(err: &RenderError) {
    match err {
        RenderError::CompilationFailed(_, diags) => {
            for d in diags {
                eprintln!("{}", d.fmt_pretty());
            }
        }
        RenderError::TemplateFailed { diag, .. } => {
            eprintln!("{}", diag.fmt_pretty());
        }
        RenderError::InvalidFrontmatter { diag, .. } => {
            eprintln!("{}", diag.fmt_pretty());
        }
        RenderError::EngineCreation { diag, .. } => {
            eprintln!("{}", diag.fmt_pretty());
        }
        RenderError::FormatNotSupported { backend, format } => {
            eprintln!("\x1b[31m[ERROR]\x1b[0m Format {:?} not supported by {} backend", format, backend);
        }
        RenderError::UnsupportedBackend(name) => {
            eprintln!("\x1b[31m[ERROR]\x1b[0m Unsupported backend: {}", name);
        }
        RenderError::DynamicAssetCollision { filename, message } => {
            eprintln!("\x1b[31m[ERROR]\x1b[0m Dynamic asset collision: {}\n  {}", filename, message);
        }
        RenderError::Internal(e) => {
            eprintln!("\x1b[31m[ERROR]\x1b[0m Internal error: {}", e);
        }
        RenderError::Template(e) => {
            eprintln!("\x1b[31m[ERROR]\x1b[0m Template error: {}", e);
        }
        RenderError::Other(e) => {
            eprintln!("\x1b[31m[ERROR]\x1b[0m {}", e);
        }
    }
}
```

### Phase 2: Enhanced Features (Medium Priority)

#### 2.1 Propagate Typst Warnings

**Update:** `quillmark-core/src/backend.rs`

```rust
/// Compile the glue content into final artifacts with warnings
fn compile(
    &self,
    glue_content: &str,
    quill: &Quill,
    opts: &RenderOptions,
) -> Result<RenderResult, RenderError>; // Changed return type
```

This would be a breaking change, so alternative: add warnings to RenderError::CompilationFailed

```rust
/// Backend compilation failed with errors (and possibly warnings)
#[error("Backend compilation failed with {0} error(s)")]
CompilationFailed(usize, Vec<Diagnostic>, Vec<Diagnostic>), // errors, warnings
```

#### 2.2 Add Error Context

**Create:** `quillmark-core/src/error_context.rs`

```rust
/// Context for error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation being performed
    pub operation: String,
    /// Template or file being processed
    pub source: Option<String>,
    /// Additional context
    pub metadata: HashMap<String, String>,
}

impl Diagnostic {
    /// Add context to diagnostic
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        // Could add to hint or related fields
        self
    }
}
```

#### 2.3 Implement Source Mapping

As described in DESIGN.md, inject `@origin:` comments and map errors back to source.

### Phase 3: Polish (Low Priority)

#### 3.1 JSON Error Output

```rust
impl Diagnostic {
    /// Serialize to JSON for tooling integration
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

pub fn print_errors_json(err: &RenderError) -> String {
    // Serialize entire error tree to JSON
    serde_json::to_string_pretty(&err).unwrap_or_else(|_| "{}".to_string())
}
```

#### 3.2 Error Aggregation

```rust
/// Collect all diagnostics from a RenderError
pub fn collect_diagnostics(err: &RenderError) -> Vec<&Diagnostic> {
    match err {
        RenderError::CompilationFailed(_, diags) => diags.iter().collect(),
        RenderError::TemplateFailed { diag, .. } => vec![diag],
        RenderError::InvalidFrontmatter { diag, .. } => vec![diag],
        RenderError::EngineCreation { diag, .. } => vec![diag],
        _ => vec![],
    }
}
```

---

## Migration Path

### Step 1: Non-Breaking Improvements
1. Add error mapping utilities (no API changes)
2. Improve Diagnostic.fmt_pretty() (backward compatible)
3. Enhance MiniJinja mapping (internal change)

### Step 2: Fix Critical Issues
1. Implement Typst error mapping
2. Remove .unwrap() from backend
3. Use RenderError::CompilationFailed properly

### Step 3: Optional Enhancements
1. Add source mapping
2. Add JSON output
3. Enhance context propagation

---

## Testing Strategy

### New Tests Required

1. **Typst Error Mapping Tests**
   ```rust
   #[test]
   fn test_typst_syntax_error_mapping() {
       // Test that Typst syntax errors map to Diagnostic correctly
   }

   #[test]
   fn test_typst_error_with_trace() {
       // Test that Typst traces map to related locations
   }

   #[test]
   fn test_typst_warnings_captured() {
       // Test that warnings are captured
   }
   ```

2. **MiniJinja Error Tests**
   ```rust
   #[test]
   fn test_minijinja_undefined_variable_hint() {
       // Test hint generation for undefined variables
   }

   #[test]
   fn test_minijinja_column_capture() {
       // Test that column information is captured
   }
   ```

3. **Integration Tests**
   ```rust
   #[test]
   fn test_end_to_end_error_reporting() {
       // Test complete error flow from Markdown to diagnostic
   }
   ```

### Test Coverage Goals
- Typst error mapping: 90%+
- MiniJinja mapping improvements: 90%+
- Error printing: 80%+

---

## Performance Considerations

1. **Error Mapping Cost:** Minimal - only on error path
2. **Source Context Extraction:** Linear in source size, but only on error
3. **String Formatting:** Current implementation adequate

**Recommendation:** No performance concerns. Error path optimization not critical.

---

## Backward Compatibility

### Breaking Changes
- None in Phase 1
- Potential in Phase 2 if changing Backend trait

### Mitigation
- Keep old functions deprecated for one version
- Provide migration guide
- Use semantic versioning

---

## Comparison with Other Systems

### Rust Compiler
- **Similar:** Multi-level diagnostics, source context
- **Learn from:** Suggestion system, color coding
- **Adopt:** Source snippet rendering

### TypeScript Compiler
- **Similar:** Error codes, hints
- **Learn from:** Error documentation links
- **Adopt:** Error code registry with explanations

### Current vs Proposed

| Aspect | Current | Proposed | Improvement |
|--------|---------|----------|-------------|
| Typst errors | String | Structured Diagnostic | ✅ Major |
| MiniJinja column | Always 0 | Actual column | ✅ Moderate |
| Hints | Limited | Context-aware | ✅ Moderate |
| Source context | None | Code snippets | ✅ Major |
| Warnings | Ignored | Captured | ✅ Major |
| Consistency | Mixed | Unified | ✅ Major |

---

## Recommendations

### Immediate Actions (Do Now)
1. ✅ **Fix Typst error mapping** - Critical for usability
2. ✅ **Remove .unwrap()** - Critical for stability
3. ✅ **Improve MiniJinja mapping** - Moderate effort, high value

### Short Term (Next Sprint)
4. ⚠️ **Add source context to fmt_pretty()** - Better debugging
5. ⚠️ **Consolidate print functions** - Consistency

### Long Term (Future Versions)
6. 📋 **Implement source mapping** - Advanced feature
7. 📋 **Add JSON output mode** - Tooling integration
8. 📋 **Error documentation** - User education

---

## Conclusion

The current error handling system has a **solid foundation** but **incomplete implementation**, particularly for Typst errors. The proposed improvements are **achievable**, **high-value**, and **mostly non-breaking**.

**Priority:** Implement Phase 1 improvements immediately to:
- Eliminate panic risks
- Provide structured Typst diagnostics
- Improve user experience significantly

**Estimated Effort:** 
- Phase 1: 8-12 hours
- Phase 2: 4-8 hours
- Phase 3: 4-6 hours

**Risk:** Low - mostly additive changes with clear migration path.
