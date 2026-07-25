# Cleanup review: typst backend

Scope: `crates/backends/typst/` — `src/overlay/span_scan.rs` (2101 LOC),
`src/emit.rs` (1759), `src/lib.rs` (1235), `src/helper.rs` (1029),
`src/world.rs` (621), `src/overlay/extract.rs` (247), `src/overlay/mod.rs`
(220), `src/error_mapping.rs` (224), `src/compile.rs` (192),
`src/lib.typ.template` (172); tests `tests/content_regions.rs` (1237),
`tests/sig_field.rs` (570), `tests/live_apply.rs` (302),
`tests/producer_meta.rs` (188), `tests/overlap_compile.rs` (93),
`tests/plate_resolution.rs` (65). ~7630 LOC total. Read
`prose/canon/CONVERT.md`, `PLATE_DATA.md`, `CARDS.md` first; verified no
finding below contradicts documented design (segment source map, value-object
dates, `$path`/`$cards` addressing, plaintext projection).

## Findings

### F1: `walk(dir) -> FileTreeNode` and `quill(yaml, plate) -> Quill` test helpers copy-pasted across 7 files
- **Category**: duplicate-helper
- **Location**:
  - `src/world.rs:569-587` (`test_asset_fonts_have_priority`'s local `fn walk`)
  - `src/error_mapping.rs:81-100` (`walk_fixture`/`fn walk`)
  - `tests/sig_field.rs:18-49` (`host_tree`/`fn walk`)
  - `tests/producer_meta.rs:16-47` (`host_tree`/`fn walk`)
  - `src/overlay/span_scan.rs:919-934` (test-mod `fn quill(yaml, plate)`)
  - `tests/content_regions.rs:21-36` (`fn quill(yaml, plate)`)
  - `tests/overlap_compile.rs:15-30` (`fn quill(yaml, plate)`)
- **Evidence**: `grep -n "fn walk(dir: &Path)"` matches all four `walk` sites
  byte-for-byte identical (recursive directory → `FileTreeNode` walk, same
  `fs::read_dir`/`FileTreeNode::File`/`FileTreeNode::Directory` shape). The
  `quill(yaml, plate)` helper (two-file `Quill.yaml` + `plate.typ` →
  `Quill::from_tree`) is byte-for-byte identical in `span_scan.rs`,
  `content_regions.rs`, and `overlap_compile.rs`; `live_apply.rs` and
  `plate_resolution.rs` inline near-variants of the same shape. No shared
  helper exists in `quillmark_core` or `quillmark_fixtures` for either
  (`fixtures::quills_path` returns a filesystem `Path`, not an in-memory
  `FileTreeNode`, and isn't usable from `quillmark-typst` since `quillmark`
  depends on `quillmark-typst`, not the reverse).
- **Recommendation**: add `tests/common/mod.rs` (not auto-run as a test
  binary) holding `walk_fixture_tree` and `quill(yaml, plate)`; have
  `sig_field.rs`, `producer_meta.rs`, `content_regions.rs`,
  `overlap_compile.rs` (and `live_apply.rs`/`plate_resolution.rs` where the
  shape matches) `mod common;` it. For the two `src/`-internal copies
  (`world.rs`, `error_mapping.rs` test mods), factor a `#[cfg(test)]
  pub(crate) fn` in one place (e.g. a `test_support` module) both can import.
- **Est. LOC removable**: ~110
- **Confidence**: high
- **Risk if removed**: none functionally — pure mechanical dedup of
  test-only scaffolding; watch for the two call sites that pass an extra
  arg (`plate_resolution.rs`'s `files: &[(&str, &[u8])]` variant) needing a
  slightly wider shared signature.

### F2: `#990` "spike" tests in `span_scan.rs` pin raw Typst language behavior, not quillmark's code — and duplicate rationale already in `PLATE_DATA.md`
- **Category**: low-value-test
- **Location**: `src/overlay/span_scan.rs:1817-1849` (`compile_frame_text` helper,
  used only by the tests below), `:1854-1858` (`VALUE_OBJECT` const),
  `:1880-1915` (`spike_990_dict_display_needs_parens_then_forwards_args`),
  `:1937-1958` (`spike_990_native_datetime_rejects_the_paren_display_grab`),
  `:2050-2068` (`spike_990_none_guard_is_invariant_across_value_object_and_blank`)
- **Evidence**: These three tests build hand-rolled Typst source strings
  (`#d.display("[year]")`, `#(dt.display)(..)`, `datetime(..) != none`) that
  never call `helper::generate_lib_typ`, `emit::emit_content`, or the
  `Classifier`/`span_scan` machinery — they compile bare Typst snippets and
  assert on Typst's own error text (`"cannot directly call dictionary keys as
  functions"`, `"cannot access fields on type datetime"`) or trivially-true
  boolean identities (`datetime(..) != none`, `none != none`). The design
  conclusion they're pinning — the value-object's `display` closure needs the
  paren-call form, and why — is already written up in
  `prose/canon/PLATE_DATA.md:48-51` ("called as `(data.<field>.display)(..)`
  — the paren form, since Typst reserves dict-key method sugar"). The actual
  shipped codegen shape (the value-object block, its paren-call consumption)
  is independently pinned by real production-path tests:
  `helper.rs::date_and_datetime_fields_become_value_object_blocks` (drives
  `generate_lib_typ`) and `content_regions.rs::date_field_display_surfaces_a_clickable_region`
  / `card_dates_surface_per_instance_regions_through_laundering` (drive
  `Backend::open` end-to-end through the real `(data.field.display)(..)`
  call). By contrast `spike_990_text_over_programmatic_string_classifies_into_recorded_window`
  (`:1979-2043`, kept) does exercise the real `Classifier`/`scan` and is not
  part of this finding.
- **Recommendation**: delete the three tests plus `compile_frame_text` and
  `VALUE_OBJECT` (dead once their only callers are gone). If the Typst-upstream
  canary value is wanted, a one-line note in `PLATE_DATA.md` ("verified
  against Typst 0.15; re-check on a Typst major bump") captures the same
  intent without ~160 lines of test code that fails on Typst's *error message
  wording*, which is itself brittle to Typst version bumps in a way that
  teaches nothing about a quillmark regression.
- **Est. LOC removable**: ~160
- **Confidence**: medium
- **Risk if removed**: loses an explicit canary for "did Typst's dict/datetime
  method-sugar rules change" on a Typst upgrade; if that's still wanted, keep
  one minimal smoke test rather than all three narrations.

### F3: Duplicate font-loading loop in `world.rs`
- **Category**: duplicate-helper
- **Location**: `src/world.rs:184-221` (`load_fonts_from_quill`), specifically
  the two loops at `:191-203` (`assets/fonts/*`) and `:207-218`
  (`packages/**`)
- **Evidence**: Both loops are the identical body — filter by
  `ttf|otf|woff|woff2` extension, `source.get_file`, push bytes — differing
  only in the glob pattern passed to `find_files`.
- **Recommendation**: factor a `fn collect_font_files(source, pattern) ->
  Vec<Vec<u8>>` and call it twice, or fold both patterns into one
  `find_files` call if the glob syntax supports alternation.
- **Est. LOC removable**: ~15
- **Confidence**: high
- **Risk if removed**: none — straight extraction, same two glob patterns,
  same load order (asset fonts still enumerated before package fonts).

### F4: `overlay/mod.rs`'s five re-export functions are single-caller pass-throughs to `span_scan`
- **Category**: over-abstraction
- **Location**: `src/overlay/mod.rs:27-85` (`scan_content_regions`,
  `field_at`, `position_at`, `locate`, `scalar_windows`)
- **Evidence**: Each function's body is exactly one call to the
  identically-shaped `span_scan::*` function with the same arguments in the
  same order (e.g. `scan_content_regions` → `span_scan::scan`, `field_at` →
  `span_scan::field_at`). `grep -rn "overlay::(scan_content_regions|field_at|position_at|locate|scalar_windows)" src/lib.rs`
  shows exactly one call site per function, all from `lib.rs`. `mod span_scan;`
  is private to `overlay`, so these exist purely to re-expose it one level up.
- **Recommendation**: mark `mod span_scan` as `pub(crate)` and have `lib.rs`
  call `overlay::span_scan::scan(..)` etc. directly, or re-export the four
  span_scan functions with `pub(crate) use span_scan::{scan as
  scan_content_regions, field_at, position_at, locate, scalar_windows};`
  instead of hand-written forwarding bodies. Either removes the wrapper
  bodies while keeping the module boundary.
- **Est. LOC removable**: ~45
- **Confidence**: medium
- **Risk if removed**: low — mechanical; the per-function doc comments
  restating "see `span_scan::X`" would need to move onto the `span_scan`
  functions themselves (some already have overlapping doc comments there).

### F5: `field_only_ink_is_transparent_but_foreign_ink_is_not` / `anonymous_ink_is_transparent_but_foreign_ink_is_not` are structurally identical tests
- **Category**: test-consolidation
- **Location**: `src/overlay/span_scan.rs:1461-1491` and `:1500-1529`
- **Evidence**: Both tests build the same two-hit-with-a-gap fixture
  (`boxable_hit(0, key, a)`, a middle hit, `boxable_hit(0, key, b)`), run
  `run_scan_machine`, and assert the same two outcomes (transparent/anonymous
  ink unions the run; a `foreign_hit` in the identical slot truncates it).
  They differ only in which `HitClass` variant (`Transparent` vs `Anonymous`)
  sits in the middle slot — a real distinction (two different match arms in
  `run_scan_machine`), but the boilerplate (fixtures, assertions, the
  `foreign_hit` control case) is duplicated rather than shared.
- **Recommendation**: parametrize one test over `[HitClass::Transparent{window:0}, HitClass::Anonymous]`,
  keeping the `foreign_hit` control case shared once.
- **Est. LOC removable**: ~20
- **Confidence**: low
- **Risk if removed**: none functionally; purely a readability/DRY call — both
  code paths stay covered either way.

## Load-bearing (looks redundant, is not)

- **`tests/content_regions.rs` (1237 LOC)** — large, but not brittle: every
  assertion is geometric/structural (region counts, positive area,
  `field_at`/`position_at` round-trips, page ordering) rather than pinned to
  an exact generated string. Each test targets one documented invariant
  (#829 segment striping, #936 decoration-mark truncation, #990 date
  click-targets, first-placement-vs-`field_at` divergence, card kind+ordinal
  addressing, scalar multi-site tracking) not covered elsewhere at this
  layer. Do not read its size as a smell.
- **`emit.rs`'s exact-string-pinned unit tests** (`coincident_strong_emph_nests_canonically`,
  `overlapping_marks_close_and_reopen`, `wrap_over_atomic_code_stays_balanced`,
  `line_anchor_paragraph_prefixes_backslash`, etc.) — these pin the mark-sweep
  algorithm's actual output contract (nesting order, bracket balance,
  escape placement), not incidental formatting; a change here without an
  intentional design change is the regression the test exists to catch.
- **`overlay::build_field_specs`'s duplicated value-coercion of pdfform's
  resolver** (`overlay/mod.rs:142-220`) — explicitly documented as
  intentional: "duplicated rather than shared because this crate must NOT
  depend on `quillmark-pdfform` — the two backends meet only at the
  `&[FieldSpec]` seam."
- **`SchemaMeta`'s five parallel per-card-kind tables** (`content`, `date`,
  `datetime`, `array`, `inline` fields, `lib.rs:762-769`) — feed directly,
  unmodified, into the generated `_qm-meta` literal (`helper.rs::meta_literal`)
  and the per-card codegen dispatch; each table backs a distinct
  classification the codegen needs at a different point, not five copies of
  one idea.
- **`world.rs`'s `#[cfg(not(target_arch = "wasm32"))]` vs `#[cfg(target_arch = "wasm32")]` `today()` bodies** —
  platform-exclusive at compile time (native `time` crate vs. `js-sys` Date),
  cannot be merged into one code path.
- **`emit::emit_content` / `emit::emit_content_inline`'s shared-looking but
  distinct block-vs-inline paths** — not a redundant pair: `emit_content_inline`
  omits the block terminator (`\n\n`/`parbreak`) `emit_content` always emits,
  which is the entire point of #872 (no "parbreak may not occur inside of a
  paragraph" warning). The fallback-to-block arm for non-`is_inline` content
  is real defensive coverage for hand-built content, not dead code (tested by
  `inline_falls_back_to_block_for_non_inline_content`).
