# Cleanup review: core/document (source)

Scope: `crates/core/src/document/{mod,dto,prescan,emit,edit,payload,assemble,wire,yaml_hints,fences,meta,limits}.rs` (`tests/` subdirectory excluded). 8131 LOC total (includes each file's embedded `#[cfg(test)] mod tests`, out of this review's remit but counted in `wc -l`).

Read first: `prose/canon/DOCUMENT_STORAGE.md`, `CARDS.md`, `PROGRAMMATIC.md`, `prose/references/markdown-spec.md`.

## Findings

### F1: `store_ext_namespace`/`store_seed_namespace` and `remove_ext_namespace`/`remove_seed_namespace` are near-identical merge/remove logic parameterized only by which meta key they touch
- **Category**: redundant-logic
- **Location**: `crates/core/src/document/edit.rs:611-622` (`store_ext_namespace`), `edit.rs:660-676` (`store_seed_namespace`), `edit.rs:634-641` (`remove_ext_namespace`), `edit.rs:682-689` (`remove_seed_namespace`)
- **Evidence**: The two `remove_*_namespace` bodies are structurally identical modulo `take_ext`/`set_ext` vs `take_seed`/`set_seed`:
  ```rust
  pub fn remove_ext_namespace(&mut self, namespace: &str) -> Option<serde_json::Value> {
      let mut map = self.payload_mut().take_ext()?;
      let removed = map.remove(namespace);
      if !map.is_empty() { self.payload_mut().set_ext(map); }
      removed
  }
  pub fn remove_seed_namespace(&mut self, card_kind: &str) -> Option<serde_json::Value> {
      let mut map = self.payload_mut().take_seed()?;
      let removed = map.remove(card_kind);
      if !map.is_empty() { self.payload_mut().set_seed(map); }
      removed
  }
  ```
  The two `store_*_namespace` bodies share the identical read-merge-depth-check-write skeleton; `store_seed_namespace` additionally validates the kind name up front (`validate_composable_kind`), which `store_ext_namespace` skips (namespaces are free-form). `Payload` already centralizes the `$ext`/`$seed` shape distinction behind the private `MetaKey`-keyed `meta()`/`set_meta()`/`take_meta()` trio (`payload.rs:384-476`) — `set_quill`/`set_kind`/`set_id` in the same file already route through one shared `upsert_meta` rather than four separate insertion-position implementations. The namespace merge/remove pair is the one place that pattern wasn't carried through to `edit.rs`.
- **Recommendation**: Bump `Payload::meta`/`set_meta`/`take_meta` to `pub(crate)` and write one private `Card` helper `merge_meta_namespace(key: MetaKey, ns: String, value: Value) -> Result<(), EditError>` and one `remove_meta_namespace(key: MetaKey, ns: &str) -> Option<Value>`; have the four public methods call through with an optional kind-validation step for the `Seed` case.
- **Est. LOC removable**: ~25
- **Confidence**: high
- **Risk if removed**: none functionally — same validation order and same public signatures preserved; only risk is a merge mistake reordering the `validate_composable_kind` check relative to `check_meta_depth` for the seed case.

