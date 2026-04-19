/**
 * Smoke tests for quillmark-wasm — new Quill.render() API
 *
 * These tests cover the canonical flow introduced by the render API overhaul:
 *   engine.quillFromTree(tree) → quill.render(markdown, opts)
 *
 * Setup: Tests use the bundler build via @quillmark-wasm alias (see vitest.config.js)
 */

import { describe, it, expect, vi } from 'vitest'
import { Quill, Quillmark, ParsedDocument } from '@quillmark-wasm'
import { makeQuill } from './test-helpers.js'

const enc = new TextEncoder()
const TEST_MARKDOWN = `---
title: Test Document
author: Test Author
QUILL: test_quill
---

# Hello World

This is a test document.`

const TEST_PLATE = `#import "@local/quillmark-helper:0.1.0": data
#let title = data.title
#let body = data.BODY

= #title

#body`

function textBytes(str) {
  return enc.encode(str)
}

// ---------------------------------------------------------------------------
// Quill.fromTree (static, no backend)
// ---------------------------------------------------------------------------

describe('Quill.fromTree', () => {
  it('should build a Quill from a Map<string, Uint8Array>', () => {
    const quill = Quill.fromTree(makeQuill({ name: 'tree_quill' }))
    expect(quill).toBeDefined()
  })

  it('should build a Quill from a plain Record<string, Uint8Array>', () => {
    const tree = {}
    for (const [k, v] of makeQuill({ name: 'tree_quill' })) tree[k] = v
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

  it('should error on render when no backend is attached', () => {
    // Quill.fromTree produces a quill with no backend — render must fail
    const quill = Quill.fromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    expect(() => quill.render(TEST_MARKDOWN, { format: 'pdf' })).toThrow()
  })
})

// ---------------------------------------------------------------------------
// engine.quillFromTree — factory path (attaches backend)
// ---------------------------------------------------------------------------

describe('Quillmark.quillFromTree', () => {
  it('should return a render-ready Quill', () => {
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    expect(quill).toBeDefined()
  })

  it('should render markdown to PDF via quill.render(string)', () => {
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))

    const result = quill.render(TEST_MARKDOWN, { format: 'pdf' })

    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].bytes.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('application/pdf')
  })

  it('should render markdown to SVG via quill.render(string)', () => {
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))

    const result = quill.render(TEST_MARKDOWN, { format: 'svg' })

    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('image/svg+xml')
  })

  it('should render a ParsedDocument via quill.render(ParsedDocument)', () => {
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)

    const result = quill.render(parsed, { format: 'pdf' })

    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('application/pdf')
  })

  it('should emit a quill::ref_mismatch warning when ParsedDocument QUILL differs from quill name', () => {
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))

    // Document declares a different quill name
    const otherMarkdown = `---
title: Mismatch Test
QUILL: other_quill
---

# Content`
    const parsed = ParsedDocument.fromMarkdown(otherMarkdown)
    const result = quill.render(parsed, { format: 'pdf' })

    expect(result.warnings.length).toBe(1)
    expect(result.warnings[0].code).toBe('quill::ref_mismatch')
    expect(result.artifacts.length).toBeGreaterThan(0)
  })
})

// ---------------------------------------------------------------------------
// compile + renderPages
// ---------------------------------------------------------------------------

describe('quill.compile + renderPages', () => {
  it('should support compile + renderPages with pageCount', () => {
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))

    const compiled = quill.compile(TEST_MARKDOWN)
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
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const compiled = quill.compile(TEST_MARKDOWN)
    const oob = compiled.pageCount + 10

    const result = compiled.renderPages([0, oob], { format: 'png', ppi: 80 })
    expect(result.artifacts.length).toBe(1)
    expect(result.warnings.length).toBeGreaterThan(0)
    expect(result.warnings[0].message).toContain('out of bounds')
  })

  it('should error when requesting page selection with PDF', () => {
    const engine = new Quillmark()
    const quill = engine.quillFromTree(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const compiled = quill.compile(TEST_MARKDOWN)

    expect(() => {
      compiled.renderPages([0], { format: 'pdf' })
    }).toThrow()
  })
})

// ---------------------------------------------------------------------------
// ParsedDocument.fromMarkdown (standalone static)
// ---------------------------------------------------------------------------

describe('ParsedDocument.fromMarkdown', () => {
  it('should parse markdown with YAML frontmatter as a standalone static call', () => {
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)

    expect(parsed).toBeDefined()
    expect(parsed.fields).toBeDefined()
    expect(parsed.fields instanceof Map).toBe(false)
    expect(parsed.fields instanceof Object).toBe(true)
    expect(parsed.fields.title).toBe('Test Document')
    expect(parsed.fields.author).toBe('Test Author')
    expect(parsed.quillRef).toBe('test_quill')
  })

  it('should throw on invalid YAML frontmatter', () => {
    const badMarkdown = `---
title: Test
QUILL: test_quill
this is not valid yaml
---

# Content`

    expect(() => {
      ParsedDocument.fromMarkdown(badMarkdown)
    }).toThrow()
  })

  it('should throw when QUILL field is absent', () => {
    const markdownWithoutQuill = `---
title: Default Test
author: Test Author
---

# Hello Default

This document has no QUILL tag.`

    expect(() => {
      ParsedDocument.fromMarkdown(markdownWithoutQuill)
    }).toThrow()
  })
})

// ---------------------------------------------------------------------------
// Deprecated Quillmark.parseMarkdown wrapper
// ---------------------------------------------------------------------------

describe('Quillmark.parseMarkdown (deprecated)', () => {
  it('should still parse markdown and emit a console.warn', () => {
    const warnSpy = vi.spyOn(console, 'warn')

    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)

    expect(parsed).toBeDefined()
    expect(parsed.fields.title).toBe('Test Document')
    expect(parsed.quillRef).toBe('test_quill')
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('deprecated')
    )

    warnSpy.mockRestore()
  })
})
