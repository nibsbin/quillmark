import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QuillRegistry } from '../registry.js';
import { RegistryError } from '../errors.js';
import type { QuillBundle, QuillManifest, QuillmarkEngine, QuillSource } from '../types.js';

const MANIFEST: QuillManifest = {
	quills: [
		{ name: 'usaf_memo', version: '1.0.0', description: 'USAF Memo' },
		{ name: 'classic_resume', version: '2.1.0' },
	],
};

function createMockBundle(name: string, version: string): QuillBundle {
	return {
		name,
		version,
		data: { files: { 'Quill.yaml': `name: ${name}\nversion: ${version}` } },
		metadata: { name, version },
	};
}

function createMockSource(bundles: QuillBundle[]): QuillSource {
	return {
		getManifest: vi.fn(async () => MANIFEST),
		loadQuill: vi.fn(async (name: string, version?: string) => {
			const bundle = bundles.find(
				(b) => b.name === name && (version === undefined || b.version === version),
			);
			if (!bundle) {
				if (version && bundles.some((b) => b.name === name)) {
					throw new RegistryError('version_not_found', `Version ${version} not found`, {
						quillName: name,
						version,
					});
				}
				throw new RegistryError('quill_not_found', `Quill ${name} not found`, {
					quillName: name,
				});
			}
			return bundle;
		}),
	};
}

function createMockEngine(): QuillmarkEngine {
	const registered = new Map<string, { name: string; version: string }>();

	return {
		registerQuill: vi.fn((data: unknown) => {
			// Simulate engine behavior: extract name/version from data
			const bundle = data as Record<string, unknown>;
			const files = bundle.files as Record<string, string>;
			const yamlContent = files?.['Quill.yaml'] ?? '';
			const nameMatch = yamlContent.match(/name:\s*(\S+)/);
			const versionMatch = yamlContent.match(/version:\s*(\S+)/);
			const name = nameMatch?.[1] ?? 'unknown';
			const version = versionMatch?.[1] ?? '0.0.0';
			const info = { name, version };
			registered.set(`${name}@${version}`, info);
			return info;
		}),
		resolveQuill: vi.fn((ref: string) => {
			// Check exact ref first, then name-only
			if (registered.has(ref)) {
				return registered.get(ref)!;
			}
			// If ref doesn't contain @, search by name
			if (!ref.includes('@')) {
				for (const [key, info] of registered.entries()) {
					if (info.name === ref) return info;
				}
			}
			return null;
		}),
		listQuills: vi.fn(() => [...registered.keys()]),
	} as unknown as QuillmarkEngine;
}

describe('QuillRegistry', () => {
	let source: QuillSource;
	let engine: QuillmarkEngine;
	let bundles: QuillBundle[];

	beforeEach(() => {
		bundles = [
			createMockBundle('usaf_memo', '1.0.0'),
			createMockBundle('classic_resume', '2.1.0'),
		];
		source = createMockSource(bundles);
		engine = createMockEngine();
	});

	describe('getManifest()', () => {
		it('should delegate to source', async () => {
			const registry = new QuillRegistry({ source, engine });
			const manifest = await registry.getManifest();
			expect(manifest).toEqual(MANIFEST);
			expect(source.getManifest).toHaveBeenCalledOnce();
		});
	});

	describe('getAvailableQuills()', () => {
		it('should return metadata from manifest', async () => {
			const registry = new QuillRegistry({ source, engine });
			const quills = await registry.getAvailableQuills();
			expect(quills).toHaveLength(2);
			expect(quills[0].name).toBe('usaf_memo');
			expect(quills[1].name).toBe('classic_resume');
		});
	});

	describe('resolve()', () => {
		it('should load quill from source and register with engine', async () => {
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('usaf_memo');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
			expect(source.loadQuill).toHaveBeenCalledWith('usaf_memo', undefined);
			expect(engine.registerQuill).toHaveBeenCalledWith(bundle.data);
		});

		it('should resolve with specific version', async () => {
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('usaf_memo', '1.0.0');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
			expect(source.loadQuill).toHaveBeenCalledWith('usaf_memo', '1.0.0');
		});

		it('should check engine before hitting source', async () => {
			const registry = new QuillRegistry({ source, engine });

			// First resolve: hits source and registers
			await registry.resolve('usaf_memo');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);

			// Second resolve: engine already has it, returns from cache
			await registry.resolve('usaf_memo');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
		});

		it('should use registry cache for versioned lookups', async () => {
			const registry = new QuillRegistry({ source, engine });

			// First resolve
			await registry.resolve('usaf_memo', '1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);

			// Reset the engine mock to not find it (test cache path specifically)
			vi.mocked(engine.resolveQuill).mockReturnValue(null);

			// Second resolve with same version: hits registry cache
			await registry.resolve('usaf_memo', '1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
		});

		it('should throw quill_not_found for unknown quill', async () => {
			const registry = new QuillRegistry({ source, engine });

			try {
				await registry.resolve('nonexistent');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('quill_not_found');
			}
		});

		it('should throw version_not_found for wrong version', async () => {
			const registry = new QuillRegistry({ source, engine });

			try {
				await registry.resolve('usaf_memo', '9.9.9');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('version_not_found');
			}
		});
	});

	describe('preload()', () => {
		it('should resolve all named quills', async () => {
			const registry = new QuillRegistry({ source, engine });
			await registry.preload(['usaf_memo', 'classic_resume']);

			expect(source.loadQuill).toHaveBeenCalledTimes(2);
			expect(engine.registerQuill).toHaveBeenCalledTimes(2);
		});

		it('should fail-fast if any quill fails', async () => {
			const registry = new QuillRegistry({ source, engine });

			await expect(
				registry.preload(['usaf_memo', 'nonexistent', 'classic_resume']),
			).rejects.toThrow(RegistryError);
		});
	});

	describe('isLoaded()', () => {
		it('should return false for unloaded quill', () => {
			const registry = new QuillRegistry({ source, engine });
			expect(registry.isLoaded('usaf_memo')).toBe(false);
		});

		it('should return true after resolve()', async () => {
			const registry = new QuillRegistry({ source, engine });
			await registry.resolve('usaf_memo');
			expect(registry.isLoaded('usaf_memo')).toBe(true);
		});

		it('should return false for different quill', async () => {
			const registry = new QuillRegistry({ source, engine });
			await registry.resolve('usaf_memo');
			expect(registry.isLoaded('classic_resume')).toBe(false);
		});
	});
});
