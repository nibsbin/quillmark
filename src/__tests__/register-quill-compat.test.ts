import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import JSZip from 'jszip';
import { QuillRegistry } from '../registry.js';
import { FileSystemSource } from '../sources/file-system-source.js';
import { HttpSource } from '../sources/http-source.js';
import type { QuillBundle, QuillManifest, QuillmarkEngine, QuillSource } from '../types.js';

const FIXTURES_DIR = path.join(import.meta.dirname, '../../.test-fixtures-compat');

const QUILL_YAML = `name: greeting_card\nversion: 1.0.0\ndescription: A greeting card template`;
const TEMPLATE_CONTENT = '#let data = json("data.json")\n= Hello, #data.name!';
const ASSET_CONTENT = '{"name": "World"}';

/** Helper: write a quill directory fixture. */
async function writeQuillFixture(
	quillsDir: string,
	name: string,
	version: string,
	files: Record<string, string>,
): Promise<void> {
	const dir = path.join(quillsDir, name, version);
	await fs.mkdir(dir, { recursive: true });
	for (const [filePath, content] of Object.entries(files)) {
		const fullPath = path.join(dir, filePath);
		await fs.mkdir(path.dirname(fullPath), { recursive: true });
		await fs.writeFile(fullPath, content, 'utf-8');
	}
}

/**
 * Creates a mock engine that mimics @quillmark/wasm's registerQuill behavior:
 * expects Record<string, Uint8Array> and decodes Quill.yaml to extract QuillInfo.
 */
function createWasmCompatEngine(): QuillmarkEngine {
	const registered = new Map<string, { name: string; version: string }>();
	const decoder = new TextDecoder();

	return {
		registerQuill: vi.fn((data: unknown) => {
			const files = data as Record<string, Uint8Array>;

			// @quillmark/wasm expects data to be a Record<string, Uint8Array>
			// and reads Quill.yaml to discover the quill identity
			const yamlBytes = files['Quill.yaml'];
			if (!yamlBytes || !(yamlBytes instanceof Uint8Array)) {
				throw new Error('registerQuill: missing or invalid Quill.yaml');
			}

			const yamlContent = decoder.decode(yamlBytes);
			const nameMatch = yamlContent.match(/name:\s*(\S+)/);
			const versionMatch = yamlContent.match(/version:\s*(\S+)/);

			if (!nameMatch || !versionMatch) {
				throw new Error('registerQuill: Quill.yaml missing name or version');
			}

			const info = { name: nameMatch[1], version: versionMatch[1] };
			registered.set(`${info.name}@${info.version}`, info);
			return info;
		}),
		resolveQuill: vi.fn((ref: string) => {
			if (registered.has(ref)) return registered.get(ref)!;
			if (!ref.includes('@')) {
				for (const [, info] of registered.entries()) {
					if (info.name === ref) return info;
				}
			}
			return null;
		}),
		listQuills: vi.fn(() => [...registered.keys()]),
	} as unknown as QuillmarkEngine;
}

/** Creates a mock zip containing realistic quill files as Uint8Array values. */
async function createQuillZip(files: Record<string, string>): Promise<ArrayBuffer> {
	const zip = new JSZip();
	for (const [name, content] of Object.entries(files)) {
		zip.file(name, content);
	}
	return zip.generateAsync({ type: 'arraybuffer' });
}

