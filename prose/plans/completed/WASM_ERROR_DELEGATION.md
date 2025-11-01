# WASM Error Delegation Implementation Plan

> **Goal**: Remove custom `QuillmarkError` wrapper from WASM bindings and delegate error handling to core crates using `SerializableDiagnostic`
>
> **Related Designs**: [ERROR.md](../designs/ERROR.md), [WASM.md](../designs/WASM.md)

---

## Current State

The WASM bindings currently use a custom error type:

**Location**: `bindings/quillmark-wasm/src/error.rs`

```rust
pub struct QuillmarkError {
    pub message: String,
    pub location: Option<Location>,
    pub hint: Option<String>,
    pub diagnostics: Option<Vec<Diagnostic>>,
}
```

**Problems:**
1. Duplicates error structure from core
2. Custom `From<RenderError>` implementation must handle all RenderError variants
3. Maintenance burden when core error types change
4. Inconsistent with Python bindings approach (which delegates to core)
5. Uses custom Location and Diagnostic types instead of core types

---

## Desired State

WASM bindings should use core types directly:

**Use**: `quillmark_core::SerializableDiagnostic` and related core types
**Remove**: `bindings/quillmark-wasm/src/error.rs` custom error wrapper
**Update**: Error handling in `bindings/quillmark-wasm/src/engine.rs` to use core types

---

## Implementation Steps

### Step 1: Update Types Module

**File**: `bindings/quillmark-wasm/src/types.rs`

- **Keep** the `Diagnostic` type conversion from `quillmark_core::Diagnostic` to WASM-compatible `Diagnostic` (already exists)
- **Keep** the `Location` type conversion (already exists)
- **Verify** that `Diagnostic` includes `source_chain` field (already present in types.rs line 85)
- **Note**: These types are used for RenderResult warnings and error serialization

### Step 2: Create Error Conversion Utilities

