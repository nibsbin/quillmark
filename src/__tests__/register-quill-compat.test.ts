import { describe, it, expect, vi, beforeAll, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { Quillmark, init } from '@quillmark/wasm';
import { QuillRegistry } from '../registry.js';
import { FileSystemSource } from '../sources/file-system-source.js';
import { HttpSource } from '../sources/http-source.js';
import type { QuillManifest, QuillmarkEngine } from '../types.js';

/** Path to the minimal quill fixtures for integration tests. */
const QUILLS_DIR = path.join(import.meta.dirname, 'fixtures/quills');

/** Temp directory for packageForHttp output. */
const HTTP_OUTPUT_DIR = path.join(import.meta.dirname, '../../.test-fixtures-compat');

describe('registerQuill compatibility with @quillmark/wasm', () => {
	let wasm: Quillmark;

	beforeAll(() => {
		init();
	});

	afterEach(async () => {
		await fs.rm(HTTP_OUTPUT_DIR, { recursive: true, force: true });
	});

	describe('FileSystemSource → real Quillmark engine', () => {
		it('should register minimal_quill from filesystem fixtures', async () => {
			wasm = new Quillmark();
			const engine = wasm as unknown as QuillmarkEngine;
			const source = new FileSystemSource(QUILLS_DIR);
			const registry = new QuillRegistry({ source, engine });

			const bundle = await registry.resolve('minimal_quill');

			expect(bundle.name).toBe('minimal_quill');
			expect(bundle.version).toBe('0.1.0');

			// Verify the real engine has it registered
			const info = engine.resolveQuill('minimal_quill');
			expect(info).not.toBeNull();
			expect(info!.name).toBe('minimal_quill');
			expect((info!.metadata as Record<string, unknown>).version).toBe('0.1.0');
			expect(engine.listQuills()).toContain('minimal_quill@0.1.0');

			wasm.free();
		});

		it('should register all quills from the fixtures', async () => {
			wasm = new Quillmark();
			const engine = wasm as unknown as QuillmarkEngine;
			const source = new FileSystemSource(QUILLS_DIR);
			const registry = new QuillRegistry({ source, engine });
			const manifest = await registry.getManifest();

			for (const quill of manifest.quills) {
				await registry.resolve(quill.name, quill.version);
			}

			const listed = engine.listQuills();
			for (const quill of manifest.quills) {
				expect(listed).toContain(`${quill.name}@${quill.version}`);
			}

			wasm.free();
		});

		it('should not re-register a quill already in the engine', async () => {
			wasm = new Quillmark();
			const engine = wasm as unknown as QuillmarkEngine;
			const spy = vi.spyOn(wasm, 'registerQuill');
			const source = new FileSystemSource(QUILLS_DIR);
			const registry = new QuillRegistry({ source, engine });

			await registry.resolve('minimal_quill');
			expect(spy).toHaveBeenCalledTimes(1);

			// Second resolve: engine already has it, skips registration
			await registry.resolve('minimal_quill');
			expect(spy).toHaveBeenCalledTimes(1);

			spy.mockRestore();
			wasm.free();
		});
	});

	describe('HttpSource → real Quillmark engine', () => {
		it('should register minimal_quill loaded via HttpSource zip', async () => {
			wasm = new Quillmark();
			const engine = wasm as unknown as QuillmarkEngine;

			// Package the fixtures for HTTP
			const fsSource = new FileSystemSource(QUILLS_DIR);
			await fsSource.packageForHttp(HTTP_OUTPUT_DIR);

			const manifestJson = await fs.readFile(
				path.join(HTTP_OUTPUT_DIR, 'manifest.json'),
				'utf-8',
			);

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(manifestJson);
				}
				const zipMatch = urlStr.match(/\/([^/?]+\.zip)/);
				if (zipMatch) {
					const zipPath = path.join(HTTP_OUTPUT_DIR, zipMatch[1]);
					try {
						const zipData = await fs.readFile(zipPath);
						return new Response(zipData);
					} catch {
						return new Response(null, { status: 404 });
					}
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			const httpSource = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});
			const registry = new QuillRegistry({ source: httpSource, engine });

			const bundle = await registry.resolve('minimal_quill');

			expect(bundle.name).toBe('minimal_quill');
			expect(bundle.version).toBe('0.1.0');
			const info = engine.resolveQuill('minimal_quill');
			expect(info).not.toBeNull();
			expect(info!.name).toBe('minimal_quill');
			expect(engine.listQuills()).toContain('minimal_quill@0.1.0');

			wasm.free();
		});

		it('should register all quills via HttpSource zips', async () => {
			wasm = new Quillmark();
			const engine = wasm as unknown as QuillmarkEngine;

			const fsSource = new FileSystemSource(QUILLS_DIR);
			await fsSource.packageForHttp(HTTP_OUTPUT_DIR);

			const manifestJson = await fs.readFile(
				path.join(HTTP_OUTPUT_DIR, 'manifest.json'),
				'utf-8',
			);
			const manifest = JSON.parse(manifestJson) as QuillManifest;

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(manifestJson);
				}
				const zipMatch = urlStr.match(/\/([^/?]+\.zip)/);
				if (zipMatch) {
					const zipPath = path.join(HTTP_OUTPUT_DIR, zipMatch[1]);
					try {
						const zipData = await fs.readFile(zipPath);
						return new Response(zipData);
					} catch {
						return new Response(null, { status: 404 });
					}
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			const httpSource = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});
			const registry = new QuillRegistry({ source: httpSource, engine });

			for (const quill of manifest.quills) {
				await registry.resolve(quill.name, quill.version);
			}

			const listed = engine.listQuills();
			for (const quill of manifest.quills) {
				expect(listed).toContain(`${quill.name}@${quill.version}`);
			}

			wasm.free();
		});
	});

	describe('FileSystemSource → packageForHttp → HttpSource roundtrip', () => {
		it('should produce identical registrations through the full roundtrip', async () => {
			const fsSource = new FileSystemSource(QUILLS_DIR);
			await fsSource.packageForHttp(HTTP_OUTPUT_DIR);

			const manifestJson = await fs.readFile(
				path.join(HTTP_OUTPUT_DIR, 'manifest.json'),
				'utf-8',
			);

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(manifestJson);
				}
				const zipMatch = urlStr.match(/\/([^/?]+\.zip)/);
				if (zipMatch) {
					const zipPath = path.join(HTTP_OUTPUT_DIR, zipMatch[1]);
					try {
						const zipData = await fs.readFile(zipPath);
						return new Response(zipData);
					} catch {
						return new Response(null, { status: 404 });
					}
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			const httpSource = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			// Register minimal_quill from both sources with separate engines
			const fsWasm = new Quillmark();
			const fsEngine = fsWasm as unknown as QuillmarkEngine;
			const fsRegistry = new QuillRegistry({ source: fsSource, engine: fsEngine });
			const fsBundle = await fsRegistry.resolve('minimal_quill');

			const httpWasm = new Quillmark();
			const httpEngine = httpWasm as unknown as QuillmarkEngine;
			const httpRegistry = new QuillRegistry({ source: httpSource, engine: httpEngine });
			const httpBundle = await httpRegistry.resolve('minimal_quill');

			// Both should register with the same identity
			const fsInfo = fsEngine.resolveQuill('minimal_quill')!;
			const httpInfo = httpEngine.resolveQuill('minimal_quill')!;
			expect(fsInfo.name).toBe(httpInfo.name);
			expect(fsInfo.metadata.version as string).toBe(httpInfo.metadata.version as string);
			expect(fsBundle.name).toBe(httpBundle.name);
			expect(fsBundle.version).toBe(httpBundle.version);

			fsWasm.free();
			httpWasm.free();
		});
	});
});
