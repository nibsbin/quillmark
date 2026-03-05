import { describe, it, expect, vi, beforeAll, afterAll, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import JSZip from 'jszip';
import { Quillmark, init } from '@quillmark/wasm';
import { QuillRegistry } from '../registry.js';
import { FileSystemSource } from '../sources/file-system-source.js';
import { HttpSource } from '../sources/http-source.js';
import type { QuillManifest, QuillmarkEngine } from '../types.js';

/** Path to the real quill fixtures from the tonguetoquill-collection. */
const QUILLS_DIR = path.join(import.meta.dirname, '../../tonguetoquill-collection/quills');

/** Temp directory for packageForHttp output. */
const HTTP_OUTPUT_DIR = path.join(import.meta.dirname, '../../.test-fixtures-compat');

const BINARY_EXT = /\.(ttf|otf|woff2?|jpg|jpeg|png|gif|pdf|zip)$/i;

/**
 * Converts a flat Record<string, Uint8Array> (as produced by FileSystemSource/HttpSource)
 * into the nested file tree that @quillmark/wasm's registerQuill expects:
 *   { files: { dir: { file: { contents: string | number[] } } } }
 *
 * Text files get string contents; binary files get number[] contents.
 */
function toEngineFileTree(flatFiles: Record<string, Uint8Array>): {
	files: Record<string, unknown>;
} {
	const decoder = new TextDecoder('utf-8', { fatal: false });
	const tree: Record<string, unknown> = {};

	for (const [filePath, bytes] of Object.entries(flatFiles)) {
		const parts = filePath.split(/[/\\]/);
		let current = tree as Record<string, Record<string, unknown>>;
		for (let i = 0; i < parts.length - 1; i++) {
			current[parts[i]] = (current[parts[i]] as Record<string, unknown>) ?? {};
			current = current[parts[i]] as Record<string, Record<string, unknown>>;
		}
		const fileName = parts[parts.length - 1];
		(current as Record<string, unknown>)[fileName] = {
			contents: BINARY_EXT.test(fileName) ? Array.from(bytes) : decoder.decode(bytes),
		};
	}

	return { files: tree };
}

/**
 * Wraps the real @quillmark/wasm Quillmark instance as a QuillmarkEngine.
 *
 * Adapts two mismatches between the real engine and the registry's interface:
 *   1. Data format: sources produce flat Record<string, Uint8Array>, but
 *      the WASM engine expects a nested { files: { ... } } tree.
 *   2. QuillInfo shape: the real engine returns { name, backend, metadata, ... }
 *      where version lives in metadata.version, but the registry interface
 *      expects { name, version }.
 */
function wrapEngine(wasm: Quillmark): QuillmarkEngine {
	return {
		registerQuill(quillData: unknown) {
			const flat = quillData as Record<string, Uint8Array>;
			const tree = toEngineFileTree(flat);
			const info = wasm.registerQuill(tree) as { name: string; metadata: { version: string } };
			return { name: info.name, version: info.metadata.version };
		},
		resolveQuill(quillRef: string) {
			const info = wasm.resolveQuill(quillRef) as {
				name: string;
				metadata: { version: string };
			} | null;
			if (!info) return null;
			return { name: info.name, version: info.metadata.version };
		},
		listQuills() {
			return wasm.listQuills();
		},
	};
}

describe('registerQuill compatibility with @quillmark/wasm', () => {
	let wasm: Quillmark;

	beforeAll(() => {
		init();
	});

	afterEach(async () => {
		await fs.rm(HTTP_OUTPUT_DIR, { recursive: true, force: true });
	});

	describe('FileSystemSource → real Quillmark engine', () => {
		it('should register classic_resume from filesystem fixtures', async () => {
			wasm = new Quillmark();
			const engine = wrapEngine(wasm);
			const source = new FileSystemSource(QUILLS_DIR);
			const registry = new QuillRegistry({ source, engine });

			const bundle = await registry.resolve('classic_resume');

			expect(bundle.name).toBe('classic_resume');
			expect(bundle.version).toBe('0.1.0');

			// Verify the real engine has it registered
			expect(engine.resolveQuill('classic_resume')).toEqual({
				name: 'classic_resume',
				version: '0.1.0',
			});
			expect(engine.listQuills()).toContain('classic_resume@0.1.0');

			wasm.free();
		});

		it('should register usaf_memo (with binary assets and packages)', async () => {
			wasm = new Quillmark();
			const engine = wrapEngine(wasm);
			const source = new FileSystemSource(QUILLS_DIR);
			const registry = new QuillRegistry({ source, engine });

			const bundle = await registry.resolve('usaf_memo');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('0.1.0');

			// Verify bundle data contains expected file types
			const data = bundle.data as Record<string, Uint8Array>;
			expect(data['Quill.yaml']).toBeInstanceOf(Uint8Array);
			expect(data['plate.typ']).toBeInstanceOf(Uint8Array);
			expect(data[path.join('assets', 'dow_seal.jpg')]).toBeInstanceOf(Uint8Array);

			// Verify the real engine accepts and resolves it
			expect(engine.resolveQuill('usaf_memo')).toEqual({
				name: 'usaf_memo',
				version: '0.1.0',
			});

			wasm.free();
		});

		it('should register all quills from the collection', async () => {
			wasm = new Quillmark();
			const engine = wrapEngine(wasm);
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
			const engine = wrapEngine(wasm);
			const spy = vi.spyOn(wasm, 'registerQuill');
			const source = new FileSystemSource(QUILLS_DIR);
			const registry = new QuillRegistry({ source, engine });

			await registry.resolve('classic_resume');
			expect(spy).toHaveBeenCalledTimes(1);

			// Second resolve: engine already has it, skips registration
			await registry.resolve('classic_resume');
			expect(spy).toHaveBeenCalledTimes(1);

			spy.mockRestore();
			wasm.free();
		});
	});

	describe('HttpSource → real Quillmark engine', () => {
		it('should register classic_resume loaded via HttpSource zip', async () => {
			wasm = new Quillmark();
			const engine = wrapEngine(wasm);

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

			const bundle = await registry.resolve('classic_resume');

			expect(bundle.name).toBe('classic_resume');
			expect(bundle.version).toBe('0.1.0');
			expect(engine.resolveQuill('classic_resume')).toEqual({
				name: 'classic_resume',
				version: '0.1.0',
			});
			expect(engine.listQuills()).toContain('classic_resume@0.1.0');

			wasm.free();
		});

		it('should register all quills via HttpSource zips', async () => {
			wasm = new Quillmark();
			const engine = wrapEngine(wasm);

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

			// Register classic_resume from both sources with separate engines
			const fsWasm = new Quillmark();
			const fsEngine = wrapEngine(fsWasm);
			const fsRegistry = new QuillRegistry({ source: fsSource, engine: fsEngine });
			const fsBundle = await fsRegistry.resolve('classic_resume');

			const httpWasm = new Quillmark();
			const httpEngine = wrapEngine(httpWasm);
			const httpRegistry = new QuillRegistry({ source: httpSource, engine: httpEngine });
			const httpBundle = await httpRegistry.resolve('classic_resume');

			// Both should register with the same identity
			expect(fsEngine.resolveQuill('classic_resume')).toEqual(
				httpEngine.resolveQuill('classic_resume'),
			);
			expect(fsBundle.name).toBe(httpBundle.name);
			expect(fsBundle.version).toBe(httpBundle.version);

			// Both should have the same file keys
			const fsData = fsBundle.data as Record<string, Uint8Array>;
			const httpData = httpBundle.data as Record<string, Uint8Array>;
			expect(Object.keys(fsData).sort()).toEqual(Object.keys(httpData).sort());

			// Text file contents should match
			const decoder = new TextDecoder();
			for (const key of Object.keys(fsData)) {
				if (!BINARY_EXT.test(key)) {
					expect(decoder.decode(httpData[key])).toBe(decoder.decode(fsData[key]));
				}
			}

			fsWasm.free();
			httpWasm.free();
		});
	});

	describe('engine state with multiple quills', () => {
		it('should track quills from different sources in one engine', async () => {
			wasm = new Quillmark();
			const engine = wrapEngine(wasm);

			// Load usaf_memo from FileSystemSource
			const fsSource = new FileSystemSource(QUILLS_DIR);
			const fsRegistry = new QuillRegistry({ source: fsSource, engine });
			await fsRegistry.resolve('usaf_memo');

			// Load classic_resume from HttpSource
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
			const httpRegistry = new QuillRegistry({ source: httpSource, engine });
			await httpRegistry.resolve('classic_resume');

			// Both quills should be tracked
			const listed = engine.listQuills();
			expect(listed).toContain('usaf_memo@0.1.0');
			expect(listed).toContain('classic_resume@0.1.0');

			expect(engine.resolveQuill('usaf_memo')).toEqual({
				name: 'usaf_memo',
				version: '0.1.0',
			});
			expect(engine.resolveQuill('classic_resume')).toEqual({
				name: 'classic_resume',
				version: '0.1.0',
			});

			wasm.free();
		});
	});

	describe('toEngineFileTree conversion', () => {
		it('should convert flat Record<string, Uint8Array> to nested tree', () => {
			const encoder = new TextEncoder();
			const flat: Record<string, Uint8Array> = {
				'Quill.yaml': encoder.encode('Quill:\n  name: test\n  version: 0.1.0'),
				'plate.typ': encoder.encode('= Hello'),
			};

			const tree = toEngineFileTree(flat);

			expect(tree).toEqual({
				files: {
					'Quill.yaml': { contents: 'Quill:\n  name: test\n  version: 0.1.0' },
					'plate.typ': { contents: '= Hello' },
				},
			});
		});

		it('should nest subdirectory paths into tree branches', () => {
			const encoder = new TextEncoder();
			const flat: Record<string, Uint8Array> = {
				'Quill.yaml': encoder.encode('name: test'),
				'assets/logo.txt': encoder.encode('logo'),
				'packages/my-pkg/typst.toml': encoder.encode('[package]'),
			};

			const tree = toEngineFileTree(flat);

			expect(tree).toEqual({
				files: {
					'Quill.yaml': { contents: 'name: test' },
					assets: {
						'logo.txt': { contents: 'logo' },
					},
					packages: {
						'my-pkg': {
							'typst.toml': { contents: '[package]' },
						},
					},
				},
			});
		});

		it('should encode binary files as number[] instead of strings', () => {
			const jpgBytes = new Uint8Array([0xff, 0xd8, 0xff, 0xe0]);
			const flat: Record<string, Uint8Array> = {
				'assets/image.jpg': jpgBytes,
			};

			const tree = toEngineFileTree(flat);
			const leaf = (tree.files as Record<string, unknown>)['assets'] as Record<
				string,
				{ contents: unknown }
			>;
			expect(leaf['image.jpg'].contents).toEqual([0xff, 0xd8, 0xff, 0xe0]);
		});
	});
});
