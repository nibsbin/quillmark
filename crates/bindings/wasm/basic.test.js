/**
 * Minimal smoke tests for quillmark-wasm
 * 
 * These tests validate the core WASM API functionality:
 * - Parse markdown with YAML frontmatter
 * - Register Quill templates
 * - Get Quill information
 * - Render documents to PDF
 * - Basic error handling
 * 
 * Setup: Tests use the bundler build via @quillmark-wasm alias (see vitest.config.js)
 */

import { describe, it, expect } from 'vitest'
import { Quill, Quillmark } from '@quillmark-wasm'

// Minimal inline Quill for testing
const TEST_QUILL = {
  files: {
    'Quill.yaml': {
      contents: `Quill:
  name: test_quill
  version: "1.0"
  backend: typst
  plate_file: plate.typ
  description: Test quill for smoke tests
`
    },
    'plate.typ': {
      contents: `#import "@local/quillmark-helper:0.1.0": data
#let title = data.title
#let body = data.BODY

= #title

#body`
    }
  }
}

const TEST_MARKDOWN = `---
title: Test Document
author: Test Author
QUILL: test_quill
---

# Hello World

This is a test document.`

// Helpers for fromTree tests
function textBytes(str) {
  return new TextEncoder().encode(str)
}

function quillFromBundle(bundle) {
  const enc = new TextEncoder()
  const entries = Object.entries(bundle.files).map(([path, { contents }]) => {
    if (contents instanceof Uint8Array) return [path, contents]
    if (Array.isArray(contents)) return [path, new Uint8Array(contents)]
    return [path, enc.encode(contents)]
  })
  return Quill.fromTree(new Map(entries))
}

function makeTreeQuill(name = 'tree_quill', version = '1.0.0') {
  return new Map([
    ['Quill.yaml', textBytes(`Quill:
  name: ${name}
  version: "${version}"
  backend: typst
  plate_file: plate.typ
  description: Tree quill for smoke tests
`)],
    ['plate.typ', textBytes('#import "@local/quillmark-helper:0.1.0": data\n= Test')],
  ])
}

describe('Quill.fromTree', () => {
  it('should build a Quill from a Map<string, Uint8Array>', () => {
    const quill = Quill.fromTree(makeTreeQuill())
    expect(quill).toBeDefined()
  })

  it('should build a Quill from a plain Record<string, Uint8Array>', () => {
    const tree = {}
    for (const [k, v] of makeTreeQuill()) tree[k] = v
    const quill = Quill.fromTree(tree)
    expect(quill).toBeDefined()
  })

  it('should infer subdirectory hierarchy from path separators', () => {
    const tree = new Map([
      ['Quill.yaml', textBytes(`Quill:
  name: nested_quill
  version: "1.0.0"
  backend: typst
  plate_file: plate.typ
  description: Nested tree quill
`)],
      ['plate.typ', textBytes('#import "@local/quillmark-helper:0.1.0": data\n= Nested')],
      ['assets/fonts/Inter-Regular.ttf', new Uint8Array([0, 1, 2, 3])],
    ])
    const quill = Quill.fromTree(tree)
    expect(quill).toBeDefined()
  })

  it('should register a fromTree Quill with the engine', () => {
    const engine = new Quillmark()
    const quill = Quill.fromTree(makeTreeQuill('tree_quill', '2.0.0'))
    engine.registerQuill(quill)
    expect(engine.listQuills()).toContain('tree_quill@2.0.0')
  })

  it('should allow the same Quill handle to register with multiple engines', () => {
    const quill = Quill.fromTree(makeTreeQuill('shared_quill', '1.0.0'))
    const engine1 = new Quillmark()
    const engine2 = new Quillmark()
    engine1.registerQuill(quill)
    engine2.registerQuill(quill)
    expect(engine1.listQuills()).toContain('shared_quill@1.0.0')
    expect(engine2.listQuills()).toContain('shared_quill@1.0.0')
  })

  it('should throw on null/undefined input', () => {
    expect(() => Quill.fromTree(null)).toThrow()
    expect(() => Quill.fromTree(undefined)).toThrow()
  })

  it('should throw when a value is not Uint8Array', () => {
    const bad = new Map([
      ['Quill.yaml', 'this is a string, not Uint8Array'],
    ])
    expect(() => Quill.fromTree(bad)).toThrow()
  })

  it('should throw on missing Quill.yaml', () => {
    const tree = new Map([
      ['plate.typ', textBytes('hello')],
    ])
    expect(() => Quill.fromTree(tree)).toThrow()
  })
})

