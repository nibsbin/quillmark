/**
 * Canvas-preview smoke tests for quillmark-wasm.
 *
 * Vitest runs in a Node environment with no DOM, so we polyfill the bare
 * minimum needed for wasm-bindgen's `instanceof` checks to pass:
 *
 *   - `globalThis.CanvasRenderingContext2D`
 *   - `globalThis.ImageData`
 *
 * The polyfill captures `putImageData` calls into a buffer so the test can
 * assert that `paint` actually invoked the context with sensibly-sized
 * pixels. This is not a pixel-correctness check — that needs a real browser
 * test — but it prevents the rasterizer path from regressing silently
 * (e.g. broken downcast, mis-sized buffer, panics).
 */

import { describe, it, expect, beforeAll } from 'vitest'

class FakeImageData {
  constructor(data, width, height) {
    this.data = data
    this.width = width
    this.height = height
  }
}

class FakeCanvasRenderingContext2D {
  constructor() {
    this.calls = []
    this.canvas = { width: 0, height: 0 }
  }
  putImageData(img, dx, dy) {
    this.calls.push({
      width: img.width,
      height: img.height,
      bytes: img.data.length,
      dx,
      dy,
    })
  }
}

beforeAll(() => {
  globalThis.ImageData = FakeImageData
  globalThis.CanvasRenderingContext2D = FakeCanvasRenderingContext2D
})

const { Quillmark, Document } = await import('@quillmark-wasm')
const { makeQuill } = await import('./test-helpers.js')

const TEST_MARKDOWN = `---
QUILL: test_quill
title: Canvas Test
---

# Hello canvas
`

const TEST_PLATE = `#import "@local/quillmark-helper:0.1.0": data
= #data.title

#data.BODY`

describe('RenderSession canvas preview', () => {
  it('exposes pageCount, backendId, warnings, and pageSize on a Typst session', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const session = quill.open(doc)
    expect(session.pageCount).toBeGreaterThan(0)
    expect(session.backendId).toBe('typst')
    expect(Array.isArray(session.warnings)).toBe(true)

    const size = session.pageSize(0)
    expect(typeof size.widthPt).toBe('number')
    expect(typeof size.heightPt).toBe('number')
    expect(size.widthPt).toBeGreaterThan(0)
    expect(size.heightPt).toBeGreaterThan(0)
  })

  it('paints a page into a fake 2D context with the expected backing-store dimensions', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const session = quill.open(doc)
    const { widthPt, heightPt } = session.pageSize(0)
    const scale = 1.5

    const ctx = new FakeCanvasRenderingContext2D()
    expect(() => session.paint(ctx, 0, scale)).not.toThrow()

    expect(ctx.calls).toHaveLength(1)
    const call = ctx.calls[0]
    expect(call.dx).toBe(0)
    expect(call.dy).toBe(0)
    expect(call.width).toBe(Math.round(widthPt * scale))
    expect(call.height).toBe(Math.round(heightPt * scale))
    // RGBA8: 4 bytes per pixel
    expect(call.bytes).toBe(call.width * call.height * 4)
  })

  it('throws when paint is called with an out-of-range page index', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const session = quill.open(doc)
    const ctx = new FakeCanvasRenderingContext2D()
    const oob = session.pageCount + 5

    expect(() => session.paint(ctx, oob, 1)).toThrow(/out of range/)
  })

  it('reports the resolved backendId in the out-of-range error message', () => {
    const engine = new Quillmark()
    const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
    const doc = Document.fromMarkdown(TEST_MARKDOWN)

    const session = quill.open(doc)
    const ctx = new FakeCanvasRenderingContext2D()

    expect(() => session.paint(ctx, session.pageCount + 1, 1)).toThrow(/typst/)
  })
})
