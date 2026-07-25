# Cleanup review: wasm bindings

Scope: `crates/bindings/wasm/` — `src/engine.rs` (2895 LOC), `src/types.rs` (529),
`src/error.rs` (129), `src/lib.rs` (65), `tests/wasm_bindings.rs` (548),
`tests/common.rs` (25), `runtime/runtime.js` (931), `runtime/runtime.d.ts` (718),
`basic.test.js` (1940), `runtime.test.js` (867), `canvas.test.js` (402),
`core.test.js` (235), `test-helpers.js` (84), `README.md`, `Cargo.toml`,
`package.json`. Cross-referenced against `prose/canon/BINDINGS.md`,
`prose/canon/PREVIEW.md`, `crates/core/src/quill/blueprint.rs`,
`crates/core/src/document/tests/`, and `crates/quillmark/tests/`.

## Findings

### F1: JS tests re-assert schema/render engine semantics, not marshalling
- **Category**: low-value-test (engine-semantics-in-binding)
- **Location**: `crates/bindings/wasm/basic.test.js:1555-1940` — three `describe` blocks: `Unendorsed / Endorsed schema model` (1555-1727), `nested !must_fill` (1729-1786), `quill.resolve` (1786-1940). Also `basic.test.js:806-825` (`applyChange setContinues` hard-break semantics) and `basic.test.js:201-229` (YAML ambiguous-string emitter escaping).
- **Evidence**: These blocks assert blueprint text generation (`'title: !must_fill # string'`, exact-string pin), zero-fill render tolerance, and `validation::must_fill` warning codes — none of it depends on the WASM boundary; every assertion is reachable by calling the same Rust APIs directly. Confirmed duplicate coverage: `crates/core/src/quill/blueprint.rs` has its own unit tests for `!must_fill` marker rendering (`must_fill_markdown_example_surfaces_as_eg_hint_not_inline_value`, `must_fill_array_example_renders_as_block_sequence_with_context_quoting`); `crates/quillmark/tests/default_values_test.rs::test_absent_must_fill_is_zero_filled` asserts the identical "absent Unendorsed field zero-fills" behavior; `crates/quillmark/tests/validate_test.rs::validate_warns_on_must_fill_marker` asserts the identical `validation::must_fill` warning. `quill.resolve` (field-provenance resolution) mirrors `crates/core/src/quill/resolved.rs`'s own logic and is not mentioned anywhere in `prose/canon/BINDINGS.md` or `PREVIEW.md` (undocumented binding surface, tested only here).
- **Recommendation**: Cut these three blocks down to one shallow test each confirming the value crosses the WASM boundary correctly (e.g., `resolve()` returns a JS array with the right shape; `!must_fill` render doesn't throw). Delete the exact-string blueprint-text pins and the multi-case validate/zero-fill matrix — that belongs to `crates/core`/`crates/quillmark`, which already have it.
- **Est. LOC removable**: ~300 (of ~386 total in the three blocks)
- **Confidence**: high
- **Risk if removed**: none — test-only change, no public API touched.

### F2: `wasm_bindings.rs` triple-duplicates render/session/region tests
- **Category**: cross-language-duplicate-test
- **Location**: `crates/bindings/wasm/tests/wasm_bindings.rs:48-59` (`test_render_name_mismatch_errors`), `:63-81` (`test_render_from_document`), `:122-135` (`test_open_session_render`), `:155-221` (`test_session_regions_and_navigation`, 67 lines), `:224-240` (`test_to_markdown_round_trip`), `:377-397` (`test_quill_seed_document`).
- **Evidence**: Each asserts real Typst-compile/engine semantics (artifact counts, page counts, region/geometry correctness) rather than JsValue↔Rust marshalling. `test_render_name_mismatch_errors` duplicates `crates/quillmark/tests/version_mismatch_test.rs`'s name-mismatch coverage. `test_session_regions_and_navigation` (the largest, 67 lines) duplicates the geometry assertions already exercised end-to-end in `crates/quillmark/tests/usaf_memo_regions_test.rs` and `cmu_letter_date_region_test.rs`, *and* is re-asserted a third time in JS: `basic.test.js:434-450` (`session.regions() is always a non-null array, keyed by DocPath`) and `canvas.test.js:127-159` (`positionAt`/`locate` round-trip) cover the same region/navigation ground. `test_render_from_document` / `test_open_session_render` duplicate `basic.test.js:380-432` and `:1294-1315`.
- **Recommendation**: Keep one minimal render + one minimal `open` test (smoke-level: doesn't throw, produces bytes/pages) to confirm the compiled wasm32 binary works end-to-end in a real browser runner (`wasm_bindgen_test_configure!(run_in_browser)` — a guarantee the node-based `vitest` suite does not give). Delete the geometry-assertion body of `test_session_regions_and_navigation` and the artifact-count assertions duplicated in JS; keep only the `serde_wasm_bindgen` deserialize-shape checks.
- **Est. LOC removable**: ~110 (of ~150 across the six tests)
- **Confidence**: high
- **Risk if removed**: low — test-only; note the `run_in_browser` execution path has narrow independent value (real-browser wasm32 execution vs. node/vitest), so don't delete the file, just its semantic-duplicate assertions.

### F3: `DocumentWriter`/`CardWriter` tests re-run the full `_commitField` error matrix
- **Category**: redundant-test
- **Location**: `crates/bindings/wasm/runtime.test.js:130-268` (`DocumentWriter / CardWriter` describe, 138 lines) vs. `crates/bindings/wasm/basic.test.js:717-905` (`Document typed-commit ABI — _commitField / _commitFields`, 189 lines).
- **Evidence**: `runtime.js:623-625` shows `DocumentWriter.set` is a one-line delegation (`this.#doc._commitField(this.#quill, name, value)`). Both test files assert the identical schema-resolution error codes on the identical underlying call: `edit::unknown_field` (`runtime.test.js:169-173` vs. `basic.test.js:756-764`), `edit::field_conform`/`edit::field_richtext_not_inline` (`runtime.test.js:196-204` vs. `basic.test.js:779-787`), `edit::index_out_of_range` (`runtime.test.js:243-248` vs. `basic.test.js:834`). Only the call-site differs (`ed.set(...)` sugar vs. `doc._commitField(quill, ...)` ABI).
- **Recommendation**: Keep `basic.test.js`'s ABI-level matrix as the source of truth for every error code. Shrink `runtime.test.js`'s block to verify delegation only — one test per verb (`set`/`setAll`/`setBody`/`reviseField`/`addCard`/`card(i)`) confirming it forwards to the right ABI call with the right address, plus one representative throw to prove errors propagate through the JS wrapper unchanged.
- **Est. LOC removable**: ~80
- **Confidence**: high
- **Risk if removed**: none — test-only.

### F4: Render-format cluster is a table-driven test written out by hand
- **Category**: test-consolidation
- **Location**: `crates/bindings/wasm/basic.test.js:352-432` (`Quillmark.quill` describe, five `it` blocks: `:353`, `:358`, `:380`, `:397`, `:411`, `:422`).
- **Evidence**: `:380-395`, `:397-409`, `:411-420`, `:422-432` differ only by the `format` option and expected `mimeType` (`application/pdf` vs. `image/svg+xml`); each repeats the same `Quill.fromTree` → `Document.fromMarkdown` → `engine.render` → assert-artifacts-non-empty scaffold.
- **Recommendation**: Replace with one `it.each([['pdf', 'application/pdf'], ['svg', 'image/svg+xml']])` case plus the one genuinely distinct assertion (default-format-is-pdf, multi-render-reuses-doc).
- **Est. LOC removable**: ~40
- **Confidence**: medium-high
- **Risk if removed**: none — test-only.

### F5: Typst/pdfform canvas paint tests duplicate their DPR-math assertion body
- **Category**: test-consolidation
- **Location**: `crates/bindings/wasm/canvas.test.js:161-209` (typst `paint sizes the canvas...`, 49 lines) vs. `:310-351` (pdfform `paint sizes the canvas per the DPR math...`, 42 lines).
- **Evidence**: Both compute `layoutWidth`/`layoutHeight`/`pixelWidth`/`pixelHeight` from the same `layoutScale`/`densityScale` formula and both do an identical ink/opaque-pixel scan (`for (let i = 0; i < call.data.length; i += 4) { ... }`, byte-identical loop body at `:200-206` and `:344-348`). Only the session setup and precision tolerance (`toBeCloseTo(..., -1)` for pdfform's rounding) differ.
- **Recommendation**: Factor the DPR-math + ink-pixel-scan assertions into one shared helper (e.g. `assertPaintedPage(result, ctx, widthPt, heightPt, opts)`) parametrized on the loose pdfform tolerance; call it from both backend tests. Per-backend coverage stays (genuinely different rasterizers), only the assertion body is deduplicated.
- **Est. LOC removable**: ~40
- **Confidence**: medium
- **Risk if removed**: none — test-only; keep both call sites, only extract the shared body.

### F6: `wasm_bindings.rs` JSON-DTO tests duplicate `basic.test.js` and core serde guarantees
- **Category**: cross-language-duplicate-test
- **Location**: `crates/bindings/wasm/tests/wasm_bindings.rs:245-263` (`test_json_dto_round_trip`), `:267-284` (`test_json_dto_drops_parse_warnings`), `:288-300` (`test_json_dto_rejects_invalid_input`) vs. `crates/bindings/wasm/basic.test.js:236-350` (`Document JSON DTO — toJson / fromJson`, 115 lines, same four behaviors: round-trip equality, dropped parse warnings, unknown-schema rejection, malformed-JSON rejection).
- **Evidence**: `toJson`/`fromJson` cross the WASM boundary as a plain `string` (`engine.rs:889-895`, `:790-799`) — there is no JsValue shape to verify beyond "it's a string that round-trips," which `serde_json`'s own round-trip guarantee (exercised in `crates/core/src/document/tests/`) already covers. The JS suite is the one a real consumer exercises; the Rust copy adds no boundary-specific signal.
- **Recommendation**: Delete the three Rust tests; keep the JS coverage (it also exercises the JS-visible `Document.tryFromJson`/`schemaVersionOf` pair that Rust doesn't).
- **Est. LOC removable**: ~50
- **Confidence**: medium
- **Risk if removed**: low — test-only; the Rust suite loses its only DTO coverage, but the same binary's DTO logic is core logic (not `wasm`-crate code) and stays covered by `crates/core`.

### F7: Truthy-only / no-shape assertions scattered across the suite
- **Category**: low-value-test
- **Location**: `basic.test.js:352-356` (`expect(quill).toBeDefined()` and nothing else), `runtime.test.js:193` and `:239` (`expect(delta).toBeTruthy()`, no `.ops` shape check — contrast the stricter `basic.test.js:616` `expect(Array.isArray(delta.ops)).toBe(true)`), `canvas.test.js:120` (`expect(Array.isArray(session.warnings)).toBe(true)`, content never checked), and the `JSON.stringify-able` cluster at `basic.test.js:1397-1406`, `:1518-1530`, `:1929-1939` (~34 lines, three near-identical "doesn't throw when stringified" tests).
- **Evidence**: verified directly by reading each cited range; each asserts only that a value exists/is an array/doesn't throw, with no assertion on its actual content.
- **Recommendation**: Either delete (the surrounding describe block already exercises the same call with real assertions elsewhere) or strengthen to check the shape once and drop the other two `JSON.stringify` duplicates.
- **Est. LOC removable**: ~50
- **Confidence**: medium
- **Risk if removed**: none — test-only.

### F8: Untested static passthrough methods on `Document`
- **Category**: surface-bloat
- **Location**: `crates/bindings/wasm/src/engine.rs:841` (`formatRules`), `:850` (`blueprintInstruction`), `:860` (`quillRefHint`), `:868` (`formatDiagnostic`); also `storeFill` (`:1197`), `isFill` (`:1060`), `getExt`/`getExtNamespace` (`:1077`, `:1094`).
- **Evidence**: grepped all five JS test files plus `wasm_bindings.rs` for each identifier — zero matches for any of these seven names in any test file. `storeFill`/`isFill`/`getExt`/`getExtNamespace` are documented in `prose/canon/BINDINGS.md`'s parity table (real, intentional API — not bloat, just untested). `formatRules`/`blueprintInstruction`/`quillRefHint`/`formatDiagnostic` are mentioned only in `crates/bindings/python/src/types.rs` doc comments (as "mirrors WASM") and, for `quillRefHint` only, in `prose/canon/ERROR.md` — none appear in `BINDINGS.md`/`PREVIEW.md`, and none has a JS consumer in this repo.
- **Recommendation**: Not a removal candidate — these are cheap, documented-as-single-source-of-truth re-exports of core constants (LLM/MCP-consumer-facing authoring text), and removing published API is a breaking change for unclear benefit. Add one consolidated smoke test asserting all four static text methods return non-empty strings, and one for the four untested address-axis reads (`storeFill`/`isFill`/`getExt`/`getExtNamespace`), rather than leaving them at zero coverage.
- **Est. LOC removable**: 0 (this is a coverage gap, not excess code)
- **Confidence**: low
- **Risk if removed**: high if actually removed — published npm API, and `formatRules`/`quillRefHint`/`blueprintInstruction` are the documented single-source-of-truth text for LLM/MCP authoring flows per their own doc comments.

## Load-bearing (looks redundant, is not)

- **`js_to_card` unknown-key check** (`engine.rs:2214-2236`) and **`Addr::from_js` unknown-key check** (`engine.rs:1809-1828`) — look like hand-rolled validation duplicating `#[serde(deny_unknown_fields)]` already on `quillmark_core::CardWire`, but the comment at `engine.rs:2209-2213` and the code confirm `serde_wasm_bindgen::from_value` does not honor `deny_unknown_fields` (it looks up known keys rather than visiting every key). This is compensation for a real deserializer gap, not redundant logic — removing it would silently accept a swapped-argument call (e.g. `storeFields(fields, {})`) as an empty write instead of throwing.
- **`paint()`'s DPR/backing-store clamp math** (`engine.rs:2660-2768`, `MAX_BACKING_DIMENSION` at `:380`) — looks like business logic leaking into a binding, but `prose/canon/PREVIEW.md` explicitly documents this as WASM/canvas-owned logic (browser backing-store limits have no equivalent in `quillmark-core`); confirmed no duplicate implementation exists elsewhere in the workspace.
- **`DocumentWriter`/`CardWriter`/`DocumentReader`/`CardReader`** (`runtime/runtime.js:601-931`) — look like over-abstraction (thin wrapper classes forwarding to `_commitField`/`_readerGet`), but `prose/canon/BINDINGS.md` states explicitly: "The hand-written runtime is the real API; the wasm class is its ABI." These are the documented, intentional public surface (`quill.writer(doc)`/`quill.reader(doc)`), not incidental wrapping — the underscored ABI they wrap is itself `skip_typescript` and hidden from consumers.
- **`Quill::metadata()`'s hand-assembled `serde_json::Map`** (`engine.rs:616-657`) — looks like binding-owned business logic duplicating a "should be core" concern, but `crates/core/src/quill.rs`'s doc comment on `STANDARD_METADATA_KEYS` states this identity-snapshot assembly is deliberately each binding's job (Python re-implements the same pattern independently). Not fixable within this crate's scope alone.
- **`core.test.js`'s overlap with `basic.test.js`** on DTO round-trip / metadata assertions (`core.test.js:39-138`, `:143-234`) — looks like duplicate test content, but `core.test.js` targets a *different WASM binary* (the Typst-less `pkg/core/` build, per `vitest.config.js`'s `@quillmark-wasm/core` alias) — re-verifying the same behavior compiles and runs correctly in the smaller, engine-free artifact is the point of the file, not redundancy.
- **Cargo feature split** (`Cargo.toml:40-59`, three artifacts — core/typst/pdfform — from one crate) — looks like it could be simplified to one build, but is the documented, deliberate multi-artifact packaging strategy (`prose/canon/BINDINGS.md` § WebAssembly, `docs/migrations/0.89-to-0.90.md`) that keeps the editor/validation path from paying for an 8 MB Typst binary.
