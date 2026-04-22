# Split `Quill` into `QuillSource` (core) and `Quill` (quillmark)

**Audience:** Quillmark core, engine, backend, and bindings maintainers
**Affects:** `quillmark-core`, `crates/quillmark`, `crates/backends/typst`, `crates/bindings/{cli,python,wasm}`

## Goal

Collapse the two parallel render pipelines into one. Rename the existing core
`Quill` to `QuillSource` (pure file-bundle + config + metadata, no rendering).
Rename today's `Workflow` to `Quill` and relocate it to `quillmark`, where it
composes `Arc<QuillSource>` with a resolved backend and exposes the single
rendering entry point.

## Motivation

Two public render pipelines exist today doing substantially the same work:

| | Core pipeline | Orchestration pipeline |
|---|---|---|
| Entry | `Quill::render` / `Quill::open` — `crates/core/src/quill/render.rs:28,38` | `Workflow::render` / `Workflow::open` — `crates/quillmark/src/orchestration/workflow.rs:114,123` |
| Data prep | `compile_data_internal` — `render.rs:87` | `compile_data` — `workflow.rs:31` |
| Backend call | `backend.open(plate, self, json)` — `render.rs:41` | `backend.open(plate, &self.quill, json)` — `workflow.rs:161` |

The two data-prep paths perform the same coerce → validate → normalize →
apply-defaults sequence. Core's `Quill` carries
`pub(crate) resolved_backend: Option<Arc<dyn Backend>>`
(`crates/core/src/quill.rs:36`) solely to support the core pipeline, along
with `with_backend` / `backend()` accessors (`render.rs:14,20`) and a
`NoBackend` error variant (`crates/core/src/error.rs:509`) whose message
tells callers to use a different API — `"use engine.quill() or
engine.quill_from_path() instead"` (`render.rs:53`).

The core `Quill` also carries backend-coupled methods that leak Typst
specifics into a "backend-agnostic" crate:
`typst_packages()` (`crates/core/src/quill/query.rs:11`) and
`build_transform_schema()` (`crates/core/src/quill/render.rs:203`).

Dynamic asset/font injection was purged in commit `4dab4be4`, so `Quill` no
longer has any legitimate reason to hold per-render mutable state. The
remaining duplication is pure historical accident.

## Design

### Type split

**`QuillSource`** (in `quillmark-core`) — pure data:

```rust
pub struct QuillSource {
    config: QuillConfig,
    files: FileTree,                        // whatever today's storage is
    metadata: HashMap<String, QuillValue>,
}

impl QuillSource {
    pub(crate) fn from_tree(tree: FileTreeNode) -> Result<Self, ...>;
    pub fn config(&self) -> &QuillConfig;
    pub fn metadata(&self) -> &HashMap<String, QuillValue>;
    pub fn find_files(&self, ...) -> ...;
    pub fn get_file(&self, ...) -> ...;
    pub fn list_directories(&self, ...) -> ...;
    // Gone: resolved_backend, with_backend, backend, render, open,
    //       compile_data_internal, typst_packages, build_transform_schema
}
```

**`Quill`** (in `quillmark`) — renderable, immutable, engine-constructed:

```rust
pub struct Quill {
    source: Arc<QuillSource>,
    backend: Arc<dyn Backend>,
}

impl Quill {
    pub fn source(&self) -> &QuillSource;
    pub fn backend_id(&self) -> &str;
    pub fn render(&self, doc: Document, opts: &RenderOptions)
        -> Result<RenderResult, RenderError>;
    pub fn open(&self, doc: Document) -> Result<RenderSession, RenderError>;
    // Construction is engine-only; no public constructor.
}
```

**`Workflow` is deleted.** Its entire responsibility moves to `Quill`; there
is no wrapper indirection between "a thing you loaded" and "a thing you
render with."

### `Backend` trait

`Backend::open` takes `&QuillSource` directly — no `BackendContext` wrapper,
no `&Quill`:

```rust
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &[OutputFormat];
    fn open(&self, plate: &str, source: &QuillSource, json: &Value)
        -> Result<RenderSession, RenderError>;
}
```

