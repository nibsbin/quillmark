// Test fixtures for e2e tests

/**
 * Small test Quill with basic Typst template
 */
export const SMALL_QUILL_JSON = {
  files: {
    'Quill.toml': {
      contents: `[Quill]
name = "test_quill"
backend = "typst"
glue = "glue.typ"
description = "Test quill for WASM e2e tests"
`,
    },
    'glue.typ': {
      contents: `= {{ title }}

{{ body | Content }}
`,
    },
  },
};

/**
 * Markdown document for testing with the test Quill
 */
export const SIMPLE_MARKDOWN = `---
title: Test Document
author: Alice
QUILL: test_quill
---

# Hello World

This is a test document with some **bold text** and *italic text*.

## Section 2

Here's a list:
- Item 1
- Item 2
- Item 3
`;

/**
 * Markdown document without QUILL field
 */
export const MARKDOWN_NO_QUILL = `---
title: No Quill Document
author: Bob
---

# Document Without Quill

This document doesn't specify a QUILL field.
`;

/**
 * Letter-style Quill with more features
 */
export const LETTER_QUILL_JSON = {
  files: {
    'Quill.toml': {
      contents: `[Quill]
name = "letter_quill"
backend = "typst"
glue = "glue.typ"
description = "Letter template for e2e tests"
`,
    },
    'glue.typ': {
      contents: `#set page(paper: "us-letter")

= {{ title }}

_{{ author }}_

{{ body | Content }}
`,
    },
  },
};

/**
 * Letter markdown document
 */
export const LETTER_MARKDOWN = `---
title: Important Letter
author: Charlie
date: 2025-10-17
QUILL: letter_quill
---

Dear Recipient,

This is an important letter. It has multiple paragraphs and demonstrates
the full workflow of the Quillmark WASM API.

## First Section

Here is some content in the first section.

## Second Section

And here is some content in the second section.

Sincerely,
Charlie
`;

/**
 * Invalid Quill JSON (missing Quill.toml)
 */
export const INVALID_QUILL_JSON = {
  files: {
    'glue.typ': {
      contents: '= Missing Quill.toml',
    },
  },
};

/**
 * Invalid markdown (malformed YAML)
 */
export const INVALID_MARKDOWN = `---
title: Unclosed
author: "Missing quote
---

Content here
`;
