# Error Handling System

> **Implementation**: `crates/core/src/`

## TL;DR

Every failure travels as a `Diagnostic`: severity, namespaced `code`, message,
optional text `location` and document-model `path`. `RenderError` carries a
non-empty `Vec<Diagnostic>` and has no failure taxonomy beyond them —
consumers route on codes, not types. Warnings ride the same currency and
never block.

## Types

**`Severity`**: `Error` | `Warning`. Fatality is this two-value ladder and
nothing else: `Error` blocks the stage that emits it, `Warning` never does.
There is no lint-level configuration and no warning-to-error promotion; an
informational aside is a `hint` on the diagnostic it annotates, not a
severity.

**`Location`**: file name, line (1-indexed), column (1-indexed)

**`Diagnostic`**: severity, optional error `code`, `message`, optional `location` (text anchor: file/line/column), optional `path` (document-model anchor — dotted/bracketed path into the typed `Document`, set by schema validation/coercion), optional `hint`, `source_chain` (omitted from serialization when empty). `location` and `path` are independent and may co-exist.

**`ParseError`**: parsing-stage error enum — `InputTooLarge`, `InvalidStructure`, `EmptyInput`, `MissingQuill`, `InvalidQuillReference`, `YamlErrorWithLocation`; converts to `Diagnostic` via `to_diagnostic()`. The `InvalidQuillReference` case (`parse::invalid_quill_reference`) attaches the canonical `$quill` grammar — `quill_ref_hint()` — as the diagnostic hint. That hint is the single source of truth for the reference grammar: bindings surface it verbatim (e.g. WASM `Document.quillRefHint`) rather than re-stating the rule.

**`YamlError`**: the one adapter every `serde-saphyr` error passes through. Sanitizes the message (the engine appends its own Rust API names — `from_multiple`, `DuplicateKeyPolicy` — which `yaml_hints::enrich_yaml_error` strips), derives the hint, and carries the 1-indexed line/column the engine located; `to_diagnostic(code, file)` renders all three. The emit side has no input to point at, so it carries neither position nor hint.

It exists so no public signature names `serde-saphyr`, whose `0.0.x` series makes every release of it a semver break. That promise covers the message as much as the type, which is why the sanitizer is inside the adapter rather than beside it.

Two surfaces return one directly (`QuillValue::from_yaml_str`, `QuillConfig::schema_yaml`); `QuillConfig::from_yaml_with_warnings` converts through it to `quill::yaml_parse_error`. The card-yaml path does not travel this way — it becomes `ParseError::YamlErrorWithLocation`, which additionally knows the enclosing block.

**`RenderError`**: the main rendering error — a struct carrying a non-empty
`Vec<Diagnostic>` (`RenderError::new` / `from_diag`; `diagnostics()` borrows,
`into_diagnostics()` consumes). There is no failure taxonomy beyond the
diagnostics themselves: the machine-routable identity of a failure is each
diagnostic's namespaced `code` (`parse::*`, `validation::*`, `quill::*`,
`edit::*`, `typst::*`, `pdfform::*`, `backend::*`, `engine::*`, `binding::*`) — consumers
route on codes, not on a type. Multi-problem stages (validation, quill config, backend
compilation) carry several diagnostics so every problem reaches the caller in
one pass. `Display` follows the count-based message rule shared with both
bindings: the primary diagnostic's message for a single diagnostic, an
`"<N> error(s): <first message>"` aggregate for more.

Notable codes: `quill::name_mismatch` / `quill::version_mismatch` — the
document is well-formed but paired with the wrong quill (see
[VERSIONING.md](VERSIONING.md)); `backend::apply_unsupported` — the default
for a backend session that does not override the incremental-`apply` seam
(both built-in backends override it); `backend::format_not_supported` — the
requested format is outside the backend's `supported_formats`, one code on
every backend so a caller matches the condition once; `engine::backend_not_found`
— the quill's declared backend is not registered.

**`edit::*` — mutator diagnostics.** Document and card mutators fail with the
`EditError` enum (`crates/core/src/document/edit.rs`), one namespaced code per
variant via `EditError::code()` (`edit::invalid_field_name`,
`edit::unknown_field`, `edit::index_out_of_range`, `edit::field_conform`, …).
Both bindings stamp that code onto the `Diagnostic` they raise — the mutator
peer of the render-path namespaces. Identity is the code, never message text:
routing coercion-vs-undeclared is `edit::field_conform` vs.
`edit::unknown_field`, read off `diagnostics[0].code`.

