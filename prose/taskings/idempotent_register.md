# Quillmark Engine: Quiver Support Tasking

**Audience:** Quillmark engine maintainer
**Consumer:** `@quillmark/quiver` (registry rewrite; successor to `@quillmark/registry`)
**Design source:** `PROGRAM.md §7` in `nibsbin/quillmark-quiver`

## Background

Quiver composes multiple versioned quill sources and resolves selector refs (`name@x.y`, bare `name`) to canonical refs (`name@x.y.z`) against its own manifest before calling the engine. On hot paths it may re-enter `registerQuill` with the same canonical ref many times per session.

Quiver does not need the engine's existing selector-resolution capability changed or removed — standalone WASM consumers that pass selector refs through `parseMarkdown → render` keep working. Quiver simply hands canonical refs across the boundary as its own convention.

We need two small additions to support the hot path.

## Tasks

### 1. Idempotent `register_quill` on duplicate canonical ref

Today `register_quill` errors with `RenderError::QuillConfig { "already registered" }` on any duplicate `(name, version)` (see `crates/quillmark/tests/versioning_test.rs::test_version_collision_error`).

**Change:** re-registering the same canonical ref (`name@x.y.z`) returns `Ok` as a no-op. **First-write-wins:** no content comparison, no hashing. Subsequent calls must be cheap (no payload re-read, no backend re-init).

Rationale for not hashing: canonical refs are content-identifiers by convention; quiver's own cache enforces that. If divergent bytes ever reach `registerQuill`, something upstream is already broken. We can tighten to "error on content mismatch" later as a pure additive change.

### 2. Cheap existence check

Add:

```rust
impl Quillmark {
    pub fn has_quill(&self, canonical_ref: &str) -> bool;
}
```

WASM binding: `engine.hasQuill(ref: string): boolean` in `crates/bindings/wasm/src/engine.rs`.

Semantics: `true` iff a quill is registered under the exact canonical ref `name@x.y.z`. No selector resolution — it's a direct lookup on the engine's registered set. This lets quiver short-circuit the boundary transfer on repeated resolves without paying even the no-op `register_quill` cost.

## Out of scope

- Changes to selector parsing or resolution inside the engine — leave the existing `QuillReference` / `VersionSelector` behavior alone. Quiver does not rely on it, but other consumers do.
- Content-mismatch detection on duplicate register (deferred; see §1 rationale).
- Any change to `render`, `parseMarkdown`, or `get_quill_info` API shape. Quiver rewrites `Document.quillRef` to canonical by constructing a new value before calling `render`; no new render overload is needed.
- Prerelease / build-metadata / semver range support.

## Test updates

In `crates/quillmark/tests/versioning_test.rs`:

- Replace `test_version_collision_error` with `test_register_same_canonical_is_noop` — register `name@1.0`, register again with identical content, assert `Ok` on both and that the engine reports one registered quill.

In `crates/bindings/wasm/tests/wasm_bindings.rs`:

- Add a test covering `hasQuill`: returns `false` before register, `true` after, `false` for an unregistered canonical ref.

## Done when

- Quiver can call `registerQuill(canonical)` repeatedly with no error and negligible cost.
- `engine.hasQuill(canonicalRef)` returns a correct boolean from JS without any rendering or resolution side effects.
- No existing standalone-consumer tests regress (selector refs in markdown still render).
