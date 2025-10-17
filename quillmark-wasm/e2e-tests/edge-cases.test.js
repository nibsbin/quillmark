/**
 * End-to-End Tests for Quillmark WASM - Edge Cases and Error Handling
 * 
 * Tests edge cases, error conditions, and boundary scenarios.
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { Quillmark } from '@quillmark-test/wasm';
import { SMALL_QUILL_JSON } from './fixtures/test-data.js';
import { getField, getFieldNames } from './test-helpers.js';

describe('Quillmark WASM - Edge Cases', () => {
  describe('Empty and minimal inputs', () => {
    it('should handle empty markdown string', () => {
      const parsed = Quillmark.parseMarkdown('');
      
      expect(parsed).toBeDefined();
      expect(parsed.fields).toBeDefined();
      expect(Object.keys(parsed.fields).length).toBe(0);
    });

    it('should handle markdown with only frontmatter', () => {
      const markdown = `---
title: Only Frontmatter
---`;
      const parsed = Quillmark.parseMarkdown(markdown);
      
      expect(getField(parsed, 'title')).toBe('Only Frontmatter');
    });

    it('should handle markdown with only content (no frontmatter)', () => {
      const markdown = 'Just some content without frontmatter.';
      const parsed = Quillmark.parseMarkdown(markdown);
      
      expect(parsed).toBeDefined();
      expect(Object.keys(parsed.fields).length).toBe(0);
    });

    it('should handle single character markdown', () => {
      const parsed = Quillmark.parseMarkdown('x');
      expect(parsed).toBeDefined();
    });

    it('should handle very long markdown', () => {
      const longContent = '# Heading\n\n' + 'Lorem ipsum '.repeat(10000);
      const markdown = `---
title: Long Document
QUILL: test_quill
---

${longContent}`;
      
      const parsed = Quillmark.parseMarkdown(markdown);
      expect(parsed).toBeDefined();
      
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
      
      const result = engine.render(parsed, { format: 'pdf' });
      expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
    });
  });

  describe('Special characters and Unicode', () => {
    it('should handle Unicode characters in markdown', () => {
      const markdown = `---
title: Unicode Test 🚀
author: François
QUILL: test_quill
---

# Hello 世界

Testing Unicode: café, naïve, Zürich, 日本語

Emojis: 🎉 🎨 💻 📝
`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'title')).toBe('Unicode Test 🚀');
      expect(getField(parsed, 'author')).toBe('François');
      
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
      
      const result = engine.render(parsed, { format: 'pdf' });
      expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
    });

    it('should handle special characters in field values', () => {
      const markdown = `---
title: "Special: chars & symbols"
description: "Line 1\\nLine 2\\nLine 3"
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'title')).toContain('Special');
    });

    it('should handle escaped characters', () => {
      const markdown = `---
title: "Escaped \\" quote"
---

Content with \\*asterisks\\* and \\[brackets\\]
`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(parsed).toBeDefined();
    });
  });

  describe('Whitespace handling', () => {
    it('should handle markdown with extra whitespace', () => {
      const markdown = `---
title: Whitespace Test
---


# Heading


Content with    lots   of     spaces.


`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'title')).toBe('Whitespace Test');
    });

    it('should handle tabs in markdown', () => {
      const markdown = `---
title: Tabs Test
---

\t# Heading with tab

Content\twith\ttabs.
`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(parsed).toBeDefined();
    });

    it('should handle mixed line endings', () => {
      const markdown = "---\ntitle: Mixed\r\n---\r\n\r\nContent\n";
      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'title')).toBe('Mixed');
    });
  });

  describe('Field type handling', () => {
    it('should handle nested YAML structures', () => {
      const markdown = `---
title: Nested Test
metadata:
  author: Alice
  date: 2025-10-17
  tags:
    - test
    - e2e
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'metadata')).toBeDefined();
      expect(typeof getField(parsed, 'metadata')).toBe('object');
    });

    it('should handle null and undefined values', () => {
      const markdown = `---
title: Null Test
nullField: null
emptyField:
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'title')).toBe('Null Test');
      // Null and undefined fields may be omitted from the map
      const nullVal = getField(parsed, 'nullField');
      expect(nullVal === null || nullVal === undefined).toBe(true);
    });

    it('should handle boolean values', () => {
      const markdown = `---
published: true
draft: false
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'published')).toBe(true);
      expect(getField(parsed, 'draft')).toBe(false);
    });

    it('should handle numeric values', () => {
      const markdown = `---
integer: 42
float: 3.14
negative: -10
scientific: 1e10
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'integer')).toBe(42);
      expect(getField(parsed, 'float')).toBe(3.14);
      expect(getField(parsed, 'negative')).toBe(-10);
    });

    it('should handle date values', () => {
      const markdown = `---
date: 2025-10-17
datetime: 2025-10-17T12:00:00Z
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(getField(parsed, 'date')).toBeDefined();
      expect(getField(parsed, 'datetime')).toBeDefined();
    });
  });

  describe('Error messages', () => {
    it('should provide helpful error for missing quill', () => {
      const engine = new Quillmark();
      const parsed = Quillmark.parseMarkdown(`---
QUILL: missing_quill
---

Content`);

      try {
        engine.render(parsed, {});
        expect.fail('Should have thrown');
      } catch (error) {
        const errorStr = error.message || error.toString();
        expect(errorStr).toContain('missing_quill');
      }
    });

    it('should provide error details for invalid Quill.toml', () => {
      const engine = new Quillmark();
      const invalidQuill = {
        files: {
          'Quill.toml': {
            contents: 'invalid toml [[[',
          },
        },
      };

      try {
        engine.registerQuill('invalid', invalidQuill);
        expect.fail('Should have thrown');
      } catch (error) {
        expect(error).toBeDefined();
      }
    });
  });

  describe('Memory and performance', () => {
    it('should handle multiple renders without leaking', () => {
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);

      const markdown = `---
title: Repeated Render
QUILL: test_quill
---

Content`;
      const parsed = Quillmark.parseMarkdown(markdown);

      // Render multiple times
      for (let i = 0; i < 10; i++) {
        const result = engine.render(parsed, { format: 'pdf' });
        expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
      }
    });

    it('should handle registering and unregistering repeatedly', () => {
      const engine = new Quillmark();

      for (let i = 0; i < 10; i++) {
        engine.registerQuill('test_quill', SMALL_QUILL_JSON);
        expect(engine.listQuills()).toContain('test_quill');
        
        engine.unregisterQuill('test_quill');
        expect(engine.listQuills()).not.toContain('test_quill');
      }
    });

    it('should handle many quills registered simultaneously', () => {
      const engine = new Quillmark();

      for (let i = 0; i < 20; i++) {
        engine.registerQuill(`quill-${i}`, SMALL_QUILL_JSON);
      }

      const quills = engine.listQuills();
      expect(quills.length).toBe(20);
    });
  });

  describe('QUILL field variations', () => {
    it('should handle QUILL field with different casing', () => {
      // Test that QUILL field is case-sensitive
      const markdown = `---
quill: test_quill
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      // 'quill' (lowercase) is not the QUILL field
      expect(parsed.quillTag).toBeUndefined();
      expect(getField(parsed, 'quill')).toBe('test_quill');
    });

    it('should prioritize QUILL field over quill field', () => {
      const markdown = `---
QUILL: quill_uppercase
quill: quill_lowercase
---

Content`;

      const parsed = Quillmark.parseMarkdown(markdown);
      expect(parsed.quillTag).toBe('quill_uppercase');
    });
  });

  describe('Render options variations', () => {
    let engine;

    beforeEach(() => {
      engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);
    });

    it('should handle undefined render options', () => {
      const parsed = Quillmark.parseMarkdown(`---
QUILL: test_quill
---

Content`);

      const result = engine.render(parsed, undefined);
      expect(result.artifacts.length).toBeGreaterThan(0);
    });

    it('should handle null render options', () => {
      const parsed = Quillmark.parseMarkdown(`---
QUILL: test_quill
---

Content`);

      const result = engine.render(parsed, null);
      expect(result.artifacts.length).toBeGreaterThan(0);
    });

    it('should handle empty object render options', () => {
      const parsed = Quillmark.parseMarkdown(`---
QUILL: test_quill
---

Content`);

      const result = engine.render(parsed, {});
      expect(result.artifacts.length).toBeGreaterThan(0);
    });
  });

  describe('Concurrency', () => {
    it('should handle concurrent renders on same engine', async () => {
      const engine = new Quillmark();
      engine.registerQuill('test_quill', SMALL_QUILL_JSON);

      const markdown = `---
title: Concurrent Test
QUILL: test_quill
---

Content`;
      const parsed = Quillmark.parseMarkdown(markdown);

      // Create multiple render promises
      const renders = Array(5).fill(null).map(() => {
        return new Promise((resolve) => {
          const result = engine.render(parsed, { format: 'pdf' });
          resolve(result);
        });
      });

      const results = await Promise.all(renders);
      
      results.forEach(result => {
        expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
      });
    });

    it('should handle independent engines in parallel', async () => {
      const engines = Array(3).fill(null).map(() => {
        const engine = new Quillmark();
        engine.registerQuill('test_quill', SMALL_QUILL_JSON);
        return engine;
      });

      const markdown = `---
QUILL: test_quill
---

Content`;
      const parsed = Quillmark.parseMarkdown(markdown);

      const renders = engines.map(engine => {
        return new Promise((resolve) => {
          const result = engine.render(parsed, { format: 'pdf' });
          resolve(result);
        });
      });

      const results = await Promise.all(renders);
      
      results.forEach(result => {
        expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
      });
    });
  });
});
