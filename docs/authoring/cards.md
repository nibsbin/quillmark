# Cards

Quillmark supports inline metadata blocks for repeated structures using `CARD`.

## Card Block Syntax

Use metadata blocks with a `CARD` key:

```markdown
---
QUILL: my_quill@1.0
title: Main Document
---

# Introduction

Some content here.

---
CARD: products
name: Widget
price: 19.99
---

Widget description.

---
CARD: products
name: Gadget
price: 29.99
---

Gadget description.
```

All card blocks are collected into the `CARDS` array.

## Rules

- `CARD` creates a card block collected into `CARDS`.
- Card names must match `[a-z_][a-z0-9_]*`.
- `BODY` and `CARDS` are reserved names.
- `QUILL` cannot appear in card blocks.
- `---` is reserved for metadata delimiters and cannot be used as a thematic break in body content.
- Invalid card-name examples: `BadCard`, `my-card`, `2nd_card`.

## Card Body Content

Each card block includes a `BODY` field containing the Markdown between that card's metadata block and the next metadata block (or document end).
