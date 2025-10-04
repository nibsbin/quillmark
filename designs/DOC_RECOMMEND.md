# Documentation Pattern Recommendations

## Executive Summary

The current hybrid documentation system using `#[doc = include_str!()]` for module-level docs creates unnecessary complexity and maintenance burden. This document proposes **three alternative approaches** ranging from conventional inline documentation to improved hybrid patterns, with recommendations based on project priorities.

**Quick Recommendation**: If minimizing Rust file lines is critical for AI agent processing, adopt **Approach 2: Consolidated External Documentation** which reduces per-module overhead while maintaining external docs.

---

## Current System Analysis

### Current Architecture

```
crate-name/
├── docs/
│   ├── lib.md                    # Crate-level overview (40-100 lines)
│   ├── {module}.md               # Per-module docs (80-130 lines each)
│   └── designs/
│       ├── API.md                # Comprehensive API reference (1400+ lines)
│       └── {DESIGN}.md           # Design specifications (800-1000 lines)
└── src/
    ├── lib.rs                     # #![doc = include_str!("../docs/lib.md")]
    └── {module}.rs                # #![doc = include_str!("../docs/{module}.md")]
```

**Documentation Distribution** (quillmark-core example):
- Module docs: 430 lines across 5 files (lib.md, backend.md, parse.md, templating.md, errors.md)
- Design docs: 2,387 lines across 2 files (API.md, PARSE.md)
- Inline summaries: ~100 lines in Rust files
- **Total**: ~2,917 lines of documentation

### Pain Points

1. **Maintenance Overhead**
   - Each module requires 2 files: `module.rs` + `docs/module.md`
   - Documentation spread across 3 locations: inline, module docs, design docs
   - Significant duplication between module docs and comprehensive API docs

2. **Cognitive Load**
   - Developers must maintain `#[doc = include_str!()]` directives
   - Module-qualified paths required for intra-doc links (e.g., `` [`parse::decompose()`] ``)
   - Context switching between Rust files and markdown files

3. **File Proliferation**
   - 13 markdown files for 3 small crates
   - Additional design documents create 4+ files per crate
   - Each public module adds 1-2 new files

4. **Intra-Doc Link Complexity**
   - Same markdown used in different contexts (lib.rs vs module.rs)
   - Requires module-qualified paths everywhere
   - Easy to create broken links

5. **IDE/Editor Experience**
   - Jumping to definition shows include_str!() instead of actual docs
   - Hover information requires reading separate files
   - Reduced inline discoverability

### Current Benefits

1. **Keeps Rust files compact** - Important for AI agent token budgets
2. **External markdown is easier to edit** - Better editing tools, preview
3. **Comprehensive docs possible** - Design docs can be very detailed
4. **Doctests work** - Markdown code blocks are still testable

---

## Proposed Approaches

### Approach 1: Conventional Inline Documentation (Standard Rust)

**Strategy**: Use standard Rust doc comments (`///` and `//!`) directly in source files. Eliminate separate markdown files except for top-level design documents.

#### Structure

```
crate-name/
├── designs/                       # Project-wide design docs (unchanged)
│   └── {DESIGN}.md
└── src/
    ├── lib.rs                     # //! Crate docs (50-100 lines)
    └── {module}.rs                # //! Module docs (80-150 lines)
```

#### Example

```rust
// quillmark-core/src/backend.rs

//! Backend trait for implementing output format backends.
//!
//! # Overview
//!
//! The [`Backend`] trait defines the interface that backends must implement
//! to support different output formats (PDF, SVG, TXT, etc.).
//!
//! # Implementation Guide
//!
//! Implement all five required methods:
//!
//! - [`Backend::id()`] - Return a unique backend identifier
//! - [`Backend::supported_formats()`] - Return supported output formats
//! - [`Backend::glue_type()`] - Return glue file extension
//! - [`Backend::register_filters()`] - Register backend-specific filters
//! - [`Backend::compile()`] - Compile glue content into artifacts
//!
//! # Examples
//!
//! ```rust
//! use quillmark_core::{Backend, OutputFormat};
//!
//! struct MyBackend;
//!
//! impl Backend for MyBackend {
//!     fn id(&self) -> &'static str {
//!         "my-backend"
//!     }
//!     
//!     fn supported_formats(&self) -> &'static [OutputFormat] {
//!         &[OutputFormat::Pdf]
//!     }
//!     
//!     // ... other methods
//! }
//! ```
//!
//! # Thread Safety
//!
//! The [`Backend`] trait requires `Send + Sync` to enable concurrent rendering.
//!
//! # See Also
//!
//! - [quillmark-typst](../quillmark_typst) for a complete implementation
//! - [DESIGN.md](../../designs/DESIGN.md) for architecture details

