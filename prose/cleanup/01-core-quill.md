# Cleanup review: core/quill

Scope: `crates/core/src/quill.rs` (123 LOC) + `crates/core/src/quill/{config,blueprint,validation,types,compose,resolved,schema,schema_yaml,fill,formats,ignore,load,query,tree}.rs` and `seed.rs`/`seed/tests.rs` (~11,545 LOC total across the module, per `wc -l`) and the in-module `tests.rs` (3468 LOC) and `seed/tests.rs` (390 LOC). Read in full. Cross-checked against `prose/canon/QUILL.md`, `SCHEMAS.md`, `BLUEPRINT.md`, and grepped the whole workspace (`crates/bindings/*`, `crates/fuzz`, `crates/fixtures`, `crates/backends/*`, `crates/quillmark`, `docs/`, `prose/`) before calling anything dead.

## Findings

### F1: Body-example fence-detection tests duplicate `document/tests/card_fence_tests.rs`
- **Category**: redundant-test
- **Location**: `crates/core/src/quill/tests.rs:2677-2854` (8 tests: `body_example_with_card_yaml_fence_line_is_an_error`, `body_example_indented_fence_line_is_not_an_error`, `body_example_four_leading_spaces_is_not_a_fence`, `body_example_bare_triple_dash_is_not_a_fence`, `body_example_bare_tilde_fence_line_is_an_error`, `body_example_four_tilde_fence_is_an_error`, `body_example_backtick_fence_is_allowed`, `body_example_card_yaml_fence_line_in_card_kind_is_an_error`)
- **Evidence**: `example_contains_fence_line` (`config.rs:1834`) is a one-line wrapper over `crate::document::fences::is_card_yaml_opener_line` (`document/fences.rs:152`), explicitly "so the guard stays in lock-step with fence detection." That predicate's full grammar — bare `~~~`, longer tilde runs, indented openers, backtick escape hatch, language-tagged fences — is already exhaustively covered by dedicated tests in `crates/core/src/document/tests/card_fence_tests.rs` (`bare_tilde_fence_opens_a_card_yaml_block`, `longer_tilde_run_still_opens_a_card`, `indented_tilde_opener_is_not_a_card`, `backtick_fence_is_the_code_block_escape_hatch`, `tilde_fence_with_language_info_is_an_ordinary_code_block`, etc. — grep-verified, `card_fence_tests.rs:110-302`). The 8 tests in `quill/tests.rs` re-walk the same character-level grammar (bare tilde, four-tilde, indented, four-space-indented, backtick, bare `---`) through the `QuillConfig::from_yaml` pipeline instead of asserting only that the wiring is correct.
- **Recommendation**: Keep one positive case (a bare `~~~` fence in a body example → `quill::body_example_contains_fence`) and one negative case (backtick fence → OK) to prove the wiring; delete the other 6, which re-verify the fence grammar itself rather than the quill-load integration point. `body_example_with_body_disabled_emits_warning` (line 2650, a different code path — `quill::body_example_unused`) is unrelated and must stay.
- **Est. LOC removable**: ~140
- **Confidence**: high
- **Risk if removed**: none — the underlying grammar keeps its dedicated coverage in `document/tests/card_fence_tests.rs`; only the duplicated re-assertion of that grammar through a second entry point goes away.

### F2: Four near-identical snake_case-rejection tests
- **Category**: test-consolidation
- **Location**: `crates/core/src/quill/tests.rs:783-886` (`test_quill_config_rejects_non_snake_case_quill_name`, `test_quill_config_rejects_non_snake_case_card_name`, `test_quill_config_rejects_non_snake_case_main_field_keys`, `test_quill_config_rejects_non_snake_case_card_field_keys`)
- **Evidence**: All four build a minimal YAML with one bad identifier in a different slot (`quill.name`, `card_kinds.<X>`, `main.fields.<X>`, `card_kinds.<k>.fields.<X>`), call `QuillConfig::from_yaml`, and assert `is_err()` plus the message contains the bad identifier and (for three of the four) `"snake_case"`. Identical assertion shape, only the YAML location and expected-substring vary.
- **Recommendation**: Collapse into one test iterating `[(yaml_snippet, bad_identifier), ...]` and asserting the same two conditions per case.
- **Est. LOC removable**: ~70
- **Confidence**: high
- **Risk if removed**: none — same four YAML shapes remain covered, just table-driven.

