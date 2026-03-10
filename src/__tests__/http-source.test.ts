import { describe, it, expect, vi } from 'vitest';
import { zipSync, strToU8 } from 'fflate';
import { HttpSource } from '../sources/http-source.js';
import { RegistryError } from '../errors.js';
import type { QuillManifest } from '../types.js';

const MANIFEST: QuillManifest = {
	quills: [
		{ name: 'usaf_memo', version: '1.0.0', description: 'USAF Memo' },
		{ name: 'classic_resume', version: '2.1.0' },
	],
};

/** Creates a mock zip containing test files. */
function createMockZip(): ArrayBuffer {
	const data = zipSync({
		'Quill.yaml': strToU8('name: usaf_memo\nversion: 1.0.0'),
		'template.typ': strToU8('// Template'),
	});
	return data.buffer as ArrayBuffer;
}

/** Creates a mock fetch function with programmable responses. */
function createMockFetch(
	responses: Record<string, { ok: boolean; status?: number; body?: unknown }>,
) {
	return vi.fn(async (url: string | URL | Request) => {
		const urlStr = typeof url === 'string' ? url : url.toString();
		for (const [pattern, config] of Object.entries(responses)) {
			if (urlStr.includes(pattern)) {
				if (!config.ok) {
					return new Response(null, {
						status: config.status ?? 500,
						statusText: 'Error',
					});
				}
				if (config.body instanceof ArrayBuffer) {
					return new Response(config.body);
				}
				return new Response(JSON.stringify(config.body));
			}
		}
		return new Response(null, { status: 404, statusText: 'Not Found' });
	}) as unknown as typeof globalThis.fetch;
}

describe('HttpSource', () => {
	describe('getManifest()', () => {
		it('should fetch manifest from baseUrl', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills',
				fetch: mockFetch,
			});

			const manifest = await source.getManifest();
			expect(manifest.quills).toHaveLength(2);
			expect(mockFetch).toHaveBeenCalledWith('https://cdn.example.com/quills/manifest.json');
		});

		it('should use pre-loaded manifest when provided', async () => {
			const mockFetch = vi.fn() as unknown as typeof globalThis.fetch;
			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				manifest: MANIFEST,
				fetch: mockFetch,
			});

			const manifest = await source.getManifest();
			expect(manifest.quills).toHaveLength(2);
			expect(mockFetch).not.toHaveBeenCalled();
		});

		it('should cache fetched manifest', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			await source.getManifest();
			await source.getManifest();
			expect(mockFetch).toHaveBeenCalledTimes(1);
		});

		it('should throw source_unavailable on network error', async () => {
			const mockFetch = vi.fn().mockRejectedValue(new Error('Network error')) as unknown as typeof globalThis.fetch;

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			try {
				await source.getManifest();
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('source_unavailable');
			}
		});

		it('should throw source_unavailable on non-ok response', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: false, status: 404 },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			try {
				await source.getManifest();
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('source_unavailable');
			}
		});
	});

	describe('loadQuill()', () => {
		it('should fetch and unzip a quill', async () => {
			const zipData = createMockZip();
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
				'usaf_memo@1.0.0.zip': { ok: true, body: zipData },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			const bundle = await source.loadQuill('usaf_memo');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
			expect(bundle.metadata.name).toBe('usaf_memo');
			expect(bundle.metadata.description).toBe('USAF Memo');

			const data = bundle.data as { files: Record<string, unknown> };
			expect(data.files['Quill.yaml']).toBeDefined();
			expect(data.files['template.typ']).toBeDefined();
		});

		it('should append ?v={version} for cache-busting', async () => {
			const zipData = createMockZip();
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
				'usaf_memo@1.0.0.zip': { ok: true, body: zipData },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			await source.loadQuill('usaf_memo', '1.0.0');
			expect(mockFetch).toHaveBeenCalledWith(
				'https://cdn.example.com/quills/usaf_memo@1.0.0.zip?v=1.0.0',
			);
		});

		it('should throw quill_not_found for unknown quill', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			try {
				await source.loadQuill('nonexistent');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('quill_not_found');
			}
		});

		it('should throw version_not_found when quill exists but version does not', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			try {
				await source.loadQuill('usaf_memo', '9.9.9');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('version_not_found');
				expect((err as RegistryError).quillName).toBe('usaf_memo');
				expect((err as RegistryError).version).toBe('9.9.9');
			}
		});

		it('should throw load_error on fetch failure for zip', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
				'usaf_memo@1.0.0.zip': { ok: false, status: 500 },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			try {
				await source.loadQuill('usaf_memo', '1.0.0');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('load_error');
			}
		});
	});

	describe('URL normalization', () => {
		it('should add trailing slash to baseUrl if missing', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills',
				fetch: mockFetch,
			});

			await source.getManifest();
			expect(mockFetch).toHaveBeenCalledWith('https://cdn.example.com/quills/manifest.json');
		});

		it('should not double trailing slash', async () => {
			const mockFetch = createMockFetch({
				'manifest.json': { ok: true, body: MANIFEST },
			});

			const source = new HttpSource({
				baseUrl: 'https://cdn.example.com/quills/',
				fetch: mockFetch,
			});

			await source.getManifest();
			expect(mockFetch).toHaveBeenCalledWith('https://cdn.example.com/quills/manifest.json');
		});
	});
});