describe('registerQuill compatibility with Quill loading systems', () => {
	let engine: QuillmarkEngine;

	beforeEach(async () => {
		engine = createWasmCompatEngine();
		await fs.mkdir(FIXTURES_DIR, { recursive: true });
	});

	afterEach(async () => {
		await fs.rm(FIXTURES_DIR, { recursive: true, force: true });
	});

	describe('FileSystemSource → registerQuill', () => {
		it('should produce Record<string, Uint8Array> data that registerQuill accepts', async () => {
			const quillsDir = path.join(FIXTURES_DIR, 'fs-basic');
			await writeQuillFixture(quillsDir, 'greeting_card', '1.0.0', {
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
			});

			const source = new FileSystemSource(quillsDir);
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('greeting_card');

			// registerQuill was called
			expect(engine.registerQuill).toHaveBeenCalledOnce();

			// Verify data shape: Record<string, Uint8Array>
			const data = bundle.data as Record<string, Uint8Array>;
			expect(data['Quill.yaml']).toBeInstanceOf(Uint8Array);
			expect(data['template.typ']).toBeInstanceOf(Uint8Array);

			// Verify engine successfully registered the quill
			expect(engine.resolveQuill('greeting_card')).toEqual({
				name: 'greeting_card',
				version: '1.0.0',
			});
		});

		it('should pass multi-file bundles with nested paths to registerQuill', async () => {
			const quillsDir = path.join(FIXTURES_DIR, 'fs-multi');
			await writeQuillFixture(quillsDir, 'greeting_card', '1.0.0', {
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
				'assets/data.json': ASSET_CONTENT,
			});

			const source = new FileSystemSource(quillsDir);
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('greeting_card');

			const data = bundle.data as Record<string, Uint8Array>;
			const fileNames = Object.keys(data).sort();
			expect(fileNames).toEqual([
				'Quill.yaml',
				path.join('assets', 'data.json'),
				'template.typ',
			]);

			// All files are Uint8Array
			for (const value of Object.values(data)) {
				expect(value).toBeInstanceOf(Uint8Array);
			}

			// registerQuill received the complete data
			expect(engine.registerQuill).toHaveBeenCalledWith(data);
		});

		it('should produce data where Quill.yaml decodes to valid metadata', async () => {
			const quillsDir = path.join(FIXTURES_DIR, 'fs-yaml');
			await writeQuillFixture(quillsDir, 'greeting_card', '1.0.0', {
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
			});

			const source = new FileSystemSource(quillsDir);
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('greeting_card');

			// Verify the Quill.yaml bytes decode to the expected content
			const data = bundle.data as Record<string, Uint8Array>;
			const decoded = new TextDecoder().decode(data['Quill.yaml']);
			expect(decoded).toContain('name: greeting_card');
			expect(decoded).toContain('version: 1.0.0');
		});
	});

	describe('HttpSource → registerQuill', () => {
		it('should produce Record<string, Uint8Array> data that registerQuill accepts', async () => {
			const manifest: QuillManifest = {
				quills: [{ name: 'greeting_card', version: '1.0.0' }],
			};
			const zipData = await createQuillZip({
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
			});

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(JSON.stringify(manifest));
				}
				if (urlStr.includes('greeting_card@1.0.0.zip')) {
					return new Response(zipData);
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('greeting_card');

			// registerQuill was called
			expect(engine.registerQuill).toHaveBeenCalledOnce();

			// Verify data shape: Record<string, Uint8Array>
			const data = bundle.data as Record<string, Uint8Array>;
			expect(data['Quill.yaml']).toBeInstanceOf(Uint8Array);
			expect(data['template.typ']).toBeInstanceOf(Uint8Array);

			// Verify engine successfully registered the quill
			expect(engine.resolveQuill('greeting_card')).toEqual({
				name: 'greeting_card',
				version: '1.0.0',
			});
		});

		it('should pass multi-file bundles from zip to registerQuill', async () => {
			const manifest: QuillManifest = {
				quills: [{ name: 'greeting_card', version: '1.0.0' }],
			};
			const zipData = await createQuillZip({
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
				'assets/data.json': ASSET_CONTENT,
			});

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(JSON.stringify(manifest));
				}
				if (urlStr.includes('greeting_card@1.0.0.zip')) {
					return new Response(zipData);
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('greeting_card');

			const data = bundle.data as Record<string, Uint8Array>;
			expect(Object.keys(data).sort()).toEqual([
				'Quill.yaml',
				'assets/data.json',
				'template.typ',
			]);

			for (const value of Object.values(data)) {
				expect(value).toBeInstanceOf(Uint8Array);
			}

			expect(engine.registerQuill).toHaveBeenCalledWith(data);
		});

		it('should produce data where Quill.yaml decodes to valid metadata', async () => {
			const manifest: QuillManifest = {
				quills: [{ name: 'greeting_card', version: '1.0.0' }],
			};
			const zipData = await createQuillZip({
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
			});

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(JSON.stringify(manifest));
				}
				if (urlStr.includes('greeting_card@1.0.0.zip')) {
					return new Response(zipData);
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('greeting_card');

			const data = bundle.data as Record<string, Uint8Array>;
			const decoded = new TextDecoder().decode(data['Quill.yaml']);
			expect(decoded).toContain('name: greeting_card');
			expect(decoded).toContain('version: 1.0.0');
		});
	});

	describe('FileSystemSource → packageForHttp → HttpSource → registerQuill', () => {
		it('should produce identical registrations through the full roundtrip', async () => {
			const quillsDir = path.join(FIXTURES_DIR, 'roundtrip-quills');
			const httpDir = path.join(FIXTURES_DIR, 'roundtrip-http');

			await writeQuillFixture(quillsDir, 'greeting_card', '1.0.0', {
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
				'assets/data.json': ASSET_CONTENT,
			});

			// Step 1: Package for HTTP
			const fsSource = new FileSystemSource(quillsDir);
			await fsSource.packageForHttp(httpDir);

			// Step 2: Serve via HttpSource using local files
			const manifestJson = await fs.readFile(
				path.join(httpDir, 'manifest.json'),
				'utf-8',
			);
			const manifest = JSON.parse(manifestJson) as QuillManifest;

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(manifestJson);
				}
				// Extract zip filename from URL
				const zipMatch = urlStr.match(/\/([^/?]+\.zip)/);
				if (zipMatch) {
					const zipPath = path.join(httpDir, zipMatch[1]);
					const zipData = await fs.readFile(zipPath);
					return new Response(zipData);
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			const httpSource = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			// Step 3: Register from FileSystemSource
			const fsEngine = createWasmCompatEngine();
			const fsRegistry = new QuillRegistry({ source: fsSource, engine: fsEngine });
			const fsBundle = await fsRegistry.resolve('greeting_card');

			// Step 4: Register from HttpSource
			const httpEngine = createWasmCompatEngine();
			const httpRegistry = new QuillRegistry({ source: httpSource, engine: httpEngine });
			const httpBundle = await httpRegistry.resolve('greeting_card');

			// Both sources should register the same quill identity
			expect(fsEngine.resolveQuill('greeting_card')).toEqual(
				httpEngine.resolveQuill('greeting_card'),
			);

			// Both bundles should have matching metadata
			expect(fsBundle.name).toBe(httpBundle.name);
			expect(fsBundle.version).toBe(httpBundle.version);

			// Both should have the same file keys
			const fsData = fsBundle.data as Record<string, Uint8Array>;
			const httpData = httpBundle.data as Record<string, Uint8Array>;
			expect(Object.keys(fsData).sort()).toEqual(Object.keys(httpData).sort());

			// File contents should match (comparing decoded text)
			const decoder = new TextDecoder();
			for (const key of Object.keys(fsData)) {
				expect(decoder.decode(httpData[key])).toBe(decoder.decode(fsData[key]));
			}

			// Both should have the same manifest
			expect(manifest.quills).toEqual(
				expect.arrayContaining([
					expect.objectContaining({ name: 'greeting_card', version: '1.0.0' }),
				]),
			);
		});
	});

	describe('engine state consistency across sources', () => {
		it('should correctly track quills registered from different sources', async () => {
			const quillsDir = path.join(FIXTURES_DIR, 'multi-source');
			await writeQuillFixture(quillsDir, 'greeting_card', '1.0.0', {
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
			});

			const reportYaml = 'name: annual_report\nversion: 2.0.0';
			const reportManifest: QuillManifest = {
				quills: [{ name: 'annual_report', version: '2.0.0' }],
			};
			const reportZip = await createQuillZip({
				'Quill.yaml': reportYaml,
				'template.typ': '// Report template',
			});

			const mockFetch = vi.fn(async (url: string | URL | Request) => {
				const urlStr = typeof url === 'string' ? url : url.toString();
				if (urlStr.includes('manifest.json')) {
					return new Response(JSON.stringify(reportManifest));
				}
				if (urlStr.includes('annual_report@2.0.0.zip')) {
					return new Response(reportZip);
				}
				return new Response(null, { status: 404 });
			}) as unknown as typeof globalThis.fetch;

			// Register greeting_card from FileSystemSource
			const fsSource = new FileSystemSource(quillsDir);
			const fsRegistry = new QuillRegistry({ source: fsSource, engine });
			await fsRegistry.resolve('greeting_card');

			// Register annual_report from HttpSource
			const httpSource = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});
			const httpRegistry = new QuillRegistry({ source: httpSource, engine });
			await httpRegistry.resolve('annual_report');

			// Engine should have both quills registered
			expect(engine.registerQuill).toHaveBeenCalledTimes(2);
			expect(engine.listQuills()).toEqual(
				expect.arrayContaining([
					'greeting_card@1.0.0',
					'annual_report@2.0.0',
				]),
			);

			// Both should be resolvable
			expect(engine.resolveQuill('greeting_card')).toEqual({
				name: 'greeting_card',
				version: '1.0.0',
			});
			expect(engine.resolveQuill('annual_report')).toEqual({
				name: 'annual_report',
				version: '2.0.0',
			});
		});

		it('should not re-register a quill already in the engine', async () => {
			const quillsDir = path.join(FIXTURES_DIR, 'no-reregister');
			await writeQuillFixture(quillsDir, 'greeting_card', '1.0.0', {
				'Quill.yaml': QUILL_YAML,
				'template.typ': TEMPLATE_CONTENT,
			});

			const source = new FileSystemSource(quillsDir);
			const registry = new QuillRegistry({ source, engine });

			// First resolve: loads and registers
			await registry.resolve('greeting_card');
			expect(engine.registerQuill).toHaveBeenCalledTimes(1);

			// Second resolve: engine already has it, skips registration
			await registry.resolve('greeting_card');
			expect(engine.registerQuill).toHaveBeenCalledTimes(1);
		});
	});

	describe('registerQuill rejects invalid data', () => {
		it('should fail when Quill.yaml is missing from bundle data', () => {
			const invalidData = {
				'template.typ': new Uint8Array([0x2f, 0x2f]),
			};

			expect(() => engine.registerQuill(invalidData)).toThrow(
				'missing or invalid Quill.yaml',
			);
		});

		it('should fail when Quill.yaml is not a Uint8Array', () => {
			const invalidData = {
				'Quill.yaml': 'name: test\nversion: 1.0.0',
			};

			expect(() => engine.registerQuill(invalidData)).toThrow(
				'missing or invalid Quill.yaml',
			);
		});
	});
});