### F3: Five example/default type-mismatch tests share one assertion shape
- **Category**: test-consolidation
- **Location**: `crates/core/src/quill/tests.rs:2897-2978` (`example_integer_type_rejects_float_example`, `example_string_type_rejects_unquoted_decimal_example`, `example_string_type_accepts_quoted_decimal_example`, `example_boolean_type_rejects_string_example`, `example_array_type_rejects_string_example`)
- **Evidence**: Each builds one field via the shared `example_default_yaml` helper, calls `from_yaml_with_warnings`, and either asserts an `Err` containing `quill::example_type_mismatch` with the field/expected/actual type names in the message, or (one case) asserts `Ok`. Same helper, same lookup-and-assert pattern, differing only in `(type, example_literal, expect_ok)`.
- **Recommendation**: Fold into one table-driven test over `(field_yaml, expect_ok, expected_substrings)`. `datetime_type_mismatch_reports_datetime_not_string` (3026) and `richtext_type_mismatch_reports_richtext_not_string` (3045) test a different concern (the message names the declared type verbatim, not the internal string-family collapse) and are worth keeping separate.
- **Est. LOC removable**: ~45
- **Confidence**: high
- **Risk if removed**: none — same type/value combinations stay covered.

### F4: Dead code — `Quill::list_files` / `Quill::list_subdirectories`
- **Category**: dead-code
- **Location**: `crates/core/src/quill/query.rs:22-25` (`list_files`) and `:27-30` (`list_subdirectories`)
- **Evidence**: `grep -rn "\.list_files(\|\.list_subdirectories("` across the whole workspace (`crates/`, including `bindings/{python,wasm,cli}`, `backends/*`, `fuzz`, `fixtures`, `quillmark`) returns hits only in `crates/core/src/quill/query.rs` itself and `crates/core/src/quill/tests.rs`. `Quill::list_directories` (used by `crates/backends/typst/src/world.rs:260`) calls `self.files.list_subdirectories(...)` directly on the `FileTreeNode`, **not** through the `Quill::list_subdirectories` wrapper — so the wrapper method is unreachable from any production call site. `find_files` and `list_directories`, by contrast, are genuinely used (`backends/typst/src/world.rs:191,206,229,260,334`) and must stay.
- **Recommendation**: Remove the two `pub fn` wrappers from `query.rs`; trim the corresponding assertions out of `test_dir_exists_and_list_apis` (`tests.rs:385-487`), keeping the `dir_exists`/`file_exists`/`list_directories` coverage.
- **Est. LOC removable**: ~20 (8 in query.rs, ~12 of test assertions)
- **Confidence**: high
- **Risk if removed**: none found — no binding surface, doctest, or fixture references either method by name.

### F5: `test_quill_config_rejects_root_level_fields` is subsumed by a stronger test
- **Category**: redundant-test
- **Location**: `crates/core/src/quill/tests.rs:766-781` vs. `crates/core/src/quill/tests.rs:2443-2480` (`test_root_level_fields_gets_targeted_hint`)
- **Evidence**: Both feed the exact same "root-level `fields:` instead of `main.fields:`" YAML shape and assert essentially the same outcome. `test_quill_config_rejects_root_level_fields` checks only `is_err()` and that the message contains `"main.fields"`. `test_root_level_fields_gets_targeted_hint` (added later, per its comment about "not a duplicate error") checks the same message content *and* asserts exactly one such diagnostic, its code (`quill::unknown_section`), and that the hint (not just the message) contains `"main.fields"` — a strict superset of the first test's assertions over the identical input.
- **Recommendation**: Delete `test_quill_config_rejects_root_level_fields`; `test_root_level_fields_gets_targeted_hint` covers everything it checked and more.
- **Est. LOC removable**: ~16
- **Confidence**: high
- **Risk if removed**: none — assertions are a strict subset of the surviving test.

