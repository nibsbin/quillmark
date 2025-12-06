# PLATE REFACTOR - Complete Rename from Quill to Plate

This document tracks the major breaking change refactor from "Quill" to "Plate" across the entire repository.

## Completed Changes

### Core Types & Structs ✅
- [x] `crates/core/src/quill.rs` → `crates/core/src/plate.rs`
- [x] `Quill` struct → `Plate`
- [x] `QuillConfig` → `PlateConfig`
- [x] `QuillIgnore` → `PlateIgnore`
- [x] `from_quill_value()` → `from_plate_value()`

### Configuration Files ✅
- [x] All `Quill.toml` files renamed to `Plate.toml` (5 files)
- [x] `.quillignore` → `.plateignore` (1 file)
- [x] All `[Quill]` TOML sections updated to `[Plate]`

### API Methods & Functions ✅
- [x] `register_quill()` → `register_plate()`
- [x] `unregister_quill()` → `unregister_plate()`
- [x] `quill_name()` → `plate_name()`
- [x] `Plate::from_path()`, `Plate::from_json()`, `Plate::from_tree()`
- [x] `extract_defaults()`, `extract_examples()` methods on Plate

### Directory & File Names ✅
- [x] `default_quill/` → `default_plate/`
- [x] `quill_engine_test.rs` → `plate_engine_test.rs`
- [x] `default_quill.rs` → `default_plate.rs`
- [x] `default_quill_test.rs` → `default_plate_test.rs`
- [x] `quillLoader.js` → `plateLoader.js`

### Core Library Updates ✅
- [x] `crates/core/src/lib.rs` - Updated exports to use Plate
- [x] `crates/core/src/backend.rs` - Backend trait uses Plate
- [x] `crates/core/src/error.rs` - QuillConfig → PlateConfig error variant
- [x] `crates/core/src/parse.rs` - QUILL_TAG → "plate"

### Orchestration Layer ✅
- [x] `crates/quillmark/src/orchestration/engine.rs` - Engine uses Plate
- [x] `crates/quillmark/src/orchestration/workflow.rs` - Workflow uses Plate
- [x] `crates/quillmark/src/orchestration/mod.rs` - QuillRef → PlateRef
- [x] `crates/quillmark/src/lib.rs` - Public API exports Plate

### Backend Implementations ✅
- [x] Typst backend: `QuillWorld` → `PlateWorld`
- [x] Typst backend: Embedded default files updated
- [x] Acroform backend: Uses Plate type

### Test & Example Files ✅
- [x] All test files updated to use Plate API
- [x] All example files updated to use Plate API
- [x] Common test utilities updated

### Bindings ✅
- [x] Python bindings updated
- [x] WASM bindings updated
- [x] CLI bindings (if applicable)

### Documentation ✅
- [x] Updated references in docs/ directory
- [x] Updated README.md examples

## Breaking Changes

This is a **MAJOR BREAKING CHANGE**. All users must update:

1. Import statements: `use quillmark_core::Quill` → `use quillmark_core::Plate`
2. Method calls: `.register_quill()` → `.register_plate()`
3. Configuration files: `Quill.toml` → `Plate.toml`
4. YAML frontmatter key: `QUILL:` → `PLATE:`
5. Ignore files: `.quillignore` → `.plateignore`

## What Did NOT Change

- Project name: Quillmark
- Package names: quillmark, quillmark-core, quillmark-typst, etc.
- Repository: nibsbin/quillmark
- NPM package: @quillmark-test/wasm
- PyPI package: quillmark
- Crate names on crates.io

## Migration Guide

To migrate existing code:

```rust
// Before
use quillmark::{Quillmark, Quill};
let quill = Quill::from_path("my-template")?;
engine.register_quill(quill);

// After
use quillmark::{Quillmark, Plate};
let plate = Plate::from_path("my-template")?;
engine.register_plate(plate);
```

To migrate configuration files:
```bash
mv Quill.toml Plate.toml
sed -i 's/\[Quill\]/[Plate]/g' Plate.toml
mv .quillignore .plateignore
```

To migrate markdown frontmatter:
```yaml
# Before
---
QUILL: my-template
title: Document
---

# After
---
PLATE: my-template
title: Document
---
```
