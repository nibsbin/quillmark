# Quill Factory API — `quillmark-wasm` Tasking

## Goal

Split `engine.registerQuill(json)` into two explicit steps:

1. **Construct** — `Quill.from*(source)` parses and validates a bundle,
   returning an opaque `Quill` handle.
2. **Register** — `engine.registerQuill(quill)` stores the handle in the
   engine.

This decouples format knowledge from the engine. The engine only ever sees
`Quill` objects; all wire-format concerns live in the factories. Adding a
new input source (e.g. a flat path-to-bytes map for registry load path)
requires no change to `registerQuill`.

## Background

The current `registerQuill(json)` API conflates parsing, validation, and
registration into one call. It also forces every input to pass through
JSON, which is a poor fit for binary-heavy bundles and for callers that
already have an unpacked in-memory tree (e.g. the registry client after
decompressing a ZIP and rehydrating fonts). The engine should accept a
`Quill` value, not a serialization format.

## Proposed API

### Factories — `Quill.from*()`

```typescript
class Quill {
  /** Parse and validate from a JSON string or plain object. */
  static fromJson(source: string | object): Quill;

  /**
   * Build from a flat path → bytes map.
   *
   * Keys are file paths relative to the quill root
   * (e.g. "Quill.yaml", "assets/fonts/Inter-Regular.ttf").
   * This is the natural output of the registry load path after ZIP
   * decompression and font rehydration.
   */
  static fromTree(tree: Map<string, Uint8Array>): Quill;
}
```

Both factories throw on invalid input with a structured error. No other
methods are required on `Quill` in v1 — it is an opaque handle.

### Engine — `registerQuill(quill)`

```typescript
class Quillmark {
  /** Register a pre-constructed Quill with this engine. */
  registerQuill(quill: Quill): QuillInfo;
}
```

`registerQuill` no longer accepts JSON or any raw format — only `Quill`
handles. The `Quill` may be registered with more than one engine; the
underlying data is shared rather than consumed on registration.

### Registry client — calling pattern

```typescript
// Build a Quill from an in-memory tree (post-rehydration)
const tree: Map<string, Uint8Array> = await loadAndRehydrate(ref, fontCache);
const quill = Quill.fromTree(tree);
engine.registerQuill(quill);

// Build a synthetic Quill for testing
const quill = Quill.fromJson({ files: { 'Quill.yaml': '...', 'main.typ': '...' } });
engine.registerQuill(quill);
```

## Decisions

- **Opaque handle.** `Quill` exposes no public properties. Callers inspect
  a registered quill through `engine.getQuillInfo(name)` as today.
- **Shared, not consumed.** Registering a `Quill` does not invalidate the
  JS handle. The same `Quill` may be registered with multiple engines
  without calling a clone method. The WASM binding holds an `Arc` over the
  inner Rust type.
- **`fromTree` accepts `Map<string, Uint8Array>`.** Also accept a plain
  `Record<string, Uint8Array>` for ergonomics. Directory structure is
  inferred from path separators (`/`).
- **No `fromZip`.** ZIP decompression stays in the registry client — it is
  Node/browser work that happens before Quillmark is involved. Accepting
  raw ZIP bytes would require a decompressor inside the WASM bundle.

## Rust-side changes

The Rust types needed already exist in `quillmark-core`:
- `Quill::from_json(json: &str)` — backs `Quill.fromJson`
- `Quill::from_tree(root: FileTreeNode)` — backs `Quill.fromTree`

The only Rust work is in `crates/bindings/wasm/src/`:

1. **Add a `Quill` WASM wrapper type** (`wasm_bindgen` struct wrapping
   `Arc<quillmark_core::Quill>`).
2. **Expose factory methods** as `#[wasm_bindgen(static_method_of = Quill)]`
   — `from_json` and `from_tree`.
3. **`from_tree` input handling**: accept a `JsValue` (Map or plain
   object), walk entries, copy `Uint8Array` values into byte vecs, build
   `FileTreeNode` by splitting paths on `/`.
4. **Update `Quillmark::register_quill`** to accept `&Quill` instead of
   `JsValue`. Clone the inner `Arc` and forward to the existing
   `quillmark::Quillmark::register_quill`.
5. **Remove the old `registerQuill(JsValue)` signature.** (See migration
   below.)

## Migration

`Quill.fromJson` accepts the same JSON shape the old `registerQuill` did,
so the migration at each call site is mechanical:

```typescript
// Before
engine.registerQuill(quillJson);

// After
engine.registerQuill(Quill.fromJson(quillJson));
```

The current `registerQuill(json)` overload may be kept as a **deprecated
shim** for one release to ease migration, but it should not persist beyond
that — it defeats the purpose of the split.

## Out of scope

- Changes to `Quill.yaml` parsing or schema validation logic.
- Changes to `engine.render`, `engine.compile`, or any rendering path.
- Font rehydration — handled by the registry client before `Quill.fromTree`
  is called.
- A `Quill.free()` method — not needed given shared-ownership semantics.
- TypeScript type generation changes beyond updating `registerQuill`'s
  parameter type.
