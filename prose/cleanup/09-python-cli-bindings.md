# Cleanup review: python + cli bindings

Scope: `crates/bindings/python/src/{lib,types,errors,enums}.rs` (1713 LOC),
`crates/bindings/python/tests/*.py` (1727 LOC); `crates/bindings/cli/src/**`
(915 LOC, no tests exist). Read for context: `crates/quillmark/tests/*.rs`,
`crates/bindings/wasm/src/*.rs`, `crates/bindings/wasm/*.test.js`,
`prose/canon/BINDINGS.md`, `prose/canon/CLI.md`.

## Findings

### F1: Python re-asserts core engine semantics (`validate`, `seed_*`, name/backend errors) instead of marshalling
- **Category**: redundant-test
- **Location**: `crates/bindings/python/tests/test_validate.py:73-204` (whole file), `test_schema.py:137-195`, `test_quill.py:14-37`, `test_engine.py:6-18`, `test_render.py:55-75`
- **Evidence**: Line-by-line correspondence with Rust engine tests that already own these semantics:
  - `test_validate_returns_empty_list_for_clean_document` / `test_validate_forwards_type_mismatch` / `test_validate_reports_unknown_card_kind` (test_validate.py:73-109) duplicate `validate_clean_document_has_no_diagnostics` / `validate_forwards_type_mismatch_with_path_and_hint` / `validate_reports_unknown_card_kind` in `crates/quillmark/tests/validate_test.rs:46-92` — same fixture shape, same codes/paths asserted.
  - `test_seed_document_commits_examples` / `test_seed_main_and_card` / `test_seed_card_with_overlay_layers_over_example` / `test_seed_overlay_validation_is_advisory` (test_validate.py:127-204) duplicate `seed_main_commits_only_example_fields`, `seed_card_for_known_and_unknown_kind`, `seed_overlay_type_mismatch_is_advisory_and_does_not_gate_render` in `crates/core/src/quill/seed/tests.rs`.
  - `test_quill_from_path_bad_backend_loads_then_fails_at_render` (test_quill.py:14-37) and `test_metadata_never_raises_for_unregistered_backend` (test_engine.py) duplicate `test_unsupported_backend_errors_at_render_time` in `crates/quillmark/tests/quill_engine_test.rs:40-61`.
  - `test_engine_render_name_mismatch_errors` (test_render.py:55-75) duplicates `name_mismatch_is_a_hard_error` in `crates/quillmark/tests/version_mismatch_test.rs:80-89`.
  - All four are *also* re-tested a third time in WASM (`basic.test.js:452-477` name_mismatch; `:1479-1527` validate type_mismatch/unknown_card; `core.test.js:130-140` unregistered backend) — the same engine decision is asserted in three languages.
- **Recommendation**: Trim these to marshalling-only assertions: one call proving `quill.validate(doc)` returns a JSON-serializable list of dicts with the right keys (`test_validate_json_serializable` already does this — keep it), one call proving a Python exception carries `.diagnostics` with the right `.code` for a backend/name-mismatch error, one call proving `seed_document()`/`seed_card()` return the right dict shape. Delete the rest; the *decision logic* (which fields are Endorsed, which markers warn, which mismatches are hard errors) is core's contract, already covered by `crates/core` and `crates/quillmark/tests`.
- **Est. LOC removable**: ~180
- **Confidence**: high
- **Risk if removed**: none — these are internal test suites, not published API; core/WASM coverage of the same semantics is unaffected.

### F2: CLI `validate.rs` reinvents `Diagnostic`/`Severity` instead of reusing `quillmark_core`'s
- **Category**: redundant-logic
- **Location**: `crates/bindings/cli/src/commands/validate.rs:19-85` (local `Severity`, `ValidationIssue`, `ValidationResult`), used through `:108-297`
- **Evidence**: `quillmark_core::{Diagnostic, Severity}` already flow through this exact file — `config_warnings` at line 113 is `Vec<Diagnostic>` from `QuillConfig::from_yaml_with_warnings`. Line 130 downgrades each real `Diagnostic` (which carries `code`/`path`/`hint`) to a bare `ValidationIssue{ severity, message: String }`, discarding that metadata. The file's own errors (`result.add_error(format!(...))`) are built as bare strings too, then printed with ad hoc `eprintln!("[ERROR] {}", ...)` (line 277) instead of `Diagnostic::fmt_pretty()` — the formatting `crate::errors::print_warnings` (`errors.rs:67-76`) already uses for every other CLI diagnostic path, including `render`'s warnings.
- **Recommendation**: Replace `Severity`/`ValidationIssue`/`ValidationResult` with `Vec<quillmark_core::Diagnostic>`, appending real diagnostics (with `.with_code(...)` for the CLI-local checks) and printing them through the shared `fmt_pretty()`/`print_warnings` path. Collapses ~65 lines of parallel type definitions and unifies output formatting with the rest of the binary.
- **Est. LOC removable**: ~40 (net, after rewiring call sites to `Diagnostic`)
- **Confidence**: high
- **Risk if removed**: none — CLI stdout/stderr text isn't a documented contract (`CLI.md` doesn't pin exact wording) and no test asserts current output.

