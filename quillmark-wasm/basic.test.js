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
    
    // fields should be a plain object, not a Map
    expect(parsed.fields instanceof Map).toBe(false)
    expect(parsed.fields instanceof Object).toBe(true)
    expect(parsed.fields.title).toBe('Test Document')
    expect(parsed.fields.author).toBe('Test Author')
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
    
    // metadata and fieldSchemas should be plain objects, not Maps
    expect(info.metadata instanceof Map).toBe(false)
    expect(info.metadata instanceof Object).toBe(true)
    expect(info.fieldSchemas instanceof Map).toBe(false)
    expect(info.fieldSchemas instanceof Object).toBe(true)
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

  it('should accept assets as plain JavaScript objects', () => {
    const parsed = Quillmark.parseMarkdown(TEST_MARKDOWN)
    const engine = new Quillmark()
    engine.registerQuill('test_quill', TEST_QUILL)
    
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
    
    // Step 2: Register and get quill info - metadata and fieldSchemas should be plain objects
    const engine = new Quillmark()
    engine.registerQuill('test_quill', TEST_QUILL)
    const info = engine.getQuillInfo('test_quill')
    
    expect(info.metadata instanceof Map).toBe(false)
    expect(info.metadata instanceof Object).toBe(true)
    expect(info.metadata.backend).toBe('typst')
    
    expect(info.fieldSchemas instanceof Map).toBe(false)
    expect(info.fieldSchemas instanceof Object).toBe(true)
    
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
})
