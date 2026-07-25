# Binding: WASM

> Scope: `crates/bindings/wasm/**` — `src/{lib,engine,types,error}.rs`, `runtime/runtime.{js,d.ts}`, `package.json`/`package.template.json`, `*.test.js`, `README.md`. Compared against core (`crates/core/src/lib.rs`), `quillmark` (`crates/quillmark/src/lib.rs`), and the Python binding (`crates/bindings/python/src/{lib,types,enums,errors}.rs`, `python/quillmark/__init__.py`) for fidelity/parity only — those crates are reviewed elsewhere.

## Surface

Three FFI classes ship from `src/engine.rs` (`#[wasm_bindgen]`), gated by cargo feature; the public npm surface is the hand-written `runtime/runtime.js` layer that wraps them. `js_name` renames every Rust `snake_case` method to `camelCase`; noted only where the mapping isn't the obvious camelCase of the Rust name.

**`Quillmark`** (engine.rs:402, render builds only — `typst`/`pdfform` feature):
`new()` ctor · `open(quill, doc): LiveSession` · `render(quill, doc, opts?): RenderResult` · `supportedFormats(quill): OutputFormat[]` (throws `engine::backend_not_found`) · `supportsCanvas(quill): boolean`.

**`Quill`** (engine.rs:408, every build): `fromTree(tree: Map<string,Uint8Array>|Record<string,Uint8Array>): Quill` (static) · `toTree(): Map<string,Uint8Array>` · get `backendId` · get `blueprint` · get `schema: QuillSchema` · get `metadata: QuillMetadata` · `validate(doc): Diagnostic[]` · `resolve(doc): Resolved` · `seedDocument(): Document` · `seedMain(): Card` · `seedCard(cardKind, overlay?): Card | undefined`.

**`Document`** (engine.rs:450, every build). Constructors/codec: `constructor(quillRef)` · `fromMarkdown(md): Document` (static) · `fromJson(json): Document` (static) · `tryFromJson(json): Document | undefined` (static) · `schemaVersionOf(json): string|undefined` (static) · `currentSchemaVersion(): string` (static) · `formatRules(): string` (static) · `blueprintInstruction(quillName): string` (static) · `quillRefHint(): string` (static) · `formatDiagnostic(diag): string` (static) · `toMarkdown()` · `toJson()` · `clone()` (`js_name = clone`, engine.rs:897) · `loadJson(json)` (mutates in place — the only cross-linear-memory write-back path, engine.rs:914). Getters: `quillRef`, `main: Card`, `cards: Card[]`, `cardCount`, `warnings: Diagnostic[]`. Reads: `getStored(addr)` · `getMarkdown(cardAddr?)` (body-only, throws on a present `field`) · `isFill(addr)` · `getExt(cardAddr?)` · `getExtNamespace(cardAddr, ns)` · `card(index): Card` · `cardIndexById(id): number|undefined` · `seedOverlay(kind)` · `equals(other)`. Mutators: `storeField(addr, value)` · `storeFill(addr, value)` · `storeFields(cardAddr, fields)` · `removeField(addr)` · `storeExt(cardAddr, value)` · `removeExt(cardAddr?)` · `storeExtNamespace(cardAddr, ns, value)` · `removeExtNamespace(cardAddr, ns)` · `storeSeedNamespace(kind, overlay)` · `removeSeedNamespace(kind)` · `setQuillRef(ref)` · `install(addr, content)` · `revise(addr, md): Delta` · `applyChange(addr, bundle)` · `makeCard(kind, fields?, body?): Card` (static) · `insertCard(card, at?)` · `removeCard(index): Card|undefined` · `moveCard(from, to)` · `setCardKind(index, kind)`. Underscored ABI hidden from `.d.ts` (`skip_typescript`, called only by `runtime.js`): `_readerGet(quill, addr)` (engine.rs:1016), `_reviseField`→`reviseField` visible as `_reviseField` (engine.rs:1446), `_commitField` (engine.rs:1519), `_commitFields` (engine.rs:1552), `_addCard` (engine.rs:1588).

**`LiveSession`** (engine.rs:425, render builds only): get `pageCount`, `backendId`, `supportsCanvas`, `warnings: Diagnostic[]` · `apply(doc): ChangeSet` · `render(opts?): RenderResult` · `regions(): FieldRegion[]` · `fieldBoxes(field): FieldRegion[]` · `fieldAt(page,x,y): string|undefined` · `positionAt(page,x,y): ContentHit|undefined` · `locate(field,pos): FieldRegion|undefined` · `pageSize(page): PageSize` · `paint(ctx, page, opts?): PaintResult`.

**Free functions** (engine.rs, every build unless noted): `init()` (lib.rs:62, `#[wasm_bindgen(start)]`) · `importMarkdown(md): Content` · `exportMarkdown(content): string` · `rebase(base, md): {content, delta}` · `mapPos(delta, pos, assoc): number` · `parseDocPath(path): DocPathSeg[]` · `formatDocPath(segs): string`.