### F3: CLI `validate` re-parses `Quill.yaml` twice
- **Category**: redundant-logic
- **Location**: `crates/bindings/cli/src/commands/validate.rs:110-127` (manual `fs::read_to_string` + `QuillConfig::from_yaml_with_warnings`) and `:153` (`quillmark::quill_from_path`, which internally re-walks the directory and re-parses `Quill.yaml` via `Quill::from_tree` → `QuillConfig::from_yaml_with_warnings` again, `crates/core/src/quill/load.rs:45`)
- **Evidence**: `crates/quillmark/src/load.rs:17-25` → `Quill::from_tree` → `crates/core/src/quill/load.rs:45` calls `from_yaml_with_warnings` a second time on the same file, and explicitly drops the warnings (`let (config, _warnings) = ...`). The CLI's Step 1 exists only because that second call's warnings are unreachable any other way.
- **Recommendation**: Out of binding scope to fix cleanly (the real fix is upstream: have `quill_from_path`/`Quill::from_tree` return warnings alongside the `Quill`), but worth flagging — the CLI is compensating for a core API gap by doing the filesystem read + YAML parse twice on every `validate` invocation.
- **Est. LOC removable**: 0 today (removal requires a core signature change, out of scope for this binding-only review)
- **Confidence**: medium
- **Risk if removed**: n/a — informational; no action within `bindings/cli` alone resolves it without changing `quillmark_core`/`quillmark` signatures.

### F4: Python enum `.all()` / `.name` are unused surface
- **Category**: dead-code
- **Location**: `crates/bindings/python/src/enums.rs:19-37` (`py_enum!` macro emits `__repr__`, `.name`, `.all()` for every enum), instantiated at `:41-55` for `PyOutputFormat`/`PySeverity`
- **Evidence**: `grep -rn "\.all(" crates/bindings/python/{tests,examples,README.md}` and `prose/canon/*.md` — zero hits. `.name` likewise has zero hits outside the macro definition itself. `README.md` and every test reference enum members only by identity/equality (`OutputFormat.PDF`, `result.format == OutputFormat.SVG`).
- **Recommendation**: Drop the `.all()` staticmethod and `.name` getter from the `py_enum!` macro (keep `__repr__`, which is a normal Python debugging affordance even if untested). `.all()` in particular is a non-idiomatic bolt-on — Python callers would reach for `list(OutputFormat)`/`__members__`, not `.all()`.
- **Est. LOC removable**: ~10
- **Confidence**: medium
- **Risk if removed**: low but published-API: `quillmark` is on PyPI, so `OutputFormat.all()`/`.name` are technically public even though nothing in-repo calls them; a third party could depend on them. Deprecate-then-remove if that risk matters, or ship as a documented breaking change in the next minor.

### F5: `Quillmark.registered_backends()` has no Python test and no canon parity-table entry
- **Category**: surface-bloat
- **Location**: `crates/bindings/python/src/types.rs:84-90`
- **Evidence**: `grep -rn "registered_backends" crates/bindings/python/tests` → 0 hits. Documented only in `crates/bindings/python/README.md:63`. Not in the `prose/canon/BINDINGS.md` parity table (which enumerates every other engine/quill/document verb and states "Drift is a reviewable diff to this table"), and not exposed on WASM (`crates/bindings/wasm/src/engine.rs` has no `registered_backends`/`registeredBackends`). Core has it and tests it (`crates/quillmark/tests/backend_registration_test.rs`, `feature_flag_test.rs`).
- **Recommendation**: Not dead (README shows real usage), but untested and undocumented in canon — either add a one-line Python test and a parity-table row, or drop it from Python if it's not actually load-bearing for any consumer (WASM manages without it).
- **Est. LOC removable**: ~7 if dropped
- **Confidence**: low
- **Risk if removed**: published PyPI API; breaking change for any consumer using it (unverifiable from this repo).

### F6: `OutputWriter` — a 3-field struct with one call site
- **Category**: over-abstraction
- **Location**: `crates/bindings/cli/src/output.rs:7-50`, sole caller `crates/bindings/cli/src/commands/render.rs:173-174`
- **Evidence**: `grep -rn "OutputWriter" crates/bindings/cli/src` → defined once, constructed once. No other command writes output files/stdout.
- **Recommendation**: Inline as a free function `write_output(bytes, stdout, output_path, quiet) -> Result<()>` in `render.rs` (keep `derive_output_path`, which is a pure helper worth naming). Minor; not worth doing in isolation, bundle with any other render.rs touch.
- **Est. LOC removable**: ~10
- **Confidence**: low
- **Risk if removed**: none, private to the crate.