**`binding::*` — boundary diagnostics.** The codes a binding mints itself, for
the failures that never reach a core producer: a JS value the signature cannot
accept (`binding::invalid_argument`, the floor), a storage DTO that is not one
(`binding::invalid_dto`), a value that is not canonical Content
(`binding::invalid_content`), a result the boundary could not serialize back
(`binding::serialize`), and a canvas host object the paint path cannot use
(`binding::canvas`). The uncoded-message door (`From<String> for WasmError`) is
floored at `invalid_argument`, so no diagnostic reaches a consumer without a
routable code — the promise the rest of this document makes, kept at the surface
that produces caller mistakes. A core producer's own code is never overwritten:
an `edit::*` mutator failure crosses the boundary as itself.

**`RenderResult`**: successful result carrying artifacts, output format, and non-fatal `Vec<Diagnostic>` warnings

## Warning flow

Warnings travel the same `Diagnostic` currency as errors, on three producer
families:

- **Parse warnings** — the `warnings` on the `Parsed` that `Document::parse`
  returns (e.g. a `~~~` opener missing its blank line). The CLI render and the
  WASM one-shot render splice them into `RenderResult.warnings` ahead of any
  compile warnings.
- **Validation warnings** — `Quill::validate(doc)` returns every
  `validation::*` diagnostic, mixing severities; `validation::must_fill` and
  the `$seed` checks are the non-fatal ones. This is the editor-facing
  surface; the render pipeline zero-fills instead of warning on incomplete
  documents.
- **Compile warnings** — the Typst backend maps `typst::compile`'s non-fatal
  diagnostics (font fallback, overfull pages, …) through the same span
  resolution as errors. They are state of the session's current compile:
  exposed via `LiveSession::warnings()` (the `SessionHandle::warnings` seam,
  default empty), refreshed by each committed `apply` — a failed apply keeps
  the last-good compile *and* its warnings — and appended to
  `RenderResult.warnings` on every `render()`, including the one-shot
  `open` → `render` path.

Ordering in a merged `RenderResult.warnings` is pipeline order: parse
warnings first, then compile warnings. No dedup — the families cannot
overlap (parse warnings anchor `location` in the markdown source, compile
warnings in Typst sources).

## Bindings Error Delegation

Python and WASM bindings delegate to core types:

- **Python**: `PyDiagnostic` wraps `Diagnostic`. Every raised exception is `QuillmarkError` (a single type). Every exception carries a `diagnostics` list; `str(exc)` follows the shared count-based message rule.
- **WASM**: `WasmError` carries a single `diagnostics: Vec<Diagnostic>` (always non-empty). The thrown JS `Error` has a `.diagnostics` array attached and a `.message` derived from `diagnostics` by the same count-based rule. Consumers read `err.diagnostics[0]` for the primary diagnostic and iterate `err.diagnostics` for the rest. Parse failures (`Document.fromMarkdown`) carry the same shape — including the `parse::input_too_large` diagnostic for inputs over `MAX_INPUT_SIZE` (10 MB) and the `edit::*` codes for post-parse mutators.

## Backend Error Mapping

### Typst

Typst diagnostics mapped via `map_typst_errors()`:
- Severity levels mapped (Error/Warning)
- Spans resolved to file/line/column
- Error codes: `"typst::<message-prefix>"` (the diagnostic message text up to the first `:`)

See `crates/backends/typst/src/error_mapping.rs`.