**Data types** (`types.rs`, `Tsify`-derived structs/enums crossing via `into_wasm_abi`/`from_wasm_abi`): `OutputFormat` (`"pdf"|"svg"|"txt"|"png"`, render builds only) · `Severity` (`"error"|"warning"`) · `Location` · `Diagnostic` · `Artifact` (render builds) · `RenderResult` (render builds) · `ChangeSet` (render builds) · `FieldRegion` (render builds) · `HitGranularity` (render builds) · `ContentHit` (render builds) · `RenderOptions` (render builds). Plus TS-only interfaces emitted via `typescript_custom_section` (no Rust struct, JSON-shaped at the boundary): `QuillFieldUi`, `QuillGroupUi`, `QuillCardUi`, `QuillCardBody`, `QuillFieldSchema`, `QuillCardSchema`, `QuillSchema`, `QuillMetadata`, `PathStep`, `PayloadItem`, `Card`, `CardInput`, `Content`, `ContentLine`, `ContentContainer`, `ContentMark`, `TableCell`, `TableProps`, `ImageProps`, `ContentIsland`, `Addr`, `CardAddr`, `Delta`, `Assoc`, `MarkOp`, `LineOp`, `ChangeBundle`, `DocPathSeg`, `FieldSource`, `ResolvedField`, `ResolvedMain`, `ResolvedCard`, `Resolved`, `PageSize`, `PaintOptions`, `PaintResult` (render builds).

