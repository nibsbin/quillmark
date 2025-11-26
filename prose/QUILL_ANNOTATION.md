# Quill Annotation Revamp

## Phase 1: Core Refactoring & Cleanup

**Goal**: Simplify the orchestration logic and consolidate workflow creation to prepare for metadata changes.

- [ ] **Refactor Orchestration**: Split `crates/quillmark/orchestration.rs` into a cleaner file structure for better maintainability.
- [ ] **Consolidate Workflow Creation**:
    - Deprecate/Remove `workflow` and `workflow`.
    - Standardize on a single entry point, e.g., `new_workflow()` (wrapping logic from `workflow_from_quill`).
- [ ] **Centralize Registry**:
    - Ensure WASM bindings do not maintain a duplicate map of registered Quills.
    - Expose and rely on the core engine's registry to avoid drift and memory overhead.

## Phase 2: Metadata Schema Strategy

**Goal**: Establish JSON Schema as the single source of truth for Quill field metadata that supports dynamic UI generation (Sections, Tooltips).

**Detailed Plan**: [`prose/plans/QUILL_ANNOTATION_PHASE_2.md`](plans/QUILL_ANNOTATION_PHASE_2.md)

- [ ] **Architecture Decision**: **DECIDED**: JSON Schema is the authoritative source.
    - *Input Format*: TOML `[fields]` parsed to `FieldSchema` (ephemeral, discarded after construction).
    - *Transformation*: `build_schema_from_fields()` converts FieldSchema to JSON Schema with custom `x-*` properties.
    - *Single Source of Truth*: JSON Schema stored in `Quill.schema` is used for validation, caching, and API exposure.
- [ ] **Schema Definition**:
    - Add `section: Option<String>` field to `FieldSchema` struct (TOML input format).
    - Add `tooltip: Option<String>` field to `FieldSchema` struct (TOML input format).
    - Extend `build_schema_from_fields()` to include `x-section` and `x-tooltip` in generated JSON Schema.
    - WASM `QuillInfo.schema` automatically exposes these custom properties (no API changes).
- [ ] **Validation**:
    - Validation uses `jsonschema::Validator` on the authoritative `Quill.schema`.
    - Verify validators ignore custom `x-*` properties per JSON Schema spec (forward compatible).

## Phase 3: WASM API & UI Integration

**Goal**: Expose rich metadata to WASM consumers to enable dynamic wizard UIs.

- [ ] **Update Bindings**:
    - Modify `QuillInfo` in `crates/bindings/wasm/src/types.rs` to include new metadata fields (`section`, `tooltip`, etc.).
- [ ] **Expose Retrieval API**:
    - Create a function in `quillmark` crate to retrieve `Quill` details from bindings.
    - Expose a WASM function to retrieve annotations/metadata for a given Quill.
- [ ] **Verify Integration Flow**:
    - `Parse Markdown` -> `Extract Quill Tag` -> `Retrieve Quill Info` -> `Update Wizard UI`.