# Cleanup review index

A workspace-wide sweep for redundant logic, low-value features, and low-signal
tests. Eleven reviews, one per area, run independently against the same
finding schema; each report ranks its findings by LOC removed × confidence ÷
risk and carries a *load-bearing* section recording what looked redundant and
survived verification.

Findings are proposals, not decisions. Nothing here has been applied. The
findings are filed as nine `hygiene` issues, grouped by theme rather than by
crate — #1056, #1060, #1062, #1065, #1066, #1067, #1068, #1069, #1070.

## Reports

| File | Area | Findings | Est. LOC |
|---|---|---|---|
| [01-core-quill.md](01-core-quill.md) | `core/src/quill/` incl. its tests | 9 | ~393 |
| [02-core-document-src.md](02-core-document-src.md) | `core/src/document/` source | 2 | ~85 |
| [03-core-document-tests.md](03-core-document-tests.md) | `core/src/document/tests/`, 281 tests | 13 | ~930 |
| [04-core-toplevel.md](04-core-toplevel.md) | `core/src/*.rs`, `core/tests/` | 5 | ~65 |
| [05-content.md](05-content.md) | `content` crate | 4 | ~155 |
| [06-typst-backend.md](06-typst-backend.md) | `backends/typst` | 5 | ~350 |
| [07-pdfform-and-pdf.md](07-pdfform-and-pdf.md) | `backends/pdfform`, `quillmark-pdf` | 6 | ~51 |
| [08-wasm-bindings.md](08-wasm-bindings.md) | `bindings/wasm` incl. JS suites | 8 | ~670 |
| [09-python-cli-bindings.md](09-python-cli-bindings.md) | `bindings/python`, `bindings/cli` | 8 | ~292 |
| [10-quillmark-fuzz-fixtures.md](10-quillmark-fuzz-fixtures.md) | `quillmark`, `fuzz`, `fixtures` | 8 | ~375 |
| [11-cross-cutting.md](11-cross-cutting.md) | Workspace seams: helpers, features, deps, docs, CI | 7 | ~244 |

75 findings, ~3,600 LOC. Roughly 85% is test code.

## Themes

**Test altitude is the dominant problem.** Engine semantics get asserted at
whichever layer the test author was working in, so the same contract is pinned
two and three times over: validate/seed/mismatch behavior lives in
`quillmark/tests`, again in Python, again in WASM (09 F1, 08 F1–F2);
`multibyte_regression_test.rs` and `default_values_test.rs` are end-to-end
renders asserting facts core already covers as unit tests (10 F1–F2); quill
body-example tests re-derive the fence grammar `card_fence_tests.rs` owns (01
F1). `spec_conformance_probe.rs` is the sharpest case — it reads as an
independent conformance check but runs the same `Document::parse` path as the
unit tests it duplicates (03 F3). A binding test should assert marshalling; an
end-to-end render should assert integration. Neither should re-litigate core.

**Copy-pasted fixture scaffolding.** The `usaf_memo` disk-walker is hand-rolled
four times in the Typst backend while the sibling pdfform backend calls
`quill_from_path` (11 F1); `walk`/`quill` builders are byte-identical across
seven Typst test files (06 F1); `decode_pdf_text`/`widget` across two pdfform
files (07 F1); `quill_from_yaml` across two core files (01 F8). This is the
cheapest category — mechanical, zero behavior risk, ~250 LOC.

**Clusters that want to be tables.** Nine multi-card parse tests, sixteen
comment/`!must_fill` scaffolds, ten value-type round-trips, six
ambiguous-string categories, five snake_case rejections, twenty-one
single-assertion string-function tests (03 F1–F2/F5–F10, 01 F2–F3, 05 F1). The
coverage is real; the line count is not. Collapsing preserves every case.

**Small dead or forwarding surface.** `QuillValue`'s eight accessors shadow its
own `Deref` (04 F1); `TypedWriter::remove_card`, `ReadValue::as_text/as_value`,
`Quill::list_files/list_subdirectories`, `apply_mark_ops/apply_line_ops` have no
callers (04 F2–F3, 01 F4, 05 F2); `overlay/mod.rs` re-exports five single-caller
pass-throughs (06 F4). Individually tiny, collectively ~150 LOC of API that
must be read and maintained to no end.

**Production logic is in good shape.** The source-side findings are small and
local. Repeated suspicions came back load-bearing on inspection: the
`dto.rs`/`wire.rs` split, the prescan/fences/yaml_hints scans, the
`quillmark-pdf`/`pdfform` two-crate boundary (the Typst overlay consumes the
stamp spine independently), `delta.rs`, error plumbing, and format/backend
registration. All seven fixture Quills are loaded by something. The noise is
concentrated in tests, not in the engine.

## Non-LOC findings worth acting on

- **11 F2** — `error.rs`'s path-grammar doc comment contradicts `DocPath`, its
  own tests, and `ERROR.md`: it shows main-card fields bare (`title`) where
  everything else agrees they are rooted (`main.title`). Actively misleading.
- **11 F4** — CI runs only `--all-features`, so `quillmark`'s
  `#[cfg(not(feature = "typst"))]` branch never executes. A coverage gap, not
  redundancy.
- **07 F2** — unbound `checkbox`/`choice` widgets are a versioned wire-format
  feature no fixture, doc, or test exercises. Either a coverage gap or dead
  surface; the wire format makes removal a judgment call.
- **03 F12, 03 F7** — two tests whose names promise coverage they never assert.
  Mis-signal costs a reader the same as a redundant test.

## Issues

| Issue | Theme | Est. LOC |
|---|---|---|
| #1056 | Engine semantics asserted at three altitudes | ~1000 |
| #1060 | Test clusters that want to be tables | ~700 |
| #1062 | Copy-pasted fixture walkers and quill builders | ~250 |
| #1065 | Tests pinning foreign behavior or asserting nothing | ~400 |
| #1066 | Dead and pure-forwarding public surface | ~150 |
| #1067 | Duplicated logic in production code | ~250 |
| #1068 | Shipped surface with no test; CI's zero-backend blind spot | — |
| #1069 | `error.rs` path-grammar drift; two misnamed tests | ~25 |
| #1070 | Dependency versions hand-pinned across crates | ~4 |

## Reading order

For quick wins: 11 F1, 06 F1, 07 F1, 04 F1 — mechanical, high confidence, no
behavior change. For the largest single reduction: 03 and 08. For a policy
decision that would prevent recurrence: the test-altitude theme above.
