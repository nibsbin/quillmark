# Simplification Cascades for Quillmark

**Analysis Date**: 2025-11-14
**Scope**: Core libraries (quillmark-core, quillmark), backends (typst, acroform)
**Audience**: Software engineers implementing refactoring tasks

## Overview

This document identifies 5 major simplification opportunities where **one insight eliminates multiple components**. Each cascade follows the pattern: "If this is true, we don't need X, Y, or Z."

**Estimated impact**: ~370 lines eliminated, 6 duplicate implementations removed, significant maintainability improvements.

---

## Cascade 1: Unify Glue Engine Implementations

**Location**: `quillmark-core/src/templating.rs:135-363`

### Problem

Two separate glue engine implementations (`TemplateGlue` and `AutoGlue`) exist with:
- Identical method signatures for `GlueEngine` trait
- Duplicate filter registration logic
- Pattern matching dispatch in the `Glue` enum
- AutoGlue stores filters but never uses them (see line 231-234 comment)

### Core Insight

> **The difference is output mode, not engine type**

Both implementations differ ONLY in output:
- TemplateGlue: renders MiniJinja template → string
- AutoGlue: serializes context as JSON → string

Everything else (filter registration, state management, interface) is identical.

### Implementation Strategy

1. Replace two structs + trait with single `Glue` struct containing `OutputMode` enum
2. OutputMode variants: `Template(String)` for template rendering, `Auto` for JSON serialization
3. Single `compose()` method matches on output mode
4. Unified filter registration (no more duplicate implementations)

### What to Eliminate

- ❌ `TemplateGlue` struct (82 lines)
- ❌ `AutoGlue` struct (56 lines)
- ❌ `GlueEngine` trait
- ❌ Pattern matching dispatch wrapper
- ❌ Duplicate `register_filter()` implementations

**Lines saved**: ~150
**Difficulty**: Medium
**Priority**: High

**Future benefit**: New output modes become single enum variants instead of full implementations.

---

## Cascade 2: Generalize Dynamic Collection Management

**Location**: `quillmark/src/orchestration.rs:630-756`

### Problem

Dynamic assets and fonts managed with 100% identical logic:
- `add_asset/add_assets/clear_assets/dynamic_asset_names` (46 lines)
- `add_font/add_fonts/clear_fonts/dynamic_font_names` (46 lines)
- Identical collision detection
- Two separate loops in `prepare_quill_with_assets` with only prefix differences

**Only differences**: field names, prefix strings (`DYNAMIC_ASSET__` vs `DYNAMIC_FONT__`), error types.

### Core Insight

> **All dynamic collections are named byte buffers with collision detection and prefixed injection**

This is a container pattern problem—the logic is identical.

### Implementation Strategy

1. Create `DynamicCollection` struct with: items map, prefix string, collision error constructor
2. Implement generic methods: `add()`, `add_many()`, `names()`, `clear()`, `inject_into_quill()`
3. Replace separate `dynamic_assets` and `dynamic_fonts` fields with `DynamicCollection` instances
4. Consolidate `prepare_quill_with_assets` to call `inject_into_quill()` on each collection

### What to Eliminate

- ❌ 46 lines of asset management methods
- ❌ 46 lines of font management methods
- ❌ Duplicate collision checking logic
- ❌ Two separate loops in preparation phase
- ❌ (Optional) Separate error types—unify to `DynamicItemCollision`

**Lines saved**: ~92
**Difficulty**: Low
**Priority**: High

**Future benefit**: Adding dynamic packages, templates, or any named resource is trivial.

---

## Cascade 3: Extract Shared Backend Infrastructure

**Locations**:
- `backends/quillmark-typst/src/lib.rs:108-118`
- `backends/quillmark-acroform/src/lib.rs:48-57`

### Problem

Both backends have identical code for:
- Format validation (checking supported formats)
- Diagnostic construction for unsupported formats
- Artifact creation and wrapping
- Error patterns (`RenderError::FormatNotSupported`, etc.)

### Core Insight

> **Format validation and artifact wrapping are backend-agnostic infrastructure**

Every backend performs: validate format → compile (backend-specific) → wrap in artifact.
Only the middle step is backend-specific.

### Implementation Strategy

1. Add default `compile()` implementation to `Backend` trait
2. Default implementation handles format validation and artifact wrapping
3. Create new `compile_bytes()` method for backend-specific logic
4. Add optional `default_format()` method (defaults to `OutputFormat::Pdf`)
5. Backends override `compile_bytes()` instead of `compile()`

### What to Eliminate

- ❌ 11 lines format validation in Typst backend
- ❌ 11 lines format validation in Acroform backend
- ❌ 4-5 lines artifact creation boilerplate per backend
- ❌ Risk of future backends forgetting validation

**Lines saved**: ~30 currently, plus ~15 per future backend
**Difficulty**: Low
**Priority**: High