### F2: `emit_field` and `emit_field_inline` duplicate the object/array/scalar dispatch structure
- **Category**: redundant-logic
- **Location**: `crates/core/src/document/emit.rs:430-517` (`emit_field`), `emit.rs:663-740` (`emit_field_inline`)
- **Evidence**: Both functions switch on the same four `JsonValue` shapes (empty object, non-empty object, empty array, non-empty array, scalar) and call the same children-emitters (`emit_mapping_children`, `emit_sequence_children`, `emit_scalar`) with the same `fill`-tag handling before that. The only structural difference is that `emit_field` additionally writes indentation and the key via `push_indent` + `emit_key_at` (which special-cases indent-0 top-level keys as unquoted), while `emit_field_inline` is called after the caller has already written `"- "` on the current line and so only writes the key via `emit_key` (always quoted, no `push_indent`). Diffing the two match bodies line-for-line, roughly 45 of ~60 body lines are identical modulo the key/indent prologue.
- **Recommendation**: Factor the shared match-on-value body into one function taking a closure/enum parameter for "how to write the key" (`Prefixed { indent, key }` vs `AlreadyOnDashLine { key }`), or thread a `bool inline_position` + `usize indent` pair through a single `emit_field` and have `emit_sequence_item`'s first-key branch call it with `indent = 0`-equivalent semantics. This is a genuine refactor, not a pure deletion — verify against the emit round-trip tests in `document/tests/emit_tests.rs` (out of this review's scope but load-bearing here) before landing.
- **Est. LOC removable**: ~55-65
- **Confidence**: medium (the two call sites differ in exactly how much of the line is already written, which is easy to get subtly wrong — e.g. `emit_field`'s indent-0 unquoted-key special case in `emit_key_at` must not leak into the inline path)
- **Risk if removed**: medium — this code underwrites the byte-stability/idempotence contract in `markdown-spec.md` §9; a consolidation bug would silently change canonical Markdown output for card fields nested inside sequence-item mappings (`- key: value` first-key case), which is exercised by both round-trip tests and fuzz tests outside this review's scope.

## Load-bearing (looks redundant, is not)

- **`dto.rs` (`CardV0_93_0`/`PayloadItemV0_92_0`/…) vs `wire.rs` (`CardWire`/`PayloadItemWire`)** — two near-parallel flat representations of a `Card`. Both module docs state the rationale explicitly: `dto.rs` is the *frozen-per-schema-version* storage envelope (`DOCUMENT_STORAGE.md`: "Frozen DTO per version... never changed once shipped"); `wire.rs` is the *current, evolvable* binding-API shape, deliberately decoupled so a bindings change doesn't force a storage schema bump or vice versa. Confirmed by the different serde policies each needs: `CommentPathSegmentV0_92_0` (dto.rs) is externally-tagged (frozen JSON shape for old rows); `PathStepWire` (wire.rs) is `#[serde(untagged)]` so it renders as a plain JS array element. Consolidating them would couple two independently-versioned surfaces.
- **`PayloadItem::Meta { key: MetaKey, .. }` unifying `$ext`/`$seed` in the live model, vs `dto.rs`/`wire.rs` each splitting back into separate `Ext`/`Seed` (or `ext`/`seed` field) representations** — looks like the unification buys nothing since every consumer un-unifies it. It does buy something: every non-serialization site that touches `$`-metadata (`meta_rank`, `meta_key`, the parser's `extract_meta_items` closed-key-set loop, `Payload::upsert_meta`'s single insertion-position algorithm) handles one `Meta` variant instead of two near-identical ones. Only the two wire boundaries, which must name the fields for external consumers anyway, re-split it — that's inherent to serialization, not evidence the in-memory unification is wasted.
- **Per-boundary card-`$id`/`$kind`/`$seed` validation duplicated across `assemble.rs` (parse: repairs), `edit.rs` (mutators: rejects), and `dto.rs` (storage: rejects)** — explicitly documented in `DOCUMENT_STORAGE.md` § Card-id identity as "caller-supplied ... parse repairs a violation, every other boundary rejects one." Parse is the lenient hand-authoring boundary (drops-with-warning); mutators and storage are strict machine boundaries (reject). These are three different policies over the same invariant, not three copies of the same check — collapsing them would change behavior at at least one boundary.
- **`prescan.rs`, `fences.rs`, `assemble.rs`, `yaml_hints.rs` "scanning" overlap** — these operate at different granularities and inputs: `fences.rs` is a line-oriented scan for `~~~`/`---` block *boundaries* over the whole document; `prescan.rs` recovers comments/`!must_fill` tags *within* one block's YAML content before handing cleaned text to `serde_saphyr`; `yaml_hints.rs` only runs after `serde_saphyr` has already rejected a block, re-scanning the raw content with looser heuristics (no quote-awareness, no block-scalar tracking) to name a concrete field for the error hint. `yaml_hints.rs`'s ad hoc `split_once(':')` line matching is *not* a duplicate of `prescan.rs`'s structural `split_key`/frame-tracking — the former is a best-effort heuristic over input that already failed to parse, and reusing the latter's stricter machinery there is neither necessary nor obviously correct on malformed input.
- **`Document::to_plate_json()` (schema-free, `pub`) with zero non-test call sites inside `crates/core`** — not dead code. `DOCUMENT_STORAGE.md` documents it as "a lossy, one-way export to Plate-shaped backends; it is core-only ... and never a storage option," and `docs/integration/persistence.md` and `docs/migrations/0.95-to-0.96.md` both point external Rust consumers of `quillmark-core` (that hold no `Quill` schema) at it directly. The schema-gated `to_plate_json_gated` is the one `QuillConfig::compile_data` (`quill/compose.rs`) actually calls internally; the two are intentionally different entry points for different callers (schema-free direct API vs. schema-aware render path), not redundant copies.
