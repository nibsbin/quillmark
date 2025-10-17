/**
 * End-to-End Tests for Quillmark WASM - Basic API
 * 
 * Tests the fundamental API operations: parsing markdown, registering quills,
 * getting quill info, and basic engine operations.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { Quillmark } from '@quillmark-test/wasm';
import {
  SIMPLE_MARKDOWN,
  MARKDOWN_NO_QUILL,
  SMALL_QUILL_JSON,
  INVALID_MARKDOWN,
  INVALID_QUILL_JSON,
} from './fixtures/test-data.js';
import { getField, getFieldNames } from './test-helpers.js';

describe('Quillmark WASM - Basic API', () => {
  describe('parseMarkdown', () => {
    it('should parse markdown with frontmatter', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      
      expect(parsed).toBeDefined();
      expect(parsed.fields).toBeDefined();
      expect(getField(parsed, 'title')).toBe('Test Document');
      expect(getField(parsed, 'author')).toBe('Alice');
      expect(parsed.quillTag).toBe('test_quill');
    });

    it('should parse markdown without QUILL field', () => {
      const parsed = Quillmark.parseMarkdown(MARKDOWN_NO_QUILL);
      
      expect(parsed).toBeDefined();
      expect(getField(parsed, 'title')).toBe('No Quill Document');
      expect(parsed.quillTag).toBeUndefined();
    });

    it('should handle markdown with no frontmatter', () => {
      const markdown = '# Just a heading\n\nSome content.';
      const parsed = Quillmark.parseMarkdown(markdown);
      
      expect(parsed).toBeDefined();
      expect(parsed.fields).toBeDefined();
      expect(getFieldNames(parsed).length).toBe(0);
    });

    it('should throw error for invalid markdown', () => {
      expect(() => {
        Quillmark.parseMarkdown(INVALID_MARKDOWN);
      }).toThrow();
    });

    it('should preserve multiple field types', () => {
      const markdown = `---
title: Test
count: 42
flag: true
items:
  - one
  - two
---

Content`;
      const parsed = Quillmark.parseMarkdown(markdown);
      
      expect(getField(parsed, 'title')).toBe('Test');
      expect(getField(parsed, 'count')).toBe(42);
      expect(getField(parsed, 'flag')).toBe(true);
      const items = getField(parsed, 'items');
      expect(Array.isArray(items)).toBe(true);
      expect(items).toEqual(['one', 'two']);
    });
  });

  describe('Engine creation', () => {
    it('should create a new engine instance', () => {
      const engine = new Quillmark();
      expect(engine).toBeDefined();
      expect(typeof engine.registerQuill).toBe('function');
      expect(typeof engine.render).toBe('function');
    });

    it('should create multiple independent engine instances', () => {
      const engine1 = new Quillmark();
      const engine2 = new Quillmark();
      
      engine1.registerQuill('test_quill', SMALL_QUILL_JSON);
      
      expect(engine1.listQuills()).toContain('test_quill');
      expect(engine2.listQuills()).not.toContain('test_quill');
    });
  });

  describe('registerQuill', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
    });

    it('should register a quill from JSON object', () => {
      expect(() => {
        engine.registerQuill('test_quill', SMALL_QUILL_JSON);
      }).not.toThrow();

      const quills = engine.listQuills();
      expect(quills).toContain('test_quill');
    });

    it('should register a quill from JSON string', () => {
      const jsonString = JSON.stringify(SMALL_QUILL_JSON);
      
      expect(() => {
        engine.registerQuill('test_quill', jsonString);
      }).not.toThrow();

      expect(engine.listQuills()).toContain('test_quill');
    });

    it('should throw error for invalid quill', () => {
      expect(() => {
        engine.registerQuill('invalid_quill', INVALID_QUILL_JSON);
      }).toThrow();
    });

    it('should allow registering multiple quills', () => {
      engine.registerQuill('quill1', SMALL_QUILL_JSON);
      engine.registerQuill('quill2', SMALL_QUILL_JSON);
      
      const quills = engine.listQuills();
      expect(quills).toContain('quill1');
      expect(quills).toContain('quill2');
      expect(quills.length).toBe(2);
    });

    it('should handle re-registration of same quill', () => {
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
      
      // Re-registering should work (overwrites)
      expect(() => {
        engine.registerQuill('test_quill', SMALL_QUILL_JSON);
      }).not.toThrow();
      
      const quills = engine.listQuills();
      expect(quills.filter(q => q === 'test_quill').length).toBe(1);
    });
  });

  describe('getQuillInfo', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
    });

    it('should get quill info for registered quill', () => {
      const info = engine.getQuillInfo('test_quill');
      
      expect(info).toBeDefined();
      expect(info.name).toBe('test_quill');
      expect(info.backend).toBe('typst');
      expect(Array.isArray(info.supportedFormats)).toBe(true);
      expect(info.supportedFormats.length).toBeGreaterThan(0);
    });

    it('should include supported formats', () => {
      const info = engine.getQuillInfo('test_quill');
      
      // Typst backend should support pdf at minimum
      expect(info.supportedFormats).toContain('pdf');
    });

    it('should throw error for non-existent quill', () => {
      expect(() => {
        engine.getQuillInfo('non_existent');
      }).toThrow();
    });

    it('should return metadata from Quill.toml', () => {
      const info = engine.getQuillInfo('test_quill');
      expect(info.metadata).toBeDefined();
      // metadata could be Map or Object
      expect(info.metadata).toBeTruthy();
    });

    it('should return all expected properties', () => {
      const info = engine.getQuillInfo('test_quill');
      expect(info).toHaveProperty('name');
      expect(info).toHaveProperty('backend');
      expect(info).toHaveProperty('metadata');
      expect(info).toHaveProperty('supportedFormats');
    });
  });

  describe('listQuills', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
    });

    it('should return empty array for new engine', () => {
      const quills = engine.listQuills();
      expect(Array.isArray(quills)).toBe(true);
      expect(quills.length).toBe(0);
    });

    it('should list registered quills', () => {
      engine.registerQuill('quill1', SMALL_QUILL_JSON);
      engine.registerQuill('quill2', SMALL_QUILL_JSON);
      
      const quills = engine.listQuills();
      expect(quills).toContain('quill1');
      expect(quills).toContain('quill2');
    });
  });

  describe('unregisterQuill', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
    });

    it('should unregister a quill', () => {
      expect(engine.listQuills()).toContain('test_quill');
      
      engine.unregisterQuill('test_quill');
      
      expect(engine.listQuills()).not.toContain('test_quill');
    });

    it('should handle unregistering non-existent quill gracefully', () => {
      expect(() => {
        engine.unregisterQuill('non_existent');
      }).not.toThrow();
    });

    it('should allow re-registration after unregister', () => {
      engine.unregisterQuill('test_quill');
      expect(engine.listQuills()).not.toContain('test_quill');
      
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
      expect(engine.listQuills()).toContain('test_quill');
    });
  });
});
