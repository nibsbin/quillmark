# Require `QUILL:` in Frontmatter

**Goal**: Make `QUILL:` a required field in the top-level YAML frontmatter block.
Remove the entire `__default__` fallback system, the `default_quill()` backend hook,
`ParsedDocument::new()`, and the optional quill override fields on WASM render options.
No backwards-compat shims. No deprecation markers. Just delete it.

---

## Motivation

Documents that omit `QUILL:` implicitly couple themselves to the execution environment.
This is the kind of hidden state that causes debugging pain. Every document should
declare its quill explicitly. We are pre-release with only internal consumers — this
is the right time to make the break cleanly.

---

## What to Delete

The following things exist solely to support the optional-QUILL path and should be
**deleted outright** (not deprecated, not feature-flagged):

| What | Location |
|------|----------|
| `ParsedDocument::new()` default constructor | `crates/core/src/parse.rs:68-73` |
| `__default__` fallback in `decompose()` | `crates/core/src/parse.rs:728` |
| `Backend::default_quill()` trait method | `crates/core/src/backend.rs:194-196` (and its 28-line doc block above) |
| Auto-registration block in `register_backend()` | `crates/quillmark/src/orchestration/engine.rs:124-141` |
| Typst backend `default_quill()` impl | `crates/backends/typst/src/lib.rs` (find the impl) |
| Embedded default Quill assets | `backends/quillmark-typst/default_quill/` directory |
| `RenderOptions.quill_ref` override field | `crates/bindings/wasm/src/types.rs:249-251` |
| `CompileOptions.quill_ref` override field | `crates/bindings/wasm/src/types.rs:264-266` |
| `__default__` guard in CLI render | `crates/bindings/cli/src/commands/render.rs:78-83` |
| `default_quill_test.rs` entirely | `crates/quillmark/tests/default_quill_test.rs` |
| Design document | `prose/designs/DEFAULT_QUILL.md` (delete or move to `plans/completed/`) |

---

## Step-by-Step Changes

### 1. Core parser — make QUILL required (`crates/core/src/parse.rs`)

**In `decompose()`**, around line 728, replace the `unwrap_or_else` fallback:

```rust
// DELETE this:
let quill_tag = quill_ref.unwrap_or_else(|| "__default__".to_string());

// REPLACE with:
let quill_tag = quill_ref.ok_or_else(|| {
    ParseError::InvalidStructure(
        "Missing required QUILL field. Add `QUILL: <name>` to the frontmatter.".to_string(),
    )
})?;
```

**Delete `ParsedDocument::new()`** (lines 68-73). Its only purpose is producing an
`__default__` quill_ref. With QUILL required, this constructor is meaningless and
misleading. Rename `with_quill_ref()` → `new()` since it is now the only constructor:

```rust
// RENAME with_quill_ref to new:
pub fn new(fields: HashMap<String, QuillValue>, quill_ref: QuillReference) -> Self {
    Self { fields, quill_ref }
}
```

Update the single call site at line 732 from `ParsedDocument::with_quill_ref(...)` to
`ParsedDocument::new(...)`.

**Tests in `parse.rs`**: The `test_no_frontmatter()` test (line 742) asserts
`quill_reference().name == "__default__"`. This test must now assert that parsing fails
with a missing-QUILL error. Similarly audit `test_with_frontmatter()` and any other test
using markdown without a `QUILL:` field — add `QUILL: some_quill` or update to expect
an error as appropriate.

Add a new test:

```rust
#[test]
fn test_missing_quill_field_errors() {
    let markdown = "---\ntitle: No quill here\n---\n# Body";
    let result = decompose(markdown);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required QUILL field"));
}
```

---

### 2. Backend trait — remove `default_quill()` (`crates/core/src/backend.rs`)

Delete `default_quill()` from the `Backend` trait: the method definition at lines 194-196
and its entire doc block (lines 166-193). Any backend that implements this method will
now fail to compile, forcing cleanup there too.

---