use crate::error::RenderError;
use crate::templating::Glue;
use crate::{Artifact, OutputFormat, Quill, RenderOptions};

/// Backend trait for rendering different output formats
pub trait Backend: Send + Sync {
    /// Get the backend identifier (e.g., "typst", "latex")
    fn id(&self) -> &'static str;

    /// Get supported output formats
    fn supported_formats(&self) -> &'static [OutputFormat];
    
    // ... rest of trait
}
```

#### Advantages

✅ **Standard Rust convention** - Familiar to all Rust developers  
✅ **IDE integration** - Full IntelliJ/rust-analyzer support for hover, navigation  
✅ **Single source of truth** - Docs live with code  
✅ **Simple intra-doc links** - Use standard Rust paths (e.g., `[Backend::id()]`)  
✅ **No file proliferation** - No separate markdown files to maintain  
✅ **Better discoverability** - Docs visible in IDE and on hover  
✅ **Standard tooling** - Works with all Rust documentation tools  

#### Disadvantages

❌ **Larger Rust files** - Violates AI agent line-count requirement (adds 80-150 lines per module)  
❌ **Harder to edit docs** - No markdown preview in editors  
❌ **Limited formatting** - Rust doc comments less flexible than standalone markdown  
❌ **Cluttered code** - Long doc blocks can obscure implementation  

#### Impact Assessment

- **Rust file size increase**: ~80-150 lines per module (~400-600 lines total for quillmark-core)
- **AI agent token impact**: Moderate increase (~15-20% more tokens per file)
- **Migration effort**: High (rewrite all module docs inline)

#### Recommendation for This Approach

**Use when**: 
- AI agent processing is NOT the primary concern
- Standard Rust conventions are highly valued
- Team is comfortable with inline docs

**Do NOT use when**:
- Minimizing Rust file lines is a hard requirement (per problem statement)

---

### Approach 2: Consolidated External Documentation (Recommended)

**Strategy**: Consolidate module-level docs into a single `API.md` or eliminate them entirely. Keep only crate-level `lib.md` externally. Use minimal inline summaries (1-2 lines) as before.

#### Structure

```
crate-name/
├── docs/
│   ├── lib.md                    # Crate overview only (~50 lines)
│   └── designs/
│       ├── API.md                # Comprehensive API reference (all public APIs)
│       └── {DESIGN}.md           # Design specifications
└── src/
    ├── lib.rs                     # #![doc = include_str!("../docs/lib.md")]
    └── {module}.rs                # /// One-liner summary only
```

#### Example

```rust
// quillmark-core/src/lib.rs
#![doc = include_str!("../docs/lib.md")]

pub mod parse;
pub mod templating;
pub mod backend;
pub mod error;
```

```rust
// quillmark-core/src/backend.rs

// NO module-level #![doc = include_str!()]

use crate::error::RenderError;
use crate::templating::Glue;
use crate::{Artifact, OutputFormat, Quill, RenderOptions};

/// Backend trait for rendering different output formats. See [API.md](../docs/designs/API.md#backend-trait) for detailed documentation.
pub trait Backend: Send + Sync {
    /// Get the backend identifier (e.g., "typst", "latex")
    fn id(&self) -> &'static str;

    /// Get supported output formats for this backend
    fn supported_formats(&self) -> &'static [OutputFormat];
    
    /// Get the glue file extension (e.g., ".typ", ".tex")
    fn glue_type(&self) -> &'static str;
    
    // ... rest of trait
}
```

```markdown
<!-- docs/lib.md -->
# Quillmark Core Overview

Core types and functionality for the Quillmark template-first Markdown rendering system.

## Quick Start

```rust,no_run
use quillmark_core::{decompose, Quill};

let markdown = "---\ntitle: Example\n---\n\n# Content";
let doc = decompose(markdown).unwrap();
```

## Modules

- **`parse`** - Markdown parsing with YAML frontmatter support
- **`templating`** - Template composition using MiniJinja
- **`backend`** - Backend trait for output format implementations
- **`error`** - Structured error handling and diagnostics

## Documentation

- [API.md](designs/API.md) - Comprehensive API reference
- [PARSE.md](designs/PARSE.md) - Parsing documentation
- [DESIGN.md](../../designs/DESIGN.md) - Architecture

<!-- No module-level details here, all in API.md -->
```

```markdown
<!-- docs/designs/API.md -->
# Quillmark Core API Documentation