**File**: `bindings/quillmark-wasm/src/error.rs` (rewrite, don't remove)

Replace custom `QuillmarkError` with error conversion utilities:

```rust
//! Error handling utilities for WASM bindings

use quillmark_core::{RenderError, SerializableDiagnostic};
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;

/// Serializable error for JavaScript consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum WasmError {
    /// Single diagnostic error
    Diagnostic {
        #[serde(flatten)]
        diagnostic: SerializableDiagnostic,
    },
    /// Multiple diagnostics (e.g., compilation errors)
    MultipleDiagnostics {
        message: String,
        diagnostics: Vec<SerializableDiagnostic>,
    },
}

impl WasmError {
    /// Convert to JsValue for throwing
    pub fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(self)
            .unwrap_or_else(|_| JsValue::from_str(&format!("{:?}", self)))
    }
}

impl From<RenderError> for WasmError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::CompilationFailed { diags } => {
                WasmError::MultipleDiagnostics {
                    message: format!("Compilation failed with {} error(s)", diags.len()),
                    diagnostics: diags.into_iter().map(|d| d.into()).collect(),
                }
            }
            // All other variants contain a single Diagnostic
            _ => {
                let diags = error.diagnostics();
                if let Some(diag) = diags.first() {
                    WasmError::Diagnostic {
                        diagnostic: (*diag).into(),
                    }
                } else {
                    // Fallback for edge cases
                    WasmError::Diagnostic {
                        diagnostic: SerializableDiagnostic {
                            severity: quillmark_core::Severity::Error,
                            code: None,
                            message: error.to_string(),
                            primary: None,
                            hint: None,
                            source_chain: vec![],
                        },
                    }
                }
            }
        }
    }
}

impl From<String> for WasmError {
    fn from(message: String) -> Self {
        WasmError::Diagnostic {
            diagnostic: SerializableDiagnostic {
                severity: quillmark_core::Severity::Error,
                code: None,
                message,
                primary: None,
                hint: None,
                source_chain: vec![],
            },
        }
    }
}

impl From<&str> for WasmError {
    fn from(message: &str) -> Self {
        WasmError::from(message.to_string())
    }
}
```

**Rationale:**
- Uses core `SerializableDiagnostic` directly
- Handles both single and multiple diagnostic cases
- Simple conversion from `RenderError`
- Minimal wrapper, delegates structure to core

### Step 3: Update Engine Error Handling

**File**: `bindings/quillmark-wasm/src/engine.rs`

Replace all instances of:
```rust
QuillmarkError::new(...).to_js_value()
```

With:
```rust
WasmError::from(...).to_js_value()
```

And replace:
```rust
.map_err(|e| QuillmarkError::from(e).to_js_value())
```

With:
```rust
.map_err(|e| WasmError::from(e).to_js_value())
```

**Example transformations:**

Before:
```rust
let parsed = quillmark_core::ParsedDocument::from_markdown(markdown).map_err(|e| {
    QuillmarkError::new(
        format!("Failed to parse markdown: {}", e),
        None,
        Some("Check markdown syntax and YAML frontmatter".to_string()),
    )
    .to_js_value()
})?;
```

After:
```rust
let parsed = quillmark_core::ParsedDocument::from_markdown(markdown).map_err(|e| {
    WasmError::from(format!("Failed to parse markdown: {}", e)).to_js_value()
})?;
```

Note: For errors that currently provide hints, we may need to add hint support to the core error types, or include hints in the error message. For WASM bindings, simple messages are acceptable as the first step.

### Step 4: Update Module Exports

**File**: `bindings/quillmark-wasm/src/lib.rs`

Update the error export:
```rust
pub use error::WasmError;
```

Remove documentation about `QuillmarkError`, update to reflect `WasmError` and delegation to core types.

### Step 5: Update Tests

**Files**: 
- `bindings/quillmark-wasm/src/types.rs` (tests section)
- `bindings/quillmark-wasm/tests/*.rs`

Update tests to:
- Expect `WasmError` structure instead of `QuillmarkError`
- Verify that `SerializableDiagnostic` is properly serialized
- Check both single diagnostic and multiple diagnostics cases

### Step 6: Validate TypeScript Types

**File**: `bindings/quillmark-wasm/package.json` and TypeScript declaration files (if any)

Ensure TypeScript types reflect the new error structure:
```typescript
type WasmError = 
  | { type: 'Diagnostic', severity: string, message: string, code?: string, location?: Location, hint?: string, sourceChain?: string[] }
  | { type: 'MultipleDiagnostics', message: string, diagnostics: Diagnostic[] };
```

---

## Migration Notes

### Breaking Changes

**For JavaScript/TypeScript consumers:**
1. Error structure changes from flat `QuillmarkError` to tagged union `WasmError`
2. Error JSON now includes a `type` discriminator field
3. Single errors nested under `diagnostic` field
4. Multiple errors have `diagnostics` array instead of optional `diagnostics` field

**Migration path:**
```typescript
// Before
try {
  render();
} catch (error) {
  console.error(error.message);
  if (error.diagnostics) {
    error.diagnostics.forEach(...);
  }
}

// After
try {
  render();
} catch (error) {
  if (error.type === 'Diagnostic') {
    console.error(error.message);
  } else if (error.type === 'MultipleDiagnostics') {
    console.error(error.message);
    error.diagnostics.forEach(...);
  }
}
```

### Non-Breaking Enhancements

- `sourceChain` now available in all diagnostics
- More consistent error structure with Python bindings
- Easier to extend with new diagnostic fields from core

---

## Testing Strategy

1. **Unit tests**: Verify `From<RenderError>` conversions for all variants
2. **Integration tests**: Test error serialization to JsValue
3. **Manual testing**: Build WASM package and test in browser/Node.js
4. **TypeScript tests**: Update existing tests to use new error structure

---

## Success Criteria

- [ ] WASM bindings use `SerializableDiagnostic` from core
- [ ] No custom error structure duplication
- [ ] All `RenderError` variants properly converted
- [ ] Tests pass for error handling
- [ ] JavaScript error handling examples work
- [ ] TypeScript types (if any) are correct
- [ ] Consistent with Python bindings error delegation approach

---

## Future Enhancements

1. Add hint support to `SerializableDiagnostic` in core if needed
2. Consider error code standardization across bindings
3. Add error recovery suggestions in diagnostic messages
4. Generate TypeScript types from Rust types automatically
