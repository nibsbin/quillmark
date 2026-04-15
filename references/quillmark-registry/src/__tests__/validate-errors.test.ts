import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import { validateQuills } from '../validate.js';
import type { QuillValidationEngine } from '../validate.js';
import type { QuillInfo } from '../types.js';

class ThrowingRenderEngine implements QuillValidationEngine {
	private quillInfo: QuillInfo | null = null;

	registerQuill(_: unknown): QuillInfo {
		this.quillInfo = {
			name: 'sample',
			backend: 'typst',
			metadata: { version: '0.1.0' },
			example: '# Example\n\nQUILL: sample:0.1.0',
			schema: '',
			defaults: {},
			examples: {},
			supportedFormats: ['pdf'],
		};
		return this.quillInfo;
	}

	resolveQuill(_: string): QuillInfo | null {
		return this.quillInfo;
	}

	listQuills(): string[] {
		return this.quillInfo ? [this.quillInfo.name] : [];
	}

	render(): { artifacts: Array<{ bytes: Uint8Array }> } {
		throw new Map([
			['code', 'typst_error'],
			['message', 'Unknown field "name"'],
		]);
	}
}

describe('validateQuills error formatting', () => {
	it('serializes structured render errors for consumers', async () => {
		const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'quill-registry-test-'));
		const quillsDir = path.join(tempRoot, 'quills');
		const quillVersionDir = path.join(quillsDir, 'sample', '0.1.0');
		await fs.mkdir(quillVersionDir, { recursive: true });
		await fs.writeFile(path.join(quillVersionDir, 'Quill.yaml'), 'name: sample\nversion: 0.1.0\n');

		try {
			const result = await validateQuills({
				quillsDir,
				engine: new ThrowingRenderEngine(),
				parseMarkdown: () => ({ fields: {}, quillName: 'sample' }),
			});

			expect(result.failed).toBe(1);
			expect(result.results).toHaveLength(1);
			expect(result.results[0].registered).toBe(true);
			expect(result.results[0].rendered).toBe(false);
			expect(result.results[0].error).toContain('"type": "Map"');
			expect(result.results[0].error).toContain('typst_error');
			expect(result.results[0].error).toContain('Unknown field \\"name\\"');
		} finally {
			await fs.rm(tempRoot, { recursive: true, force: true });
		}
	});
});
