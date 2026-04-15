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
		data: {
			files: {
				'Quill.yaml': {
					contents: `Quill:\n  name: ${name}\n  version: ${version}\n  backend: typst\n  plate_file: plate.typ\n`,
				},
			},
		},
		metadata: { name, version },
	};
}

function createMockSource(bundles: QuillBundle[]): QuillSource {
	const manifest: QuillManifest = {
		quills: bundles.map((b) => ({ name: b.name, version: b.version })),
	};
	return {
		getManifest: vi.fn(async () => manifest),
		loadQuill: vi.fn(async (name: string, version: string) => {
			const bundle = bundles.find((b) => b.name === name && b.version === version);
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
			// Walk the tree to find Quill.yaml contents
			const tree = data as { files?: Record<string, unknown> };
			const yamlNode = tree?.files?.['Quill.yaml'] as { contents?: string } | undefined;
			const yamlContent = yamlNode?.contents ?? '';
			const nameMatch = yamlContent.match(/name:\s*(\S+)/);
			const versionMatch = yamlContent.match(/version:\s*"?(\S+)"?/);
			const name = nameMatch?.[1] ?? 'unknown';
			const version = versionMatch?.[1] ?? '0.0.0';
			registered.set(`${name}@${version}`, { name, version });
			return {
				name,
				backend: 'typst',
				metadata: { version },
				schema: '',
				defaults: {},
				examples: {},
				supportedFormats: ['pdf'],
			};
		}),
		resolveQuill: vi.fn((ref: string) => {
			// Check exact ref first, then name-only
			if (registered.has(ref)) {
				const info = registered.get(ref)!;
				return {
					name: info.name,
					backend: 'typst',
					metadata: { version: info.version },
					schema: '',
					defaults: {},
					examples: {},
					supportedFormats: ['pdf'],
				};
			}
			// If ref doesn't contain @, search by name
			if (!ref.includes('@')) {
				for (const [, info] of registered.entries()) {
					if (info.name === ref) {
						return {
							name: info.name,
							backend: 'typst',
							metadata: { version: info.version },
							schema: '',
							defaults: {},
							examples: {},
							supportedFormats: ['pdf'],
						};
					}
				}
			}
			return null;
		}),
		listQuills: vi.fn(() => [...registered.keys()]),
	} as unknown as QuillmarkEngine;
}

