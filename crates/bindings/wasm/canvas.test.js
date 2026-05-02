/**
 * Canvas-preview smoke tests for quillmark-wasm.
 *
 * Vitest runs in a Node environment with no DOM, so we polyfill the bare
 * minimum needed for wasm-bindgen's `instanceof` checks to pass:
 *
 *   - `globalThis.CanvasRenderingContext2D`
 *   - `globalThis.OffscreenCanvasRenderingContext2D`
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

// In real browsers, OffscreenCanvasRenderingContext2D and
// CanvasRenderingContext2D do NOT share an inheritance chain — they're
// siblings. Defining the polyfill as an independent class (not a subclass)
// ensures the Rust-side `instanceof` dispatch actually exercises the
// second branch, instead of matching `CanvasRenderingContext2D` via
// inheritance.
class FakeOffscreenCanvasRenderingContext2D {
  constructor() {
    this.calls = []
    this.canvas = { width: 0, height: 0 }
  }
  putImageData(img, dx, dy) {
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
  globalThis.OffscreenCanvasRenderingContext2D = FakeOffscreenCanvasRenderingContext2D
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

function openQuill() {
  const engine = new Quillmark()
  return engine.quill(makeQuill({ name: 'test_quill', plate: TEST_PLATE }))
}

function openSession() {
  return openQuill().open(Document.fromMarkdown(TEST_MARKDOWN))
}

/** Build a fake context already sized to fit page `page` at `scale`. */
function ctxForPage(session, page, scale, CtxCtor = FakeCanvasRenderingContext2D) {
  const { widthPt, heightPt } = session.pageSize(page)
  const ctx = new CtxCtor()
  ctx.canvas.width = Math.round(widthPt * scale)
  ctx.canvas.height = Math.round(heightPt * scale)
  return ctx
}

describe('RenderSession canvas preview', () => {
  it('exposes pageCount, backendId, supportsCanvas, warnings, and pageSize on a Typst session', () => {
    const quill = openQuill()
    expect(quill.supportsCanvas).toBe(true)

    const session = quill.open(Document.fromMarkdown(TEST_MARKDOWN))
    expect(session.pageCount).toBeGreaterThan(0)
    expect(session.backendId).toBe('typst')
    expect(session.supportsCanvas).toBe(true)
    expect(Array.isArray(session.warnings)).toBe(true)

    const size = session.pageSize(0)
    expect(size.widthPt).toBeGreaterThan(0)
    expect(size.heightPt).toBeGreaterThan(0)
  })

  it('paints a page with the expected backing-store dimensions and non-trivial pixel content', () => {
    const session = openSession()
    const scale = 1.5
    const ctx = ctxForPage(session, 0, scale)
    const { widthPt, heightPt } = session.pageSize(0)

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

  it('also paints into an OffscreenCanvasRenderingContext2D', () => {
    const session = openSession()
    const scale = 1
    const ctx = ctxForPage(session, 0, scale, FakeOffscreenCanvasRenderingContext2D)
    expect(() => session.paint(ctx, 0, scale)).not.toThrow()
    expect(ctx.calls).toHaveLength(1)
  })

  it('throws on canvas/scale dimension mismatch instead of silently clipping', () => {
    const session = openSession()
    const scale = 1
    const ctx = ctxForPage(session, 0, scale)
    // Sabotage the height after sizing — a common foot-gun is forgetting to
    // resize when the user changes zoom.
    ctx.canvas.height -= 10

    expect(() => session.paint(ctx, 0, scale)).toThrow(/canvas size mismatch/)
    expect(ctx.calls).toHaveLength(0)
  })

  it('throws an out-of-range error when paint is called with a bad page index', () => {
    const session = openSession()
    // For OOB we never reach the size validation — a 1×1 canvas is fine.
    const ctx = new FakeCanvasRenderingContext2D()
    ctx.canvas.width = 1
    ctx.canvas.height = 1
    expect(() => session.paint(ctx, session.pageCount + 5, 1)).toThrow(
      /out of range.*pageCount=/,
    )
  })
})
