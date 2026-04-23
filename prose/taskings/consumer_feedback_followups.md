# Quillmark WASM: Downstream Consumer Feedback Followups

**Audience:** Quillmark WASM binding maintainer
**Source:** Consumer feedback from a 0.54 → 0.58 migration (registry-side)

## Background

A downstream consumer migrating from 0.54 to 0.58 surfaced a set of friction points in the WASM binding. Most are narrow fixes (doc comments, small API additions). Two are deliberate behaviors we will keep but document more clearly. One (the `init()` footgun) is inherent to `wasm-bindgen --target web` and needs a JS-side mitigation.

Tasks are ordered by consumer impact × implementation cost. Items marked **docs-only** need no code changes beyond comments and the migration guide.

## Tasks

### 1. Accept `Record<string, Uint8Array>` in `engine.quill(tree)`

Today `engine.quill()` rejects plain objects with `quill requires a Map<string, Uint8Array>`. The README advertises both shapes. Every consumer whose source of truth is a `Record` (most of them) has to write `new Map(Object.entries(files))` at the call site.

**Change:** at the NAPI/WASM boundary in `crates/bindings/wasm/src/engine.rs`, normalize the input:

```
input instanceof Map ? input : new Map(Object.entries(input))
```

Update the TS signature to `Map<string, Uint8Array> | Record<string, Uint8Array>`. The Rust side keeps taking a single canonical shape — normalization stays at the boundary.

### 2. Expose `Quill.metadata` (read-only snapshot of Quill.yaml)

Consumers that used `engine.resolveQuill(name).example` / `.supportedFormats` in 0.54 have no replacement. They are re-parsing `Quill.yaml` with regexes to recover the data the engine already owns.

**Change:** add a `metadata` getter on the `Quill` handle returning a plain JS object projection of the loaded `Quill.yaml`:

```ts
readonly metadata: {
  name: string;
  backend: string;
  description: string;
  version: string;
  author: string;
  example?: string;          // example_file contents (if present)
  supportedFormats: string[];
  schema: unknown;           // raw schema from Quill.yaml; consumer validates
  // ...other unstructured metadata from the Quill: section
};
```

Snapshot at `Quill` construction time, not live. One marshalling hop.

The `schema` field ships raw (as parsed from YAML). The engine deliberately no longer owns schema validation in WASM; consumers that need it run their own validator against `metadata.schema`. This is consistent with the "schema APIs are no longer engine-level in WASM" note and gives consumers a supported path forward instead of regex-parsing the bytes themselves.

### 3. Add `Document.clone()`

`Document` has ~10 in-place mutators (`set_field`, `remove_field`, `push_card`, `insert_card`, `remove_card`, `move_card`, `update_card_field`, `update_card_body`, `replace_body`, `set_quill_ref` at `crates/bindings/wasm/src/engine.rs:266-408`). Once a consumer mutates, they cannot cheaply recover the pristine parse without holding the original markdown and re-calling `Document.fromMarkdown`.

**Change:** add a `clone()` method on `Document`:

```rust
#[wasm_bindgen(js_name = clone)]
pub fn clone_doc(&self) -> Document {
    Document {
        inner: self.inner.clone(),
        parse_warnings: self.parse_warnings.clone(),
    }
}
```

Doc comment must state explicitly: parse-time warnings are snapshotted (they describe the document, not the edit history).

### 4. Ship a JS shim that lazy-inits the WASM module

Forgetting `await init()` still produces cryptic panics deep inside the wasm module. This is inherent to `wasm-bindgen --target web` and cannot be fixed on the Rust side.

**Change:** ship a thin JS wrapper around the generated bindings. The wrapper exports lazy-initialized proxies for `Quillmark`, `Document`, etc.; first access awaits the generated `init()` once and caches the promise. Subsequent calls are zero-cost.

Done well this turns the landmine into a non-issue. The consumer's complaint that this was "unchanged from 0.54" implies they expect it to stay broken — fixing it is a real DX upgrade with no Rust-side cost.

