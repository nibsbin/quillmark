import type {
	QuillBundle,
	QuillManifest,
	QuillmarkEngine,
	QuillMetadata,
	QuillSource,
} from './types.js';
import { RegistryError } from './errors.js';

export interface QuillRegistryOptions {
	source: QuillSource;
	engine?: QuillmarkEngine;
}

function compareSemver(a: string, b: string): number {
	const partsA = a.split('.').map(Number);
	const partsB = b.split('.').map(Number);
	const len = Math.max(partsA.length, partsB.length);

	for (let i = 0; i < len; i++) {
		const numA = partsA[i] ?? 0;
		const numB = partsB[i] ?? 0;
		if (numA !== numB) return numA - numB;
	}

	return 0;
}

function isCanonicalSemver(version: string): boolean {
	return /^\d+\.\d+\.\d+$/.test(version);
}

function matchesSemverSelector(version: string, selector: string): boolean {
	if (selector === version) return true;
	const selectorParts = selector.split('.');
	const versionParts = version.split('.');
	if (
		selectorParts.length === 0 ||
		selectorParts.length > 3 ||
		selectorParts.some((p) => p.length === 0 || Number.isNaN(Number(p)))
	) {
		return false;
	}
	if (selectorParts.length > versionParts.length) return false;
	for (let i = 0; i < selectorParts.length; i++) {
		if (selectorParts[i] !== versionParts[i]) return false;
	}
	return true;
}

function chooseHighestVersion(versions: string[]): string | null {
	if (versions.length === 0) return null;
	const copy = [...versions];
	copy.sort((a, b) => compareSemver(b, a));
	return copy[0] ?? null;
}