### F6: Real-filesystem integration tests duplicate coverage already given by in-memory tests
- **Category**: low-value-test
- **Location**: `crates/core/src/quill/tests.rs:123-155` (`test_in_memory_file_system`), `:157-185` (`test_quillignore_integration`)
- **Evidence**: Both write files to a real `tempfile::TempDir`, walk them through the test-local `load_tree`/`load_dir` helper (`tests.rs:11-65`, a hand-rolled filesystem walker reimplementing the shape of `quillmark::load_dir` but *without* its symlink-skip or file-size-limit safety checks — so it doesn't validate that production path either), and assert `file_exists`/`get_file` results. `test_in_memory_file_system`'s assertions (file exists at nested paths, content round-trips) are already covered by `test_from_tree` (270-312) and `test_dir_exists_and_list_apis` (385-487), both built from an in-memory `FileTreeNode` with no disk I/O. `test_quillignore_integration`'s ignore-pattern behavior is already unit-tested directly and more precisely by `test_quillignore_matching` (97-120), which exercises `QuillIgnore::is_ignored` on many path shapes without the filesystem round trip.
- **Recommendation**: Delete both; keep `test_quillignore_parsing`/`test_quillignore_matching` (direct `QuillIgnore` coverage) and the in-memory `FileTreeNode` tests. The `load_tree`/`load_dir` helper and `load_from_path` must stay — `test_find_files_pattern`, `test_new_standardized_yaml_format`, and `check_schema_snapshot` (which loads the real `usaf_memo` fixture) still need it.
- **Est. LOC removable**: ~62
- **Confidence**: medium
- **Risk if removed**: low — no other core test exercises ".quillignore discovered from a real directory + `Quill::from_tree`" end-to-end, but the current test doesn't validate the production loader either (`quillmark::load_dir` has symlink/size-limit logic this helper lacks), so its integration value is already partly illusory.

### F7: `Quill::find_files` re-implements the tree walk `FileTreeNode::flatten` already does
- **Category**: duplicate-helper
- **Location**: `crates/core/src/quill/query.rs:51-66` (`find_files`) and `:68-93` (`find_files_recursive`) vs. `crates/core/src/quill/tree.rs:195-222` (`FileTreeNode::flatten`/`flatten_into`)
- **Evidence**: Both are recursive descents over `FileTreeNode` that accumulate `"/"`-joined relative paths (`current_path.join(name)` vs. `format!("{prefix}/{name}")`) and collect file entries. `flatten()` already produces every `(path, contents)` pair in the tree; `find_files_recursive` re-walks the same structure independently just to glob-match on the path and drop the contents.
- **Recommendation**: Replace `find_files`'s body with `self.files.flatten().into_iter().filter(|(p, _)| pattern.matches(p)).map(|(p, _)| PathBuf::from(p)).collect()`, then delete `find_files_recursive`.
- **Est. LOC removable**: ~20
- **Confidence**: medium
- **Risk if removed**: low — `flatten()` already visits every file (verified by `tree.rs`'s own round-trip test); behavior is preserved as long as glob matching stays on the full joined path, which it already is.

### F8: `quill_from_yaml` test helper duplicated verbatim across two files
- **Category**: duplicate-helper
- **Location**: `crates/core/src/quill/resolved.rs:214-224` and `crates/core/src/quill/seed/tests.rs:17-28`
- **Evidence**: Both define a private `fn quill_from_yaml(yaml: &str) -> Quill` that inserts `yaml` as `Quill.yaml` into a one-entry `FileTreeNode::Directory` and calls `Quill::from_tree(...).expect(...)` — identical except for a `HashMap` import alias and the panic message text. `crates/core/src/quill/tests.rs` needs the same construction ~15 times but inlines it manually each time instead of sharing either copy.
- **Recommendation**: Hoist one copy into a shared test-only location (or `pub(crate)` `#[cfg(test)]` helper on `Quill`) that both `resolved.rs` and `seed/tests.rs` import; opportunistically point `tests.rs`'s single-file `Quill::from_tree` constructions at it too.
- **Est. LOC removable**: ~10 direct (more if `tests.rs`'s repeated inline constructions are folded in, but that's a larger, lower-precision change)
- **Confidence**: high (on the two-file duplication itself)
- **Risk if removed**: none — behaviorally identical.

### F9: `field_source_serializes_lowercase` asserts serde attribute behavior, not Quillmark semantics
- **Category**: low-value-test
- **Location**: `crates/core/src/quill/resolved.rs:488-499`
- **Evidence**: The test asserts that `#[serde(rename_all = "lowercase")]` on `FieldSource` produces `"authored"`/`"default"`/`"zero"` — i.e., that the derive macro did what its attribute says. The adjacent `field_state_is_name_value_and_source_only` test (same file, 501-514) already pins the wire *shape* (exactly `name`/`value`/`source`, a genuine consumer contract), and any real drift in the rung spelling would also break `every_row_is_byte_for_byte_with_compile_data` and the fixture/binding-level golden tests that read these strings.
- **Recommendation**: Delete; fold a one-line spelling check into `field_state_is_name_value_and_source_only` if the exact strings are worth pinning at all.
- **Est. LOC removable**: ~10
- **Confidence**: low
- **Risk if removed**: very low, but this is the softest finding in the report — flagging for the reviewer's judgment rather than as a clear-cut removal.

## Load-bearing (looks redundant, is not)

- **`config::conform_value` (coercion, `Leniency::Render`/`Write`) vs. `validation::validate_value`** — these two dispatch over the same `FieldType` match arms and look like parallel/duplicated logic, but the module docs on both sides (`config.rs:238-246`, `validation.rs` module intro) explicitly document them as a *deliberately* parallel pair kept in lock-step via shared helpers (`scalar_as_string`, `decode_richtext_value`, `decode_plaintext_value`) — one normalizes, the other reports. Collapsing them would remove the "coercion adopts it silently, validation reports if it can't" split the whole render pipeline depends on.
- **`Quill::compile_data`/`dry_run`/`check_quill_reference` (`compose.rs:18-33`) forwarding to `QuillConfig`'s methods of the same name** — one-line forwards, but intentional: the module doc explains a config-only consumer (e.g. a live session's `apply`) can hold just the `QuillConfig` and skip the file tree/font bytes a full `Quill` carries. Not a stray abstraction layer.
- **`GroupSchema`/`GroupRegistry` (`types.rs`) having no Rust importer outside `quill/`** — grepped clean across bindings/backends, but they're the necessary field type of `UiCardSchema::groups`, which is genuinely exercised (group-registry tests, `blueprint.rs` clustering) and consumed by bindings only through the serialized JSON schema, not by importing the Rust type. Not dead.
- **`is_snake_case_identifier` (`config.rs:1268`) vs. `is_valid_kind_name` (`document/meta.rs:167`)** — look like the same "identifier grammar" duplicated, but they're different grammars for different name classes: field/quill names reject a leading underscore, card-kind names accept one (`test_quill_config_accepts_leading_underscore_card_name`, `tests.rs:824`). Not interchangeable.
- **`CardSchemaDef.fields` marked `#[allow(dead_code)]` (`config.rs:85-86`)** — looks like dead data, but the comment explains it exists solely so `#[serde(deny_unknown_fields)]` accepts a `fields:` key on the card shell before `parse_fields` re-parses it separately for per-field diagnostics. Removing the field would make `card_kinds.<x>.fields` an unknown-key error.
