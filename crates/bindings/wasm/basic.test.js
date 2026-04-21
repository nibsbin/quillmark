/**
 * Smoke tests for quillmark-wasm — Document API (Phase 2)
 *
 * These tests cover the canonical flow:
 *   engine.quill(tree) → Document.fromMarkdown(markdown) → quill.render(doc, opts)
 *
 * Setup: Tests use the bundler build via @quillmark-wasm alias (see vitest.config.js)
 */

import { describe, it, expect } from 'vitest'
import { Quillmark, Document } from '@quillmark-wasm'
import { makeQuill } from './test-helpers.js'

const TEST_MARKDOWN = `---
QUILL: test_quill
title: Test Document
author: Test Author
---

# Hello World

This is a test document.`

const TEST_PLATE = `#import "@local/quillmark-helper:0.1.0": data
#let title = data.title
#let body = data.BODY

= #title

#body`

describe('Document.fromMarkdown', () => {
  it('should parse markdown with YAML frontmatter', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    expect(doc).toBeDefined()
    expect(doc.quillRef).toBe('test_quill')
  })

  it('should expose typed frontmatter (no QUILL/BODY/CARDS)', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    expect(doc.frontmatter).toBeDefined()
    expect(doc.frontmatter instanceof Object).toBe(true)
    expect(doc.frontmatter.title).toBe('Test Document')
    expect(doc.frontmatter.author).toBe('Test Author')
    // QUILL, BODY, CARDS must NOT appear in frontmatter
    expect(doc.frontmatter.QUILL).toBeUndefined()
    expect(doc.frontmatter.BODY).toBeUndefined()
    expect(doc.frontmatter.CARDS).toBeUndefined()
  })

  it('should expose body as a string', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    expect(typeof doc.body).toBe('string')
    expect(doc.body).toContain('Hello World')
  })

  it('should expose cards as an array', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    expect(Array.isArray(doc.cards)).toBe(true)
    expect(doc.cards.length).toBe(0)
  })

  it('should expose card fields and body', () => {
    const md = `---
QUILL: test_quill
---

Global body.

---
CARD: note
foo: bar
---

Card body.
`
    const doc = Document.fromMarkdown(md)

    expect(doc.cards.length).toBe(1)
    expect(doc.cards[0].tag).toBe('note')
    expect(doc.cards[0].fields.foo).toBe('bar')
    expect(doc.cards[0].body).toContain('Card body.')
  })

  it('should expose warnings array', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(Array.isArray(doc.warnings)).toBe(true)
    expect(doc.warnings.length).toBe(0)
  })

  it('should throw on invalid YAML frontmatter', () => {
    const badMarkdown = `---
title: Test
QUILL: test_quill
this is not valid yaml
---

# Content`

    expect(() => {
      Document.fromMarkdown(badMarkdown)
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
      Document.fromMarkdown(markdownWithoutQuill)
    }).toThrow()
  })
})

describe('Document.toMarkdown (stub)', () => {
  it('should throw "not yet implemented (phase 4)"', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    expect(() => {
      doc.toMarkdown()
    }).toThrow(/phase 4/i)
  })
})

describe('Quillmark.quill', () => {
  it('should return a render-ready Quill', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    expect(quill).toBeDefined()
  })

  it('should render markdown to PDF via quill.render(doc) with default opts', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const result = quill.render(doc)

    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].bytes.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('application/pdf')
  })

  it('should render markdown to PDF via quill.render(doc, opts)', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const result = quill.render(doc, { format: 'pdf' })

    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].bytes.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('application/pdf')
  })

  it('should render markdown to SVG via quill.render(doc)', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const result = quill.render(doc, { format: 'svg' })

    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('image/svg+xml')
  })

  it('should emit a quill::ref_mismatch warning when Document QUILL differs from quill name', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))

    // Document declares a different quill name
    const otherMarkdown = `---
QUILL: other_quill
title: Mismatch Test
---

# Content`
    const doc = Document.fromMarkdown(otherMarkdown)
    const result = quill.render(doc, { format: 'pdf' })

    expect(result.warnings.length).toBe(1)
    expect(result.warnings[0].code).toBe('quill::ref_mismatch')
    expect(result.artifacts.length).toBeGreaterThan(0)
  })
})

// ---------------------------------------------------------------------------
// open + session.render
// ---------------------------------------------------------------------------

describe('quill.open + session.render', () => {
  it('should support open + session.render with pageCount', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const session = quill.open(doc)
    expect(typeof session.pageCount).toBe('number')
    expect(session.pageCount).toBeGreaterThan(0)

    const defaultFmt = session.render()
    expect(defaultFmt.artifacts.length).toBeGreaterThan(0)
    expect(defaultFmt.artifacts[0].mimeType).toBe('application/pdf')

    const allPages = session.render({ format: 'svg' })
    expect(allPages.artifacts.length).toBe(session.pageCount)
    expect(allPages.artifacts[0].mimeType).toBe('image/svg+xml')

    const subset = session.render({ format: 'png', ppi: 80, pages: [0, 0] })
    expect(subset.artifacts.length).toBe(2)
    expect(subset.artifacts[0].mimeType).toBe('image/png')
  })

  it('should warn and skip out-of-bounds page indices', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    const session = quill.open(doc)
    const oob = session.pageCount + 10

    const result = session.render({ format: 'png', ppi: 80, pages: [0, oob] })
    expect(result.artifacts.length).toBe(1)
    expect(result.warnings.length).toBeGreaterThan(0)
    expect(result.warnings[0].message).toContain('out of bounds')
  })

  it('should error when requesting page selection with PDF', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    const session = quill.open(doc)

    expect(() => {
      session.render({ format: 'pdf', pages: [0] })
    }).toThrow()
  })
})
