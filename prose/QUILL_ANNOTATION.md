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

**Goal**: Establish a single source of truth for Quill configuration that supports dynamic UI generation (Sections, Tooltips), while keeping validation robust.

**Detailed Plan**: [`prose/plans/QUILL_ANNOTATION_PHASE_2.md`](plans/QUILL_ANNOTATION_PHASE_2.md)

- [ ] **Architecture Decision**: **DECIDED**: Keep `jsonschema` as internal implementation detail.
    - *Source of Truth*: QuillConfig TOML fields (Rust structs) are authoritative.
    - *Validation*: Generate/use `jsonschema` internally for validation logic, but do not expose it in the API for consumers.
- [ ] **Schema Definition**:
    - Add `section: Option<String>` field to `FieldSchema` struct (field-level UI grouping).
    - Add `tooltip: Option<String>` field to `FieldSchema` struct (short help text).
    - Extend JSON Schema generation to include `x-section` and `x-tooltip` custom properties.
    - Ensure fields are serializable and accessible via WASM `QuillInfo.schema`.
- [ ] **Validation**:
    - Ensure validation logic uses the internal `jsonschema` derived from TOML fields.
    - Verify validators ignore custom `x-*` properties (forward compatible).

## Phase 3: WASM API & UI Integration

**Goal**: Expose rich metadata to WASM consumers to enable dynamic wizard UIs.

- [ ] **Update Bindings**:
    - Modify `QuillInfo` in `crates/bindings/wasm/src/types.rs` to include new metadata fields (`section`, `tooltip`, etc.).
- [ ] **Expose Retrieval API**:
    - Create a function in `quillmark` crate to retrieve `Quill` details from bindings.
    - Expose a WASM function to retrieve annotations/metadata for a given Quill.
- [ ] **Verify Integration Flow**:
    - `Parse Markdown` -> `Extract Quill Tag` -> `Retrieve Quill Info` -> `Update Wizard UI`.