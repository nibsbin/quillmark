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
 * Setup: Tests use the bundler build from ../pkg/bundler/
 */

import { describe, it, expect } from 'vitest'
import { Quillmark } from '../pkg/bundler/wasm.js'

// Minimal inline Quill for testing
const TEST_QUILL = {
  files: {
    'Quill.toml': {
      contents: `[Quill]
name = "test_quill"
backend = "typst"
glue = "glue.typ"
description = "Test quill for smoke tests"
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

# Hello World

This is a test document.`

describe('quillmark-wasm smoke tests', () => {
  it('should parse markdown with YAML frontmatter', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    
    expect(parsed).toBeDefined()
    expect(parsed.fields).toBeDefined()
    // fields is a Map, not a plain object
    expect(parsed.fields instanceof Map).toBe(true)
    expect(parsed.fields.get('title')).toBe('Test Document')
    expect(parsed.fields.get('author')).toBe('Test Author')
    expect(parsed.quillTag).toBe('test_quill')
  })

  it('should create engine and register quill', () => {
    const engine = new Quillmark()
    
    expect(() => {
      engine.registerQuill('test_quill', TEST_QUILL)
    }).not.toThrow()
    
    const quills = engine.listQuills()
    expect(quills).toContain('test_quill')
  })

  it('should get quill info after registration', () => {
    const engine = new Quillmark()
    engine.registerQuill('test_quill', TEST_QUILL)
    
    const info = engine.getQuillInfo('test_quill')
    
    expect(info).toBeDefined()
    expect(info.name).toBe('test_quill')
    expect(info.backend).toBe('typst')
    expect(info.supportedFormats).toContain('pdf')
  })

  it('should render glue template', () => {
    const engine = new Quillmark()
    engine.registerQuill('test_quill', TEST_QUILL)
    
    const glue = engine.renderGlue('test_quill', TEST_MARKDOWN)
    
    expect(glue).toBeDefined()
    expect(typeof glue).toBe('string')
    expect(glue).toContain('Test Document')
  })

  it('should complete full workflow: parse → register → render', () => {
    // Step 1: Parse markdown
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    expect(parsed).toBeDefined()
    
    // Step 2: Create engine and register quill
    const engine = new Quillmark()
    engine.registerQuill('test_quill', TEST_QUILL)
    
    // Step 3: Get quill info
    const info = engine.getQuillInfo('test_quill')
    expect(info.supportedFormats).toContain('pdf')
    
    // Step 4: Render to PDF
    const result = engine.render(parsed, { format: 'pdf' })
    
    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].bytes).toBeDefined()
    expect(result.artifacts[0].bytes.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('application/pdf')
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
    engine.registerQuill('test_quill', TEST_QUILL)
    
    const result = engine.render(parsed, { format: 'svg' })
    
    expect(result).toBeDefined()
    expect(result.artifacts).toBeDefined()
    expect(result.artifacts.length).toBeGreaterThan(0)
    expect(result.artifacts[0].mimeType).toBe('image/svg+xml')
  })

  it('should unregister quill', () => {
    const engine = new Quillmark()
    engine.registerQuill('test_quill', TEST_QUILL)
    
    expect(engine.listQuills()).toContain('test_quill')
    
    engine.unregisterQuill('test_quill')
    
    expect(engine.listQuills()).not.toContain('test_quill')
  })
})