## Backend Trait

The `Backend` trait defines the interface for output format backends.

### Overview

Backends implement five required methods to support rendering:

1. **`id()`** - Unique identifier for the backend
2. **`supported_formats()`** - Array of supported OutputFormat variants
3. **`glue_type()`** - File extension for glue templates
4. **`register_filters()`** - Register backend-specific MiniJinja filters
5. **`compile()`** - Compile glued content to final artifacts

### Trait Definition

```rust
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn glue_type(&self) -> &'static str;
    fn register_filters(&self, glue: &mut Glue);
    fn compile(
        &self,
        glue_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<Vec<Artifact>, RenderError>;
}
```

### Implementation Guide

[... detailed examples and documentation ...]

---

## Parse Module

[... continue with all other APIs ...]
```

#### Advantages

✅ **Minimal Rust file lines** - Satisfies AI agent requirement (only 1-2 line summaries)  
✅ **Single comprehensive reference** - All details in one API.md file  
✅ **Easier maintenance** - Update one file instead of many  
✅ **No per-module file overhead** - Eliminate 5+ module markdown files  
✅ **Simpler intra-doc links** - Reference API.md sections consistently  
✅ **IDE hover still works** - One-liner summaries provide context  
✅ **Reduced file count** - ~50% fewer documentation files  

#### Disadvantages

❌ **Large API.md files** - Single file can be 1000+ lines (but easier to navigate than many files)  
❌ **Less modular docs** - Can't incrementally view just one module's docs  
❌ **External links required** - Module rustdoc points to external markdown  
❌ **No module-level rustdoc** - Each module page is minimal  

#### Impact Assessment

- **Rust file size change**: Negligible (only 1-2 line summaries)
- **AI agent token impact**: Minimal (actually reduces tokens per file)
- **Migration effort**: Low-Medium (consolidate existing docs into API.md, remove module md files)
- **File count reduction**: ~40% fewer doc files

#### Implementation Path

1. **Phase 1**: Merge all module-level docs into `docs/designs/API.md`
2. **Phase 2**: Remove individual module markdown files (`backend.md`, `parse.md`, etc.)
3. **Phase 3**: Remove `#![doc = include_str!()]` from module files
4. **Phase 4**: Update inline summaries to link to API.md sections
5. **Phase 5**: Update CONTRIBUTING.md with new pattern

**Migration Script** (conceptual):
```bash
# For each crate
for crate in quillmark-core quillmark-typst quillmark; do
  # Create or append to API.md
  cat $crate/docs/backend.md >> $crate/docs/designs/API.md
  cat $crate/docs/parse.md >> $crate/docs/designs/API.md
  # ... repeat for all modules
  
  # Remove module docs
  rm $crate/docs/backend.md
  rm $crate/docs/parse.md
  # ... etc
  
  # Update src files to remove #![doc = include_str!()]
  # (manual editing required)
done
```

#### Recommendation for This Approach

**Use when**: 
- ✅ Minimizing Rust file lines is critical (MATCHES REQUIREMENT)
- ✅ Team prefers comprehensive reference docs over modular docs
- ✅ Want to reduce file proliferation

**This is the RECOMMENDED approach** based on the problem statement requirement to minimize Rust file lines.

---

### Approach 3: Enhanced Hybrid with Auto-Generation

**Strategy**: Keep external markdown but generate module doc stubs automatically. Use tooling to reduce manual maintenance.

#### Structure

```
crate-name/
├── docs/
│   ├── lib.md                    # Hand-written crate overview
│   ├── modules/                  # Auto-generated module summaries
│   │   └── {module}.md           # Generated from inline /// docs
│   └── designs/                  # Hand-written comprehensive docs
│       └── API.md
├── src/
│   ├── lib.rs                     # #![doc = include_str!("../docs/lib.md")]
│   └── {module}.rs                # /// Inline docs + generation marker
└── xtask/
    └── gen-docs.rs                # Tool to generate module.md files
```

#### Example

```rust
// quillmark-core/src/backend.rs

/// Backend trait for rendering different output formats
///
/// See [comprehensive guide](../docs/designs/API.md#backend-trait)
pub trait Backend: Send + Sync {
    /// Get the backend identifier (e.g., "typst", "latex")
    fn id(&self) -> &'static str;
    // ...
}
```

```bash
# Generate module docs from inline summaries
cargo xtask gen-docs

# This creates docs/modules/backend.md:
```

