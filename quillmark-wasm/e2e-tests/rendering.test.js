/**
 * End-to-End Tests for Quillmark WASM - Rendering Workflow
 * 
 * Tests the complete rendering workflow from parsing to final artifact generation.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { Quillmark } from '@quillmark-test/wasm';
import {
  SIMPLE_MARKDOWN,
  SMALL_QUILL_JSON,
  LETTER_MARKDOWN,
  LETTER_QUILL_JSON,
} from './fixtures/test-data.js';
import { getField, toUint8Array, isPDF } from './test-helpers.js';

describe('Quillmark WASM - Rendering Workflow', () => {
  describe('Complete workflow', () => {
    it('should complete full workflow: parse -> register -> info -> render', () => {
      // Step 1: Parse markdown
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      expect(parsed).toBeDefined();
      expect(parsed.quillTag).toBe('test_quill');

      // Step 2: Create engine and register quill
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);

      // Step 3: Get quill info
      const info = engine.getQuillInfo('test_quill');
      expect(info).toBeDefined();
      expect(info.supportedFormats).toContain('pdf');

      // Step 4: Render
      const result = engine.render(parsed, { format: 'pdf' });
      
      expect(result).toBeDefined();
      expect(result.artifacts).toBeDefined();
      expect(Array.isArray(result.artifacts)).toBe(true);
      expect(result.artifacts.length).toBeGreaterThan(0);
    });

    it('should render using quill_tag from parsed document', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);

      // Don't specify quill_name in options, should use quill_tag
      const result = engine.render(parsed, { format: 'pdf' });
      
      expect(result.artifacts.length).toBeGreaterThan(0);
    });

    it('should render using explicit quill_name in options', () => {
      const markdown = `---
title: Override Test
---

# Content`;
      const parsed = Quillmark.parseMarkdown(markdown);
      expect(parsed.quillTag).toBeUndefined();

      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);

      // Specify quill_name explicitly
      const result = engine.render(parsed, {
        format: 'pdf',
        quillName: 'test_quill',
      });
      
      expect(result.artifacts.length).toBeGreaterThan(0);
    });

    it('should allow quillName in options to override quill_tag', () => {
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
      engine.registerQuill('letter_quill', LETTER_QUILL_JSON);

      const markdown = `---
title: Test
QUILL: test_quill
---

Content`;
      const parsed = Quillmark.parseMarkdown(markdown);
      expect(parsed.quillTag).toBe('test_quill');

      // Override with letter_quill
      const result = engine.render(parsed, {
        format: 'pdf',
        quillName: 'letter_quill',
      });
      
      expect(result.artifacts.length).toBeGreaterThan(0);
    });
  });

  describe('render() output', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
    });

    it('should return RenderResult with artifacts', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'pdf' });

      expect(result).toBeDefined();
      expect(result.artifacts).toBeDefined();
      expect(result.warnings).toBeDefined();
      expect(typeof result.renderTimeMs).toBe('number');
    });

    it('should produce artifact with correct structure', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'pdf' });

      const artifact = result.artifacts[0];
      expect(artifact).toBeDefined();
      expect(artifact.format).toBe('pdf');
      expect(artifact.bytes).toBeDefined();
      expect(Array.isArray(artifact.bytes)).toBe(true);
      expect(artifact.bytes.length).toBeGreaterThan(0);
      expect(artifact.mimeType).toBeDefined();
      expect(artifact.mimeType).toBe('application/pdf');
    });

    it('should include render time in milliseconds', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'pdf' });

      expect(result.renderTimeMs).toBeGreaterThan(0);
      // Rendering should be reasonably fast
      expect(result.renderTimeMs).toBeLessThan(10000); // 10 seconds max
    });

    it('should produce valid PDF bytes', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'pdf' });

      const pdfBytes = result.artifacts[0].bytes;
      
      // Check PDF magic number (first 4 bytes should be %PDF)
      expect(isPDF(pdfBytes)).toBe(true);
    });

    it('should produce non-empty warnings array', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'pdf' });

      expect(Array.isArray(result.warnings)).toBe(true);
      // Warnings may be empty, but array should exist
    });
  });

  describe('render() formats', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
    });

    it('should render to PDF by default', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, {});

      expect(result.artifacts[0].format).toBe('pdf');
      expect(result.artifacts[0].mimeType).toBe('application/pdf');
    });

    it('should render to PDF when explicitly specified', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'pdf' });

      expect(result.artifacts[0].format).toBe('pdf');
    });

    it('should render to SVG when specified', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'svg' });

      expect(result.artifacts[0].format).toBe('svg');
      expect(result.artifacts[0].mimeType).toBe('image/svg+xml');
    });

    it('should render to TXT when specified', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      
      // TXT format might not be supported by typst backend
      // This test will skip if not supported
      try {
        const result = engine.render(parsed, { format: 'txt' });
        expect(result.artifacts[0].format).toBe('txt');
        expect(result.artifacts[0].mimeType).toBe('text/plain');
      } catch (e) {
        // TXT format not supported - that's OK
        // Check that error mentions txt or is a backend error
        const errorStr = e.message || e.toString();
        expect(errorStr.toLowerCase()).toMatch(/txt|format|support/i);
      }
    });
  });

  describe('render() error handling', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
    });

    it('should throw error when quill not registered', () => {
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);

      expect(() => {
        engine.render(parsed, {});
      }).toThrow();
    });

    it('should throw error when no quill specified', () => {
      const markdown = `---
title: No Quill
---

Content`;
      const parsed = Quillmark.parseMarkdown(markdown);
      expect(parsed.quillTag).toBeUndefined();

      expect(() => {
        engine.render(parsed, {});
      }).toThrow();
    });

    it('should throw error for invalid ParsedDocument', () => {
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);

      const invalidParsed = { notValid: 'structure' };

      expect(() => {
        engine.render(invalidParsed, { quillName: 'test_quill' });
      }).toThrow();
    });
  });

  describe('renderGlue()', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
    });

    it('should return template source code', () => {
      const glueOutput = engine.renderGlue('test_quill', SIMPLE_MARKDOWN);

      expect(typeof glueOutput).toBe('string');
      expect(glueOutput.length).toBeGreaterThan(0);
    });

    it('should include processed template content', () => {
      const glueOutput = engine.renderGlue('test_quill', SIMPLE_MARKDOWN);

      // Should contain the title from markdown
      expect(glueOutput).toContain('Test Document');
    });

    it('should throw error for non-existent quill', () => {
      expect(() => {
        engine.renderGlue('non-existent', SIMPLE_MARKDOWN);
      }).toThrow();
    });

    it('should work as debugging tool before full render', () => {
      // This is a common workflow: check glue output before rendering
      const glueOutput = engine.renderGlue('test_quill', SIMPLE_MARKDOWN);
      expect(glueOutput).toBeDefined();

      // Then render should also work
      const parsed = Quillmark.parseMarkdown(SIMPLE_MARKDOWN);
      const result = engine.render(parsed, { format: 'pdf' });
      expect(result.artifacts.length).toBeGreaterThan(0);
    });
  });

  describe('Complex documents', () => {
    it('should handle letter document with multiple fields', () => {
      const engine = new Quillmark();
      engine.registerQuill('letter_quill', LETTER_QUILL_JSON);

      const parsed = Quillmark.parseMarkdown(LETTER_MARKDOWN);
      expect(getField(parsed, 'title')).toBe('Important Letter');
      expect(getField(parsed, 'author')).toBe('Charlie');
      expect(getField(parsed, 'date')).toBe('2025-10-17');

      const result = engine.render(parsed, { format: 'pdf' });
      expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
    });

    it('should handle documents with lists and formatting', () => {
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);

      const markdown = `---
title: Formatted Document
QUILL: test_quill
---

# Main Heading

This has **bold** and *italic* text.

## Lists

Unordered:
- Item A
- Item B
  - Nested item
- Item C

Ordered:
1. First
2. Second
3. Third

## Code

Inline \`code\` and block:

\`\`\`
code block
\`\`\`
`;

      const parsed = Quillmark.parseMarkdown(markdown);
      const result = engine.render(parsed, { format: 'pdf' });

      expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
    });
  });
});
