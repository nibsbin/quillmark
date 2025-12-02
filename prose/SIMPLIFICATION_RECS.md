# Code Simplification Recommendations — Quillmark

This document captures recommended simplifications and reduction of overengineering in the
`parse.rs`, `convert.rs`, and `guillemet.rs` modules. Each recommendation includes a
concise rationale, a proposal for change (sometimes a small diff), risk assessment, and a
priority level.

---

## TL;DR: Highest-impact, low-effort suggestions

1. Remove the unused `ranges` return value from `preprocess_markdown_guillemets` in `guillemet.rs`.
2. Inline `trim_guillemet_content` and remove trivial helpers in `guillemet.rs`.
3. Avoid serializing & reparsing YAML in `find_metadata_blocks` / `decompose`.
4. Merge the two guillemet preprocessors (or centralize common logic) and expose a config flag.
5. Factor out repeated "word-boundary `#{}` insertion" logic in `convert.rs` into a small helper function.

---

## Detailed Recommendations

### 1) Remove `ranges: Vec<Range<usize>>` from `preprocess_markdown_guillemets`

What I found
- `preprocess_markdown_guillemets` returns `(String, Vec<Range<usize>>)` but the returned `ranges`
  are only used by unit tests — production code discards them with `_`.

Why simplify
- `ranges` tracking adds code and complexity (byte-range bookkeeping, `Range` import)
- The preprocessing approach intentionally removed event-based context awareness; `ranges`
  is a leftover from early implementations and not used today (see `GUILLEMET_SIMPLIFICATION_CASCADE.md`).

Change proposal
- Change signature to `pub fn preprocess_markdown_guillemets(markdown: &str) -> String`.
- Remove all `ranges.push(start..end)` logic and code that calculates byte positions.
- Update tests to assert the preprocessed string contents, and where range assertions remain
  necessary, either convert them into tests against the returned string or keep a private helper
  in `guillemet.rs` behind `#[cfg(test)]`.

Example diff (sketch):
```diff
- pub fn preprocess_markdown_guillemets(markdown: &str) -> (String, Vec<Range<usize>>) {
+ pub fn preprocess_markdown_guillemets(markdown: &str) -> String {
@@
-    let mut ranges: Vec<Range<usize>> = Vec::new();
@@
-                    let start = result.len();
-                    result.push_str(&clean);
-                    let end = result.len();
-                    result.push('»');
-                    ranges.push(start..end);
+                    result.push_str(&clean);
+                    result.push('»');
@@
-    (result, ranges)
+    result
```

Risks & Mitigation
- Tests relying on `ranges` must be updated. Keep the range assertions only where necessary in test-only helpers.
- If any downstream user expects ranges, document the change in `CONVERT.md` and `GUILLEMET_CONVERSION.md`.

Priority: High — low effort, improves clarity and removes unused features.

---

### 2) Inline `trim_guillemet_content` and simplify small helpers

What I found
- `trim_guillemet_content` calls `content.trim().to_string()` and is used only a few times.
- `count_consecutive` and `count_leading_spaces` are small but acceptable helpers; `trim_` is trivially inlineable.

Why simplify
- Removes trivial indirection and slightly reduces cognitive overhead with no behavior change.

Change proposal
- Replace `let clean = trim_guillemet_content(&content);` with `let clean = content.trim();` or `content.trim().to_string()` where required.
- Delete `trim_guillemet_content()` completely.

Risks & Mitigation
- Simple inlining is safe given tests already cover behavior.

Priority: Low — trivial.

---

### 3) Avoid re-serializing & re-parsing YAML in `find_metadata_blocks` and `decompose`

What I found
- `find_metadata_blocks` parses YAML to search for `SCOPE`/`QUILL`, possibly removes the keys and then serializes a modified mapping back to a String.
- `decompose` re-parses the YAML string (serialised earlier) using `serde_yaml::from_str`.

