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

### `quill.render(parsed, opts?)`
Render with a pre-parsed `Document`.

### `quill.open(parsed)` + `session.render(opts?)`
Open once, render all or selected pages (`opts.pages`).

## Notes

- Parsed markdown requires top-level `QUILL` in frontmatter.
- QUILL mismatch during `quill.render(parsed)` is a warning (`quill::ref_mismatch`), not an error.
- Output schema APIs are no longer engine-level in WASM.

## License

Apache-2.0
