# Consolidate Quill Creation to `engine.quill()`

**Goal**: Make `engine.quill(tree)` the single public entrypoint for creating a
`Quill`. The core constructors `Quill::from_tree` and `Quill::from_path` become
`pub(crate)`. Every binding layer and call site outside `quillmark-core` routes
through the engine.

---

## Motivation

Two public paths currently exist for creating a `Quill`:

- `Quill::from_tree(tree)` / `Quill::from_path(path)` — parse only, no backend
- `engine.load_quill(tree)` / `engine.quill_from_path(path)` — parse + attach backend

There is no legitimate external use case for a backend-less `Quill`. Every
consumer either renders or inspects metadata — both work fine with a backend
attached. The `Option<Arc<dyn Backend>>` field in the `Quill` struct exists
solely because construction and backend-attachment were historically separate
steps that both leaked public. The engine already self-documents this: the
error message inside `render.rs` tells callers who hit `NoBackend` to use
`engine.load_quill()` instead.

Collapsing to one entrypoint eliminates the dual-path confusion at every
binding layer, removes the `Option` as a meaningful runtime state outside core,
and gives the API a name that reflects what it does: `engine.quill(tree)` —
ask the engine for a Quill backed by whatever backend the tree declares.

---

## Rename: `load_quill` → `quill`

`engine.load_quill(tree)` becomes `engine.quill(tree)`. The word "load"
implies I/O; the method accepts an already-in-memory `FileTreeNode`. The new
name follows the established Rust idiom of naming a factory after what it
returns (`HashMap::entry`, `BufReader::lines`), and matches the WASM surface.

`engine.quill_from_path(path)` keeps its name — the `_from_path` suffix
carries the input-type information that `quill(tree)` doesn't need.

---

## What Changes

### 1. Core library — visibility (`crates/core/src/quill/load.rs`)

Change `pub fn from_tree` (line 58) and `pub fn from_path` (line 12) to
`pub(crate)`. No other changes to signatures, logic, or error types. Internal
callers within `quillmark-core` — `from_config` (line 90), all tests in
`crates/core/src/quill/tests.rs` — are unaffected.

### 2. Engine — rename (`crates/quillmark/src/orchestration/engine.rs`)

Rename `load_quill` (line 46) to `quill`. Update the doc comment: the method
"builds and returns a render-ready Quill from an in-memory file tree." No logic
changes. The private `attach_backend` helper (line 73) is untouched.

Update `crates/quillmark/src/lib.rs` re-exports and the doc example at line 11
to use `engine.quill(tree)`.

### 3. Engine call sites — Typst backend tests

`Quill::from_tree` calls in the Typst backend are outside `quillmark-core` and
will no longer compile. Migrate them through the engine:

| File | Line | Change |
|------|------|--------|
| `crates/backends/typst/src/compile.rs` | 315 | `let quill = Quill::from_tree(root)` → construct `Quillmark::new()` and call `engine.quill(root)` |
| `crates/backends/typst/src/world.rs` | 728 | same pattern |

If these tests are only exercising Typst compilation logic and do not need a
real backend resolution, move them to an integration test in `crates/quillmark/tests/`
where an engine is already available rather than constructing one inside the
backend crate.

### 4. CLI binding (`crates/bindings/cli/src/commands/`)

Each CLI command that calls `Quill::from_path` directly must instead construct
`Quillmark::new()` (cheap) and call `engine.quill_from_path(path)`:

| File | Line | Current |
|------|------|---------|
| `render.rs` | 58 | `Quill::from_path(path)` |
| `info.rs` | 31 | `Quill::from_path(path)` |
| `schema.rs` | 28 | `Quill::from_path(path)` |
| `validate.rs` | 145 | `Quill::from_path(path)` |

`Quillmark::new()` auto-registers feature-flagged backends, so the existing CLI
behavior is preserved. If a shared engine instance is desirable across
subcommands, thread it through the CLI context struct — but that is out of
scope here; one `Quillmark::new()` per command invocation is acceptable.