**Public npm root** (`runtime/runtime.js` / `.d.ts`, re-exports `Quill`/`Document`/`init`/content codec/`parseDocPath`/`formatDocPath` verbatim from the core build, plus): `MAIN_CARD_ADDR` (frozen `{}`) · `isQuillmarkError(e)` · `isTableIsland`/`isImageIsland`/`isLinkMark`/`isAnchorMark` (open-set narrowing guards) · class `Engine` (`constructor(options?)`, `render`, `open`, `supportedFormats`, `supportsCanvas` — all `async`) · class `LiveSession` (thin wrapper over the FFI session; same surface, `apply` re-wraps `doc` into the backend's memory) · class `DocumentWriter`/`CardWriter` (`quill.writer(doc)`, patched onto the re-exported `Quill.prototype`; `set`/`setAll`/`setBody`/`reviseField`/`addCard`/`removeCard`/`card(i)`) · class `DocumentReader`/`CardReader` (`quill.reader(doc)`; `get`/`getBody`/`card(i)`).

## Findings

### `Quill.resolve` has no Python counterpart, and canon doesn't scope it out
**Severity: High.** `engine.rs:688-697` exposes `Quill.resolve(doc): Resolved` — per-field resolved value + `FieldSource` provenance (`authored`/`default`/`zero`), tested in `basic.test.js:1786-1937`. `crates/bindings/python/src/types.rs`'s `PyQuill` `#[pymethods]` block (lines 101-256) has no `resolve` method. `prose/canon/BINDINGS.md`'s parity table and Python scope note (`## Python`, lines 140-160) enumerate every deliberate Python omission (opaque store, content lane) as "WASM-only by scope, not by lag" but never mentions `resolve` — so this reads as undocumented lag, not an intended asymmetry. Caller impact: a Python consumer building a form/preview UI has `validate` (errors) and `schema` (guidance) but no single call for "what value will render, and why" — it must reimplement the authored›default›zero ladder client-side or fall back to `Document` payload inspection.

### `Quill::from_tree` (in-memory quill construction) is WASM-only; Python has no counterpart to `from_path`
**Severity: Medium.** Core's `Quill::from_tree` (`crates/core/src/quill/mod.rs`, re-exported `crates/core/src/lib.rs:64`) is a pure, filesystem-free constructor — `prose/canon/QUILL.md:51-59` documents it as the base primitive that `Quill.fromTree` (WASM, `engine.rs:558`) exposes directly. `crates/bindings/python/src/types.rs:106-110` exposes only `Quill.from_path` (filesystem walk via `quillmark::quill_from_path`). A Python service receiving quill bytes over a network boundary (object storage, an upload, a zip) has no way to construct a `Quill` without first materializing it to a temp directory. `Quill.toTree()` (the inverse, `engine.rs:578`) is likewise WASM-only. Not called out in `BINDINGS.md`'s parity table.

### `Document.cardIndexById` (durable `$id` lookup) is WASM-only
**Severity: Medium.** `engine.rs:1128-1134` wraps core's `Document::find_card`, documented in `prose/canon/DOCUMENT_STORAGE.md:211-241` (§ Card-id identity) as general-purpose durable addressing, not an editor-only concern. Python's `PyDocument` has no equivalent — a Python caller must linear-scan `doc.cards` and compare `.get("id")` by hand. Absent from the `BINDINGS.md` parity table's scope notes, so likely lag rather than intent.

### `parseDocPath`/`formatDocPath` are WASM-only
**Severity: Low.** `engine.rs:2029-2063` exports the `DocPath` parser/serializer so a consumer routes on `Diagnostic.path` segments instead of regexing the string (documented in `prose/canon/ERROR.md:150-204`). Python diagnostics carry the same `path` string (`PyDiagnostic.path`, `types.rs:1175`) but no structured parser — a Python consumer that wants to branch on path shape must hand-roll the `main.<field>` / `cards.<kind>[<i>].<field>` grammar. Low severity: the string form is still usable directly, and Python's typical consumer (server-side validation display) rarely needs structural routing.

### `Quillmark.registeredBackends()` is Python-only
**Severity: Low.** `crates/bindings/python/src/types.rs:84-90` exposes `Quillmark.registered_backends() -> Vec<String>`; the WASM `Quillmark` class (`engine.rs:402-547`) has no equivalent introspection method (the runtime `Engine` derives capability from its own descriptor manifest instead, `runtime.js:394-421`, so the *effective* answer is reachable via `Object.keys` on the options passed to `new Engine({backends})`, but the built-in registry itself isn't introspectable). Minor — a debugging/discovery convenience, not a functional gap.

### `Document.card(index)` single-card getter has no Python equivalent
**Severity: Low.** WASM `Document.card(index)` (`engine.rs:1119-1122`) reads one composable card without materializing the whole array; Python's `PyDocument` only exposes the O(n) `cards` getter (`types.rs:443-450`; `Writer.card`/`Reader.card` exist but return write/read cursors, not a plain card snapshot). Minor ergonomic/perf gap for documents with many cards.

## Parity gaps vs Python

| Concept | WASM | Python |
|---|---|---|
| Resolved-value view (value + `FieldSource` per field) | `quill.resolve(doc): Resolved` | absent |
| In-memory quill construction | `Quill.fromTree(tree)` / `quill.toTree()` | absent (`Quill.from_path` only, filesystem) |
| Durable `$id` → index lookup | `doc.cardIndexById(id)` | absent (linear-scan `doc.cards` by hand) |
| `DocPath` structural parse/format | `parseDocPath(str)` / `formatDocPath(segs)` | absent (raw `Diagnostic.path` string only) |
| Single composable-card read | `doc.card(index)` | absent (`doc.cards[index]`, O(n) materialize) |
| Engine backend introspection | absent (descriptor manifest passed to `new Engine`, not queryable) | `Quillmark.registered_backends()` |
| Live preview / canvas (`open`, `LiveSession`, `paint`, `regions`, `fieldAt`, `positionAt`, `locate`) | full surface | absent — **documented non-goal**, `prose/canon/PREVIEW.md` § Non-goals: "Native (CLI / Python) exposure. Capability is WASM-only." |
| Opaque verbatim store (`storeField`/`storeFields`/`storeFill`) | present | absent — **documented scope**, `BINDINGS.md` § Python: "WASM-only by scope, not by lag" (#970) |
| Content lane (`install`/`revise`/`applyChange`/`importMarkdown`/`exportMarkdown`/`rebase`/`mapPos`) | present | absent — **documented scope**, same note |
| Quill-free field reads (`getStored`/`isFill`/`getExt`/`getExtNamespace`) | present | absent — **documented scope** (`$ext`/body read off `main`/`cards` dict snapshots instead) |
| `writer.reviseField` return value | returns `Delta` | discards it (documented — "the position-mapping receipt is an editor concern, WASM-only") |
| Error contract | `WasmError` → thrown `Error` + `.diagnostics` | `QuillmarkError` + `.diagnostics` — same shape, same count-based `.message` rule |
| Output format representation | string-literal union `"pdf"\|"svg"\|"txt"\|"png"` | real enum class `OutputFormat.PDF/SVG/TXT/PNG` | idiom difference only, not a gap |

## Cross-cutting

- `Quill.resolve` and `Document.cardIndexById` are core capabilities (`crates/core/src/quill/mod.rs`, `Document::find_card`) with no `BINDINGS.md` parity-table row — worth a canon pass to either scope them explicitly (Python-omit like the opaque store) or track their Python port under #970.
- `Quill::from_tree` vs `quill_from_path` is a `quillmark-core`/`quillmark` split (`ARCHITECTURE.md:23,38`); Python only wraps the latter. Adding `Quill.from_tree`/`to_tree` to Python is a `crates/bindings/python` change, not a WASM one — flagging here since the asymmetry is only visible by reading both bindings side by side.
- The `LiveSession`/canvas non-goal for native bindings (`PREVIEW.md` § Non-goals) is correctly honored on both sides — no drift to report there.
- Error shapes (`Diagnostic`, namespaced `edit::*`/`validation::*`/`parse::*` codes, count-based `.message` aggregation) are faithfully mirrored between `WasmError` (`error.rs`) and Python's `errors.rs` — both delegate to the same core `Diagnostic`/`EditError::code()`/`RenderError::summary_message`, so this is a genuine shared-currency success, not a finding.
