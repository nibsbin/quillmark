# Quill Annotation Revamp - Phase 3 Plan

**Status**: Planning Phase
**Goal**: Expose rich metadata to WASM consumers to enable dynamic wizard UIs and verify the integration flow.
**Related**: [`prose/QUILL_ANNOTATION.md`](../QUILL_ANNOTATION.md), [`prose/plans/completed/QUILL_ANNOTATION_PHASE_2.md`](completed/QUILL_ANNOTATION_PHASE_2.md)

---

## Overview

Phase 3 focuses on verifying that the metadata enhancements from Phase 2 (specifically the `x-ui` schema extensions) are correctly propagated through the WASM bindings and available to consumers. It also includes updating documentation and adding integration tests.

---

## Current State Analysis

- **Phase 2 Implementation**: The core engine (`crates/core`) has been updated to parse `ui` tables from `Quill.toml` and inject them into the JSON Schema as `x-ui` properties.
- **WASM Bindings**: The `QuillInfo` struct in `crates/bindings/wasm` already exposes the `schema` field as a raw JSON object.
- **Gap**: There is no explicit verification that `x-ui` properties are preserved and accessible via the WASM API. Documentation for the new `ui` configuration in `Quill.toml` is missing.

---

## Desired State

1.  **Verified Flow**: A test case confirms that `Quill.toml` -> `Quill` -> `QuillInfo` (WASM) preserves `x-ui` metadata.
2.  **Documentation**: The `Creating Quills` guide documents the `[fields.name.ui]` configuration table.
3.  **Example**: A fixture or example demonstrates the usage of UI metadata.

---

## Implementation Plan

### 1. Integration Testing

Create a new test case in `crates/bindings/wasm/tests/` (or update an existing one) to verify metadata retrieval.

- **Action**: Create `crates/bindings/wasm/tests/metadata.rs` (or similar integration test).
- **Test Logic**:
    1.  Define a Quill JSON with `ui` metadata in `Quill.toml` (simulated or loaded).
    2.  Register the Quill via `Quillmark.registerQuill`.
    3.  Call `Quillmark.getQuillInfo`.
    4.  Assert that `info.schema.properties.field_name.x-ui` exists and contains expected values.

### 2. Documentation Update

Update `docs/guides/creating-quills.md` to include the `ui` configuration.

- **Action**: Add a section "UI Configuration" to `docs/guides/creating-quills.md`.
- **Content**: Explain `group`, `tooltip`, and `extra` properties in `[fields.name.ui]`.

### 3. Verification

Run the new test to ensure the flow works as expected.

---

## Verification Steps

1.  Run `cargo test -p quillmark-wasm` (or relevant test command).
2.  Verify that the new test passes.
3.  Check `docs/guides/creating-quills.md` for correctness.
