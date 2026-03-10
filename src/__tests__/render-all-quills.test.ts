import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'node:path';
import { Quillmark, init } from '@quillmark/wasm';
import type { QuillInfo as WasmQuillInfo } from '@quillmark/wasm';
import { QuillRegistry } from '../registry.js';
import { FileSystemSource } from '../sources/file-system-source.js';
import type { QuillmarkEngine } from '../types.js';

/** Path to the real quill fixtures from the tonguetoquill-collection. */
const QUILLS_DIR = path.join(import.meta.dirname, '../../tonguetoquill-collection/quills');

describe('render every Quill in the registry', () => {
	let source: FileSystemSource;

	beforeAll(() => {
		init();
		source = new FileSystemSource(QUILLS_DIR);
	});

	it('should discover at least one quill in the registry', async () => {
		const manifest = await source.getManifest();
		expect(manifest.quills.length).toBeGreaterThan(0);
	});

	it('should render every quill and version without error', async () => {
		const manifest = await source.getManifest();
		const wasm = new Quillmark();

		try {
			const engine = wasm as unknown as QuillmarkEngine;
			const registry = new QuillRegistry({ source, engine });

			for (const quill of manifest.quills) {
				const bundle = await registry.resolve(`${quill.name}@${quill.version}`);
				expect(bundle.name).toBe(quill.name);
				expect(bundle.version).toBe(quill.version);

				const info = engine.resolveQuill(quill.name) as unknown as WasmQuillInfo;
				expect(info).not.toBeNull();
				expect(info.supportedFormats.length).toBeGreaterThan(0);

				// The example field contains markdown content embedded in the quill
				expect(info.example).toBeTruthy();

				// Replace the colon-style QUILL reference (e.g. "name:0.1") with
				// the engine-compatible "@" format (e.g. "name@0.1.0")
				const ref = `${quill.name}@${quill.version}`;
				const exampleMd = info.example!.replace(/^QUILL:.*$/m, `QUILL: ${ref}`);

				const parsed = Quillmark.parseMarkdown(exampleMd);
				const result = wasm.render(parsed, { format: info.supportedFormats[0] });

				expect(result.artifacts.length).toBeGreaterThan(0);
				expect(result.artifacts[0].bytes.length).toBeGreaterThan(0);
			}
		} finally {
			wasm.free();
		}
	});
});
