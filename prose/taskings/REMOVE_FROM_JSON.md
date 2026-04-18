# Remove `Quill::from_json` / `Quill.fromJson`

**Goal**: Delete the `from_json` factory from both `quillmark-core` and the
WASM bindings. `fromTree` is the single authoritative in-memory constructor.
No deprecation, no shim, no compatibility wrapper — just delete it and migrate
every call site.

---

## Motivation

`Quill::from_json` is a thin wrapper that parses a JSON string into a
`FileTreeNode` and then calls `Quill::from_tree`. It adds no semantics, only
a second input format. Two factories for the same operation means two code
paths to test, two sets of docs to keep in sync, and two error surfaces for
consumers to learn. Since `fromTree` already accepts both `Map<string,
Uint8Array>` and plain `Record<string, Uint8Array>`, consumers that held JSON
can encode each file's `contents` to bytes at the call site — a few lines of
glue, no capability lost.

This is strictly a public-API removal. The `QuillValue::from_json` utility
in `crates/core/src/value.rs` is an entirely unrelated function (serde_json
value conversion) and **must not be touched**.

---

## What to Delete

| What | Location |
|------|----------|
| `Quill::from_json` (core) | `crates/core/src/quill/load.rs:205-235` |
| `FileTreeNode::from_json_value` (internal helper, only used by `from_json`) | `crates/core/src/quill/tree.rs:180` |
| `Quill::from_json` (WASM binding) | `crates/bindings/wasm/src/engine.rs:377-413` |
| `fromJson` entry in `QUILL_FACTORY_TS` | `crates/bindings/wasm/src/engine.rs:356-362` |
| Core Rust `from_json` tests (8 tests) | `crates/core/src/quill/tests.rs:423-628` |
| WASM Rust test calls | `crates/bindings/wasm/tests/wasm_bindings.rs:44, 75` |
| WASM Rust test calls | `crates/bindings/wasm/tests/resolve_quill.rs:18, 28` |
| WASM Rust test calls | `crates/bindings/wasm/tests/metadata.rs:18, 59` |
| JS test calls (11) | `crates/bindings/wasm/basic.test.js` |
| JS test calls (2) | `crates/bindings/wasm/resolve.test.js` |
| JS example calls (2) | `crates/bindings/wasm/resolve.js` |

Docs that reference `fromJson` and must be rewritten to use `fromTree`:

| Doc | Location |
|-----|----------|
| WASM README | `crates/bindings/wasm/README.md` (lines 82, 109, 130, 135-139) |
| JS API reference | `docs/integration/javascript/api.md` (lines 15, 35, 46) |
| Validation guide | `docs/integration/validation.md:37` |
| Dynamic assets | `docs/integration/dynamic-assets.md:44` |
| Integration overview | `docs/integration/overview.md` (lines 32, 63) |
| Quickstart | `docs/getting-started/quickstart.md:76` |
| WASM design doc | `prose/designs/WASM.md` (lines 11, 33) |
| Quill design doc | `prose/designs/QUILL.md:85` |
| Original proposal | `prose/proposals/quill_factory_api.md` (add a note at top: `> Superseded in part — fromJson removed. See taskings/REMOVE_FROM_JSON.md.`) |

---

## Step-by-Step Changes

### 1. Core library — delete `Quill::from_json` (`crates/core/src/quill/load.rs`)

Delete the entire method at lines 205-235 (the doc block plus the body). The
`from_tree` method it ultimately calls remains unchanged.

Check whether the `use serde_json::Value as JsonValue;` and `use std::collections::HashMap;`
imports inside the function body were local — they were, so deletion is
self-contained. Verify no other function in `load.rs` now has an unused import.

### 2. Core library — delete `FileTreeNode::from_json_value` (`crates/core/src/quill/tree.rs`)

Confirm `from_json_value` has no callers outside `Quill::from_json`:

```
rg 'from_json_value' crates/
```

It is a private helper used solely to recursively decode the JSON `files`
object. Delete the method and any now-unused imports it pulled in.

### 3. Core library — delete the `from_json` test suite (`crates/core/src/quill/tests.rs`)

Delete all eight tests in the `from_json` block (lines ~423-628):

- `test_from_json`
- `test_from_json_with_byte_array`
- `test_from_json_missing_files`
- `test_from_json_tree_structure`
- `test_from_json_nested_tree_structure`
- `test_from_json_with_metadata_override`
- `test_from_json_empty_directory`
- (plus the eighth — audit the block)

These tests exercise JSON parsing, not bundle semantics; equivalent coverage
already exists in the `from_tree` tests. Do not port them. If any of them
tests a bundle-loading behavior not covered by a `from_tree` test (e.g. a
specific error path around empty directories), add the missing assertion to
the corresponding `from_tree` test instead.

### 4. WASM binding — delete the factory method (`crates/bindings/wasm/src/engine.rs`)

Delete the `fromJson` block in `QUILL_FACTORY_TS` (lines ~356-362) — keep
only the `fromTree` declaration inside the namespace.

Delete the `#[wasm_bindgen(js_name = fromJson, skip_typescript)] pub fn from_json(...)`
method and its entire doc comment (lines ~377-413).

Nothing else in `engine.rs` calls `from_json`; `file_tree_from_js_tree` and
the `fromTree` method stay.

### 5. WASM tests (Rust) — migrate to `from_tree`

