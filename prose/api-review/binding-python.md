# Binding: Python

## Surface

`crates/bindings/python/src/{lib.rs,types.rs,enums.rs,errors.rs}`, PyO3 module `_quillmark`, re-exported as `quillmark` from `python/quillmark/__init__.py`.

Classes (`#[pyclass]`, exact Python name via `#[pyo3(name = ...)]` where renamed):

- `Quillmark` (`PyQuillmark`) — `__new__()`; `render(quill, doc, format=None, ppi=None, pages=None, producer=None, regions=False)`; `supported_formats(quill)`; `registered_backends()`.
- `Quill` (`PyQuill`) — `from_path(path)` (static); properties `backend_id`, `quill_ref`, `metadata` (dict), `schema` (dict), `blueprint` (str); `writer(doc)`, `reader(doc)`; `validate(doc)` → list[dict]; `seed_document()`, `seed_main()`, `seed_card(card_kind, overlay=None)`.
- `Document` (`PyDocument`) — `__new__(quill_ref)`; static `from_markdown`, `from_json`, `try_from_json`, `schema_version_of`, `current_schema_version`, `format_rules`, `blueprint_instruction`, `quill_ref_hint`, `make_card(kind, fields=None, body=None)`; instance `to_markdown`, `to_json`, `clone`, `__copy__`, `__deepcopy__`, `equals`, `__eq__`, `__repr__`; properties `quill_ref`, `card_count`, `warnings`, `body`, `main`, `cards`; mutators `remove_field(name, card=None)`, `store_ext(value, card=None)`, `remove_ext(card=None)`, `store_ext_namespace(namespace, value, card=None)`, `remove_ext_namespace(namespace, card=None)`, `store_seed_namespace(card_kind, overlay)`, `remove_seed_namespace(card_kind)`, `set_quill_ref(ref_str)`, `insert_card(card, at=None)`, `remove_card(index)`, `move_card(from_idx, to_idx)`, `set_card_kind(index, new_kind)`.
- `Writer` (`PyWriter`) — property `document`; `set(name, value)`, `set_all(fields)`, `set_body(markdown)`, `revise_field(name, markdown)`, `add_card(kind, fields=None, body=None, at=None)`, `remove_card(index)`, `card(index)` → `CardWriter`.
- `CardWriter` (`PyCardWriter`) — properties `index`, `kind`; `set`, `set_all`, `set_body`, `revise_field`.
- `Reader` (`PyReader`) — property `document`; `get(name)`, `get_body()`, `card(index)` → `CardReader`.
- `CardReader` (`PyCardReader`) — properties `index`, `kind`; `get(name)`, `get_body()`.
- `RenderResult` (`PyRenderResult`) — properties `artifacts`, `warnings`, `format`, `render_time_ms`, `regions`.
- `Artifact` (`PyArtifact`) — properties `bytes`, `format`, `mime_type`; `save(path)`.
- `Diagnostic` (`PyDiagnostic`) — `__str__`, `__repr__`; properties `severity`, `message`, `code`, `location`, `hint`, `path`, `source_chain`.
- `Location` (`PyLocation`) — properties `file`, `line`, `column`.
- Enums (`#[pyclass(eq, eq_int)]`): `OutputFormat` (`PDF`/`SVG`/`TXT`/`PNG`) and `Severity` (`ERROR`/`WARNING`), each with `.name`, `__repr__`, static `.all()`.
- Exception: `QuillmarkError(PyException)` — single type, `.diagnostics` attribute attached dynamically via `setattr` in `raise_with_diagnostics` (`errors.rs:52`).

`__init__.py`'s `__all__` matches the `_quillmark` module's exported classes 1:1 — no drift there.

