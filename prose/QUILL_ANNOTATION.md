# Quill Annotation Revamp

## Phase 1: Core Refactoring & Cleanup

**Goal**: Simplify the orchestration logic and consolidate workflow creation to prepare for metadata changes.

- [ ] **Refactor Orchestration**: Split `crates/quillmark/orchestration.rs` into a cleaner file structure for better maintainability.
- [ ] **Consolidate Workflow Creation**:
    - Deprecate/Remove `workflow_from_quill_name` and `workflow_from_parsed`.
    - Standardize on a single entry point, e.g., `new_workflow()` (wrapping logic from `workflow_from_quill`).
- [ ] **Centralize Registry**:
    - Ensure WASM bindings do not maintain a duplicate map of registered Quills.
    - Expose and rely on the core engine's registry to avoid drift and memory overhead.

## Phase 2: Metadata Schema Strategy

**Goal**: Establish a single source of truth for Quill configuration that supports dynamic UI generation (Sections, Tooltips), while keeping validation robust.

- [ ] **Architecture Decision**: **DECIDED**: Keep `jsonschema` as internal implementation detail.
    - *Source of Truth*: QuillConfig TOML fields (Rust structs) are authoritative.
    - *Validation*: Generate/use `jsonschema` internally for validation logic, but do not expose it in the API for consumers.
- [ ] **Schema Definition**:
    - Add `Section` field to Quill definition.
    - Add `Tooltip` field to Quill definition.
    - Ensure these fields are serializable/accessible for the API.
    - *Note*: Do not store/expose the raw `jsonschema` in the `Quill` struct unless needed for internal caching.
- [ ] **Validation**:
    - Ensure validation logic uses the internal `jsonschema` derived from the authoritative TOML fields.

## Phase 3: WASM API & UI Integration

**Goal**: Expose rich metadata to WASM consumers to enable dynamic wizard UIs.

- [ ] **Update Bindings**:
    - Modify `QuillInfo` in `crates/bindings/wasm/src/types.rs` to include new metadata fields (`section`, `tooltip`, etc.).
- [ ] **Expose Retrieval API**:
    - Create a function in `quillmark` crate to retrieve `Quill` details from bindings.
    - Expose a WASM function to retrieve annotations/metadata for a given Quill.
- [ ] **Verify Integration Flow**:
    - `Parse Markdown` -> `Extract Quill Tag` -> `Retrieve Quill Info` -> `Update Wizard UI`.