Files: `crates/bindings/wasm/tests/{wasm_bindings,resolve_quill,metadata}.rs`.

Each `Quill::from_json(json_str)` call must be replaced with a `Quill::from_tree(root)`
call. The test fixtures currently build a JSON string with `"files": { "Quill.yaml": { "contents": "..." } }`
shape — convert those fixtures to build `FileTreeNode::Directory` with
`FileTreeNode::File { contents: bytes }` entries directly. A small helper in
a shared `mod common;` test module is appropriate if the conversion appears
in more than one file:

```rust
fn tree(entries: &[(&str, &[u8])]) -> FileTreeNode {
    let mut root = FileTreeNode::Directory { files: HashMap::new() };
    for (path, bytes) in entries {
        root.insert(path, FileTreeNode::File { contents: bytes.to_vec() }).unwrap();
    }
    root
}
```

### 6. WASM tests (JS) — migrate to `Quill.fromTree`

Files: `crates/bindings/wasm/basic.test.js`, `resolve.test.js`, `resolve.js`.

The canonical migration pattern at each call site:

```javascript
// Before
const quill = Quill.fromJson({
  files: {
    "Quill.yaml": { contents: yamlString },
    "plate.typ":  { contents: plateString },
  },
});

// After
const enc = new TextEncoder();
const quill = Quill.fromTree(new Map([
  ["Quill.yaml", enc.encode(yamlString)],
  ["plate.typ",  enc.encode(plateString)],
]));
```

For fixtures that already hold `Uint8Array` (binary files like fonts), pass
them through unchanged.

If the same fixture appears in multiple JS tests, lift the tree construction
into a helper in a test-support file rather than duplicating `TextEncoder`
calls.

### 7. Public documentation

Rewrite every `fromJson` example listed in the "What to Delete" docs table
to use `fromTree`. The recommended prose for the JS docs is:

> `Quill.fromTree` accepts a flat path-to-bytes map. Use a `TextEncoder` to
> convert string contents to `Uint8Array`:
>
> ```javascript
> const enc = new TextEncoder();
> const quill = Quill.fromTree(new Map([
>   ["Quill.yaml", enc.encode(yamlString)],
>   ["plate.typ",  enc.encode(plateString)],
> ]));
> ```

In `docs/integration/dynamic-assets.md` and `docs/integration/validation.md`,
prefer the `Map<string, Uint8Array>` form — these pages already discuss
binary assets, and `Map` parallels that mental model better than a plain
object.

Update `crates/bindings/wasm/README.md` so the "Factory methods" section
lists only `fromTree`, and the migration note (if any remains) points to
this tasking.

### 8. Design documents

- `prose/designs/WASM.md` — remove lines 11 and 33 and any surrounding prose
  that says "two factory methods". The design doc should now describe a single
  `fromTree` factory.
- `prose/designs/QUILL.md` — remove the line 85 reference in the Rust API
  enumeration.
- `prose/proposals/quill_factory_api.md` — prepend a banner:
  `> **Partially superseded.** `fromJson` was removed; only `fromTree` remains. See `prose/taskings/REMOVE_FROM_JSON.md`.`

### 9. Python and CLI bindings

No changes needed — neither binding exposes `from_json`. Confirm with:

```
rg 'from_json' crates/bindings/python crates/bindings/cli
```

Expected: zero hits unrelated to `QuillValue::from_json`.

---

## Migration Notes for External Consumers

For downstream JS consumers who receive Quill bundles as JSON from an HTTP
API, the migration is:

```javascript
// Before: server returns { files: { "Quill.yaml": { contents: "..." }, ... } }
const quill = Quill.fromJson(await res.json());

// After: transform the JSON shape into a bytes map at the boundary
const enc = new TextEncoder();
const payload = await res.json();
const entries = Object.entries(payload.files).map(([path, { contents }]) => [
  path,
  typeof contents === "string" ? enc.encode(contents) : new Uint8Array(contents),
]);
const quill = Quill.fromTree(new Map(entries));
```

If any first-party client code in this monorepo follows that pattern, move
the transform into a shared helper (e.g. `crates/bindings/wasm/examples/`)
so downstream users can copy it. Do **not** re-add `fromJson` under a
different name.

---

## Verification Checklist

- [ ] `cargo build --workspace` compiles clean
- [ ] `cargo test --workspace` passes
- [ ] `wasm-pack test --node crates/bindings/wasm` passes (or whatever the
      project's WASM test command is — check `crates/bindings/wasm/package.json`)
- [ ] `npm test` inside `crates/bindings/wasm` passes
- [ ] `rg '\bfrom_json\b' crates/core crates/bindings` returns only hits
      inside `crates/core/src/value.rs` and its call sites (the unrelated
      `QuillValue::from_json`)
- [ ] `rg '\bfromJson\b' crates docs prose` returns zero hits (except inside
      the superseded proposal banner, if that phrasing was kept)
- [ ] `rg 'from_json_value' crates/` returns zero hits
- [ ] Generated TypeScript `.d.ts` for the WASM package no longer declares
      `Quill.fromJson`
- [ ] Every doc page in the table under "What to Delete" has been rewritten
      to use `fromTree`; spot-check each example block actually runs
- [ ] No new helper named `fromJson` or `from_json` has been introduced
      under a different module path — the capability is gone, not relocated
