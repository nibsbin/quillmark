# Cleanup review: cross-cutting

Scope: workspace seams — duplicated helpers, error plumbing, format registration, features, deps, docs, CI

## Findings

### F1: `usaf_memo` fixture disk-walker reimplemented 4x in the typst backend
- **Category**: duplicated-helper-cluster
- **Locations**:
  - `crates/backends/typst/src/error_mapping.rs:81-116` (`walk_fixture()`, inner `fn walk`)
  - `crates/backends/typst/src/world.rs:562-596` (`test_asset_fonts_have_priority`, inner `fn walk`)
  - `crates/backends/typst/tests/sig_field.rs:16-45` (`host_tree()`, inner `fn walk`)
  - `crates/backends/typst/tests/producer_meta.rs:15-43` (`host_tree()`, inner `fn walk`)
- **Evidence**: all four define a byte-for-byte identical `fn walk(dir: &Path) -> std::io::Result<FileTreeNode>` (recursive dir → `HashMap<String, FileTreeNode>` walk), followed by the same `.parent().unwrap().parent().unwrap().join("fixtures").join("resources").join("quills").join("usaf_memo").join("0.2.0")` path build. Three of the four additionally duplicate a `source_with_plate`/plate-substitution helper (`error_mapping.rs:127-138`, `sig_field.rs:53-?`, `producer_meta.rs:51-?`). Meanwhile `crates/quillmark/src/load.rs:17-25` (`quill_from_path`) plus `quillmark_fixtures::quills_path` already implement the identical disk→`FileTreeNode`→`Quill` pipeline — used verbatim by the sibling pdfform backend's tests, e.g. `crates/backends/pdfform/tests/sample_form.rs:22` (`quillmark::quill_from_path(quillmark_fixtures::quills_path("sample_form"))`). `quill_from_path`'s walker is also strictly better: it honours `.quillignore` / a default ignore set (`.git/`, `target/`, `node_modules/`), which the 4 hand-rolled walkers do not.
- **Canonical home**: `crates/quillmark/src/load.rs::quill_from_path` + `quillmark_fixtures::quills_path`, already the pattern used by `crates/backends/pdfform/tests/*`.
- **Recommendation**: add `quillmark = { path = "../../quillmark" }` and `quillmark-fixtures = { workspace = true }` as dev-dependencies to `crates/backends/typst/Cargo.toml` (pdfform's Cargo.toml already documents why a path-only dev-dep here isn't a publish cycle — `crates/backends/pdfform/Cargo.toml:34-36`), then replace all 4 walkers with `quillmark::quill_from_path(quillmark_fixtures::quills_path("usaf_memo"))`. For the 3 sites that need a swapped `plate.typ`, keep one shared `source_with_plate` helper (e.g. in a `tests/common/mod.rs` used by both `sig_field.rs` and `producer_meta.rs`; `error_mapping.rs`'s copy is in `src/`, so it stays but can call the same `quill_from_path`-based loader).
- **Est. LOC removable**: ~150
- **Confidence**: high
- **Risk if removed**: low — behavior is a strict superset (adds ignore-file handling); only risk is a test fixture accidentally containing a `.git`/`target` dir that the old walker picked up and the new one would skip (none do today).

### F2: `error.rs`'s path-grammar doc comment has drifted from `DocPath`'s actual (and correctly documented) grammar
- **Category**: duplicated-normative-docs
- **Locations**:
  - `crates/core/src/error.rs:15-39` (module doc "### Path grammar" section and table)
  - `crates/core/src/path.rs:14-42` (module doc grammar, `root := "main" | "cards" "[" index "]" | ...`) and `crates/core/src/path.rs:351,354,360` (round-trip tests)
  - `prose/canon/ERROR.md:150-177` ("Document-model paths" section)
- **Evidence**: `error.rs:30-31` states `Root-block field → title` and `Nested in array of objects → recipients[0].name` — i.e. main-card fields are documented as **unrooted** bare paths. But `path.rs:23-24` states "a main field is `main.<field>` (`main.title`, `main.recipients[0].name`)" and the test at `path.rs:351` asserts `round_trip(DocPath::main().field("title"), "main.title")`. `prose/canon/ERROR.md:160-161` agrees with `path.rs`: `Main-card field → main.recipient`, `Nested in an array of objects → main.recipients[0].name`. `error.rs`'s grammar line 18 (`path := segment ( "." field_name | "[" index "]" )*`) also omits the mandatory `root` production that `path.rs:14-17` defines — the same drift, one level up. `error.rs`'s copy is the odd one out and is simply wrong against current, tested behavior.
- **Canonical home**: `crates/core/src/path.rs` (owns `DocPath`, the type that "constructs, renders, and parses" the grammar) plus `prose/canon/ERROR.md`, which already agrees with it.
- **Recommendation**: delete the "### Path grammar" section and table in `error.rs:15-39` (or replace it with a one-line pointer to `crate::path::DocPath`'s docs) rather than maintaining a second, now-incorrect copy.
- **Est. LOC removable**: ~25
- **Confidence**: high (verified against `path.rs`'s own passing test assertions)
- **Risk if removed**: none — it's a doc comment; nothing compiles against it. Risk of *not* fixing it is higher: a reader trusting `error.rs`'s table will construct wrong paths.

### F3: lowercase-snake-case identifier charset (`[a-z_][a-z0-9_]*`) validated by 3 separate implementations
- **Category**: duplicated-helper-cluster
- **Locations**:
  - `crates/core/src/document/meta.rs:166-182` (`pub fn is_valid_kind_name`, re-exported at `crate::document::is_valid_kind_name`)
  - `crates/core/src/quill/config.rs:1268-1276` (`fn is_snake_case_identifier`, used only by `is_valid_quill_name` at `config.rs:1278-1280`)
  - `crates/core/src/version.rs:229-247` (inlined equivalent charset check inside `QuillReference::from_str`)
- **Evidence**: all three implement "first char lowercase-ascii-or-`_`, remaining chars lowercase-ascii-alphanumeric-or-`_`". `is_valid_kind_name` is already `pub` and already imported/used elsewhere in the *same file* as one of the duplicates: `config.rs:1649` calls `crate::document::is_valid_kind_name(card_name)` directly, a few hundred lines from its own private reimplementation (`is_snake_case_identifier`) of the identical rule. `error.rs:19`'s grammar comment itself documents that card kinds and (by the same rule) quill names share this "lowercase-only" grammar, confirming the three sites are meant to enforce the same rule, not three independent ones.
- **Canonical home**: `crates/core/src/document/meta.rs::is_valid_kind_name` (already `pub`, already cross-module).
- **Recommendation**: replace `config.rs`'s `is_snake_case_identifier` body with a call to `crate::document::is_valid_kind_name`, and replace `version.rs`'s inlined two-`if` block with the same call (keeping its two distinct error messages, just gating on the shared predicate instead of re-deriving it).
- **Est. LOC removable**: ~25
- **Confidence**: high
- **Risk if removed**: low — behaviorally identical for every case examined, including the empty-string edge (all three reject it, `is_snake_case_identifier` via `chars.next() == None`, the others via an explicit `is_empty` guard already present at the call site).

### F4: `quillmark`'s zero-backend feature-flag branch is never exercised by CI
- **Category**: untested-feature-flag
- **Locations**:
  - `crates/quillmark/tests/feature_flag_test.rs:17-32` (`#[cfg(not(feature = "typst"))] fn test_typst_backend_not_registered`)
  - `.github/workflows/ci.yml:75` (`cargo test --workspace --all-features --locked` — the only native Rust test invocation in CI)
  - `crates/quillmark/Cargo.toml:26-32` (`default = ["typst", "pdfform"]`, both features `dep:`-gated)
- **Evidence**: CI's `test` job runs exactly one `cargo test` invocation, and it passes `--all-features`. That flag forces `typst` on, so `#[cfg(not(feature = "typst"))]` never compiles in CI — the "no backends registered" code path in `crates/quillmark/src/orchestration/engine.rs` (the `Quillmark::new()` branch reachable only with both features off) has no CI-executed assertion. Contrast with the WASM binding, whose `--no-default-features` (core, Typst-free) build *is* both compiled and test-driven (`scripts/build-wasm.sh` builds a `core` variant; `crates/bindings/wasm/core.test.js` exercises it via vitest in `ci.yml`'s `wasm` job) — the native crate has no equivalent.
- **Canonical home**: N/A — this is a coverage gap, not a duplicate to consolidate.
- **Recommendation**: either add a `cargo test -p quillmark --no-default-features` (or `--no-default-features --locked`) step to `ci.yml`'s `test` job so the negative-feature branch actually runs, or delete `test_typst_backend_not_registered` if the zero-backend configuration is considered unsupported/untested by design (state that decision explicitly if so — CLAUDE.md's own "commit early" culture suggests recording such calls rather than leaving silently-dead test code).
- **Est. LOC removable**: 0 (recommend adding coverage, not removing code — but ~15 LOC could be deleted if the configuration is deliberately dropped)
- **Confidence**: high (directly verified: single `cargo test` invocation, `--all-features` flag, `cfg(not(...))` guard)
- **Risk if removed**: if the negative-branch test is deleted rather than covered, a future regression that silently registers a backend when no feature is enabled (or vice versa) would go unnoticed.

### F5: two near-identical `JsonValue → type name` helpers
- **Category**: duplicated-helper-cluster
- **Locations**:
  - `crates/core/src/document/meta.rs:155-164` (`fn yaml_type_name`)
  - `crates/core/src/quill/validation.rs:257-274` (`fn yaml_scalar_type`)
- **Evidence**: both match `serde_json::Value`'s 6 variants to a short type-name string for a diagnostic message. They differ only in label choice (`"sequence"/"mapping"` vs `"array"/"object"`) and in that `yaml_scalar_type` additionally splits `Number` into `"integer"`/`"number"`. Not byte-identical, but the same responsibility implemented twice with no shared root.
- **Canonical home**: neither is objectively more canonical; `yaml_scalar_type`'s integer/number split is the more useful message, so it's the better base to extend (`meta.rs`'s callers don't currently need the split, but folding two labels into one function via a `bool`/enum arg keeps both).
- **Recommendation**: merge into one `pub(crate)` helper in `crate::document` (e.g. `yaml_value_type_name`) parameterized on whether to distinguish integer/number, and point both call sites at it.
- **Est. LOC removable**: ~15
- **Confidence**: medium (functions are similar-but-not-identical; consolidation requires picking one label vocabulary, a small user-facing message change)
- **Risk if removed**: low-medium — changes diagnostic message wording at one of the two call sites (`"sequence"`→`"array"` or vice versa), which is a visible (if minor) message-text change consumers could be pattern-matching on in tests.

### F6: `lopdf` and `js-sys`/`wasm-bindgen` versions hand-pinned separately in multiple crates instead of via `workspace.dependencies`
- **Category**: redundant-dependency
- **Locations**:
  - `crates/quillmark-pdf/Cargo.toml:20` (`lopdf = "0.36"`, dev-dep)
  - `crates/backends/pdfform/Cargo.toml:33` (`lopdf = "0.36"`, dev-dep)
  - `crates/backends/typst/Cargo.toml:40` (`lopdf = "0.36"`, dev-dep)
  - `crates/backends/typst/Cargo.toml:34-35` (`js-sys = "^0.3"`, `wasm-bindgen = "^0.2"`, target-specific)
  - `crates/bindings/wasm/Cargo.toml:24,31` (`wasm-bindgen = "0.2"`, `js-sys = "0.3"`)
- **Evidence**: `grep -n lopdf crates/quillmark-pdf/Cargo.toml crates/backends/pdfform/Cargo.toml crates/backends/typst/Cargo.toml` shows the identical literal `lopdf = "0.36"` three times; `workspace.dependencies` in the root `Cargo.toml` centralizes every other shared dep (`tempfile`, `proptest`, etc.) but not `lopdf`, `js-sys`, or `wasm-bindgen`, so a version bump for any of these means editing 2-3 files instead of one.
- **Canonical home**: root `Cargo.toml`'s `[workspace.dependencies]`.
- **Recommendation**: hoist `lopdf`, `js-sys`, and `wasm-bindgen` into `[workspace.dependencies]` and switch the 5 sites to `{ workspace = true }`.
- **Est. LOC removable**: ~4 net (mostly a maintenance-surface reduction, not a line-count win)
- **Confidence**: high
- **Risk if removed**: none — same resolved versions either way (unless a version-conflict is currently masked by having 3 independent `"0.36"` ranges, which is not the case here).

### F7: WASM binding links two major versions of `serde-wasm-bindgen` (0.5 via `tsify`, 0.6 direct)
- **Category**: redundant-dependency
- **Locations**:
  - `crates/bindings/wasm/Cargo.toml:28` (`serde-wasm-bindgen = "0.6"`, direct, used extensively in `crates/bindings/wasm/src/engine.rs`)
  - `crates/bindings/wasm/Cargo.toml:36` (`tsify = { version = "0.4.5", features = ["js"] }`, which pins `serde-wasm-bindgen v0.5.0` internally)
- **Evidence**: `cargo tree --workspace --duplicates` shows both `serde-wasm-bindgen v0.5.0` (under `tsify v0.4.5 → quillmark-wasm`) and `serde-wasm-bindgen v0.6.5` (direct `quillmark-wasm` dep) in the same dependency graph; `cargo tree -p quillmark-wasm -i serde-wasm-bindgen@0.5.0` confirms the only path is through `tsify`. `tsify`-derived types (`crates/bindings/wasm/src/types.rs:4`) are gated `#[cfg(any(feature = "typst", feature = "pdfform"))]`, so this doesn't affect the size-budgeted Typst-free `core` WASM variant, only the `typst`/`pdfform` variants — which have no equivalent size gate in CI.
- **Canonical home**: not directly fixable from this repo — `tsify`'s internal pin is upstream. Noted for awareness, not immediate action.
- **Recommendation**: check whether a newer `tsify` release tracks `serde-wasm-bindgen` 0.6 (would dedupe for free on a version bump); otherwise no local action — do not attempt to vendor or patch a third-party crate's internal dependency for this.
- **Est. LOC removable**: 0 (no source change available)
- **Confidence**: high (verified via `cargo tree`)
- **Risk if removed**: N/A — nothing to remove locally.

## Load-bearing (looks duplicated, is not)

- **`crates/core/src/normalize.rs` vs `crates/content/src/normalize.rs`** — look like the same "normalize" concern but are disjoint: `core::normalize` does document-level payload *field-name* NFC normalization post-parse; `content::normalize` does markdown-*string* preprocessing (CRLF→LF, bidi-char strip, HTML-comment-fence repair) at import time, before parsing. `core::normalize.rs:7-11`'s own doc comment states card bodies are deliberately *not* touched by the core pass because `content::normalize_markdown` already ran. Different inputs, different pipeline stages, correctly separated.
- **Per-binding `OutputFormat` mirror enums** (`crates/bindings/wasm/src/types.rs:16-45`, `crates/bindings/python/src/enums.rs:42-75`) — each is a 4-arm `From`/`Into` bridge to `quillmark_core::OutputFormat`. This looks like "a list edited in N places to add a format," but it's the unavoidable cost of exposing a native Rust enum as a typed FFI enum (a `tsify`-derived TS enum for WASM, a `pyo3` enum for Python); the *string* and *MIME* mappings themselves (`OutputFormat::as_str`, `OutputFormat::mime_type`) each have exactly one implementation in `crates/core/src/types.rs:29-47`, and every binding calls through to it (verified: `crates/bindings/wasm/src/types.rs:173-176`, `crates/bindings/python/src/types.rs:1118-1120`) rather than re-deriving the mapping.
- **CLI/Python/WASM error types** (`crates/bindings/cli/src/errors.rs`, `crates/bindings/python/src/errors.rs`, `crates/bindings/wasm/src/error.rs`) — distinct types by necessity (each binding's host language has its own exception/error convention), but all three delegate their message-summarization to the single `RenderError::summary_message` (`crates/core/src/error.rs:354-360`) and their pretty-printing to `Diagnostic::fmt_pretty` (`crates/core/src/error.rs:168-196`) rather than reimplementing either. `crates/bindings/python/src/types.rs:1133` and `crates/bindings/wasm/src/engine.rs:866-871` both cite this explicitly ("same rendering the CLI and WASM emit"). No duplicated error plumbing found here.
- **Backend/format registration** — `OutputFormat::ALL` (`crates/core/src/types.rs:19-24`) is the single enumeration of formats; `Quillmark::new()`'s `#[cfg(feature = ...)]` block (`crates/quillmark/src/orchestration/engine.rs:31-38`) is the single place backends are registered. Bindings query `registered_backends()`/`supported_formats()` at runtime rather than hand-listing backends or formats a second time.
- **`docs/reference/markdown-spec.md`** is a 1-line `pymdownx.snippets` include (`--8<-- "prose/references/markdown-spec.md"`) of the canon reference, not a copy — editing one location updates both rendered pages. Not a duplicate.
- **Superseded `docs/migrations/*.md` guides absent from `mkdocs.yml`'s `nav`** — intentional, via the `not_in_nav` directive (`mkdocs.yml`, "Superseded migration guides stay published..."); they remain reachable by URL and from `migrations/index.md`. Not orphaned.
- **PDF-writer (`crates/quillmark-pdf`, production) vs `lopdf` (dev-dep only, 3 crates)** — not two PDF libraries doing the same job; `pdf-writer` is the only production PDF *writer*, `lopdf` is used exclusively in `#[cfg(test)]`/`tests/` code to reparse and assert on the stamped output, a different job (see F6 for the version-pin duplication itself, which is a maintenance issue, not a functional overlap).
- **String-escaping helpers** (`crates/content/src/export.rs:813` markdown-export escaping, `crates/backends/typst/src/emit.rs:57,78` Typst markup/string escaping, `crates/quillmark-pdf/src/writer.rs:40` PDF text-string escaping) — three different output grammars, each correctly owning its own escaper. No overlap.
