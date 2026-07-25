# API Surface Review

Eleven parallel reviews, one per surface, of every public API in the
workspace. Each file enumerates its surface, then ranks findings with
`file:line` citations and a High/Medium/Low severity.

| Surface | File | Public items |
| --- | --- | --- |
| `core/document/**` | [core-document.md](core-document.md) | `Document`, `Card`, `Payload`, `Parsed`, wire/DTO |
| `core/quill/**` | [core-quill.md](core-quill.md) | `Quill`, `QuillConfig`, schema, resolve, seed, blueprint |
| `core` top level | [core-runtime.md](core-runtime.md) | `Backend`, `LiveSession`, `Diagnostic`, `QuillValue`, `DocPath` |
| `quillmark` + `fixtures` | [quillmark-orchestration.md](quillmark-orchestration.md) | `Quillmark`, `quill_from_path` |
| `quillmark-content` | [content.md](content.md) | `Content`, import/export, ops, delta, serial |
| `quillmark-pdf` | [quillmark-pdf.md](quillmark-pdf.md) | `stamp`, `PdfUpdate`, `FieldSpec`, reader/writer |
| `backends/typst` | [backend-typst.md](backend-typst.md) | `TypstBackend`, `pub mod emit` |
| `backends/pdfform` | [backend-pdfform.md](backend-pdfform.md) | `PdfformBackend` |
| `bindings/wasm` | [binding-wasm.md](binding-wasm.md) | JS/TS classes, `LiveSession`, canvas |
| `bindings/python` | [binding-python.md](binding-python.md) | PyO3 classes, exceptions |
| `bindings/cli` | [binding-cli.md](binding-cli.md) | command grammar, flags, exit codes |

## Cross-crate themes

Ranked by how much each costs a caller. Per-surface findings stay in the
per-surface files; these are the ones no single reviewer owns.

### 1. The backend extension point is unreachable from published docs

`Backend::open` returns a `LiveSession`, constructible only through
`LiveSession::new(Box<dyn SessionHandle>)`. Both `SessionHandle`
(`core/src/session.rs:22`) and `LiveSession::new` (`session.rs:180`) are
`#[doc(hidden)]`, and neither `quillmark_core` nor `quillmark` re-exports
`SessionHandle`. Both in-tree backends take this path; an out-of-tree
implementor reading rustdoc cannot find it. `ARCHITECTURE.md:71` compounds
this by documenting a `LiveSession::handle()` + `SessionHandle::as_any`
downcast escape hatch that does not exist in the source.

Same shape one level out: `Quill::from_tree` needs `FileTreeNode`, which
`quillmark` does not re-export, so in-memory quill loading forces a direct
`quillmark-core` dependency while disk loading does not — despite
`ARCHITECTURE.md` presenting them as peers. `QuillConfig`, `CardSchema`,
`FieldSchema`, and `FieldType` are likewise absent from core's root
re-exports, so using `Quill::config()`'s contents means reaching through
`quillmark_core::quill::`.

The extension and in-memory paths are second-class across all three layers.

### 2. `RenderOptions` is backend-neutral; behavior under it is not

`RenderOptions::pages` is honored, bounds-checked, and rejected-for-PDF by
the Typst backend (`compile.rs:83-122`, `lib.rs:304`). The pdfform backend
never reads it — `grep` for `opts.pages` across `backends/pdfform/src/`
returns nothing — and silently renders every page, including for
out-of-range indices and the PDF+`pages` combination Typst rejects. Found
independently by three reviewers.

Core's doc comment for the field (`types.rs:139-147`) states its contract
purely in `typst::`-namespaced error codes, as if one backend existed. That
framing is likely why the gap went unnoticed.

Adjacent: `format_not_supported` is emitted as `pdfform::format_not_supported`
by one backend and `backend::format_not_supported` by the other for the
identical condition, with a third dead `typst::format_not_supported` in
Typst's `compile.rs`. A caller cannot dispatch on one code.

### 3. `OutputFormat::Txt` is advertised everywhere and implemented nowhere

`Txt` is a variant (`types.rs:7`), parses from `"txt"` (line 82), carries a
`text/plain` MIME (line 45), and appears in `OutputFormat::ALL`, CLI
`--help`, the README, and `CLI.md`. `SUPPORTED_FORMATS` in both backends is
`[Pdf, Svg, Png]`. Every request for it fails at render time. The CLI
surfaces this first only because it is the one binding that free-text-lists
formats.

### 4. Validation strength varies by entry path

The same invariant is enforced, downgraded, or skipped depending on which
door a value comes through.

- `Document::from_main_and_cards` (`document/mod.rs:385`) checks all four
  invariants with `debug_assert!` while every sibling constructor
  (`push_card`, `insert_card`, `Card::new`, storage `TryFrom`) returns
  `Result`. In release, it accepts a main card without `$quill`; then
  `Document::quill_reference` (`mod.rs:428`) panics on `.expect(...)`. Both
  are `pub`, and the constructor's doc does not disclose that the check is
  debug-only.
- `QuillConfig`/`CardSchema`/`FieldSchema` have all-`pub` fields and derived
  `Deserialize` with no checked constructor; `from_yaml`'s validation is
  opt-in. `fuzz/src/coerce_fuzz.rs`, `backends/pdfform/src/bind.rs`, and
  core's own reader/writer tests already hand-build these. `blueprint()`
  (`quill/blueprint.rs:119`) then panics via `.expect("quill name@version is
  always a valid QuillReference")` on exactly the malformed config this
  makes reachable — while `seed.rs` handles the identical failure
  gracefully.