**No `.pyi` stubs and no `py.typed` marker anywhere in the package** (`python/quillmark/` contains only `__init__.py`; `pyproject.toml`'s `include` list is `python/quillmark/**/*.py`, no stub glob). See Findings.

## Findings

### 1. `render(..., regions=True)` returns plate-space field addresses, not canonical `DocPath` — contradicts core's own contract and WASM parity
`crates/bindings/python/src/types.rs:1070-1083`, `crates/bindings/python/src/types.rs:39-69` (`PyQuillmark::render`). Severity: **High**.

Core's `RenderedRegion.field` doc comment (`crates/core/src/region.rs:96-103`) is explicit: the field is delivered in *backend-native plate-space* form (`"$cards.<kind>.<ordinal>.<field>"`) and "a binding that owns the document's card kinds translates it to a canonical `DocPath` at its boundary … so its consumers see one absolute-index grammar." WASM does exactly this: `Quillmark::render` (`crates/bindings/wasm/src/engine.rs:509-519`) builds `kinds` from `doc.inner.cards()` and calls `regions_to_docpath(result.regions, &kinds)` before returning, so a WASM `FieldRegion.field` reads `"cards.indorsement[2].signature_block"` — the same grammar `Diagnostic.path` and `parseDocPath` use.

Python's `PyQuillmark::render` never performs this translation — it forwards `self.inner.render(...)`'s raw `RenderResult` straight through, and `PyRenderResult::regions` (`types.rs:1070-1083`) copies `r.field` verbatim. `PyDocument` has the same `doc.inner.cards()` needed to build the kind lookup, so nothing structural blocks fixing it. The bug is locked in by the test suite itself: `test_render.py:122` asserts `r["field"] == "$body"` (the plate-space form) rather than the canonical `"main.body"`.

Caller impact: any Python consumer trying to correlate a `regions` entry with a `Diagnostic.path` (both are advertised as "the same grammar" in the design) gets two different address dialects for card fields — `"$cards.indorsement.1.on"` vs `"cards.indorsement[2].on"` — and per-kind *ordinal* addresses, not the document's absolute card index, silently breaking on card reordering.

### 2. `render` (and `validate`) never release the GIL
`crates/bindings/python/src/types.rs:39-69` (`render`), `types.rs:199-212` (`validate`). Severity: **High**.

No `py.allow_threads` call exists anywhere in `crates/bindings/python/src` (verified by grep). `Quillmark::render` invokes the full Typst/pdfform compile pipeline — potentially seconds of CPU-bound Rust work — while holding the GIL the whole time. Every other Python thread (including a `ThreadPoolExecutor` a server might use to parallelize renders, or just the event loop of an async caller) stalls for the render's full duration. This is the single highest-value threading fix available: `PyDocument`/`PyQuill`'s underlying `quillmark_core` types are plain data (no live borrows across the call), so the pattern is "borrow, clone/extract the owned pieces needed, then `py.allow_threads(|| ...)`" — the same shape every other CPU-bound PyO3 extension uses.

### 3. No type stubs, no `py.typed` — package ships with zero static type information
`crates/bindings/python/python/quillmark/` (only `__init__.py`), `crates/bindings/python/pyproject.toml:42`. Severity: **Medium**.

There is no `.pyi` anywhere in the tree and no `py.typed` marker file, so per PEP 561 a type checker treats the entire `quillmark` package as untyped — `reveal_type(Document.from_markdown(...))` is `Any`, every method call and property access on every class is unchecked, and the dynamically-`setattr`'d `QuillmarkError.diagnostics` (`errors.rs:52-64`) is invisible to any tool. For a schema-driven document engine whose whole value proposition is structured, typed data, the Python surface currently offers no IDE autocomplete and no mypy/pyright coverage. The WASM binding gets this for free via its hand-written `.d.ts`/`typescript_custom_section`; Python has no equivalent artifact at all.

### 4. `Quill.resolve` — the resolved-value/`FieldSource` view — is absent from Python
`crates/bindings/python/src/types.rs` (no `resolve` method on `PyQuill`), vs. `crates/core/src/quill/resolved.rs:87` (`Quill::resolve`) and `crates/bindings/wasm/src/engine.rs:688-697` (`Quill.resolve`). Severity: **Medium**.

`Quill::resolve(doc)` is a core capability (not preview/canvas-gated) that returns, for every declared field, the value the render projection would use plus its `FieldSource` rung (`"authored" | "default" | "zero"`) — a form-editor's "what will actually render" question, distinct from `validate`. WASM exposes it as `Quill.resolve`. `prose/canon/BINDINGS.md`'s Python scope note (lines 145-155) lists exactly what is WASM-only-by-scope — the opaque store and the anchor-preserving content lane — and does not mention `resolve`; nothing marks this omission deliberate. A Python consumer building a form/preview UI (the natural audience for `resolve`, same as `validate`) has no way to ask "what value would render for this field" without reimplementing the default/zero-fill ladder client-side.

### 5. `Quillmark.supports_canvas` missing from `PyQuillmark`
`crates/bindings/python/src/types.rs:19-91`, vs. core `Quillmark::supports_canvas` (`crates/quillmark/src/orchestration/engine.rs:118`) and WASM `Quillmark.supportsCanvas` (`crates/bindings/wasm/src/engine.rs:543-546`). Severity: **Low**.

Consistent with `prose/canon/PREVIEW.md`'s explicit non-goal ("Native (CLI / Python) exposure. Capability is WASM-only") for the whole canvas/`LiveSession` surface, so this is very likely deliberate — flagged only because the probe itself (`true`/`false`, no session, no canvas) costs nothing to expose and its absence is not spelled out anywhere in `BINDINGS.md`'s Python scope note the way the opaque-store/content-lane omissions are.

### 6. `Document.body` is a Python-only convenience with no WASM counterpart
`crates/bindings/python/src/types.rs:425-433`. Severity: **Low**.

WASM's `Document` has no `body` getter (only `main`/`cards`, each nesting `body`). Python adds a top-level `body` property returning the main card's content dict directly. Harmless, but it is exactly the kind of asymmetry the parity review is meant to surface: a consumer moving code from Python to WASM (or vice versa) has to remember this one extra accessor exists only on one side.

### 7. Value types other than `Document`/`Diagnostic` lack `__repr__`
`PyArtifact`, `PyQuill`, `PyLocation`, `PyRenderResult`, `PyWriter`/`PyReader`/`PyCardWriter`/`PyCardReader` have no `__repr__`. Severity: **Low**. `Document` and `Diagnostic` get one; the rest print as the default `<quillmark._quillmark.Artifact object at 0x...>`, which is a minor debugging/REPL ergonomics gap, not a functional one.

## Parity gaps vs WASM

| Concept | Python | WASM |
|---|---|---|
| Regions sidecar field address | raw plate-space (`"$cards.note.1.on"`) — **not translated** (Finding 1) | canonical `DocPath` (`"cards.note[2].on"`) |
| GIL / threading on render | GIL held for the whole compile (Finding 2) | N/A (single-threaded WASM) |
| Static types | none (no `.pyi`, no `py.typed`) (Finding 3) | full `.d.ts` via `typescript_custom_section` / `tsify` |
| `Quill.resolve` (`FieldSource` view) | absent (Finding 4) | `Quill.resolve(doc)` |
| `Quillmark.supports_canvas` probe | absent (Finding 5) | `Quillmark.supportsCanvas(quill)` |
| `Document.body` convenience getter | present (Finding 6) | absent — only via `main.body` |
| Live preview (`LiveSession`: `apply`, `paint`, `pageSize`, `regions()`, `fieldAt`, `positionAt`, `locate`, `fieldBoxes`) | absent — documented WASM-only non-goal (`PREVIEW.md`) | full surface |
| Opaque verbatim store (`storeField`/`storeFill`/`storeFields`) | absent — documented WASM-only by scope (`BINDINGS.md`) | present |
| Anchor-preserving content lane (`install`/`revise`/`applyChange` + `importMarkdown`/`exportMarkdown`/`rebase`/`mapPos` codec) | absent — documented WASM-only by scope | present |
| `DocPath` structured parse (`parseDocPath`/`formatDocPath`) | absent — `Diagnostic.path`/regions `field` are raw strings only | present, and used internally for regions translation |
| Typed writer front door | `quill.writer(doc)` — `set`/`set_all`/`set_body`/`revise_field`/`add_card`/`card(i)` | `quill.writer(doc)` — `set`/`setAll`/`setBody`/`reviseField`/`addCard`/`card(i)` (camelCase only; same verbs) |
| Interpreted reader front door | `quill.reader(doc)` — `get`/`get_body`/`card(i)` | `quill.reader(doc)` — `get`/`getBody`/`card(i)` (camelCase; internally routed through `_readerGet`) |
| `revise_field` delta | discarded (documented, editor-receipt is WASM-only concern) | returns `Delta` |
| Naming convention | `snake_case` throughout | `camelCase` throughout |
| Exception model | single `QuillmarkError`, `.diagnostics` list (matches ERROR.md contract) | single `Error` (via `WasmError`), `.diagnostics` array (identical contract) |

## Cross-cutting

- Finding 1 implicates `crates/core/src/region.rs` (the `RenderedRegion` contract) and `crates/quillmark/src/orchestration/engine.rs` (`Quillmark::render`) only as the source of the plate-space value Python fails to translate — the fix is entirely inside the Python binding (mirror `regions_to_docpath` from `crates/bindings/wasm/src/engine.rs:2434-2445`), not a core change.
- Finding 4 (`resolve`) is a `prose/canon/BINDINGS.md` documentation gap as much as a code gap: the Python scope note enumerates every other deliberate omission but not this one, so it reads as an oversight rather than a decision. Either add `PyQuill::resolve` or add it to the scope note's WASM-only list with a reason.
- Finding 3 (no stubs) is pure binding-surface work; no other crate is implicated.