`QuillSource` is already the narrow, read-only shape backends need. Nothing
else needs to be introduced.

### Binding surface

Bindings expose `Quill` only; `QuillSource` does not cross the FFI boundary.
The Rust type split is an internal concern. In JS and Python there is one
type: `Quill`, constructed through the engine, with delegating methods for
inspection (e.g. `quill.config()`, `quill.projectForm()`, `quill.backendId()`).
No `QuillSource` constructor is exported.

Rationale: two-step construction buys nothing in languages that cannot hold
a sourceless bundle usefully, and each wasm-bindgen'd type adds glue and
bundle size. If a future use case demands a headless `QuillSource` on the
binding side, it can be added without breaking the single-type story.

### Naming

- `QuillSource` (the user chose this; `QuillBundle` and `QuillModel` were
  considered and rejected).
- `Quill` is the renderable type and the binding-visible type.
- `from_path` lives in `quillmark` (orchestration), not core — filesystem I/O
  is not a core concern.

### Invariants

- `Quill` is immutable post-construction. No `add_asset`, no `add_font`, no
  mutable setters. Dynamic assets are out of scope for this tasking (purged
  upstream in `4dab4be4`); any reintroduction is a separate design.
- `RenderSession::warning: Option<Diagnostic>` at `crates/core/src/session.rs:12`
  stays as-is. `Quill::open` in quillmark attaches the ref-mismatch warning
  the same way core's `Quill::open` does today. Return-type changes are
  deferred.
- Core does not depend on `std::fs`. `QuillSource::from_tree` accepts an
  already-in-memory `FileTreeNode`.

## Tasks

### 1. Rename core `Quill` → `QuillSource`

**Files:** `crates/core/src/quill.rs`, `crates/core/src/quill/*.rs`, `crates/core/src/lib.rs`

Rename the struct and every reference inside `quillmark-core`. Preserve
`pub use` re-exports but rename them. Module path
`quillmark_core::quill::Quill` becomes `quillmark_core::quill::QuillSource`.

Do not yet touch callers outside core; they break in task 5 and get migrated
there.

### 2. Strip render/backend state from `QuillSource`

**File:** `crates/core/src/quill/render.rs`, `crates/core/src/quill.rs`, `crates/core/src/quill/load.rs`, `crates/core/src/error.rs`

Delete from `QuillSource`:

- Field `resolved_backend: Option<Arc<dyn Backend>>` (`quill.rs:36`)
- `with_backend` (`render.rs:14`)
- `backend()` accessor (`render.rs:20`)
- `render` (`render.rs:28`)
- `open` (`render.rs:38`)
- `compile_data_internal` (`render.rs:87`)
- `require_backend` helper (`render.rs:47`)
- Initializer `resolved_backend: None` (`load.rs:194`)

Delete `RenderError::NoBackend` (`error.rs:509`) and its handling at
`error.rs:526`. Remove the doc reference at `error.rs:30`.

After this task, `crates/core/src/quill/render.rs` contains only
`build_transform_schema` and its helpers — handled in task 4.

### 3. Move `typst_packages` out of core

**From:** `crates/core/src/quill/query.rs:11`
**To:** `crates/backends/typst/src/lib.rs` (or a new `metadata.rs` in the
typst backend)

Replace the method with a free function or backend-private helper that reads
`source.metadata().get("typst_packages")` and returns `Vec<String>`. Update
the call site at `crates/backends/typst/src/world.rs:239`.

Delete `crates/core/src/quill/query.rs` if `typst_packages` is its sole
contents.

### 4. Move `build_transform_schema` out of core

**From:** `crates/core/src/quill/render.rs:203`
**To:** `crates/backends/typst/src/` (new module, or extend `convert.rs`)

The method converts `FieldSchema` → JSON Schema with
`contentMediaType: "text/markdown"` tags. The only consumer is the Typst
backend's field-transform logic (`crates/backends/typst/src/lib.rs:120`).
Move the function there. It takes `&FieldSchema` (or `&QuillSource`) and
returns `QuillValue`.

If a second backend later needs the same logic, promote it to a helper in
`quillmark` — not back into core.

