/**
 * Smoke tests for quillmark-wasm — new Quill.render() API
 *
 * These tests cover the canonical flow introduced by the render API overhaul:
 *   engine.quill(tree) → ParsedDocument.fromMarkdown(markdown) → quill.render(parsed, opts)
 *
 * Setup: Tests use the bundler build via @quillmark-wasm alias (see vitest.config.js)
 */

import { describe, it, expect } from 'vitest'
import { Quillmark, ParsedDocument } from '@quillmark-wasm'
import { makeQuill } from './test-helpers.js'

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

describe('Quillmark.quill', () => {
  it('should return a render-ready Quill', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    expect(quill).toBeDefined()
  })

  it('should render markdown to PDF via quill.render(parsed)', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)

    const result = quill.render(parsed, { format: 'pdf' })

    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].bytes.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('application/pdf')
  })

  it('should render markdown to SVG via quill.render(parsed)', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)

    const result = quill.render(parsed, { format: 'svg' })

    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('image/svg+xml')
  })

  it('should render a ParsedDocument via quill.render(ParsedDocument)', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)

    const result = quill.render(parsed, { format: 'pdf' })

    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('application/pdf')
  })

  it('should emit a quill::ref_mismatch warning when ParsedDocument QUILL differs from quill name', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))

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
// open + session.render
// ---------------------------------------------------------------------------

describe('quill.open + session.render', () => {
  it('should support open + session.render with pageCount', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)

    const session = quill.open(parsed)
    expect(typeof session.pageCount).toBe('number')
    expect(session.pageCount).toBeGreaterThan(0)

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
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)
    const session = quill.open(parsed)
    const oob = session.pageCount + 10

    const result = session.render({ format: 'png', ppi: 80, pages: [0, oob] })
    expect(result.artifacts.length).toBe(1)
    expect(result.warnings.length).toBeGreaterThan(0)
    expect(result.warnings[0].message).toContain('out of bounds')
  })

  it('should error when requesting page selection with PDF', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const parsed = ParsedDocument.fromMarkdown(TEST_MARKDOWN)
    const session = quill.open(parsed)

    expect(() => {
      session.render({ format: 'pdf', pages: [0] })
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
