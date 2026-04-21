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
// Document editor surface (Phase 3)
// ---------------------------------------------------------------------------

describe('Document editor surface — setField / removeField', () => {
  it('setField inserts a new frontmatter field', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    doc.setField('subtitle', 'A subtitle')
    expect(doc.frontmatter.subtitle).toBe('A subtitle')
  })

  it('setField updates an existing field', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    doc.setField('title', 'Updated')
    expect(doc.frontmatter.title).toBe('Updated')
  })

  it('setField throws EditError::ReservedName for BODY', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(() => doc.setField('BODY', 'x')).toThrow(/ReservedName/)
  })

  it('setField throws EditError::ReservedName for CARDS', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(() => doc.setField('CARDS', [])).toThrow(/ReservedName/)
  })

  it('setField throws EditError::ReservedName for QUILL', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(() => doc.setField('QUILL', 'x')).toThrow(/ReservedName/)
  })

  it('setField throws EditError::ReservedName for CARD', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(() => doc.setField('CARD', 'x')).toThrow(/ReservedName/)
  })

  it('setField throws EditError::InvalidFieldName for uppercase name', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(() => doc.setField('Title', 'x')).toThrow(/InvalidFieldName/)
  })

  it('removeField returns the removed value', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    const removed = doc.removeField('title')
    expect(removed).toBe('Test Document')
    expect(doc.frontmatter.title).toBeUndefined()
  })

  it('removeField returns undefined when field absent', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(doc.removeField('nonexistent')).toBeUndefined()
  })
})

describe('Document editor surface — setQuillRef / replaceBody', () => {
  it('setQuillRef changes the quillRef', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    doc.setQuillRef('new_quill')
    expect(doc.quillRef).toBe('new_quill')
  })

  it('setQuillRef throws on invalid reference', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(() => doc.setQuillRef('INVALID QUILL REF WITH SPACES')).toThrow()
  })

  it('replaceBody changes the body', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    doc.replaceBody('Brand new body.')
    expect(doc.body).toBe('Brand new body.')
  })
})

describe('Document editor surface — card mutations', () => {
  const MD_WITH_CARDS = `---
QUILL: test_quill
---

Body.

---
CARD: note
foo: bar
---

Card one.

---
CARD: summary
---

Card two.
`

  it('pushCard appends a card', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    doc.pushCard({ tag: 'note', fields: {}, body: 'My card.' })
    expect(doc.cards.length).toBe(1)
    expect(doc.cards[0].tag).toBe('note')
    expect(doc.cards[0].body).toBe('My card.')
  })

  it('pushCard throws on invalid tag', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(() => doc.pushCard({ tag: 'BadTag' })).toThrow(/InvalidTagName/)
  })

  it('insertCard inserts at specified index', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARDS)
    doc.insertCard(0, { tag: 'intro' })
    expect(doc.cards[0].tag).toBe('intro')
    expect(doc.cards[1].tag).toBe('note')
  })

  it('insertCard throws IndexOutOfRange when index > len', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN) // 0 cards
    expect(() => doc.insertCard(5, { tag: 'note' })).toThrow(/IndexOutOfRange/)
  })

  it('removeCard removes and returns the card', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARDS)
    const removed = doc.removeCard(0)
    expect(removed).toBeDefined()
    expect(removed.tag).toBe('note')
    expect(doc.cards.length).toBe(1)
    expect(doc.cards[0].tag).toBe('summary')
  })

  it('removeCard returns undefined when out of range', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)
    expect(doc.removeCard(0)).toBeUndefined()
  })

  it('moveCard swaps positions correctly', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARDS)
    doc.moveCard(1, 0) // summary → front
    expect(doc.cards[0].tag).toBe('summary')
    expect(doc.cards[1].tag).toBe('note')
  })

  it('moveCard no-op when from == to', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARDS)
    doc.moveCard(0, 0)
    expect(doc.cards[0].tag).toBe('note')
  })

  it('moveCard throws IndexOutOfRange on out-of-range index', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARDS) // 2 cards
    expect(() => doc.moveCard(5, 0)).toThrow(/IndexOutOfRange/)
  })
})

describe('Document editor surface — updateCardField / updateCardBody', () => {
  const MD_WITH_CARD = `---
QUILL: test_quill
---

Body.

---
CARD: note
foo: bar
---

Card body.
`

  it('updateCardField sets a field on a card', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARD)
    doc.updateCardField(0, 'content', 'hello')
    expect(doc.cards[0].fields.content).toBe('hello')
  })

  it('updateCardField throws EditError::ReservedName for BODY', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARD)
    expect(() => doc.updateCardField(0, 'BODY', 'x')).toThrow(/ReservedName/)
  })

  it('updateCardField throws IndexOutOfRange when card absent', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN) // 0 cards
    expect(() => doc.updateCardField(0, 'title', 'x')).toThrow(/IndexOutOfRange/)
  })

  it('updateCardBody replaces card body', () => {
    const doc = Document.fromMarkdown(MD_WITH_CARD)
    doc.updateCardBody(0, 'New card body.')
    expect(doc.cards[0].body).toBe('New card body.')
  })

  it('updateCardBody throws IndexOutOfRange when card absent', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN) // 0 cards
    expect(() => doc.updateCardBody(0, 'x')).toThrow(/IndexOutOfRange/)
  })
})

describe('Document editor surface — parse→mutate→read round-trip', () => {
  it('mutated document reflects changes in subsequent reads', () => {
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    // Mutate
    doc.setField('author', 'Bob')
    doc.replaceBody('New body text.')
    doc.pushCard({ tag: 'note', body: 'Card content.' })
    doc.setQuillRef('updated_quill')

    // Assert state
    expect(doc.frontmatter.author).toBe('Bob')
    expect(doc.body).toBe('New body text.')
    expect(doc.cards.length).toBe(1)
    expect(doc.cards[0].tag).toBe('note')
    expect(doc.cards[0].body).toBe('Card content.')
    expect(doc.quillRef).toBe('updated_quill')

    // Original title still present
    expect(doc.frontmatter.title).toBe('Test Document')

    // Warnings untouched
    expect(Array.isArray(doc.warnings)).toBe(true)
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