**Quill-load warnings** are the backend's other warning source, hand-coded
rather than derived from a Typst diagnostic: `typst::path_skipped` (a file
Typst's `VirtualPath` rejected — asset or package file alike),
`typst::package_manifest`, and `typst::package_entrypoint_missing`. Each marks a
file the world had to skip, which otherwise surfaces only as an unresolved
`#import` pointing at the plate instead of at the defect. They are properties of the quill, not of a compile, so
`QuillWorld` holds them and the session serves them ahead of every compile's own
— an `apply` swaps the compile half and keeps these.

## Validation message contract

Field-level validation diagnostics — `validation::type_mismatch` (fatal) and
`validation::must_fill` (non-fatal, `Severity::Warning`) — emit a single
canonical shape:

- **Field path** — the document-model anchor of the offending field
  (`recipient`, `cards.indorsement[2].author`); see [Document-model
  paths](#document-model-paths).
- **Source token** — the YAML scalar that triggered the error, rendered
  verbatim in its YAML-canonical form (`42`, `null`, `true`, `""`). Strings
  appear quoted; primitives appear bare. (Absent fields have no source
  token.)
- **Schema declaration** — the field's declared type and, when present,
  its default. Defaults render with the same verbatim formatting.
- **Both exits when applicable** — the message names two ways out. The
  parser does not silently coerce; the message is the lever.

Example messages:

```
Field `build_number` got integer `42`, schema declares `string`.
Either quote the value (`build_number: "42"`) or change the schema's
`type:` to `integer`.
```

```
Field `name` is marked `!must_fill` — a placeholder awaiting a value.
```

with the hint *"Replace the value and drop the `!must_fill` marker, or remove
the marker if the current value is intended."* It is a warning, not an error:
the field still renders (the marked cell zero-fills or uses its suggested
value).

A present-null value (`subtitle:`, `subtitle: null`, `subtitle: ~`) is
treated exactly like an omitted field — null ≡ absent. It validates clean
and zero-fills at render (authored › `default:` › type-zero), so it produces
no diagnostic. Field absence is likewise not surfaced as a diagnostic (see
[SCHEMAS.md](SCHEMAS.md) § "Native validation"), so a merely incomplete
document also produces no field-level diagnostic.

Implementation: `crates/core/src/quill/validation.rs` (the `ValidationError`
`Display` impl, for `validation::type_mismatch`) and
`crates/core/src/quill/compose.rs` (`validate_fills`/`fill_warning`, for
`validation::must_fill`).

## Document-model paths

`Diagnostic.path` is a **document-model** anchor into a typed `Document`:
one canonical grammar, one serializer, one parser — `DocPath`
(`crates/core/src/path.rs`). Every emit site (schema validation,
`!must_fill` collection) constructs a `DocPath`; no site assembles a path
with `format!`, so the engine never ships two shapes for one anchor.

| Anchor | Path |
|---|---|
| Main-card field | `main.recipient` |
| Nested in an array of objects | `main.recipients[0].name` |
| Main body | `main.body` |
| Typed card (whole) | `cards.indorsement[0]` |
| Field on a typed card | `cards.indorsement[0].signature_block` |
| Body on a typed card | `cards.indorsement[0].body` |
| Card with unknown kind | `cards[0]` |

Every path is **rooted** — a main field at `main.<field>`, a card field
kind-qualified at `cards.<kind>[<index>].<field>` (kind and document-array index
fused so a consumer gets both without a second lookup). The unknown-kind
whole-card `cards[<index>]` is the only bare-index form. Rooting keeps the
grammar total against a field named for a root (`main.cards`, `main.main`); only
a field literally named `body` still collides with the body terminal. Field
names and card kinds exclude `.`, `[`, `]`, so the rendered form round-trips;
the WASM build exports `parseDocPath` / `formatDocPath` (structured
`DocPathSeg[]` ↔ string) so a consumer routes on segments instead of regexing
the string.

`DocPath` is the anchor on **every** address that crosses to a consumer, not
only `Diagnostic.path`. Mutator (`edit::*`) diagnostics carry it (a field error
at `main.<field>` or `cards.<kind>[<i>].<field>`, a structural out-of-range op at
`cards[<i>]`);
and `LiveSession` geometry (`regions` / `fieldAt` / `positionAt` / `locate`)
keys on it — the session translates the backend's plate-space
`$cards.<kind>.<ordinal>` form to the `DocPath` absolute index at the boundary,
so one parser routes diagnostics and geometry alike.

**Three grammars, one that crosses.** Only `DocPath` reaches a consumer. The
other two stay backend/template-internal and are named here so they are not
confused with it:

- **Plate JSON** — the sigiled `data.$cards` a template author composes
  ([CARDS.md](CARDS.md)), and the plate-space `$cards.<kind>.<ordinal>.<field>`
  geometry address a plate's `$path` mints. A template-author contract, so it
  keeps its own spelling (renaming it is a blast radius) and translates to
  `DocPath` before it crosses.
- **Schema-space coercion anchors** — `CoercionError` keeps its own
  `card_kinds.<kind>.<field>` / bare-field anchors, a schema-declaration
  namespace, not a document path. Where a coercion becomes an
  `edit::field_conform`, the binding re-anchors it in `DocPath` space at the
  field being written; the raw schema-space anchor does not cross.

Config-space anchors (`$seed.<kind>.<field>`, Quill.yaml schema-literal owner
labels) ride the `DocPath` serializer with their prefix as a leading segment.

## Error Presentation

**Pretty printing** (`Diagnostic::fmt_pretty()`):
```
[ERROR] Undefined variable (E001)
  --> template.typ:10:5
  hint: Check variable spelling
```

**Source chain**: `with_source` walks an attached cause eagerly into `source_chain`. No Rust formatter prints it — `fmt_pretty` covers severity, message, code, location, and hint only. It reaches consumers through serialization instead: WASM as the `source_chain` field, Python as `Diagnostic.source_chain`.

**Consolidated printing**: `print_errors()` pretty-prints every diagnostic a `RenderError` carries.

**Machine-readable**: all diagnostic types implement `serde::Serialize`.
