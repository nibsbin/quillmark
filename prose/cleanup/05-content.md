# Cleanup review: content crate

Scope: `crates/content/src/{lib,model,import,export,serial,delta,ops,normalize,island,usv}.rs`
(~8440 LOC incl. inline `#[cfg(test)]`), `crates/content/tests/properties.rs` (692 LOC).
Cross-checked against the whole workspace (`core`, `backends/{typst,pdfform}`,
`bindings/{wasm,python}`, `fuzz`) via grep for every `pub` item's callers.
Read `prose/references/markdown-spec.md` and `prose/canon/CONVERT.md` first; no
spec-mandated path is flagged below.

## Findings

### F1: `normalize.rs` — 21 single-input-output unit tests for two pure string functions
- **Category**: test-consolidation
- **Location**: `crates/content/src/normalize.rs:150-320` (`mod tests`)
- **Evidence**: `strip_bidi_formatting` gets 8 tests (`test_strip_bidi_no_change` … `test_strip_bidi_unicode_preserved`, lines 150-203), each a single `assert_eq!(strip_bidi_formatting(input), expected)`. `fix_html_comment_fences` gets 13 tests (`test_fix_html_comment_no_comment` … `test_fix_html_comment_triple_hyphen_single_line`, lines 223-320), same one-assert shape. Both functions are pure `&str -> String`, so every case is a `(input, expected)` pair; none depends on shared mutable state or setup. Neither function has property/fuzz coverage (the `document()` generator in `properties.rs` never emits bidi controls or `<!--…-->` fences), so this is the only coverage — worth keeping, not worth 21 separate `#[test]` functions.
- **Recommendation**: Collapse each cluster into one `#[test]` iterating a `&[(&str, &str)]` table (input, expected), one per function (plus `normalize_markdown`'s 2 combinator tests, lines 206-220, into a third small table or left as-is since they test composition, not enumeration).
- **Est. LOC removable**: ~110
- **Confidence**: high
- **Risk if removed**: none — same assertions, same inputs, just fewer function bodies; no coverage lost.

### F2: `apply_mark_ops` / `apply_line_ops` public wrappers have no production caller
- **Category**: over-abstraction
- **Location**: `crates/content/src/ops.rs:436-440` (`apply_mark_ops`), `ops.rs:539-543` (`apply_line_ops`)
- **Evidence**: Grepped every non-test, non-`content/src` call site in the workspace for `apply_mark_ops`, `apply_line_ops`, `apply_text_delta`, `apply_field_change`: only `apply_field_change` is called from `core/src/document/edit.rs:909,947` and, indirectly, from the wasm `applyChange` verb (`bindings/wasm/src/engine.rs:1483` → `parse_change_bundle` → `apply_field_change`). `apply_field_change` (ops.rs:598) does **not** call the public `apply_mark_ops`/`apply_line_ops` — it calls the private `_inner` variants directly and normalizes once at the end (by design, per its own doc comment on canonicalizing once). `apply_text_delta` (ops.rs:351) *is* live production code — `apply_field_change`'s fast path calls it directly when there are no line/mark ops (the per-keystroke hot path). So of the three single-channel public methods, two (`apply_mark_ops`, `apply_line_ops`) are exercised only by this crate's own unit tests (e.g. `apply_mark_ops_add_and_remove` ops.rs:1007, `line_op_split_and_join` ops.rs:1138) — never by `core`, `wasm`, `python`, or either backend.
- **Recommendation**: Either drop the two wrapper methods (rewrite their ~10 direct-caller tests to go through `apply_field_change(&Delta{ops:vec![]}, line_ops, mark_ops)`, which already exercises the same `_inner` logic) or keep them but downgrade in the module doc from "the edit surface" to "single-channel test/debug helpers" so a reader doesn't infer they're a supported alternate entry point.
- **Est. LOC removable**: ~10 (methods only; test call sites change, not shrink)
- **Confidence**: medium
- **Risk if removed**: low functionally (no production caller found), but this is a published-looking API on a public type (`Content`) — if any out-of-tree consumer of `quillmark-content` exists it would break. No such consumer is visible in this workspace.

### F3: Duplicated image-alt interception between top-level and table-cell import paths
- **Category**: duplicate-helper
- **Location**: `crates/content/src/import.rs:452-466` (`Builder::run`'s `image_depth > 0` branch) vs. `import.rs:716-741` (`Builder::table_event`'s `img_depth > 0` branch)
- **Evidence**: Both blocks intercept a nested `Image` tag by depth-counting `Start(Tag::Image)`/`End(TagEnd::Image)`, and both treat `Text`/`Code` as alt-text append and `SoftBreak`/`HardBreak` as a space — the same four-arm match, twice, against two different accumulators (`self.image_alt: String` for the real island vs. `Inline` cell text for the degraded in-cell case). Confirmed no third copy exists elsewhere (grep for `image_depth`/`img_depth` in `crates/content/src/import.rs` returns exactly these two sites).
- **Recommendation**: Factor the shared "accumulate alt text from a nested image's inline events" step into one helper taking a `&mut dyn FnMut(&str)` (or a small enum target), called from both `run()` and `table_event()`. Keep the differing depth-counter fields and post-close behavior (mint island vs. mark degraded) at the call sites.
- **Est. LOC removable**: ~15
- **Confidence**: medium
- **Risk if removed**: low — both call sites are covered by `table_with_cell_image_degrades` (import.rs:1445) and `image_is_inline_island` (import.rs:1461), plus the `#900`/`#848` properties in `properties.rs`; a refactor that preserves those tests is safe, but the two paths differ subtly enough (one drops the url and flags `degraded`, one doesn't) that a merge is worth doing carefully, not blindly.

### F4: Scattered per-file table/island `Content` fixture builders
- **Category**: duplicate-helper
- **Location**: `crates/content/src/model.rs:929-944` (`table_rt`), `model.rs:946-948` (`cell`); `crates/content/src/ops.rs:1217-1238` (`island`, `content_with_island`); `crates/content/tests/properties.rs:604-632` (`import_row`, `table_content`)
- **Evidence**: Each file hand-builds a minimal single-island `Content` (`text = ISLAND_SLOT`, one `Line{kind: Island}`, one `Island{..}`) with slightly different signatures for its own tests. No shared test-support module exists in the crate (`Cargo.toml` has no `[dev-dependencies]` internal test-util crate, confirmed by reading `crates/content/Cargo.toml` is not needed — the pattern is just copy-pasted per file).
- **Recommendation**: Low priority; the three builders differ enough in purpose (invariant-violation fixtures vs. delta-cascade fixtures vs. property-driven shape fixtures) that consolidating them into one shared helper is a marginal win. Only worth doing if one of the other refactors above touches these files anyway.
- **Est. LOC removable**: ~20
- **Confidence**: low
- **Risk if removed**: low, but low payoff — not recommended as a standalone change.

## Load-bearing (looks redundant, is not)

- **`delta.rs`'s diff/move-detector machinery** (`diff`, `diff_import`, `rebase_anchor`, `relocate_span`/`relocate_point`, `MIN_MOVE`, `CHAR_DIFF_LIMIT`/`coarse_replace`) — confirmed real consumers: `core/src/document/edit.rs:794,823` (`Card::revise`/`Writer::revise_field`) and the wasm `rebase`/`revise` verbs (`bindings/wasm/src/engine.rs:1944`). Not speculative; every non-obvious branch (the `#849` perf cutoff, the `#900`/`#848` escaping) traces to a numbered issue with a regression test.
- **`MarkKind::Anchor` / `MarkOp::RemoveAnchor` / the anchor-id invariants** — never constructed by `import` (by design), but wired end-to-end through the wasm `ContentMark`/`applyChange` TypeScript surface (`bindings/wasm/src/engine.rs:241,336-339`) as the comment-thread/collaborative-annotation feature. Real API surface, not dead model bloat.
- **`MarkKind::Unknown` / `Loss::Unrepresentable` / unknown island types** — the open-set escape hatches never round-trip through `import` (which only mints the closed set), but are reachable via storage deserialization and are explicitly typed at the wasm boundary (`{ type: string; attrs: unknown }`, `loss: "unrepresentable"`) for forward-compatibility with producers this build doesn't know about yet. Documented, deliberate, not speculative.
- **`KnownIslandType`'s closed-dispatch pattern** (`island.rs`) — the `Some(k)` exhaustive-match design this file documents is real: the typst backend's `emit.rs:735-737` matches the same enum, and `parse_cell`/`table_cells` are shared cross-crate (`backends/typst/src/emit.rs:972`), not a single-caller abstraction.
- **`sorted_value` / `sort_keys_owned` / `is_value_key_sorted`** (`model.rs`) — three functions that look like one job done three ways, but each is a distinct perf tier (check-only, clone-and-sort, move-and-sort) used on different paths (`normalize`'s skip-if-sorted fast path vs. `to_canonical_value`'s always-owned path); not redundant, it's the documented "per-keystroke" optimization.
- **`golden_bytes_are_feature_independent`** (`serial.rs:645-654`) — pins an exact canonical-JSON byte string. This looks like the exact anti-pattern the review brief warns about ("tests pinned to exact output strings that break on any refactor"), but here the pinned bytes *are* the contract: the crate's whole purpose is byte-deterministic canonical JSON, and the test's own comment says to bump the schema version if it ever needs to change. Correct use of a golden test, not a fragile one.
- **`LineOp::SetContainers` / `SetKind` / `SetContinues`** (`ops.rs`) — thin apply-time coverage in this crate (mostly wire round-trip + a couple of characterization tests), but each is a typed member of the documented wasm `applyChange` bundle contract (`bindings/wasm/src/engine.rs:342-356`) built to fix specific reachability gaps (issue `#926`), not speculative surface.
