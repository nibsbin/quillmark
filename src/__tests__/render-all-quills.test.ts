import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'node:path';
import { Quillmark, init } from '@quillmark/wasm';
import { validateQuills } from '../validate.js';

/** Path to the minimal quill fixtures for integration tests. */
const QUILLS_DIR = path.join(import.meta.dirname, 'fixtures/quills');

describe('validateQuills', () => {
	beforeAll(() => {
		init();
	});

	it('should validate every quill and version without error', async () => {
		const wasm = new Quillmark();

		try {
			const { passed, failed, results } = await validateQuills({
				quillsDir: QUILLS_DIR,
				engine: wasm,
				parseMarkdown: Quillmark.parseMarkdown,
			});

			expect(results.length).toBeGreaterThan(0);
			expect(failed).toBe(0);

			for (const entry of results) {
				expect(entry.registered).toBe(true);
				expect(entry.rendered).toBe(true);
				expect(entry.error).toBeUndefined();
			}

			expect(passed).toBe(results.length);
		} finally {
			wasm.free();
		}
	});

	it('should throw for a non-existent quills directory', async () => {
		const wasm = new Quillmark();

		try {
			await expect(
				validateQuills({
					quillsDir: path.join(QUILLS_DIR, 'does-not-exist'),
					engine: wasm,
					parseMarkdown: Quillmark.parseMarkdown,
				}),
			).rejects.toThrow('Failed to read quills directory');
		} finally {
			wasm.free();
		}
	});
});

