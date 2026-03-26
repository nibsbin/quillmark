# Maintainability Recommendations

## Scope Reviewed

This review focused on architectural maintainability in core orchestration and parsing paths:

- `crates/quillmark/src/orchestration/engine.rs`
- `crates/quillmark/src/orchestration/workflow.rs`
- `crates/core/src/backend.rs`
- `crates/core/src/parse.rs`
- `crates/core/src/quill.rs`

## Key Findings

### 1) Library-level stderr side effects

`register_backend` and `parse_fields_with_order` write warnings using `eprintln!` from core library code.

- `engine.register_backend` logs directly when default quill registration fails.
- `QuillConfig::parse_fields_with_order` logs parse failures and continues.

**Maintainability risk:** callers cannot control reporting behavior, tests become noisy, and diagnostics cannot be consistently aggregated across CLI/bindings.

**Recommendation:** return structured non-fatal diagnostics (or use a logger interface injected at boundary layers) and keep direct console output in binaries only.

**Proposed implementation plan:**

1. **Introduce a warnings collector type in core**
   - Add a small `NonFatalDiagnostic` (or `WarningEvent`) struct that carries a stable code, human-readable message, and optional location/context.
   - Add a `WarningSink` abstraction (trait or callback) with a no-op default so library callers are not forced to handle warnings.

2. **Refactor `QuillConfig::parse_fields_with_order` to emit warnings instead of printing**
   - Replace `eprintln!` branches with pushes to the warning collector.
   - Return parsed fields plus collected warnings, or accept a mutable sink parameter.
   - Keep existing tolerant behavior (continue on recoverable field parse failures) so runtime behavior remains backward compatible.

3. **Refactor `engine.register_backend` default-quill failure handling**
   - Remove direct `eprintln!` usage and surface a warning through the same sink/collector pipeline.
   - Ensure backend registration still succeeds when failure is non-fatal, but warning metadata preserves backend id and failure reason.

4. **Add boundary-layer wiring in CLI/bindings**
   - In CLI entrypoints, map emitted warnings to current stderr formatting so user-visible behavior is preserved.
   - In library/bindings contexts, expose warnings to callers as structured data rather than forcing console output.

5. **Add targeted tests and migration guardrails**
   - Unit tests: verify warning emission content for parse failures and default-quill registration failures.
   - Integration tests: verify no stderr output occurs from core library paths under normal execution.
   - Add a changelog note documenting the new warning channel and recommended migration pattern for downstream callers.

6. **Follow-up cleanup**
   - Audit other `eprintln!` usages in `crates/core` and `crates/quillmark` orchestration paths for similar conversion.
   - Keep binary-specific stderr output in `crates/bindings/cli` only, establishing a clear boundary rule.

---

### 2) Duplicated workflow pipeline preparation

`Workflow::compile` and `Workflow::render_with_options` duplicate core preparation steps:

- `compile_data(parsed)`
- `get_plate_content`
- `prepare_quill_with_assets`

**Maintainability risk:** future pipeline changes may drift between code paths, causing inconsistent behavior.

**Recommendation:** extract a shared internal preparation method (e.g., `prepare_render_context`) returning `{ json_data, plate_content, prepared_quill }`.

---

### 3) Backend trait mixes multiple optional capabilities

The `Backend` trait includes baseline compile plus optional capabilities (`compile_to_document`, `render_pages`, `default_quill`, `transform_fields`) with fallback "unsupported" implementations.

**Maintainability risk:** capability discovery is runtime-error-driven; trait contract is broad and harder to reason about as features expand.

**Recommendation:** split into focused capability traits and compose where needed, for example:

- `BackendCompile`
- `BackendPagedRender`
- `BackendDefaultQuill`
- `BackendFieldTransform`

This can keep baseline implementations minimal and make capability support explicit at type level.

---

### 4) `quill.rs` is a multi-responsibility module

`crates/core/src/quill.rs` currently combines:

- schema model types
- file tree abstraction and mutation
- ignore-pattern logic
- config parsing and metadata extraction
- quill domain model

**Maintainability risk:** unrelated changes collide in one file, increasing cognitive load and merge conflict frequency.

**Recommendation:** split into cohesive modules such as:

- `schema_types.rs`
- `file_tree.rs`
- `quill_ignore.rs`
- `quill_config.rs`
- `quill_model.rs`

Keep re-exports in `quill.rs` to preserve public API stability.

---

### 5) Frontmatter parsing relies on complex manual scanning

`parse.rs` implements manual delimiter/fence/HR scanning with position arithmetic.

**Maintainability risk:** subtle edge cases are difficult to validate and safely evolve.

**Recommendation:** isolate this into a dedicated parser state machine with explicit states and transitions, then back it with scenario fixtures and property tests for line endings/fence edge cases.

## Suggested Prioritization

1. Remove library stderr side effects.
2. Consolidate workflow preparation pipeline.
3. Introduce backend capability trait decomposition.
4. Decompose `quill.rs` into focused modules.
5. Refactor frontmatter scanning to explicit parser state machine.

## Expected Benefits

- More predictable diagnostics and cleaner library boundaries.
- Lower regression risk when pipeline behavior changes.
- Clearer backend contracts and easier extension.
- Reduced coupling and improved code navigation.
- More robust parsing behavior under edge cases.
