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

**Goal**: Establish JSON Schema as the single source of truth for Quill field metadata, including UI layout via `x-ui` properties.

**Detailed Plan**: [`prose/plans/QUILL_ANNOTATION_PHASE_2.md`](plans/QUILL_ANNOTATION_PHASE_2.md)

- [ ] **Architecture Decision**: **DECIDED**: Embedded `x-ui` object in JSON Schema.
    - *Input Format*: TOML `[fields.name.ui]` table.
    - *Transformation*: `build_schema_from_fields()` injects `x-ui` object into schema properties.
    - *Output*: `QuillInfo.schema` contains all metadata.
- [ ] **Schema Definition**:
    - Add `ui: Option<UiSchema>` to `FieldSchema`.
    - Implement `UiSchema` struct with `group`, `component`, `order`, etc.
    - Update `build_schema_from_fields()` to serialize `ui` to `x-ui`.
- [ ] **Validation**:
    - Validation uses `jsonschema::Validator` on the schema (ignores `x-ui`).

## Phase 3: WASM API & UI Integration

**Goal**: Expose rich metadata to WASM consumers to enable dynamic wizard UIs.

**Detailed Plan**: [`prose/plans/QUILL_ANNOTATION_PHASE_3.md`](plans/QUILL_ANNOTATION_PHASE_3.md)

- [ ] **Update Bindings**:
    - No changes required to `QuillInfo` structure (schema already exposed).
- [ ] **Expose Retrieval API**:
    - Create a function in `quillmark` crate to retrieve `Quill` details from bindings.
    - Expose a WASM function to retrieve annotations/metadata for a given Quill.
- [ ] **Verify Integration Flow**:
    - `Parse Markdown` -> `Extract Quill Tag` -> `Retrieve Quill Info`