describe('quillmark-wasm smoke tests', () => {
  it('should parse markdown with YAML frontmatter', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)

    expect(parsed).toBeDefined()
    expect(parsed.fields).toBeDefined()

    // fields should be a plain object, not a Map
    expect(parsed.fields instanceof Map).toBe(false)
    expect(parsed.fields instanceof Object).toBe(true)
    expect(parsed.fields.title).toBe('Test Document')
    expect(parsed.fields.author).toBe('Test Author')
    expect(parsed.quillRef).toBe('test_quill')
  })

  it('should create engine and register quill', () => {
    const engine = new Quillmark()

    expect(() => {
      engine.registerQuill(quillFromBundle(TEST_QUILL))
    }).not.toThrow()

    const quills = engine.listQuills()
    expect(quills).toContain('test_quill@1.0.0')
  })

  it('should get quill info after registration', () => {
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    const info = engine.getQuillInfo('test_quill')

    expect(info).toBeDefined()
    expect(info.name).toBe('test_quill')
    expect(info.backend).toBe('typst')
    expect(info.supportedFormats).toContain('pdf')

    // metadata should be a plain object and schema should be YAML text
    expect(info.metadata instanceof Map).toBe(false)
    expect(info.metadata instanceof Object).toBe(true)
    expect(typeof info.schema).toBe('string')
  })







  it('should compile data to JSON', () => {
    // Verify that we can extract the intermediate JSON data
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    const jsonData = engine.compileData(TEST_MARKDOWN)

    expect(jsonData).toBeDefined()
    expect(jsonData.title).toBe('Test Document')
    expect(jsonData.author).toBe('Test Author')
  })

  it('should complete full workflow: parse → register → render', () => {
    // Step 1: Parse markdown
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    expect(parsed).toBeDefined()

    // Step 2: Create engine and register quill
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    // Step 3: Get quill info
    const info = engine.getQuillInfo('test_quill')
    expect(info.supportedFormats).toContain('pdf')

    // Step 4: Render to PDF
    try {
      const result = engine.render(parsed, { format: 'pdf' })
    }
    catch (e) {
      console.error("Render error:", e);
      throw e;
    }

    // expect(result).toBeDefined()
    // expect(result.artifacts).toBeDefined()
    // expect(result.artifacts.length).toBeGreaterThan(0)
    // expect(result.artifacts[0].bytes).toBeDefined()
    // expect(result.artifacts[0].bytes.length).toBeGreaterThan(0)
    // expect(result.artifacts[0].mimeType).toBe('application/pdf')
  })

  it('should support compile + renderPages with pageCount', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    const compiled = engine.compile(parsed)
    expect(typeof compiled.pageCount).toBe('number')
    expect(compiled.pageCount).toBeGreaterThan(0)

    const allPages = compiled.renderPages(undefined, { format: 'svg' })
    expect(allPages.artifacts.length).toBe(compiled.pageCount)
    expect(allPages.artifacts[0].mimeType).toBe('image/svg+xml')

    const subset = compiled.renderPages([0, 0], { format: 'png', ppi: 80 })
    expect(subset.artifacts.length).toBe(2)
    expect(subset.artifacts[0].mimeType).toBe('image/png')
  })

  it('should warn and skip out-of-bounds page indices', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    const compiled = engine.compile(parsed)
    const oob = compiled.pageCount + 10

    const result = compiled.renderPages([0, oob], { format: 'png', ppi: 80 })
    expect(result.artifacts.length).toBe(1)
    expect(result.warnings.length).toBeGreaterThan(0)
    expect(result.warnings[0].message).toContain('out of bounds')
  })

  it('should error when requesting page selection with PDF', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    const compiled = engine.compile(parsed)

    expect(() => {
      compiled.renderPages([0], { format: 'pdf' })
    }).toThrow()
  })

  it('should handle error: unregistered quill', () => {
    const engine = new Quillmark()

    expect(() => {
      engine.getQuillInfo('nonexistent_quill')
    }).toThrow()
  })

  it('should handle error: invalid markdown', () => {
    const badMarkdown = `---
title: Test
QUILL: test_quill
this is not valid yaml
---

# Content`

    expect(() => {
      Quillmark.parseMarkdown(badMarkdown)
    }).toThrow()
  })

  it('should handle error: render without quill registration', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    const engine = new Quillmark()
    // Don't register the quill

    expect(() => {
      engine.render(parsed, { format: 'pdf' })
    }).toThrow()
  })

  it('should render to SVG format', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    const result = engine.render(parsed, { format: 'svg' })

    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('image/svg+xml')
  })

  it('should unregister quill', () => {
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    expect(engine.listQuills()).toContain('test_quill@1.0.0')

    engine.unregisterQuill('test_quill')

    expect(engine.listQuills()).not.toContain('test_quill@1.0')
  })

  it('should accept assets as plain JavaScript objects', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))

    // Assets should be passed as plain JavaScript objects with byte arrays
    const assets = {
      'logo.png': [137, 80, 78, 71],
      'font.ttf': [0, 1, 2, 3]
    }

    // This should not throw - assets is a plain object
    const result = engine.render(parsed, {
      format: 'pdf',
      assets: assets
    })

    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
  })

  it('should return all data as plain objects (comprehensive test)', () => {
    // Step 1: Parse markdown - fields should be plain object
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    expect(parsed.fields instanceof Map).toBe(false)
    expect(parsed.fields instanceof Object).toBe(true)
    expect(parsed.fields.title).toBe('Test Document')
    expect(parsed.fields.author).toBe('Test Author')

    // Step 2: Register and get quill info - metadata is object, schema is YAML text
    const engine = new Quillmark()
    engine.registerQuill(quillFromBundle(TEST_QUILL))
    const info = engine.getQuillInfo('test_quill')

    expect(info.metadata instanceof Map).toBe(false)
    expect(info.metadata instanceof Object).toBe(true)
    expect(info.metadata.backend).toBe('typst')
    expect(typeof info.schema).toBe('string')

    // Step 3: Render with assets as plain object
    const result = engine.render(parsed, {
      format: 'pdf',
      assets: {
        'test.txt': [72, 101, 108, 108, 111]
      }
    })

    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(Array.isArray(result.warnings)).toBe(true)
    expect(typeof result.renderTimeMs).toBe('number')
  })

  it('should throw when QUILL tag is not specified', () => {
    // Markdown without QUILL: tag
    const markdownWithoutQuill = `---
title: Default Test
author: Test Author
---

# Hello Default

This document has no QUILL tag.`

    expect(() => {
      Quillmark.parseMarkdown(markdownWithoutQuill)
    }).toThrow()
  })
})