function createMockEngineExactRefOnly(): QuillmarkEngine {
	const registered = new Map<string, { name: string; version: string }>();

	return {
		registerQuill: vi.fn((data: unknown) => {
			const tree = data as { files?: Record<string, unknown> };
			const yamlNode = tree?.files?.['Quill.yaml'] as { contents?: string } | undefined;
			const yamlContent = yamlNode?.contents ?? '';
			const nameMatch = yamlContent.match(/name:\s*(\S+)/);
			const versionMatch = yamlContent.match(/version:\s*"?(\S+)"?/);
			const name = nameMatch?.[1] ?? 'unknown';
			const version = versionMatch?.[1] ?? '0.0.0';
			registered.set(`${name}@${version}`, { name, version });
			return {
				name,
				backend: 'typst',
				metadata: { version },
				schema: '',
				defaults: {},
				examples: {},
				supportedFormats: ['pdf'],
			};
		}),
		resolveQuill: vi.fn((ref: string) => {
			const info = registered.get(ref);
			if (!info) return null;
			return {
				name: info.name,
				backend: 'typst',
				metadata: { version: info.version },
				schema: '',
				defaults: {},
				examples: {},
				supportedFormats: ['pdf'],
			};
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
			expect(manifest).toEqual({
				quills: [
					{ name: 'usaf_memo', version: '1.0.0' },
					{ name: 'classic_resume', version: '2.1.0' },
				],
			});
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
			expect(source.loadQuill).toHaveBeenCalledWith('usaf_memo', '1.0.0');
			expect(engine.registerQuill).toHaveBeenCalledWith(bundle.data);
		});

		it('should resolve with specific version', async () => {
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.resolve('usaf_memo@1.0.0');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
			expect(source.loadQuill).toHaveBeenCalledWith('usaf_memo', '1.0.0');
		});

		it('should resolve semver selector with missing segments to highest matching version', async () => {
			bundles = [
				createMockBundle('usaf_memo', '1.0.0'),
				createMockBundle('usaf_memo', '1.2.0'),
				createMockBundle('usaf_memo', '2.0.0'),
			];
			source = createMockSource(bundles);
			const registry = new QuillRegistry({ source, engine });

			const bundleMajor = await registry.resolve('usaf_memo@1');
			expect(bundleMajor.version).toBe('1.2.0');

			const bundleMinor = await registry.resolve('usaf_memo@1.0');
			expect(bundleMinor.version).toBe('1.0.0');
		});

		it('should throw version_not_found when semver selector has no matches', async () => {
			const registry = new QuillRegistry({ source, engine });
			await expect(registry.resolve('usaf_memo@3')).rejects.toThrow(RegistryError);
		});

		it('should throw quill_not_found when semver selector targets missing quill name', async () => {
			const registry = new QuillRegistry({ source, engine });
			await expect(registry.resolve('nonexistent@1')).rejects.toMatchObject({
				code: 'quill_not_found',
			});
		});

		it('should check engine before hitting source', async () => {
			const registry = new QuillRegistry({ source, engine });

			// First resolve: hits source and registers
			await registry.resolve('usaf_memo');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
			expect(source.getManifest).toHaveBeenCalledTimes(1);

			// Second resolve: re-evaluates latest from startup-loaded manifest,
			// reuses canonical fetch without reloading manifest.
			await registry.resolve('usaf_memo');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
			expect(source.getManifest).toHaveBeenCalledTimes(1);
		});

		it('should use registry cache for versioned lookups', async () => {
			const registry = new QuillRegistry({ source, engine });

			// First resolve
			await registry.resolve('usaf_memo@1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);

			// Reset the engine mock to not find it (test cache path specifically)
			vi.mocked(engine.resolveQuill).mockReturnValue(null);

			// Second resolve with same version: hits registry cache
			await registry.resolve('usaf_memo@1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
		});

		it('should reuse cached versioned bundle for unversioned resolve after fetch', async () => {
			const exactOnlyEngine = createMockEngineExactRefOnly();
			const registry = new QuillRegistry({ source, engine: exactOnlyEngine });

			await registry.fetch('usaf_memo@1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
			expect(exactOnlyEngine.registerQuill).toHaveBeenCalledTimes(0);

			const bundle = await registry.resolve('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
			expect(exactOnlyEngine.registerQuill).toHaveBeenCalledTimes(1);
		});

		it('should pick highest cached version for unversioned resolve', async () => {
			bundles = [
				createMockBundle('usaf_memo', '1.0.0'),
				createMockBundle('usaf_memo', '2.0.0'),
			];
			source = createMockSource(bundles);
			const exactOnlyEngine = createMockEngineExactRefOnly();
			const registry = new QuillRegistry({ source, engine: exactOnlyEngine });

			await registry.resolve('usaf_memo@1.0.0');
			await registry.resolve('usaf_memo@2.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(2);

			const bundle = await registry.resolve('usaf_memo');
			expect(bundle.version).toBe('2.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(2);
			expect(exactOnlyEngine.registerQuill).toHaveBeenCalledTimes(2);
		});

		it('should coalesce concurrent resolve() calls for the same ref', async () => {
			const registry = new QuillRegistry({ source, engine });
			const deferred = Promise.withResolvers<QuillBundle>();
			vi.mocked(source.loadQuill).mockReturnValueOnce(deferred.promise);

			const first = registry.resolve('usaf_memo@1.0.0');
			const second = registry.resolve('usaf_memo@1.0.0');

			expect(source.loadQuill).toHaveBeenCalledTimes(1);

			deferred.resolve(createMockBundle('usaf_memo', '1.0.0'));
			const [firstResolved, secondResolved] = await Promise.all([first, second]);
			expect(firstResolved).toBe(secondResolved);
			expect(firstResolved).toMatchObject({ name: 'usaf_memo', version: '1.0.0' });
			expect(engine.registerQuill).toHaveBeenCalledTimes(1);
		});

		it('should resolve a prefetched bundle after engine is attached later', async () => {
			const registry = new QuillRegistry({ source });
			await registry.fetch('usaf_memo@1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);

			registry.setEngine(engine);
			const bundle = await registry.resolve('usaf_memo@1.0.0');
			expect(bundle.version).toBe('1.0.0');
			expect(source.loadQuill).toHaveBeenCalledTimes(1);
			expect(engine.registerQuill).toHaveBeenCalledTimes(1);
		});

		it('should fail fast when resolve is called without an engine', async () => {
			const registry = new QuillRegistry({ source });
			await expect(registry.resolve('usaf_memo')).rejects.toThrow(
				'resolve() requires an attached engine',
			);
			expect(source.loadQuill).not.toHaveBeenCalled();
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
				await registry.resolve('usaf_memo@9.9.9');
				expect.unreachable('Should have thrown');
			} catch (err) {
				expect(err).toBeInstanceOf(RegistryError);
				expect((err as RegistryError).code).toBe('version_not_found');
			}
		});
	});

	describe('fetch()', () => {
		it('should fetch a canonical quill ref without registering', async () => {
			const registry = new QuillRegistry({ source, engine });
			const bundle = await registry.fetch('usaf_memo@1.0.0');

			expect(bundle.name).toBe('usaf_memo');
			expect(bundle.version).toBe('1.0.0');
			expect(source.loadQuill).toHaveBeenCalledWith('usaf_memo', '1.0.0');
			expect(engine.registerQuill).toHaveBeenCalledTimes(0);
		});

		it('should reject non-canonical refs', async () => {
			const registry = new QuillRegistry({ source, engine });
			await expect(registry.fetch('usaf_memo')).rejects.toThrow(
				'fetch() requires a canonical ref',
			);
		});

		it('should reject non-canonical semver selectors', async () => {
			const registry = new QuillRegistry({ source, engine });
			await expect(registry.fetch('usaf_memo@1')).rejects.toThrow(
				'fetch() requires a canonical ref',
			);
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
