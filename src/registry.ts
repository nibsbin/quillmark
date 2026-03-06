import type {
	QuillBundle,
	QuillManifest,
	QuillmarkEngine,
	QuillMetadata,
	QuillSource,
} from './types.js';

export interface QuillRegistryOptions {
	source: QuillSource;
	engine: QuillmarkEngine;
}

/**
 * Orchestrates quill sources, resolves versions, caches loaded quills,
 * and registers them with the engine.
 *
 * The registry is scoped to a specific engine instance. On resolve(), it
 * fetches quill data from the source and registers it with that engine.
 * Loading is lazy — quills are fetched and pushed to the engine on first
 * resolve() call, not at construction time.
 */
export class QuillRegistry {
	private source: QuillSource;
	private engine: QuillmarkEngine;
	/** In-memory cache of resolved QuillBundle objects, keyed by `name@version`. */
	private cache: Map<string, QuillBundle> = new Map();

	constructor(options: QuillRegistryOptions) {
		this.source = options.source;
		this.engine = options.engine;
	}

	/** Returns the manifest from the underlying source. */
	async getManifest(): Promise<QuillManifest> {
		return this.source.getManifest();
	}

	/** Returns metadata for all available quills from the source manifest. */
	async getAvailableQuills(): Promise<QuillMetadata[]> {
		const manifest = await this.source.getManifest();
		return manifest.quills;
	}

	/**
	 * Resolves a quill by name and optional version.
	 *
	 * Resolution flow:
	 * 1. Check engine via resolveQuill() — return immediately if already registered
	 * 2. Check registry cache — return if cached
	 * 3. Ask source for the bundle (or throw version_not_found / quill_not_found)
	 * 4. Register with engine via registerQuill()
	 *
	 * When no version is specified, resolves to latest available.
	 */
	async resolve(name: string, version?: string): Promise<QuillBundle> {
		// 1. Check engine — return immediately if already registered
		const quillRef = version ? `${name}@${version}` : name;
		const engineInfo = this.engine.resolveQuill(quillRef);
		if (engineInfo) {
			const engineVersion = engineInfo.metadata?.version;
			if (typeof engineVersion === 'string') {
				const cacheKey = `${engineInfo.name}@${engineVersion}`;
				const cached = this.cache.get(cacheKey);
				if (cached) {
					return cached;
				}
			}
		}

		// 2. Check registry cache
		if (version) {
			const cacheKey = `${name}@${version}`;
			const cached = this.cache.get(cacheKey);
			if (cached) {
				return cached;
			}
		}

		// 3. Ask source for the bundle
		const bundle = await this.source.loadQuill(name, version);

		// 4. Register with engine
		this.engine.registerQuill(bundle.data);

		// Cache the resolved bundle
		const cacheKey = `${bundle.name}@${bundle.version}`;
		this.cache.set(cacheKey, bundle);

		return bundle;
	}

	/**
	 * Preloads multiple quills. Fail-fast: if any quill fails to load,
	 * rejects immediately. Callers who want best-effort can call resolve()
	 * individually and catch per-quill.
	 * 
	 * Accepts an array of quill names, or objects with `name` and optional `version`.
	 */
	async preload(quills: Array<string | { name: string; version?: string }>): Promise<void> {
		await Promise.all(
			quills.map((q) => {
				if (typeof q === 'string') {
					return this.resolve(q);
				}
				return this.resolve(q.name, q.version);
			})
		);
	}

	/**
	 * Checks whether a quill is currently loaded in the engine.
	 * Delegates to engine.resolveQuill().
	 */
	isLoaded(name: string): boolean {
		return this.engine.resolveQuill(name) !== null;
	}
}
