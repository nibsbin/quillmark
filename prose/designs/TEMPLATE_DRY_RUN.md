# Template Dry Run Validation

**Status:** Design Phase

This document describes the template dry run system for inexpensive input validation without full backend compilation.

## Overview

Template dry run provides a lightweight validation pass that surfaces input errors quickly to consumers (especially LLM agents) without incurring the cost of full Typst/PDF compilation. This enables fast feedback loops for iterative document generation.

## Motivation

The current `Workflow::render()` pipeline:
1. Parse markdown → `ParsedDocument`
2. Apply schema defaults and coercion
3. Validate against JSON schema
4. Compose template via MiniJinja → plated output
5. **Compile via backend** (expensive: Typst → PDF/SVG)
6. Return artifacts

For LLM-driven document generation, steps 1-4 can catch ~95% of input errors (malformed YAML, missing required fields, type mismatches, undefined template variables, filter errors) at a fraction of the cost. Backend compilation (step 5) is only needed for final output.

## Dry Run Stages

A dry run executes the following validation stages, stopping at the first error:

| Stage | Validates | Error Type |
|-------|-----------|------------|
| **Parse** | YAML syntax, structure, size limits | `InvalidFrontmatter`, `InputTooLarge` |
| **Schema Defaults** | Default value application | (always succeeds) |
| **Schema Coercion** | Type coercion | (always succeeds) |
| **Schema Validation** | Required fields, types, constraints | `ValidationFailed` |
| **Field Normalization** | Bidi characters, guillemets | (always succeeds) |
| **Template Composition** | Variable references, filter usage, syntax | `TemplateFailed` |

## API Design

### Rust API

The dry run function will be added to the `Workflow` struct:

```rust
impl Workflow {
    /// Perform a dry run validation without backend compilation.
    /// 
    /// Executes parsing, schema validation, and template composition to
    /// surface input errors quickly. Returns `Ok(())` on success, or
    /// `Err(RenderError)` with structured diagnostics on failure.
    pub fn dry_run(&self, parsed: &ParsedDocument) -> Result<(), RenderError>;
}
```

**Design rationale:**
- Takes `ParsedDocument` rather than raw markdown to match the existing `render()` signature and allow consumers to handle parsing separately
- Returns `Result<(), RenderError>` rather than custom type to leverage existing error infrastructure
- Method on `Workflow` (not standalone) because it requires the Quill's schema and plate template

### Alternative Considered: Return Composed Output

```rust
pub fn dry_run(&self, parsed: &ParsedDocument) -> Result<String, RenderError>;
```

This would return the composed plate content on success. While more informative for debugging, it:
- Increases allocations for typical usage (consumers usually only check for errors)
- The existing `process_plate()` already provides this functionality

**Recommendation:** Use the void return variant. Consumers who need the composed output can call `process_plate()` directly.

## Error Surfacing

All dry run errors are returned via the existing `RenderError` enum with `Diagnostic` payloads:

### Example: Missing Required Field

```rust
RenderError::ValidationFailed {
    diag: Diagnostic {
        severity: Error,
        code: Some("validation::document_invalid"),
        message: "Missing required property: title",
        primary: None,
        hint: Some("Ensure all required fields are present..."),
    }
}
```

### Example: Undefined Template Variable

```rust
RenderError::TemplateFailed {
    diag: Diagnostic {
        severity: Error,
        code: Some("minijinja::UndefinedError"),
        message: "Undefined variable 'author_name'",
        primary: Some(Location { file: "main", line: 5, col: 4 }),
        hint: Some("Check variable spelling and ensure it's defined in frontmatter"),
    }
}
```

## Integration Points

### WASM Bindings

Expose `dry_run` through the WASM interface:

```typescript
interface QuillmarkWorkflow {
    dryRun(parsedDocument: ParsedDocument): void; // throws on error
}
```

Errors serialize to `SerializableDiagnostic` for JavaScript consumption.

### Python Bindings

```python
class Workflow:
    def dry_run(self, parsed: ParsedDocument) -> None:
        """Raises QuillmarkError with diagnostic payload on validation failure."""
        ...
```

## Implementation Notes

### Leveraging Existing Code

The `process_plate()` method already performs all dry run stages:

```rust
pub fn process_plate(&self, parsed: &ParsedDocument) -> Result<String, RenderError> {
    // Apply defaults, coercion, validation, normalization
    // Compose template
    // Return composed output
}
```

The `dry_run()` implementation is trivially:

```rust
pub fn dry_run(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
    self.process_plate(parsed)?;
    Ok(())
}
```

### Filter Behavior in Dry Run

Template filters (String, Lines, Date, Dict, Content, Asset) execute during `compose()`. Some considerations:

- **Content filter**: Performs markdown→Typst conversion. This is lightweight and catches syntax issues.
- **Asset filter**: References dynamic assets. May fail if assets are missing, which is desirable for dry run.

All filter errors become `TemplateFailed` diagnostics with location information.

## Relations to Other Documents

- [ARCHITECTURE.md](ARCHITECTURE.md) - Overall pipeline and Workflow design
- [ERROR.md](ERROR.md) - Diagnostic and RenderError patterns
- [SCHEMAS.md](SCHEMAS.md) - Field validation schema specification