### 5. Introduce `Quill` in `quillmark`; delete `Workflow`

**Files:** `crates/quillmark/src/orchestration/workflow.rs` (delete),
`crates/quillmark/src/orchestration/mod.rs`, `crates/quillmark/src/lib.rs`

Create a new file `crates/quillmark/src/orchestration/quill.rs` containing
the `Quill` struct defined in the Design section. Port the following from
`workflow.rs`:

- `compile_data` (line 31) → method on `Quill`
- `render` (line 114) → method on `Quill`
- `open` (line 123) → method on `Quill`
- `render_with_options` (line 136) → method on `Quill` (or inline into `render`)
- `prepare_render_context` (line 152) → private method on `Quill`
- `render_plate_with_quill_and_data` (line 161) → private method on `Quill`

The ref-mismatch warning logic currently inside `Workflow::open` attaches to
the returned `RenderSession` the same way.

Delete `workflow.rs` and the `Workflow` re-export from `lib.rs`.
Remove `Quillmark::workflow` (`crates/quillmark/src/orchestration/engine.rs:99`).

### 6. Engine constructs `Quill` directly

**File:** `crates/quillmark/src/orchestration/engine.rs`

`engine.quill(tree)` (line 46) currently returns `quillmark_core::Quill` with
backend attached. After the rename, it should:

1. Call `QuillSource::from_tree(tree)` (now `pub(crate)` in core — expose via
   a `pub(crate)` helper if the engine crate cannot reach it, or promote
   `QuillSource::from_tree` to `pub` and rely on the engine as the documented
   construction path).
2. Resolve the backend from the source's declared backend id.
3. Return `Quill { source: Arc::new(source), backend }`.

`attach_backend` (line 73) is rewritten to assemble a `Quill` instead of
mutating a core `Quill`.

Add `engine.quill_from_path(path)` (already exists at line 60) — no signature
change, but its body now uses `QuillSource::from_tree` internally.

### 7. Migrate `Backend` trait to `&QuillSource`

**File:** `crates/core/src/backend.rs:15`

Change the trait signature:

```rust
fn open(
    &self,
    plate: &str,
    source: &QuillSource,      // was: &Quill
    json: &Value,
) -> Result<RenderSession, RenderError>;
```

Update every implementor — currently only `TypstBackend` in
`crates/backends/typst/src/lib.rs`. All existing call sites in
`crates/backends/typst/src/world.rs` (`find_files`, `get_file`,
`list_directories`, `metadata`) already take the shape that `QuillSource`
provides; the rename is mechanical.

### 8. Binding: WASM exposes `Quill` only

**File:** `crates/bindings/wasm/src/engine.rs`

Change the held type from `quillmark_core::Quill` (line 35) to
`quillmark::Quill`. Do **not** expose `QuillSource` at the wasm-bindgen
boundary.

Add delegating methods as needed on the WASM `Quill`:

- `config()` → delegates to `self.inner.source().config()`
- `projectForm(document)` → already exists at line 145; ensure it still works
  against `source`
- `backendId()` → delegates to `self.inner.backend_id()`
- `render`, `open`, `quillRef` — keep existing

Do not add speculative accessors. Add surface only when a concrete
consumer needs it.

Delete any TypeScript declarations or tests that reference `QuillSource` or
the old core-backed `Quill` shape — there should be none today, this is a
guard against regressions.

### 9. Binding: Python exposes `Quill` only; delete `Workflow`

**File:** `crates/bindings/python/src/types.rs`, `crates/bindings/python/src/lib.rs`

Delete `PyWorkflow` (line 58) and `Quillmark.workflow` (line 40).

`PyQuill` (line 120) changes its inner type from `quillmark::Quill` (the
old core-re-exported one) to the new `quillmark::Quill`. Port the methods
that lived on `PyWorkflow` onto `PyQuill`: `render`, `open` (if separately
exposed), anything else the Python surface documents.

The Python `Quill` becomes the single user-facing type. No `QuillSource` is
added to `pyo3` bindings. Update `crates/bindings/python/src/types.rs:9-10`
imports accordingly.

