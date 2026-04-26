# Quillmark WASM

WebAssembly bindings for Quillmark.

Maintained by [TTQ](https://tonguetoquill.com).

## Overview

Use Quillmark in browsers/Node.js with explicit in-memory trees (`Map<string, Uint8Array>` / `Record<string, Uint8Array>`).

## Build

```bash
wasm-pack build --target bundler --scope quillmark
```

## Test

```bash
bash scripts/build-wasm.sh
cd crates/bindings/wasm
npm install
npm test
```

## Usage

```ts
import { Document, Quillmark } from "@quillmark-test/wasm";

const engine = new Quillmark();
const quill = engine.quill(tree);

const markdown = `---
QUILL: my_quill
title: My Document
---

# Hello`;

const parsed = Document.fromMarkdown(markdown);
const result = quill.render(parsed, { format: "pdf" });
```

## API

### `new Quillmark()`
Create engine.

### `engine.quill(tree)`
Build + validate + attach backend. Returns a render-ready `Quill`.

### `Document.fromMarkdown(markdown)`
Parse markdown to parsed document.

### `doc.toMarkdown()`
Emit canonical Quillmark Markdown. Type-fidelity round-trip safe:
`Document.fromMarkdown(doc.toMarkdown())` returns a document equal to `doc`.

### `quill.render(parsed, opts?)`
Render with a pre-parsed `Document`.

### `quill.open(parsed)` + `session.render(opts?)`
Open once, render all or selected pages (`opts.pages`).

### Errors

Every method that can fail throws a JS `Error` with a flat shape:

```ts
{ message: string, diagnostics: Diagnostic[] }
```

`diagnostics` is always non-empty — length 1 for most failures, length N for
backend compilation errors. Read `err.diagnostics[0]` for the primary
diagnostic; iterate the array for compilation failures.

## Notes

- Parsed markdown requires top-level `QUILL` in frontmatter. Empty input
  surfaces a dedicated "Empty markdown input cannot be parsed" message.
- QUILL mismatch during `quill.render(parsed)` is a warning (`quill::ref_mismatch`), not an error.
- Output schema APIs are no longer engine-level in WASM.

## License

Apache-2.0
