import { Quillmark } from '../pkg/bundler/wasm.js'

const TEST_QUILL = {
  files: {
    'Quill.toml': {
      contents: `[Quill]
name = "test_quill"
backend = "typst"
glue = "glue.typ"
description = "Test quill"
`
    },
    'glue.typ': {
      contents: `= {{ title | String }}

{{ body | Content }}`
    }
  }
}

const TEST_MARKDOWN = `---
title: Test Document
author: Test Author
QUILL: test_quill
---

# Hello World`

console.log('Testing parsed document fields...');
const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN);
console.log('parsed.fields type:', typeof parsed.fields);
console.log('parsed.fields instanceof Map:', parsed.fields instanceof Map);
console.log('parsed.fields instanceof Object:', parsed.fields instanceof Object);
console.log('parsed.fields:', parsed.fields);
console.log('parsed.fields.title:', parsed.fields.title);
console.log('parsed.fields.get:', typeof parsed.fields.get);

console.log('\nTesting QuillInfo...');
const engine = new Quillmark();
engine.registerQuill('test_quill', TEST_QUILL);
const info = engine.getQuillInfo('test_quill');
console.log('info.metadata type:', typeof info.metadata);
console.log('info.metadata instanceof Map:', info.metadata instanceof Map);
console.log('info.metadata instanceof Object:', info.metadata instanceof Object);
console.log('info.metadata:', info.metadata);
console.log('info.fieldSchemas type:', typeof info.fieldSchemas);
console.log('info.fieldSchemas instanceof Map:', info.fieldSchemas instanceof Map);
console.log('info.fieldSchemas instanceof Object:', info.fieldSchemas instanceof Object);
console.log('info.fieldSchemas:', info.fieldSchemas);