```markdown
<!-- Auto-generated from inline docs - DO NOT EDIT -->

# Backend Module

Backend trait for rendering different output formats.

See [comprehensive guide](designs/API.md#backend-trait)

## API

- `trait Backend` - Backend trait for rendering different output formats
  - `fn id(&self) -> &'static str` - Get the backend identifier
  - ...
```

#### Advantages

✅ **Reduced manual maintenance** - Auto-generation from inline docs  
✅ **Moderate Rust file size** - Brief inline docs (smaller than full docs)  
✅ **DRY principle** - Single source generates multiple outputs  
✅ **Consistency** - Generated docs always match code  

#### Disadvantages

❌ **Tooling complexity** - Requires custom xtask or build script  
❌ **Build step required** - Must run generator before committing  
❌ **Still file proliferation** - Generated files still clutter repo  
❌ **Unclear value** - Complexity may not justify benefits  
❌ **Maintenance burden** - Custom tooling needs maintenance  

#### Impact Assessment

- **Rust file size increase**: Minor (~20-40 lines per module for inline docs)
- **AI agent token impact**: Small increase
- **Migration effort**: High (build tooling + rewrite docs)
- **Ongoing cost**: Medium (maintain generator)

#### Recommendation for This Approach

**Do NOT recommend** - Complexity outweighs benefits. Adds tooling burden without solving core issues.

---

## Comparison Matrix

| Criterion | Approach 1: Inline | Approach 2: Consolidated | Approach 3: Auto-Gen |
|-----------|-------------------|-------------------------|---------------------|
| **Rust file lines** | ❌ +400-600 lines | ✅ Minimal (+0-50) | ⚠️ +100-200 lines |
| **AI agent friendly** | ❌ No | ✅ Yes | ⚠️ Moderate |
| **IDE integration** | ✅ Excellent | ⚠️ Basic hover only | ⚠️ Basic hover only |
| **Maintenance burden** | ✅ Low (single source) | ✅ Low (fewer files) | ❌ High (tooling) |
| **File count** | ✅ Low | ✅ Very Low | ⚠️ Medium |
| **Migration effort** | ❌ High | ✅ Low-Medium | ❌ High |
| **Rust conventions** | ✅ Standard | ❌ Non-standard | ❌ Non-standard |
| **Intra-doc links** | ✅ Simple | ⚠️ External links | ⚠️ Mixed |
| **Comprehensive docs** | ⚠️ Harder to maintain | ✅ Natural fit | ⚠️ Complex |
| **Editing experience** | ⚠️ No markdown preview | ✅ Markdown editors | ⚠️ Mixed |

---

## Recommendations

### Primary Recommendation: Approach 2 (Consolidated External)

**Adopt Approach 2: Consolidated External Documentation** for the following reasons:

1. ✅ **Meets hard requirement** - Minimizes Rust file line count (critical for AI agents per problem statement)
2. ✅ **Reduces complexity** - Eliminates per-module markdown files and include_str!() directives
3. ✅ **Lower maintenance** - Single API.md per crate vs 5+ module files
4. ✅ **Easier migration** - Low-to-medium effort to consolidate existing docs
5. ✅ **Familiar pattern** - Many projects use comprehensive API reference docs

#### Implementation Checklist

- [ ] Create/expand `docs/designs/API.md` in each crate with comprehensive API documentation
- [ ] Keep `docs/lib.md` for crate-level overview (minimal ~50 lines)
- [ ] Remove per-module markdown files (`backend.md`, `parse.md`, etc.)
- [ ] Remove `#![doc = include_str!()]` from module files
- [ ] Update inline `///` summaries to reference API.md sections: `/// {summary}. See [API.md](../docs/designs/API.md#{section})`
- [ ] Update CONTRIBUTING.md documentation standards
- [ ] Run `cargo doc --no-deps` to verify no broken links
- [ ] Update CI to check API.md for broken links

#### Updated Documentation Standard

```markdown
## Documentation Structure (Proposed)

All Quillmark crates use consolidated external documentation:

### Inline Documentation (Minimal)

- **Public items**: 1-2 line summary using `///` with link to API.md section
- **Example**: `/// Parse markdown with frontmatter. See [API.md](../docs/designs/API.md#parsing)`

### External Documentation

- **`docs/lib.md`** - Crate-level overview only (~50 lines)
  - Quick start example
  - Module list with one-line descriptions
  - Links to comprehensive docs
- **`docs/designs/API.md`** - Complete API reference
  - All public types, traits, and functions
  - Usage examples and patterns
  - Error handling guidance
- **`docs/designs/{DESIGN}.md`** - Design specifications and rationale

### File Structure

