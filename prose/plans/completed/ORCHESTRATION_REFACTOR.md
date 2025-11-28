# Orchestration Module Refactoring Plan

> **Status**: Design Phase
> **Design Reference**: `prose/designs/ARCHITECTURE.md`
> **Target**: Improve organization of `crates/quillmark/src/orchestration.rs` (851 lines)

---

## Objective

Refactor `orchestration.rs` into logical modules while keeping the public API unchanged and avoiding over-engineering.

---

## Current State

**File**: `crates/quillmark/src/orchestration.rs` (851 lines)

Contains three distinct concerns mixed in a single file:

1. **QuillRef enum** (lines 177-214)
   - Ergonomic reference type for quill lookup
   - `From` trait implementations for `&str`, `&String`, `Cow<str>`, `&Quill`, `&ParsedDocument`

2. **Quillmark struct** (lines 217-565)
   - Engine that orchestrates backends and quills
   - Backend registration and auto-registration
   - Quill registration with validation
   - Workflow factory method
   - Query methods (list backends, quills, get metadata)
   - Quill lifecycle (register, unregister)

3. **Workflow struct** (lines 567-851)
   - Sealed rendering pipeline
   - Render methods (full, processed, glue-only)
   - Document validation against schema
   - Dynamic asset management (add, clear, list)
   - Dynamic font management (add, clear, list)
   - Internal helper for preparing quill with assets

**Public API** (from `lib.rs`):
```rust
pub use orchestration::{QuillRef, Quillmark, Workflow};
```

---

## Desired State

Split `orchestration.rs` into a module directory with focused files while preserving the flat public API.

**Proposed structure:**
```
crates/quillmark/src/
├── lib.rs                  # Re-exports (unchanged)
└── orchestration/
    ├── mod.rs              # Module exports + QuillRef
    ├── engine.rs           # Quillmark struct
    └── workflow.rs         # Workflow struct
```

---

## Rationale

### Why This Split?

1. **Natural responsibility boundaries**
   - `Quillmark` = engine configuration and registry
   - `Workflow` = rendering execution pipeline
   - `QuillRef` = shared type used by both

2. **File size**
   - Current: 1 file × 851 lines
   - Proposed: 3 files × ~280 lines each

3. **Discoverability**
   - Contributors can find engine logic in `engine.rs`
   - Rendering logic isolated in `workflow.rs`

### Why Keep QuillRef in mod.rs?

- Small (38 lines)
- Used by both `Quillmark` and `Workflow`
- No benefit from separate file

### Why Not Split Further?

- Dynamic assets/fonts are integral to `Workflow` (used in `prepare_quill_with_assets`)
- Validation is part of rendering pipeline
- Quill registration validation is part of engine registration flow

---

## Implementation Steps

### Step 1: Create Module Directory

- Create `crates/quillmark/src/orchestration/` directory
- Create empty `mod.rs`, `engine.rs`, `workflow.rs`

### Step 2: Move QuillRef to mod.rs

Move lines 176-214 (including use statements needed for `QuillRef`):
- `QuillRef` enum
- All `From` implementations

Add module declarations and re-exports:
```rust
mod engine;
mod workflow;

pub use engine::Quillmark;
pub use workflow::Workflow;
```

### Step 3: Move Quillmark to engine.rs

Move lines 216-565:
- `Quillmark` struct
- `Quillmark` impl block (new, register_backend, register_quill, workflow, registered_backends, registered_quills, get_quill, get_quill_metadata, unregister_quill)
- `Default` impl

Add necessary imports at top of file.

### Step 4: Move Workflow to workflow.rs

Move lines 567-851:
- `Workflow` struct
- `Workflow` impl block (new, render, render_processed, render_processed_with_quill, process_glue, validate, validate_document, backend_id, supported_formats, quill_name, dynamic_asset_names, add_asset, add_assets, clear_assets, dynamic_font_names, add_font, add_fonts, clear_fonts, prepare_quill_with_assets)

Add necessary imports at top of file.

### Step 5: Update mod.rs Imports

Consolidate the shared imports needed by the module:
- `quillmark_core::*` types
- `std::collections::HashMap`
- `std::sync::Arc`

### Step 6: Delete Original File

Remove `crates/quillmark/src/orchestration.rs` (now replaced by directory).

### Step 7: Verify lib.rs Unchanged

Confirm `lib.rs` still compiles with same public API:
```rust
pub mod orchestration;
pub use orchestration::{QuillRef, Quillmark, Workflow};
```

---

## File Contents Summary

### orchestration/mod.rs (~60 lines)

- Module documentation (copy from top of original file)
- Module declarations (`mod engine; mod workflow;`)
- Re-exports (`pub use engine::Quillmark; pub use workflow::Workflow;`)
- `QuillRef` enum and `From` implementations
- Shared imports used by child modules

### orchestration/engine.rs (~330 lines)

- `Quillmark` struct definition
- `Quillmark::new()` with backend auto-registration
- `Quillmark::register_backend()`
- `Quillmark::register_quill()` with validation
- `Quillmark::workflow()` factory
- Query methods (registered_backends, registered_quills, get_quill, get_quill_metadata)
- `Quillmark::unregister_quill()`
- `Default` impl

### orchestration/workflow.rs (~420 lines)

- `Workflow` struct definition
- `Workflow::new()`
- Render methods (render, render_processed, render_processed_with_quill)
- `Workflow::process_glue()`
- Validation methods (validate, validate_document)
- Accessor methods (backend_id, supported_formats, quill_name)
- Dynamic asset methods (add_asset, add_assets, clear_assets, dynamic_asset_names)
- Dynamic font methods (add_font, add_fonts, clear_fonts, dynamic_font_names)
- `prepare_quill_with_assets()` internal helper

---

## What Stays Together

| Component | Location | Reason |
|-----------|----------|--------|
| QuillRef + From impls | mod.rs | Small, shared by both structs |
| Backend auto-registration | engine.rs | Part of engine initialization |
| Quill validation | engine.rs | Part of registration flow |
| Dynamic assets/fonts | workflow.rs | Used in render preparation |
| Schema validation | workflow.rs | Part of render pipeline |

---

## What Not To Do

- **Don't** split dynamic assets into separate module (too coupled to Workflow)
- **Don't** create separate validation module (only ~30 lines)
- **Don't** create separate types module for QuillRef (too small)
- **Don't** change public API or re-export structure
- **Don't** rename types or methods

---

## Verification Criteria

1. `cargo build -p quillmark` succeeds
2. `cargo test -p quillmark` passes all existing tests
3. Public API unchanged (same re-exports in lib.rs)
4. Documentation preserved (module docs, rustdoc comments)
5. No new dependencies

---

## Cross-References

- **Architecture**: `prose/designs/ARCHITECTURE.md` - Crate structure and responsibilities
- **Core module pattern**: `crates/core/src/` - Reference for module organization
- **Public API contract**: `crates/quillmark/src/lib.rs` - Re-exports to preserve