function isCanonicalRef(ref: string): boolean {
	const [name, version, ...rest] = ref.split('@');
	return Boolean(name && version && rest.length === 0 && isCanonicalSemver(version));
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
	private engine: QuillmarkEngine | null;
	private manifestPromise: Promise<QuillManifest>;
	/**
	 * In-memory cache of in-flight and settled fetch operations.
	 * Keyed by quill ref (`name` or `name@version`).
	 */
	private fetched: Map<string, Promise<QuillBundle>> = new Map();
	/**
	 * In-memory cache of in-flight and settled resolve operations.
	 * Keyed by quill ref (`name` or `name@version`).
	 */
	private resolving: Map<string, Promise<QuillBundle>> = new Map();
	/** Coalesces registration to avoid duplicate registerQuill() races. */
	private registering: Map<string, Promise<void>> = new Map();

	constructor(options: QuillRegistryOptions) {
		this.source = options.source;
		this.engine = options.engine ?? null;
		// Eagerly load the manifest at startup so resolve() can assume availability.
		this.manifestPromise = this.source.getManifest();
		// Prevent unhandled rejection noise if consumers never await manifest-dependent APIs.
		void this.manifestPromise.catch(() => undefined);
	}

	/** Attaches or replaces the engine instance used by resolve(). */
	setEngine(engine: QuillmarkEngine): void {
		this.engine = engine;
	}

	/** Returns the manifest from the underlying source. */
	async getManifest(): Promise<QuillManifest> {
		return this.manifestPromise;
	}

	/** Returns metadata for all available quills from the source manifest. */
	async getAvailableQuills(): Promise<QuillMetadata[]> {
		const manifest = await this.source.getManifest();
		return manifest.quills;
	}

	/**
	 * Fetches a quill by canonical ref (`name@version`) without registering it.
	 * Intended for loading in parallel before engine initialization.
	 */
	async fetch(canonicalRef: string): Promise<QuillBundle> {
		const [name, version, ...rest] = canonicalRef.split('@');
		if (!name || !version || rest.length > 0 || !isCanonicalSemver(version)) {
			throw new Error(
				`fetch() requires a canonical ref in the form "name@version". Received "${canonicalRef}"`,
			);
		}

		const cacheKey = `${name}@${version}`;
		const cachedPromise = this.fetched.get(cacheKey);
		if (cachedPromise) return cachedPromise;

		const fetchPromise = this.source.loadQuill(name, version).then((bundle) => {
			const resolvedKey = `${bundle.name}@${bundle.version}`;
			this.fetched.set(resolvedKey, fetchPromise);
			this.fetched.set(canonicalRef, fetchPromise);
			return bundle;
		}).catch((error) => {
			this.fetched.delete(cacheKey);
			this.fetched.delete(canonicalRef);
			throw error;
		});

		this.fetched.set(cacheKey, fetchPromise);
		if (canonicalRef !== cacheKey) {
			this.fetched.set(canonicalRef, fetchPromise);
		}
		return fetchPromise;
	}

	/**
	 * Resolves a quill by reference (e.g., `name@version` or `name`) and
	 * ensures it is registered with the attached engine.
	 *
	 * Resolution flow:
	 * 1. Check resolve cache — return if cached
	 * 2. Fetch bundle (or reuse fetch cache)
	 * 3. Ask source for the bundle (or throw version_not_found / quill_not_found)
	 * 4. Register with engine via registerQuill() (coalesced by canonical ref)
	 *
	 * When no version is specified, resolves to latest available.
	 */
	async resolve(ref: string): Promise<QuillBundle> {
		if (!this.engine) {
			throw new Error(
				'resolve() requires an attached engine. Provide one in constructor or call setEngine().',
			);
		}

		const canonicalInput = isCanonicalRef(ref);
		const inFlight = this.resolving.get(ref);
		if (inFlight) {
			return inFlight;
		}

		const resolvePromise = this.fetchForResolve(ref)
			.then(async (bundle) => {
				await this.ensureRegistered(bundle);
				const resolvedKey = `${bundle.name}@${bundle.version}`;
				this.resolving.set(resolvedKey, resolvePromise);
				return bundle;
			})
			.finally(() => {
				// Keep canonical refs cached, but do not pin selector/name refs.
				if (!canonicalInput) {
					this.resolving.delete(ref);
				}
			});

		this.resolving.set(ref, resolvePromise);
		return resolvePromise;
	}

	/**
	 * Checks whether a quill is currently loaded in the engine.
	 * Delegates to engine.resolveQuill().
	 */
	isLoaded(name: string): boolean {
		return this.engine?.resolveQuill(name) !== null;
	}

	private async fetchForResolve(ref: string): Promise<QuillBundle> {
		if (isCanonicalRef(ref)) {
			const cached = this.fetched.get(ref);
			if (cached) return cached;
		}

		const [name, version] = ref.split('@');
		if (version) {
			if (!isCanonicalSemver(version)) {
				const selector = version;
				const candidateVersionsFromCache: string[] = [];
				for (const key of this.fetched.keys()) {
					if (!key.startsWith(`${name}@`)) continue;
					const candidateVersion = key.slice(name.length + 1);
					if (matchesSemverSelector(candidateVersion, selector)) {
						candidateVersionsFromCache.push(candidateVersion);
					}
				}

				const manifest = await this.getManifest();
				const manifestByName = manifest.quills.filter((q) => q.name === name);
				const candidateVersionsFromManifest = manifestByName
					.filter((q) => matchesSemverSelector(q.version, selector))
					.map((q) => q.version);

				const candidateVersions = [
					...new Set([...candidateVersionsFromCache, ...candidateVersionsFromManifest]),
				];
				if (candidateVersions.length === 0) {
					const hasQuillInCache = [...this.fetched.keys()].some((k) => k.startsWith(`${name}@`));
					const hasQuillInManifest = manifestByName.length > 0;
					if (!hasQuillInCache && !hasQuillInManifest) {
						throw new RegistryError('quill_not_found', `Quill "${name}" not found in source`, {
							quillName: name,
							version: selector,
						});
					}
					throw new RegistryError(
						'version_not_found',
						`Quill "${name}" exists but version selector "${selector}" was not found`,
						{ quillName: name, version: selector },
					);
				}
				const canonicalVersion = chooseHighestVersion(candidateVersions)!;
				const canonicalRef = `${name}@${canonicalVersion}`;
				return this.fetch(canonicalRef);
			}
			return this.fetch(`${name}@${version}`);
		}

		const manifest = await this.getManifest();
		const manifestByName = manifest.quills.filter((q) => q.name === name).map((q) => q.version);
		const cachedVersionsByName = [...this.fetched.keys()]
			.filter((k) => k.startsWith(`${name}@`))
			.map((k) => k.slice(name.length + 1));
		const latestVersion = chooseHighestVersion([...new Set([...manifestByName, ...cachedVersionsByName])]);
		if (!latestVersion) {
			throw new RegistryError('quill_not_found', `Quill "${name}" not found in source`, {
				quillName: name,
			});
		}
		return this.fetch(`${name}@${latestVersion}`);
	}

	private async ensureRegistered(bundle: QuillBundle): Promise<void> {
		if (!this.engine) {
			throw new Error('resolve() requires an attached engine. Provide one in constructor or call setEngine().');
		}

		const canonical = `${bundle.name}@${bundle.version}`;
		const existing = this.registering.get(canonical);
		if (existing) {
			return existing;
		}

		const registerPromise = Promise.resolve()
			.then(() => {
				const exactInfo = this.engine!.resolveQuill(canonical);
				const byNameInfo = this.engine!.resolveQuill(bundle.name);
				const info = exactInfo ?? byNameInfo;
				const existingVersion = info?.metadata?.version;
				if (typeof existingVersion === 'string' && existingVersion === bundle.version) {
					return;
				}
				this.engine!.registerQuill(bundle.data);
			})
			.finally(() => {
				this.registering.delete(canonical);
			});
		this.registering.set(canonical, registerPromise);
		return registerPromise;
	}
}