```
crate-name/
├── docs/
│   ├── lib.md                    # Crate overview (~50 lines)
│   └── designs/
│       ├── API.md                # Complete API reference
│       └── {DESIGN}.md           # Design docs
└── src/
    ├── lib.rs                     # #![doc = include_str!("../docs/lib.md")]
    └── {module}.rs                # /// One-line summaries only
```
```

### Fallback Recommendation: Approach 1 (Inline)

**If minimizing Rust file lines is NOT actually critical**, adopt standard inline Rust documentation:

- Simpler for Rust developers
- Better IDE integration
- Industry standard pattern
- Accept the token cost for AI agents

This should only be chosen if the problem statement's requirement to minimize Rust file lines is reconsidered.

---

## Alternative: Hybrid Compromise

If neither extreme is acceptable, consider a **minimal hybrid**:

1. **Keep** crate-level `lib.md` (included in `lib.rs`)
2. **Keep** comprehensive `API.md` in `docs/designs/`
3. **Add** moderate inline module docs (20-40 lines) with `//!` in module files
4. **Remove** separate per-module markdown files

This provides:
- Modest inline context for common operations
- Comprehensive external reference for deep dives
- Lower file count than current system
- Moderate Rust file size increase (~20-40 lines per module)

**Structure**:
```
crate-name/
├── docs/
│   ├── lib.md                    # Crate overview
│   └── designs/
│       └── API.md                # Comprehensive reference
└── src/
    ├── lib.rs                     # #![doc = include_str!("../docs/lib.md")]
    └── {module}.rs                # //! Brief module doc (20-40 lines)
                                   # /// Item summaries (1-2 lines)
```

---

## Migration Guide (Approach 2)

### Step 1: Consolidate Documentation

For each crate (quillmark-core, quillmark-typst, quillmark):

```bash
cd quillmark-core

# Create or expand API.md
cat docs/backend.md >> docs/designs/API.md
cat docs/parse.md >> docs/designs/API.md
cat docs/templating.md >> docs/designs/API.md
cat docs/errors.md >> docs/designs/API.md

# Add section headers and reorganize in your editor
# Ensure proper markdown hierarchy
```

### Step 2: Update Source Files

Remove module-level doc includes:

```rust
// Before: quillmark-core/src/backend.rs
#![doc = include_str!("../docs/backend.md")]

use crate::error::RenderError;
// ...

// After: quillmark-core/src/backend.rs
use crate::error::RenderError;

/// Backend trait for rendering different output formats. See [API reference](../docs/designs/API.md#backend-trait).
pub trait Backend: Send + Sync {
    /// Get the backend identifier (e.g., "typst", "latex")
    fn id(&self) -> &'static str;
    // ...
}
```

### Step 3: Update lib.md

Simplify crate-level docs to overview only:

```markdown
# Quillmark Core Overview

Core types and functionality for template-first Markdown rendering.

## Quick Start

```rust,no_run
use quillmark_core::{decompose, Quill};
let doc = decompose("---\ntitle: Test\n---\nContent").unwrap();
```

## Modules

- **`parse`** - Markdown parsing with YAML frontmatter
- **`templating`** - Template composition with MiniJinja
- **`backend`** - Backend trait for output formats
- **`error`** - Structured error handling

## Documentation

- [API.md](designs/API.md) - Complete API reference
- [PARSE.md](designs/PARSE.md) - Parsing specification
- [DESIGN.md](../../designs/DESIGN.md) - Architecture
```

### Step 4: Remove Old Files

```bash
rm docs/backend.md
rm docs/parse.md
rm docs/templating.md
rm docs/errors.md
```

### Step 5: Update CONTRIBUTING.md

Replace current documentation section with new standard (see above).

### Step 6: Verify

```bash
cargo doc --no-deps --workspace
cargo test --doc --workspace
```

---

## Conclusion

**Recommendation**: Adopt **Approach 2: Consolidated External Documentation**

This approach:
1. ✅ Satisfies the hard requirement to minimize Rust file line count for AI agent processing
2. ✅ Reduces maintenance burden by consolidating scattered module docs
3. ✅ Maintains comprehensive documentation in accessible markdown format
4. ✅ Requires low-to-medium migration effort
5. ✅ Results in cleaner codebase with fewer files

The current hybrid system is the "worst of both worlds" - it has file proliferation without inline IDE benefits. Consolidating to a single API.md per crate provides a better balance of external documentation with minimal code bloat.

If the line count requirement is reconsidered, standard inline Rust documentation (Approach 1) would be preferable for IDE integration and conventional practices.
