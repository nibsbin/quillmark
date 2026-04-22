# WASM Migration Guide

Migration guide for `@quillmark/wasm` consumers after the **Canonical Document
Model** refactor (commit `f8c7ee3`, PR #444).

Previous in-tree migration notes lived in `MIGRATION.md`, which was deleted as
part of this refactor. This document replaces them for the WASM surface only.

## TL;DR

```diff
- import { ParsedDocument, Quillmark } from "@quillmark/wasm";
+ import { Document, Quillmark } from "@quillmark/wasm";

  const engine = new Quillmark();
  const quill  = engine.quill(tree);

- const parsed = ParsedDocument.fromMarkdown(md);
- const title  = parsed.fields.title;
- const body   = parsed.fields.BODY;
- const cards  = parsed.fields.CARDS;
+ const doc    = Document.fromMarkdown(md);
+ const title  = doc.frontmatter.title;   // no QUILL / BODY / CARDS
+ const body   = doc.body;                // string (never undefined)
+ const cards  = doc.cards;               // array<{tag, fields, body}>

- const result = quill.render(parsed, { format: "pdf" });
+ const result = quill.render(doc,    { format: "pdf" });
```

There is **no compatibility alias**. `ParsedDocument` is gone from the exports
(`crates/bindings/wasm/src/lib.rs:36`). Consumers must rename.

---

## 1. `ParsedDocument` → `Document`

### Rename

| Before | After |
| --- | --- |
| `import { ParsedDocument } from "@quillmark/wasm"` | `import { Document } from "@quillmark/wasm"` |
| `ParsedDocument.fromMarkdown(md)` | `Document.fromMarkdown(md)` |

### Shape change: `fields` → `frontmatter` + `body` + `cards`

The old `ParsedDocument.fields` was a single flat object that included the
reserved keys `BODY` and (when present) `CARDS` alongside user frontmatter. The
new `Document` splits these into typed accessors:

| Before (flat `fields`)         | After (typed getters)                                   |
| ------------------------------ | ------------------------------------------------------- |
| `parsed.fields.title`          | `doc.frontmatter.title`                                 |
| `parsed.fields.BODY`           | `doc.body` — always a string (empty when absent)        |
| `parsed.fields.CARDS`          | `doc.cards` — always an array (empty when absent)       |
| `parsed.fields.QUILL`          | not in `frontmatter`; use `doc.quillRef`                |
| `parsed.quillRef`              | `doc.quillRef` (unchanged)                              |
| `parsed.warnings`              | `doc.warnings` (unchanged)                              |

`doc.frontmatter` **never** contains `QUILL`, `BODY`, or `CARDS`. Checking for
those keys in `frontmatter` always yields `undefined`.

### Shape change: `doc.cards[i]`

Each element is `{ tag: string, fields: Record<string, unknown>, body: string }`.
The `tag` reflects the card's `CARD:` sentinel value, not a reserved `CARD` key
inside `fields`.

```js
doc.cards[0].tag       // "note"
doc.cards[0].fields    // { foo: "bar" }   — no CARD key
doc.cards[0].body      // "Card body..."   — string, may be ""
```

### `Document` is now an opaque WASM handle, not a serialized plain object

This is the **subtlest** behavioural change and the one most likely to bite.

`ParsedDocument` used to round-trip through `serde-wasm-bindgen` as a plain JS
object — you could spread it, `JSON.stringify` it, and pass the same value to
`quill.render` multiple times. `Document` is a real `#[wasm_bindgen]` class
(`crates/bindings/wasm/src/engine.rs:54`). That has two consequences:

**a. Reading fields goes through getters.** `doc.frontmatter`, `doc.body`,
   `doc.cards`, `doc.warnings`, `doc.quillRef` are all getters that allocate
   and deserialize on every access. If you read them in hot loops, cache the
   value locally.

**b. `quill.render(doc)` and `quill.open(doc)` *consume* the handle.**
   Both take `Document` by value (`engine.rs:95`, `engine.rs:115`). After the
   call, the JS reference is moved into Rust and freed; any further access on
   the old reference throws *"null pointer passed to rust"*. In contrast,
   `quill.projectForm(doc)` takes `&Document` and leaves the handle usable.

   Old pattern that **stops working**:
   ```js
   const parsed = Document.fromMarkdown(md);
   const pdf = quill.render(parsed, { format: "pdf" });
   const svg = quill.render(parsed, { format: "svg" }); // ❌ throws
   ```

   Workarounds (pick one):
   ```js
   // (a) Parse once per render.
   const pdf = quill.render(Document.fromMarkdown(md), { format: "pdf" });
   const svg = quill.render(Document.fromMarkdown(md), { format: "svg" });

   // (b) Use quill.open() for multi-format / multi-page output from one parse.
   const session = quill.open(Document.fromMarkdown(md));
   const pdf = session.render({ format: "pdf" });
   const svg = session.render({ format: "svg" });
   const png = session.render({ format: "png", ppi: 300 });

   // (c) Emit → re-parse if you need a separate handle for a different call.
   const doc2 = Document.fromMarkdown(doc.toMarkdown());
   ```

   `quill.open(doc)` itself consumes the handle — the session owns the parse.
   The session is also the right entrypoint for page-selective rendering.

---

## 2. New editor surface on `Document`

`Document` now supports in-place mutation. Every mutator enforces the parser's
invariants and throws `EditError` (as a JS `Error` whose message starts with
`[EditError::<Variant>]`) on violations:

| Method                                        | Purpose                                    |
| --------------------------------------------- | ------------------------------------------ |
| `setField(name, value)`                       | Insert or replace a frontmatter field      |
| `removeField(name)`                           | Remove a frontmatter field (returns it)    |
| `setQuillRef(refString)`                      | Replace the `QUILL` reference              |
| `replaceBody(body)`                           | Replace the global Markdown body           |
| `pushCard({ tag, fields?, body? })`           | Append a card                              |
| `insertCard(index, { tag, fields?, body? })`  | Insert at `0..=cards.length`               |
| `removeCard(index)`                           | Remove and return the card (or `undefined`)|
| `moveCard(from, to)`                          | Reorder                                    |
| `updateCardField(index, name, value)`         | Convenience: edit a card's field           |
| `updateCardBody(index, body)`                 | Convenience: replace a card's body         |

`EditError` variants surfaced to JS: `ReservedName`, `InvalidFieldName`,
`InvalidTagName`, `IndexOutOfRange`. Reserved frontmatter field names are
`BODY`, `CARDS`, `QUILL`, `CARD`. Field names must match `[a-z_][a-z0-9_]*`
(NFC); tag names must match the tag grammar from the parser.

Mutators never modify `doc.warnings`; warnings remain a frozen record of the
original parse.

```js
const doc = Document.fromMarkdown(md);
doc.setField("title", "New title");
doc.pushCard({ tag: "note", fields: { author: "Alice" }, body: "Hello" });

try {
  doc.setField("BODY", "x");              // throws
} catch (e) {
  // e.message starts with "[EditError::ReservedName] ..."
}
```

---

## 3. New emitter: `doc.toMarkdown()`

`doc.toMarkdown()` returns canonical Quillmark Markdown. It is type-fidelity
round-trip safe:

```js
const doc2 = Document.fromMarkdown(doc.toMarkdown());
// doc2 equals doc by value AND by type variant.
```

This is the fix for the YAML "Norway" / numeric-string / date-string bug
family: strings are always double-quoted on emission, so `"on"`, `"off"`,
`"01234"`, `"2024-01-15"`, `"null"` all survive as strings through the
round-trip.

Use this when a form editor mutates a parsed document and needs to persist
back to `.md` on disk.

---

## 4. New: `quill.projectForm(doc)`

Schema-aware projection for form editors. Returns a plain JSON-ready object
(not a class) with the shape:

```ts
{
  main:  { schema: {...}, values: Record<string, FieldSource> },
  cards: Array<{ tag: string, schema: ..., values: ..., diagnostics: [...] }>,
  diagnostics: Diagnostic[],
}
```

Each `FieldSource` carries the value plus a discriminator
(`Document | Default | Missing`). It is a **snapshot** — subsequent mutations
on `doc` require calling `projectForm` again.

This takes `&Document`, so the handle survives the call.

---

## 5. Render options — `assets` field removed

`RenderOptions` shape on the wire:

```ts
{ format?: "pdf"|"svg"|"png"|"txt", ppi?: number, pages?: number[] }
```

Dynamic asset injection was removed from the pipeline in this refactor.
`RenderOptions.assets` was **deleted** from the WASM surface — it is no longer
part of the TypeScript type and passing it is now a type error at compile
time (or an unknown-property warning in plain JS).

**Migration:** move any assets or fonts you were injecting through
`RenderOptions.assets` into the quill tree you pass to `engine.quill(tree)`:

```diff
  const tree = new Map();
  tree.set("Quill.yaml", quillYamlBytes);
  tree.set("plate.typ", plateBytes);
+ tree.set("assets/logo.png", logoBytes);
+ tree.set("assets/fonts/MyFont-Regular.ttf", fontBytes);
  const quill = engine.quill(tree);
```

Assets and fonts travel through the file tree only.

---

## 6. Quick reference: full before/after

```js
// ── Before ────────────────────────────────────────────────────────────────
import { ParsedDocument, Quillmark } from "@quillmark/wasm";

const engine = new Quillmark();
const quill  = engine.quill(tree);

const parsed = ParsedDocument.fromMarkdown(md);
console.log(parsed.fields.title, parsed.fields.BODY);

const r1 = quill.render(parsed, { format: "pdf" });
const r2 = quill.render(parsed, { format: "svg" }); // was fine


// ── After ─────────────────────────────────────────────────────────────────
import { Document, Quillmark } from "@quillmark/wasm";

const engine = new Quillmark();
const quill  = engine.quill(tree);

const doc = Document.fromMarkdown(md);
console.log(doc.frontmatter.title, doc.body);

// Option A: one parse per render
const r1 = quill.render(Document.fromMarkdown(md), { format: "pdf" });
const r2 = quill.render(Document.fromMarkdown(md), { format: "svg" });

// Option B: open a session
const session = quill.open(Document.fromMarkdown(md));
const rA = session.render({ format: "pdf" });
const rB = session.render({ format: "svg" });
const rC = session.render({ format: "png", ppi: 300, pages: [0, 2] });
```

---

## Unchanged

The following are behaviorally unchanged by this refactor:

- `new Quillmark()` constructor.
- `engine.quill(tree)` where `tree` is `Map<string, Uint8Array>`.
- `quill.open(doc)` → `session.pageCount` + `session.render(opts)`.
- `quill.backendId` getter.
- `RenderResult` shape: `{ artifacts, warnings, outputFormat, renderTimeMs }`.
- `Diagnostic` shape: `{ severity, code?, message, location?, hint?, sourceChain }`.
- QUILL-ref mismatch behaviour: `quill.render(doc)` with a mismatched
  `doc.quillRef` still emits a `quill::ref_mismatch` warning, not an error.
- npm package name and import path.

---

## Leftovers cleaned up alongside this migration

This migration pass also resolved stale references to the removed APIs:

- **`RenderOptions.assets`** — deleted from `crates/bindings/wasm/src/types.rs`.
  The TypeScript type no longer exposes it. Inject assets through the quill
  tree.
- **`docs/format-designer/typst-backend.md`** — Python and JS code snippets
  rerouted from `workflow.render(parsed, …)` to `quill.render(doc, …)`.
- **`prose/schema-rework/`** — deleted. The plan's success criteria (delete
  `schema.rs`, drop `jsonschema` crate, remove `SchemaProjection`, expose
  `FormProjection` through bindings) all landed in the Document refactor, so
  the planning directory followed the same pattern as the 30+ other landed
  plans purged by the refactor.
- **`crates/bindings/python/examples/workflow_demo.py`** → renamed to
  `quill_demo.py`. Docstring updated to drop the removed `Workflow`
  terminology.
