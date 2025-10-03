# Error Handling

Structured error handling with diagnostics and source location tracking.

## Overview

The `error` module provides error types and diagnostic types for actionable
error reporting with source location tracking.

## Key Types

- **`RenderError`**: Main error enum for rendering operations
- **`TemplateError`**: Template-specific errors
- **`Diagnostic`**: Structured diagnostic information
- **`Location`**: Source file location (file, line, column)
- **`Severity`**: Error severity levels (Error, Warning, Note)
- **`RenderResult`**: Result type with artifacts and warnings

## Error Hierarchy

### RenderError Variants

- **`EngineCreation`**: Failed to create rendering engine
- **`InvalidFrontmatter`**: Malformed YAML frontmatter
- **`TemplateFailed`**: Template rendering error
- **`CompilationFailed`**: Backend compilation errors
- **`FormatNotSupported`**: Requested format not supported
- **`UnsupportedBackend`**: Backend not registered
- **`DynamicAssetCollision`**: Asset filename collision
- **`Internal`**: Internal error
- **`Other`**: Other errors
- **`Template`**: Template error

## Examples

### Error Handling

```rust,no_run
use quillmark_core::{RenderError, error::print_errors};
# use quillmark_core::RenderResult;
# struct Workflow;
# impl Workflow {
#     fn render(&self, _: &str, _: Option<()>) -> Result<RenderResult, RenderError> {
#         Ok(RenderResult::new(vec![]))
#     }
# }
# let workflow = Workflow;
# let markdown = "";

match workflow.render(markdown, None) {
    Ok(result) => {
        // Process artifacts
        for artifact in result.artifacts {
            std::fs::write(
                format!("output.{:?}", artifact.output_format),
                &artifact.bytes
            )?;
        }
    }
    Err(e) => {
        // Print structured diagnostics
        print_errors(&e);
        
        // Match specific error types
        match e {
            RenderError::CompilationFailed(count, diags) => {
                eprintln!("Compilation failed with {} errors:", count);
                for diag in diags {
                    eprintln!("{}", diag.fmt_pretty());
                }
            }
            RenderError::InvalidFrontmatter { diag, .. } => {
                eprintln!("Frontmatter error: {}", diag.message);
            }
            _ => eprintln!("Error: {}", e),
        }
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Creating Diagnostics

```rust
use quillmark_core::{Diagnostic, Location, Severity};

let diag = Diagnostic::new(Severity::Error, "Undefined variable".to_string())
    .with_code("E001".to_string())
    .with_location(Location {
        file: "template.typ".to_string(),
        line: 10,
        col: 5,
    })
    .with_hint("Check variable spelling".to_string());

println!("{}", diag.fmt_pretty());
```

Example output:
```text
[ERROR] Undefined variable (E001) at template.typ:10:5
  hint: Check variable spelling
```

### Result with Warnings

```rust,no_run
# use quillmark_core::{RenderResult, Diagnostic, Severity};
# let artifacts = vec![];
let result = RenderResult::new(artifacts)
    .with_warning(Diagnostic::new(
        Severity::Warning,
        "Deprecated field used".to_string(),
    ));
```

## Pretty Printing

The `Diagnostic` type provides `fmt_pretty()` for human-readable output with error code, location, and hints.

## Machine-Readable Output

All diagnostic types implement `serde::Serialize` for JSON export:

```rust,no_run
# use quillmark_core::{Diagnostic, Severity};
# let diagnostic = Diagnostic::new(Severity::Error, "Test".to_string());
let json = serde_json::to_string(&diagnostic).unwrap();
```
