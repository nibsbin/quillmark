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
 * pixels and non-empty pixel content. Pixel-perfect correctness needs a
 * real browser test; this catches regressions like broken downcast,
 * mis-sized buffer, swapped channels, missing demultiply, or panics.
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
    // Copy the byte view so the test can inspect pixels even if Rust later
    // reuses the underlying buffer.
    this.calls.push({
      width: img.width,
      height: img.height,
      data: new Uint8ClampedArray(img.data),
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

function openSession() {
  const engine = new Quillmark()
  const quill = engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
  return quill.open(Document.fromMarkdown(TEST_MARKDOWN))
}

describe('RenderSession canvas preview', () => {
  it('exposes pageCount, backendId, warnings, and pageSize on a Typst session', () => {
    const session = openSession()
    expect(session.pageCount).toBeGreaterThan(0)
    expect(session.backendId).toBe('typst')
    expect(Array.isArray(session.warnings)).toBe(true)

    const size = session.pageSize(0)
    expect(size.widthPt).toBeGreaterThan(0)
    expect(size.heightPt).toBeGreaterThan(0)
  })

  it('paints a page with the expected backing-store dimensions and non-trivial pixel content', () => {
    const session = openSession()
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
    expect(call.data.length).toBe(call.width * call.height * 4)

    // Pixel-content sanity. The test plate renders a title heading, so the
    // rasterized buffer must contain non-white pixels (visible glyph ink)
    // *and* opaque pixels (page background). A regression that wrote zeros,
    // swapped channels, or skipped demultiply would fail at least one of
    // these.
    let inkPixels = 0
    let opaquePixels = 0
    for (let i = 0; i < call.data.length; i += 4) {
      const [r, g, b, a] = [call.data[i], call.data[i + 1], call.data[i + 2], call.data[i + 3]]
      if (a > 0 && (r < 250 || g < 250 || b < 250)) inkPixels++
      if (a === 255) opaquePixels++
    }
    expect(inkPixels).toBeGreaterThan(0)
    expect(opaquePixels).toBeGreaterThan(0)
  })

  it('throws an out-of-range error when paint is called with a bad page index', () => {
    const session = openSession()
    const ctx = new FakeCanvasRenderingContext2D()
    expect(() => session.paint(ctx, session.pageCount + 5, 1)).toThrow(
      /out of range.*pageCount=/,
    )
  })
})