- `TryFrom<CardWire> for Card` (`document/wire.rs:218`) never validates
  `$kind`; `WireError` has no variant for it.
- `LineOp::SetKind` (`content/ops.rs:553`) accepts an out-of-range heading
  level that the JSON wire decoder rejects — the Rust API is weaker than the
  wire path.
- `Content`'s four fields are `pub` with no construction-time invariant, so
  each consuming crate must remember `validate()`. `export::to_markdown` has
  no nesting-depth guard where import and the Typst emitter both do, making
  stack overflow reachable from a validated-but-deep `Content`.

### 5. Diagnostics leak, drop, and bypass the documented namespaces

`ERROR.md` states every failure travels as a `Diagnostic`. Three leaks:

- Typst's `world.rs` (`237-241`, `294-298`, `347-352`, `380-384`)
  `eprintln!`s asset, package, and font failures and swallows them. pdfform
  has no `eprintln!` anywhere.
- `map_pdf_err` forwards `quillmark-pdf`'s raw `pdf::*` codes, outside
  `ERROR.md`'s namespace list, in both backends.
- The CLI's `validate` (`commands/validate.rs:130`) discards `code`,
  `location`, and `hint` from real `Diagnostic`s in favor of a local
  `Severity`/`ValidationIssue` model.

Structural ceilings: `Diagnostic::hint` is `Option<String>`, so only the
first Typst hint survives; `Diagnostic::source_chain` is captured by
`with_source` but no formatter prints it, and canon references a
`fmt_pretty_with_source()` that does not exist.

`Version`, `VersionSelector`, and `QuillReference::from_str` return bare
`String` errors — not `std::error::Error` — while `DocPath` and
`OutputFormat` in the same scope return structured parse errors. Both
bindings call `QuillReference::from_str` directly.

### 6. Binding parity is narrower than item counts suggest

Raw `pub` counts (WASM ~106, Python ~17) overstate the gap: `LiveSession`,
canvas, the opaque store, and the content lane are WASM-only by documented
decision (`PREVIEW.md` non-goals, `BINDINGS.md` scope note), and the error
model — `Diagnostic`, namespaced codes, count-based message aggregation —
is faithfully mirrored on both sides. The real gaps:

| Concept | WASM | Python |
| --- | --- | --- |
| `Quill.resolve` (`FieldSource` provenance) | yes | missing, not in scope note |
| `Quill.fromTree`/`toTree` | yes | missing — no path for quill bytes off the wire |
| `Document.cardIndexById` | yes | missing |
| `parseDocPath`/`formatDocPath` | yes | missing |
| `registeredBackends()` | missing | yes |
| `Document.body` | missing | yes |

Python-specific, independent of parity: `render(regions=True)` returns raw
plate-space addresses (`"$cards.note.1.on"`) where core's `RenderedRegion`
doc mandates `DocPath` translation at the binding boundary — WASM does this
via `regions_to_docpath`, Python does not, and `test_render.py:122` locks
the bug in. No `py.allow_threads` anywhere, so `render`/`validate` hold the
GIL through a full compile. No `.pyi` stubs and no `py.typed`, leaving the
surface `Any` to type checkers and the `setattr`'d
`QuillmarkError.diagnostics` invisible.

### 7. Smaller cross-crate items

- `quillmark_core::ParseError` (`error.rs:206`) and
  `quillmark_content::ParseError` (`serial.rs:23`) are distinct types, both
  exported at their crate roots. A caller depending on both must rename one.
  (`ApplyError` appears twice by name but is one type — core re-exports
  content's at `session.rs:4`.)
- `map_pdf_err` is byte-identical in `backends/typst/src/compile.rs:24` and
  `backends/pdfform/src/lib.rs:356`. `quillmark-pdf`'s stated reason for no
  `From` impl — that it "would invert the dependency" — does not hold: the
  crate already depends on `quillmark-core` for `RenderedRegion`.
- The workspace root pins `quillmark-typst` and `quillmark-pdfform` with
  `default-features = false`, but neither crate declares a `[features]`
  table — the pin is a no-op, possibly standing in for a feature that was
  dropped.
- `publish` is explicit on `core`, `content`, and `cli`; omitted (defaulting
  true) on `quillmark`, `quillmark-pdf`, and both backends.
- `quillmark-pdf`'s `reader`/`writer` are `pub mod`, exposing byte-scanner
  primitives while their companion parsers are `pub(crate)` — an external
  caller can locate a dict value but cannot decode it.
- `prescan.rs`'s helpers are fully `pub` with no external consumer, unlike
  the structurally identical `yaml_hints.rs`, which is `pub(crate)`.

## What holds together

Recording these so a later pass does not relitigate them.

- Import/export in `content` are a matched pair; the two exceptions are
  documented in the module doc, and a proptest suite covers the round trip.
- `serial`/`import`/`export`/`ops` share one wire vocabulary rather than
  forking it.
- `delta`/`ops` and `core::session` are one edit vocabulary — core
  re-exports content's types instead of defining competitors. `ChangeSet` is
  a disjoint concept (dirty-page reporting), not a rival representation.
- Both backends present the same near-opaque shape: `TypstBackend` and
  `PdfformBackend`, no `typst::*` or `quillmark-pdf` types in public
  signatures.
- The `quillmark` crate is a genuinely thin shell: five lines and one import
  for the happy path, no `quillmark-core` dependency needed.
- CLI flag naming is consistent across subcommands, and `CLI.md` matches
  behavior.
- `pdfform-backend.md`'s design claims — Typst-free, fresh-AcroForm-only,
  regions as a decoupled session query, flatten as a separate lossy path —
  all check out against the code.