### F7: `test_to_markdown_ambiguous_string_survival` duplicates core's exhaustive YAML-quoting coverage
- **Category**: cross-language-duplicate-test
- **Location**: `crates/bindings/python/tests/test_api_requirements.py:470-509` (9 keyword variants: `on/off/yes/no/true/false/null/octal/date`)
- **Evidence**: The identical keyword set is exhaustively tested at the source in `crates/core/src/document/tests/ambiguous_strings_tests.rs` (the emitter behavior itself), and again in WASM's `basic.test.js:201-235` (`'ambiguous-string survival: YAML-keyword values are preserved as strings'`). Three layers assert the same 9-value table.
- **Recommendation**: Trim the Python (and WASM) copies to 1-2 representative keywords (e.g. `"on"`, `"2024-01-15"`) — enough to prove the binding's card-yaml writer path round-trips through the same emitter, without re-verifying the full keyword table core already owns exhaustively.
- **Est. LOC removable**: ~25 (Python side only, in scope here)
- **Confidence**: medium
- **Risk if removed**: none; core's emitter tests remain the source of truth for the quoting rule itself.

### F8: CLI schema-quality warnings (`validate.rs`) are untested, undocumented, and unique to CLI
- **Category**: speculative-feature
- **Location**: `crates/bindings/cli/src/commands/validate.rs:225-268` (`validate_field_schemas`, `validate_card_schema` — "enum constraint is empty" / "missing or empty description" warnings), `:181-216` (`validate_file_references` — `plate_file` existence/traversal check)
- **Evidence**: `crates/bindings/cli` has **no test directory and no dev-dependencies for testing at all** (`find crates/bindings/cli -iname "*test*"` → empty; `Cargo.toml` has no `[dev-dependencies]`). No script or CI workflow invokes any `quillmark` CLI subcommand (`grep -rn "quillmark validate\| quillmark render" scripts/ .github/` → empty). `CLI.md` documents `validate` only as "Validates quill configuration" / `-v` for "all validation details including warnings" — it doesn't contract these specific checks. Neither WASM nor Python implement the same advisory checks (missing-description warning, empty-enum warning) — this quality-linting logic exists in exactly one binding, deciding policy core doesn't enforce. `validate_file_references` is also Typst-specific (`plate_file`) with no pdfform-backend equivalent, hardcoding backend knowledge into the CLI.
- **Recommendation**: Either promote these checks into `quillmark_core` (so every binding gets consistent schema-quality linting) or drop them from the CLI — as written they're unverified, single-binding-only policy. Given zero consumers found, the pragmatic move is deletion; if the checks are wanted, they belong in `QuillConfig::from_yaml_with_warnings` where every surface (CLI/Python/WASM) already receives its output.
- **Est. LOC removable**: ~55 (`validate_field_schemas` + `validate_card_schema` + `validate_file_references` combined; `ValidationResult` plumbing accounted in F2)
- **Confidence**: medium
- **Risk if removed**: low — nothing in-repo depends on the specific warning text; `-v`/`--verbose`'s general contract ("show all validation details including warnings") survives on the `config_warnings` already forwarded from core.

## Load-bearing (looks redundant, is not)

- **`test_api_requirements.py` (728 LOC)** — despite the name (and being flagged as the prime suspect for a shape-test), it contains **no** `hasattr`/`dir`/`inspect.signature` assertions (`grep -rn "hasattr|dir(|inspect\." crates/bindings/python/tests/*.py` finds them only in `test_render.py`, and there they check `.diagnostics`/`.message` *content*, not mere existence). Every test in the file exercises real behavior through the pyo3 marshalling boundary (writer/reader typed commits, error codes, transactional rollback). Keep as-is; it is the load-bearing test of the Tier-1 writer/reader contract described in `prose/canon/BINDINGS.md`.
- **Per-binding error-mapping boilerplate** (`crates/bindings/python/src/errors.rs`, `crates/bindings/cli/src/errors.rs`, `crates/bindings/wasm/src/error.rs`) — looks like triplicated logic but the actual *message-selection* rule (`RenderError::summary_message`) is already centralized in `quillmark_core` and called identically from all three. What remains per-binding is only the unavoidable last mile: wrapping into a `PyErr`/JS `Error`/`eprintln!`, which differs by target language and cannot be shared further.
- **`PyWriter`/`PyCardWriter` and `PyReader`/`PyCardReader` near-duplicate method bodies** (`types.rs:676-1018`) — each `CardWriter`/`CardReader` method re-borrows `quill`/`doc` and re-derives a cursor rather than delegating to `Writer`/`Reader`; this mirrors core's own `TypedWriter`/`CardWriter` split (documented in `prose/canon/BINDINGS.md`'s parity table) and WASM's identical structure. Collapsing them would break the documented parity contract, not simplify it.