Why simplify
- Double parsing (parse -> modify -> serialize -> reparse) is unnecessary and expensive.
- Returning already parsed `serde_yaml::Value` from `find_metadata_blocks` saves steps and clarifies logic.

Change proposal
- Update `find_metadata_blocks` to return `yaml_content_parsed: Option<serde_yaml::Value>` instead of `yaml_content: String`.
- Pass `serde_yaml::Value` forward so `decompose` can operate on the parsed structure.
- When producing the `YamlContent` to add to `fields`, avoid serializing/unserializing unless needed elsewhere.

Example diff outline:
```diff
- struct MetadataBlock { yaml_content: String, ... }
+ struct MetadataBlock { yaml_value: Option<serde_yaml::Value>, ... }
@@
- match serde_yaml::from_str::<serde_yaml::Value>(content) {
+ match serde_yaml::from_str::<serde_yaml::Value>(content) {
@@
-     blocks.push(MetadataBlock{..., yaml_content: new_yaml, ...});
+     blocks.push(MetadataBlock{..., yaml_value: Some(serde_yaml::Value::Mapping(new_mapping)) , ...});
```

Risks & Mitigation
- This change impacts function signatures and return types - tests and call sites (e.g., `decompose`) will require updates.
- Improves performance and clarity.

Priority: High — medium effort, high payoff for maintainability.

---

### 4) Merge or centralize `preprocess_guillemets` and `preprocess_markdown_guillemets`

What I found
- Two functions do almost the same scanning work; one is markdown-aware and one is not.

Why simplify
- Reduce duplication, fewer maintenance surfaces. When you fix chevron parsing logic in one function, the other should gain that fix.

Change proposal
- Merge logic into a single function with a `skip_code_blocks: bool` flag, or keep a small plain wrapper for YAML use-cases.
- Keep `pub fn preprocess_guillemets` as a wrapper that calls the general function with `skip_code_blocks=false`.

Risks & Mitigation
- Slightly more complex signature; opt for `#[inline]` wrappers to preserve convenience if needed.

Priority: Medium — moderate effort, reduces duplicated logic.

---

### 5) Factor out repeated "word-boundary `#{}` insertion" logic in `convert.rs`

What I found
- Code in `convert.rs` duplicates the same peek-ahead logic for Emphasis, Strong, and Bold to insert `#{}` for Typst function safety.