Python test files in `crates/bindings/python/tests/` need mechanical updates
to construct `engine.quill_from_path(path)` and call `.render()` directly
rather than `engine.workflow(quill).render()`.

### 10. Binding: CLI updates

**File:** `crates/bindings/cli/src/validate.rs:4`

CLI currently imports `quillmark_core::quill::{CardSchema, FieldSchema,
FieldType, QuillConfig}`. Under the rename, `quillmark_core::quill` still
exists as a module path; the imports should keep working unchanged. Verify.

If CLI commands construct a `Workflow`, migrate to `engine.quill_from_path()`
+ `.render()` directly (pattern in `prose/taskings/consolidate-quill-creation.md`
§4 still applies).

### 11. `from_path` orchestration

**File:** move `from_path` logic out of `crates/core/src/quill/load.rs:12`

Filesystem walking + tree construction moves to `quillmark`. Possible home:
`crates/quillmark/src/orchestration/engine.rs` as a free helper called by
`Quillmark::quill_from_path`. Core keeps `QuillSource::from_tree` only.

Update `Cargo.toml` for `quillmark-core` if this allows dropping any `std::fs`
feature gate.

### 12. Documentation

| Doc | Change |
|-----|--------|
| `prose/designs/QUILL.md` | Rename `Quill` → `QuillSource`; document the type split and the binding-surface decision. |
| `prose/designs/ARCHITECTURE.md` (if present) | Update layering diagram: core owns `QuillSource`, quillmark owns `Quill`, bindings see only `Quill`. |
| `docs/integration/overview.md` | User-facing: `engine.quill(tree)` returns `Quill` — unchanged from today's documented surface. No mention of `QuillSource`. |
| `crates/bindings/wasm/README.md` | Remove any stale `QuillSource` or `Workflow` references. |
| `crates/bindings/python/README.md` | Remove `Workflow` section; fold methods into `Quill`. |
| `MIGRATION.md` | Add entry: `Workflow` type is removed; `Quill.render()` is the replacement. `QuillSource` is internal. |
| `prose/taskings/render-session.md` | Cross-reference: `RenderSession::warning` intentionally unchanged here. |
| `prose/taskings/consolidate-quill-creation.md` | Mark partially superseded; the `engine.quill(tree)` API stays, but it now returns `quillmark::Quill` not `quillmark_core::Quill`. |

---

## Verification Checklist

- [ ] `cargo build --workspace` compiles clean
- [ ] `cargo test --workspace` passes
- [ ] `rg 'Workflow' crates/` returns zero hits
- [ ] `rg 'resolved_backend|with_backend|NoBackend' crates/` returns zero hits
- [ ] `rg 'compile_data_internal' crates/` returns zero hits
- [ ] `rg 'QuillSource' crates/bindings/` returns zero hits (not exposed)
- [ ] `rg 'typst_packages|build_transform_schema' crates/core/` returns zero hits
- [ ] `Backend::open` signature takes `&QuillSource` (grep `crates/backends/`)
- [ ] `wasm-pack test --node crates/bindings/wasm` passes
- [ ] `npm test` inside `crates/bindings/wasm` passes
- [ ] Python binding tests pass; no `Workflow` remains in `crates/bindings/python/`
- [ ] Generated `.d.ts` for WASM declares one `Quill` class; no `QuillSource`
- [ ] CLI `render`, `info`, `schema`, `validate` subcommands work end-to-end

---

## Ordering

Tasks are grouped for single-PR landing, but may split if review scope demands:

1. **Rename + strip** (tasks 1–4): pure core refactor. Compiles only after
   task 5 lands because external callers break.
2. **Renderable type** (tasks 5–7): introduces `Quill` in quillmark, migrates
   `Backend` trait. After this, workspace compiles again.
3. **Bindings** (tasks 8–10): mechanical updates to expose the new surface.
4. **Polish** (tasks 11–12): `from_path` relocation and docs.

Tasks 1–7 should land as one PR; tasks 8–10 may be separate if diff size
warrants. Task 11 is independent and low-risk; task 12 can trail the code.
