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
	/**
	 * In-memory cache of in-flight and settled resolve operations.
	 * Keyed by quill ref (`name` or `name@version`).
	 */
	private inflight: Map<string, Promise<QuillBundle>> = new Map();

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
	 * Resolves a quill by a reference string (e.g., `name@version` or just `name`).
	 *
	 * Resolution flow:
	 * 1. Check engine via resolveQuill() — return immediately if already registered
	 * 2. Check registry cache — return if cached
	 * 3. Ask source for the bundle (or throw version_not_found / quill_not_found)
	 * 4. Register with engine via registerQuill()
	 *
	 * When no version is specified, resolves to latest available.
	 */
	async resolve(ref: string): Promise<QuillBundle> {
		const cachedPromise = this.inflight.get(ref);
		if (cachedPromise) {
			return cachedPromise;
		}

		// Parse ref into name and optional version
		const [name, version] = ref.split('@');

		// 1. Check engine — return immediately if already registered
		const engineInfo = this.engine.resolveQuill(ref);
		if (engineInfo) {
			const engineVersion = engineInfo.metadata?.version;
			if (typeof engineVersion === 'string') {
				const cacheKey = `${engineInfo.name}@${engineVersion}`;
				const cached = this.inflight.get(cacheKey);
				if (cached) {
					this.inflight.set(ref, cached);
					return cached;
				}
			}
		}

		// 2. Check registry cache (only for explicit versioned lookups)
		if (version) {
			const cacheKey = `${name}@${version}`;
			const cached = this.inflight.get(cacheKey);
			if (cached) {
				this.inflight.set(ref, cached);
				return cached;
			}
		}

		const resolvePromise = this.source
			.loadQuill(name, version)
			.then((bundle) => {
				this.engine.registerQuill(bundle.data);
				const resolvedKey = `${bundle.name}@${bundle.version}`;
				this.inflight.set(resolvedKey, resolvePromise);
				this.inflight.set(ref, resolvePromise);
				return bundle;
			})
			.catch((error) => {
				this.inflight.delete(ref);
				throw error;
			});

		this.inflight.set(ref, resolvePromise);
		return resolvePromise;
	}

	/**
	 * Preloads multiple quills using reference strings (e.g., `name@version`).
	 * Fail-fast: if any quill fails to load, rejects immediately.
	 */
	async preload(refs: string[]): Promise<void> {
		await Promise.all(refs.map((ref) => this.resolve(ref)));
	}

	/**
	 * Checks whether a quill is currently loaded in the engine.
	 * Delegates to engine.resolveQuill().
	 */
	isLoaded(name: string): boolean {
		return this.engine.resolveQuill(name) !== null;
	}
}