**Future benefit**: Every new backend automatically gets format validation and artifact wrapping. Eliminates "forgot to validate" bugs.

---

## Cascade 4: Extract Common Schema Property Pattern

**Location**: `quillmark-core/src/schema.rs:105-165`

### Problem

Three functions with identical structure extracting different fields:
- `extract_defaults_from_schema` (21 lines)
- `extract_examples_from_schema` (27 lines)
- Similar logic in `build_schema_from_fields`

**Common pattern**: Get schema properties → iterate → extract sub-field → build map.

### Core Insight

> **Schema property extraction is always: get properties → iterate → extract field → build map**

Only differences: which sub-field to extract and how to transform the value.

### Implementation Strategy

1. Create generic `extract_property_field<T, F>()` function
2. Takes schema, field name, and extractor closure
3. Returns `HashMap<String, T>` where `T` is determined by extractor
4. Rewrite existing functions as thin wrappers calling generic version

### What to Eliminate

- ❌ ~20 lines duplicate iteration in `extract_defaults_from_schema`
- ❌ ~25 lines duplicate iteration in `extract_examples_from_schema`
- ❌ Copy-paste bug risks
- ❌ Inconsistent handling variations

**Lines saved**: ~40
**Difficulty**: Low
**Priority**: Medium

**Future benefit**: Extracting new schema metadata (descriptions, constraints) becomes 3-line closures.

---

## Cascade 5: Unify Collision Detection

**Locations**:
- `quillmark/src/orchestration.rs:644-656` (dynamic assets)
- `quillmark/src/orchestration.rs:695-707` (dynamic fonts)
- `quillmark/src/orchestration.rs:294-303` (quill names)

### Problem

Identical collision checking pattern repeated 3+ times:
- Check if key exists in HashMap
- Build Diagnostic with severity, message, code, hint
- Wrap in appropriate RenderError variant

### Core Insight

> **All name collision checks follow: check map → build diagnostic → return error**

Pattern is always the same; only item type name and error variant differ.

### Implementation Strategy

1. Create `check_collision()` helper function
2. Parameters: map reference, key, item type string, error builder closure
3. Handles diagnostic construction with consistent formatting
4. Replace all collision checks with calls to helper

**Note**: Partially addressed by Cascade 2, but pattern extends beyond assets/fonts to any named collection.

### What to Eliminate

- ❌ ~10 lines per collision site × 3 sites = 30 lines
- ❌ Inconsistent diagnostic messages
- ❌ Risk of forgetting collision checks
- ❌ Duplicate diagnostic construction

**Lines saved**: ~30
**Difficulty**: Low
**Priority**: Low (largely covered by Cascade 2)

**Future benefit**: New registries automatically get consistent collision handling.

---

## Implementation Roadmap

### Phase 1: Quick Wins (1-2 hours)
**Priority: High**

1. **Cascade 2** (Dynamic Collections)
   - Easiest implementation
   - Highest line savings (92 lines)
   - Clear container pattern

2. **Cascade 4** (Schema Extraction)
   - Small, clean abstraction
   - Low risk
   - Immediate clarity improvement

### Phase 2: Infrastructure (2-4 hours)
**Priority: High**

3. **Cascade 3** (Backend Infrastructure)
   - Benefits all current backends
   - Prevents bugs in future backends
   - Enforces consistency

4. **Cascade 1** (Glue Unification)
   - Larger refactor
   - High impact on architecture clarity
   - Removes conceptual complexity

### Phase 3: Polish (optional)
**Priority: Low**

5. **Cascade 5** (Collision Detection)
   - Nice-to-have
   - Lower ROI (partially covered by Cascade 2)
   - Consider only if time permits

---

## Key Principle

The unifying insight across all cascades:

> **When implementing the same pattern with different names/types/messages, the difference is usually configuration, not algorithm**

When you encounter:
- ✅ Correct algorithm
- ✅ Well-structured code
- ❌ Duplicated pattern with minor variations

Recognize that **variation = configuration**, then:
1. Extract the common pattern
2. Parameterize the differences
3. Eliminate the duplicates

This is the essence of simplification cascades: **one abstraction eliminates many implementations**.

---

## Summary Table

| Cascade | Impact | Lines Saved | Difficulty | Priority |
|---------|--------|-------------|------------|----------|
| 1. Glue Unification | 2 structs, 1 trait, dispatch | ~150 | Medium | High |
| 2. Dynamic Collections | 8 methods, 2 loops, 2 errors | ~92 | Low | High |
| 3. Backend Infrastructure | Validation × N backends | ~30 + 15/backend | Low | High |
| 4. Schema Extraction | 2 duplicate functions | ~40 | Low | Medium |
| 5. Collision Detection | 3+ duplicate checks | ~30 | Low | Low* |

*Low priority due to overlap with Cascade 2

**Total**: ~370 lines eliminated, 6 major duplicate implementations removed