### 5. Python binding (`crates/bindings/python/src/types.rs`)

`PyQuill::from_path` at line 188 calls `Quill::from_path` directly. Route it
through the engine: the `PyQuillmark` type at line 28 already constructs
`Quillmark::new()`, so the pattern is established. Either:

- Deprecate `PyQuill::from_path` and direct Python callers to
  `PyQuillmark.quill_from_path(path)`, or
- Keep the Python `Quill.from_path(path)` surface but implement it by
  constructing an engine internally (same ergonomics, correct backend).

Pick whichever matches the existing Python binding conventions. Do **not** leave
`Quill::from_path` called directly from Python after this change.

### 6. WASM binding (`crates/bindings/wasm/src/engine.rs`)

Two JS entrypoints exist today:

| JS name | Rust impl | Backend |
|---------|-----------|---------|
| `Quillmark.quillFromTree(tree)` | line 82 | yes — calls `engine.load_quill` |
| `Quill.fromTree(tree)` | line 154 | no — calls `Quill::from_tree` directly |

**Delete** the `Quill.fromTree` static factory (line 154) and its TypeScript
declaration. **Rename** `quillFromTree` → `quill` in both the Rust
`#[wasm_bindgen]` method and the `QUILL_FACTORY_TS` / `ENGINE_TS` declaration
blocks.

The resulting JS surface is:

```typescript
class Quillmark {
  quill(tree: Map<string, Uint8Array> | Record<string, Uint8Array>): Quill;
}
```

No `Quill` static constructor is exported.

Migrate all JS tests in `crates/bindings/wasm/basic.test.js` that call
`Quill.fromTree(...)` to `engine.quill(...)`. The `describe('Quill.fromTree')`
block (line 41) and the no-backend error test (line 90) should be deleted
entirely — the behavior they guard against (`NoBackend`) is no longer reachable
from JS.

### 7. Documentation

| Doc | Change |
|-----|--------|
| `docs/integration/javascript/api.md` | Replace `engine.quillFromTree` → `engine.quill`; delete the `Quill.fromTree` note at line 76 |
| `docs/integration/overview.md:31` | Update factory call |
| `docs/integration/validation.md:37` | Update factory call |
| `docs/getting-started/quickstart.md:51` | Update factory call |
| `crates/bindings/wasm/README.md` | Remove `Quill.fromTree` section; rename `quillFromTree` → `quill` |
| `README.md` | Update any `load_quill` references |
| `prose/designs/WASM.md:32` | Remove `registerQuill`/`fromTree` paragraph; describe `engine.quill(tree)` as the sole constructor |
| `prose/designs/QUILL.md` | Update "In-memory Tree Contract" section; note that `from_tree` is internal |
| `prose/proposals/quill_factory_api.md` | Prepend: `> **Superseded.** See prose/taskings/consolidate-quill-creation.md.` |
| `prose/taskings/quill_render_api.md` | Update all `engine.quillFromTree` and `Quill.fromTree` references |
| `prose/taskings/REMOVE_FROM_JSON.md` | Update the closing note to reference `engine.quill` not `Quill.fromTree` |

---

## Verification Checklist

- [ ] `cargo build --workspace` compiles clean
- [ ] `cargo test --workspace` passes
- [ ] `rg 'Quill::from_tree|Quill::from_path' crates/` returns hits only inside `crates/core/`
- [ ] `rg 'load_quill' crates/ docs/ prose/` returns zero hits
- [ ] `rg 'quillFromTree|fromTree' crates/ docs/` returns zero hits
- [ ] `wasm-pack test --node crates/bindings/wasm` passes
- [ ] `npm test` inside `crates/bindings/wasm` passes
- [ ] Generated `.d.ts` for the WASM package declares `quill()` on `Quillmark` and no static `Quill.fromTree`
- [ ] Python binding tests pass; `PyQuill` no longer calls `Quill::from_path` directly
- [ ] All CLI commands (`render`, `info`, `schema`, `validate`) continue to work end-to-end
- [ ] No `NoBackend` error is reachable from any public binding surface
