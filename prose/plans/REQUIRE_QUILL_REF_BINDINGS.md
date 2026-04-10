# Require `QUILL:` — Bindings Cascade

**Companion to**: `REQUIRE_QUILL_REF.md`

Once the core change lands (parser errors on missing QUILL, `ParsedDocument::new()` →
`ParsedDocument::new(fields, quill_ref)`, `__default__` system removed), each language
binding has its own surface to clean up. This document covers that work in full.

The core doc already identifies the WASM `RenderOptions.quill_ref` / `CompileOptions.quill_ref`
removal and the CLI guard deletion. Those are included here for completeness and to keep
all binding work in one place; do not duplicate the effort.

---

## WASM Binding (`crates/bindings/wasm/`)

### `src/types.rs`

**Remove `RenderOptions.quill_ref`** (lines 249-251) — the optional quill override field
exists solely because a document might not have specified QUILL. Delete:

```rust
/// Optional quill name that overrides or fills in for the markdown's QUILL frontmatter field
#[serde(skip_serializing_if = "Option::is_none")]
pub quill_ref: Option<String>,
```

**Remove `CompileOptions.quill_ref`** (lines 264-266):

```rust
/// Optional quill reference that overrides the parsed document's `quillRef`.
#[serde(skip_serializing_if = "Option::is_none")]
pub quill_ref: Option<String>,
```

Check that `CompileOptions` still derives `Default` after the removal — it should, since
all remaining fields must have defaults (or the struct is empty).

Update the `ParsedDocument` doc comment at line 216 — remove the clause
`"or \"__default__\" if not specified"`.

### `src/engine.rs` — three methods need updating

**`render()`** (lines 286-289): Replace the `unwrap_or_else` fallback:

```rust
// DELETE:
let quill_ref_to_use = opts
    .quill_ref
    .clone()
    .unwrap_or_else(|| parsed.quill_ref.clone());

// REPLACE with:
let quill_ref_to_use = parsed.quill_ref.clone();
```

**`compile()`** (line 346): Same pattern, same fix. This method is not mentioned in the
core doc but has an identical `opts.quill_ref.unwrap_or_else(|| parsed.quill_ref.clone())`
pattern. Delete the unwrap_or_else and use `parsed.quill_ref.clone()` directly.

**`to_core_parsed()`** (line 418): This private helper constructs a
`quillmark_core::ParsedDocument` from the WASM `ParsedDocument` type. It currently calls
`ParsedDocument::with_quill_ref(fields, quill_ref)`. After the core rename, this becomes
`ParsedDocument::new(fields, quill_ref)`. The compiler will catch this, but note it here
so the SWE isn't surprised.

**Doc comments to update** — remove all references to `__default__` or "inferred" quill:
- `parse_markdown()` at lines 60-62
- `dry_run()` at line 219
- `compile_data()` at line 244
- `render()` at lines 277-278

### TypeScript generated types

After removing the two fields, regenerate the WASM TypeScript bindings. Verify the
emitted `.d.ts` does **not** contain `quillRef` in `RenderOptions` or `CompileOptions`.
If there are TypeScript consumers (tests, examples) that pass `quillRef` in options
objects, find and remove those call sites.

---

## CLI Binding (`crates/bindings/cli/`)

### `src/commands/render.rs`

**Delete the `__default__` guard** (lines 78-83):

```rust
// DELETE — unreachable after core change:
if quill_ref == "__default__" {
    return Err(CliError::InvalidArgument(
        "No QUILL field in frontmatter and --quill not specified".to_string(),
    ));
}
```

`from_markdown()` now rejects markdown without `QUILL:` before this point is reached.
The guard is dead code.

The code path that follows (treating `quill_ref` as a filesystem path, lines 85-94)
remains correct — the quill_ref is now guaranteed to be a real value, not `"__default__"`.

---

## Python Binding (`crates/bindings/python/`)

### `tests/test_api_requirements.py`

**`test_parsed_document_quill_ref()`** (lines 15-34): The second half of this test
parses markdown without `QUILL:` and asserts the result equals `"__default__"`. After
the core change, `ParsedDocument.from_markdown()` raises `ParseError` instead. Change
the second half to:

```python
from quillmark import ParseError

markdown_without_quill = """---
title: Test
---

# Content
"""
with pytest.raises(ParseError):
    ParsedDocument.from_markdown(markdown_without_quill)
```

Make sure `ParseError` is imported (it's already defined in the binding).

**`test_render_without_quill_tag()`** (lines 165-179): The fixture `taro_md` already
contains `QUILL: taro@0.1`, so `from_markdown(taro_md)` will succeed after the core
change. This test will continue to pass, but its name and comment ("Parse markdown
without QUILL tag") are now false. Rename the test and update the comment to reflect
what it actually tests: "workflow can be created by explicit name even when document
declares a different quill." If that use case is intentional, keep it; if not, delete it.

### `python/quillmark/__init__.pyi`

**`Quillmark.workflow()` docstring** (line 62): Delete the sentence
`"Note that the quill reference is optional to specify and can be inferred from the
markdown content's frontmatter when passing a ParsedDocument."` This is confusing now
that QUILL is required — the quill reference is not optional, it is always present in
the ParsedDocument. The remainder of the docstring is fine.

**`ParsedDocument.from_markdown()` docstring** (line 236): Add that `QUILL:` is required:

```python
@staticmethod
def from_markdown(markdown: str) -> ParsedDocument:
    """Parse markdown with YAML frontmatter.

    The frontmatter must include a QUILL field specifying the quill name.

    Raises:
        ParseError: If YAML frontmatter is invalid or QUILL field is absent
    """
```

---

## Verification Checklist

Run after the core change lands and all binding changes are applied:

- [ ] `cargo build --workspace` — zero compilation errors (catches `with_quill_ref` rename
      in `to_core_parsed()` and any lingering `default_quill` references)
- [ ] `cargo test --workspace` — all tests pass
- [ ] WASM: `quillRef` absent from generated `.d.ts` for `RenderOptions` and `CompileOptions`
- [ ] WASM: `compile()` no longer references `opts.quill_ref` — grep `engine.rs` for `opts.quill_ref` → zero hits
- [ ] Python: `pytest crates/bindings/python/tests/` — all tests pass, including the updated `test_parsed_document_quill_ref()`
- [ ] Python: grep `tests/` for `"__default__"` → zero hits
- [ ] CLI: `cargo test -p quillmark-cli` passes
- [ ] Manual smoke test: attempt to render a markdown file without `QUILL:` via the CLI → clear error message, not a panic