Why simplify
- DRY (Don't Repeat Yourself) - fewer lines, better maintainability, single place to change the behavior.

Change proposal
- Add a helper function `fn push_word_boundary_guard(output: &mut String, iter: &mut Peekable...)`.
- Replace repeated code parts with calls to helper.

Example snippet:
```rust
fn push_word_boundary_guard<'a, I, P>(output: &mut String, iter: &mut I)
where I: Iterator<Item=(Event<'a>, Range<usize>)> + Peekable {
    if let Some((Event::Text(text), _)) = iter.peek() {
        if text.chars().next().map_or(false, |c| c.is_alphanumeric()) {
            output.push_str("#{}");
        }
    }
}
```

Risks & Mitigation
- Behavior remains the same; minor change to code structure so minimal test impacts.

Priority: Low — small refactor.

---

### 6) Use the crate's typed `ParseError` instead of `Box<dyn Error>` in `parse.rs`

What I found
- `find_metadata_blocks` and `decompose` return `Box<dyn std::error::Error + Send + Sync>`, which loses semantic error typing.

Why simplify
- Typed errors help with contextual diagnostics and test assertions.

Change proposal
- Use `crate::error::ParseError` or a new `ParseError` type with dedicated variants (e.g., `InvalidYaml`, `DuplicateQuill`, `NameCollision`) and update call sites to return `ParseError`.

Risks & Mitigation
- Requires creating/expanding `ParseError` variants; but yields clearer error messages and better code routes to produce targeted errors.

Priority: Medium — improves UX and error handling.

---

### 7) Simplify horizontal rule detection in `find_metadata_blocks`

What I found
- `find_metadata_blocks` does special-case checks for `---` surrounded by different newline combos and treats them as HR vs frontmatter differently.

Why simplify
- This logic is complex and brittle; treat a `---` line as frontmatter only when the next line is not blank, and skip HR when there’s a blank line above. Avoid line-ending-specific logic unless necessary.

Change proposal
- Normalize line endings at start (`markdown = markdown.replace("\r\n", "\n")`) at the start of parse and then check using `
` only.
- Simplify blank detection using a `is_blank_line()` helper.

Risks & Mitigation
- This is small but behavior altering for documents with `
` line endings; tests should be added for such cases.

Priority: Low — small clarity gain, low risk.

---

### 8) Evaluate the need for `StrongKind` (Bold vs Underline)

What I found
- A `StrongKind` determines whether `Tag::Strong` was `**` or `__` to produce either bold (`*...*`) or underline (`#underline[...]`) in Typst.

Why simplify
- Supporting `__` as underline can increase complexity in source mapping and test cases; consider whether supporting both is essential.

Change proposal
- If underline is not widely used, or the Typst target does not require it, consider mapping both `__` and `**` to bold `*...*` to simplify parsing and state.
- Alternatively, keep existing behavior if backward compatibility required.

Risks & Mitigation
- Backward-incompatible for users relying on `__` for underline. Add a documented migration/behavior note if changed.

Priority: Low — UX decision.

---

## Testing and Validation Plan

- Update unit tests in `guillemet.rs` to check only preprocessed string content.
- Keep range assertions only for test-only helper functions (if needed) behind `#[cfg(test)]`.
- Add tests for `












































- `prose/debriefs/GUILLEMET_SIMPLIFICATION_CASCADE.md`- `prose/plans/completed/GUILLEMET_CONVERSION.md`- `crates/backends/typst/src/convert.rs`- `crates/core/src/parse.rs`- `crates/core/src/guillemet.rs`Appendix: References---- Keep an eye on convert.rs complexity; most refactors are small and focused on clarity and removing duplicated logic.- Follow up with the YAML parsing improvement; this is a medium-sized change but results in a simplification cascade by eliminating repeated re-serialization and parsing.- Implement the high-priority changes incrementally (small PRs). The `ranges` removal can be done immediately and provides a quick simplification.## Final Thoughts & Next Steps---- Changing YAML roundtrips should preserve semantics but may affect formatting/serialization for tests that assert exact YAML strings. Favor behavior assertions rather than exact serialized YAML string tests.- Removing `ranges` is likely NOT a breaking API change because the crate exposes `preprocess_markdown_guillemets` via the crate root; if external users rely on `ranges`, they will need a deprecation path: keep the old signature for one release returning ranges but with empty vectors, log a dep warning, then remove.## Notes on Backward Compatibility & Users---- Avoid API breakage where possible; if an API break is necessary, note it in the changelog.- Be run with `cargo test --workspace`.- Include unit test updates and new tests for altered behaviors.Each PR should:7. **Low effort PR:** Simplify `is_blank_line` logic and normalize newline handling.6. **Medium PR:** Replace boxed errors with typed `ParseError` variants.5. **Small PR:** Add `push_word_boundary_guard` helper in `convert.rs` and refactor duplicated code.4. **Medium PR:** Merge guillemet helper functions; add `skip_code_blocks` flag.3. **Medium PR:** Return parsed YAML from `find_metadata_blocks` instead of serialized strings.2. **Small PR:** Inline `trim_guillemet_content` and remove unused helpers.1. **Small PR:** Remove `ranges` from `preprocess_markdown_guillemets` and update tests.## Suggested PR Roadmap---- After changing YAML parsing, update tests that rely on string roundtrips for parsing/serializing.- Add integration tests showing the removal of `ranges` has no effect on outputs.\n` line endings if horizontal rule detection is simplified.