### 3. Engine — remove auto-registration (`crates/quillmark/src/orchestration/engine.rs`)

In `register_backend()`, delete the entire block that calls `backend.default_quill()` and
conditionally registers `__default__` (lines 124-141, beginning with the comment
`"If the backend provides a default Quill..."`).

---

### 4. Typst backend — remove embedded default Quill

Find the `default_quill()` impl in `crates/backends/typst/src/lib.rs` and delete it.
Delete the embedded `default_quill/` directory (including `include_str!`/`include_bytes!`
references to it). The `__default__` name reservation logic in the engine can also be
reviewed — without auto-registration, `__default__` is now just a normal (albeit odd)
quill name that nobody should use.

---

### 5. WASM bindings — remove optional quill overrides (`crates/bindings/wasm/src/types.rs`)

Delete:
- `RenderOptions.quill_ref: Option<String>` (lines 249-251, including the `#[serde(...)]`
  attribute and doc comment)
- `CompileOptions.quill_ref: Option<String>` (lines 264-266, same)

Update the `ParsedDocument` doc comment at line 216 to remove the
`"or \"__default__\" if not specified"` clause.

---

### 6. WASM engine — simplify render/compile/dryRun (`crates/bindings/wasm/src/engine.rs`)

**`render()`** (lines 286-295): Remove the `opts.quill_ref.clone().unwrap_or_else(...)`
fallback. The quill_ref is now always in the parsed document:

```rust
// DELETE:
let quill_ref_to_use = opts
    .quill_ref
    .clone()
    .unwrap_or_else(|| parsed.quill_ref.clone());

// REPLACE with:
let quill_ref_to_use = parsed.quill_ref.clone();
```

Update the `render()` doc comment (lines 277-278) to remove the sentence about
quill_ref being optional.

**`dryRun()` and `compileData()`**: Update doc comments at lines 219 and 244 that
reference the `__default__` fallback.

**`parseMarkdown()`**: Update the doc comment at lines 60-62.

---

### 7. CLI — remove `__default__` guard (`crates/bindings/cli/src/commands/render.rs`)

Delete lines 78-83:

```rust
// DELETE:
if quill_ref == "__default__" {
    return Err(CliError::InvalidArgument(
        "No QUILL field in frontmatter and --quill not specified".to_string(),
    ));
}
```

This guard is now unreachable — `from_markdown()` will have already rejected any markdown
without a `QUILL:` field. The code path that follows (treating the quill_ref as a path)
can now assume it always has a real value.

---

### 8. Delete the default Quill test suite

Delete `crates/quillmark/tests/default_quill_test.rs` entirely. Every test in that file
validates behavior that will no longer exist.

---

### 9. Fuzz targets

Check `crates/fuzz/src/parse_fuzz.rs`. If it seeds with markdown that lacks `QUILL:`,
those inputs will now produce parse errors instead of successful parses — which is valid
fuzz behavior. No structural change needed, but verify the fuzz harness does not unwrap
the result unconditionally.

---

### 10. Design doc

Delete `prose/designs/DEFAULT_QUILL.md` or move it to `prose/plans/completed/` with a
one-line note at the top: `> Superseded. QUILL: is now required. See REQUIRE_QUILL_REF.md.`

---

## Verification Checklist

- [ ] `cargo build --workspace` compiles clean (no `default_quill` method references remain)
- [ ] `cargo test --workspace` passes (pay particular attention to `parse.rs` tests and any
      test that builds a `ParsedDocument` from markdown without `QUILL:`)
- [ ] Grep for `__default__` across `crates/` — only acceptable hits are in error messages
      from the engine's `quill_not_found` path (which now says "Available quills: ..."
      and may still name `__default__` if someone registered it manually)
- [ ] Grep for `default_quill` across `crates/` — zero hits
- [ ] Grep for `ParsedDocument::new()` with a single argument (the old default constructor
      signature) — zero hits
- [ ] WASM TypeScript types regenerated; `quillRef` field absent from `RenderOptions` and
      `CompileOptions` in the generated `.d.ts`