If the shim approach is rejected, fall back to:
- A prominent "You must `await init()` first" block at the top of the README.
- A custom panic hook that detects "accessed before init" and throws a legible error rather than a wasm trap.

### 5. Document `RenderOptions.pages` indexing **(docs-only)**

The pages array is 0-indexed (confirmed: `crates/backends/typst/src/compile.rs:175-178` uses the values as direct indices into `document.pages`, default is `(0..page_count).collect()`). The TS type has no JSDoc, so the convention is not self-evident and 0.54 callers migrating may assume 1-indexed.

**Change:** update the doc comment on `pages` in both `crates/bindings/wasm/src/types.rs:182` and `crates/core/src/types.rs:34`:

> Optional 0-based page indices to render (e.g., `[0, 2]` for first and third pages). `undefined` renders all pages. **Not supported for PDF output** — see `FormatNotSupported`.

wasm-bindgen propagates Rust doc comments to the generated `.d.ts`, so IDE hover picks this up automatically.

### 6. Document `Document` getter allocation cost **(docs-only)**

`frontmatter`, `cards`, and `warnings` each build a fresh `serde_json::Value` and call `serialize_maps_as_objects` on every access (`crates/bindings/wasm/src/engine.rs:199-256`). `body` allocates a `String` but is much cheaper; `quillRef` is trivial.

**Change:** add a one-line cost note to the three serializing getters' doc comments, e.g.:

> Allocates and serializes on each call — cache locally if read in a hot loop.

No memoization, no `toJSON()`. Deferred until more consumers hit this.

### 7. Migration guide updates **(docs-only)**

The following are intentional behaviors being called out by consumers. No code change; add to the migration guide.

- **`Document.fromMarkdown` now requires `QUILL:` in frontmatter.** Parse-time failure, not render-time. Fix: add `QUILL: <name>` to frontmatter. Note the shift from render-time to parse-time explicitly (test fixtures rot silently).
- **`Quill.yaml` requires a nested `Quill:` section.** Flat top-level keys were never supported in 0.58+ and will not be. The required fields inside `Quill:` are `name`, `backend`, `description`, `version` (only `author` has a default, `"Unknown"`). See `crates/core/src/quill/config.rs:615-666`.

## Out of scope

- Flattening `Quill.yaml` back to top-level keys. The nested shape is deliberate (room for sibling sections like `typst:`, `cards:`, `main:`).
- Making `description` optional. It is required by design.
- `Document.toJSON()` — deferred. No clear need from the current feedback.
- A `Document.fromMarkdown(md, { quill: name })` overload that injects `QUILL:` — deferred. Possible future ergonomic win; not required by the current feedback.
- Replacing the Document handle with a plain JSON object. `toMarkdown()` needs Rust state, and a JSON-only API would force awkward statics.

## Test updates

- `crates/bindings/wasm/tests/` — add a test that `engine.quill({ "Quill.yaml": ..., ... })` (plain object) succeeds with the same result as the `Map` form.
- `crates/bindings/wasm/tests/metadata.rs` — extend to assert `quill.metadata` exposes `name`, `backend`, `description`, `version`, `supportedFormats`, and the raw `schema` field.
- `crates/bindings/wasm/tests/` — add a test that `doc.clone()` returns a fresh handle, mutations on the clone do not affect the original, and parse warnings are preserved on the clone.

## Done when

- Consumers can pass a `Record<string, Uint8Array>` to `engine.quill()` without a helper.
- `quill.metadata` returns the data consumers used to get from `engine.resolveQuill(name)` — no regex-parsing of `Quill.yaml` required.
- `doc.clone()` produces a mutable copy without re-parsing markdown.
- The JS shim (or the documented fallback) eliminates the `init()` footgun for common integration paths.
- `RenderOptions.pages` and the three serializing Document getters carry doc comments that make their behavior obvious from IDE hover.
- The migration guide explicitly calls out the `QUILL:` parse-time requirement and the `Quill.yaml` required-field list.
