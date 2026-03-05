import JSZip from 'jszip';
import type { QuillBundle, QuillManifest, QuillSource } from '../types.js';
import { RegistryError } from '../errors.js';
import { toEngineFileTree } from '../format.js';

export interface HttpSourceOptions {
	/** Base URL serving zips + manifest (e.g., "https://cdn.example.com/quills/"). */
	baseUrl: string;
	/** Optional pre-loaded manifest to skip the initial fetch (for SSR bootstrap). */
	manifest?: QuillManifest;
	/** Optional custom fetch function (for testing or non-browser environments). */
	fetch?: typeof globalThis.fetch;
}

/**
 * QuillSource that fetches quill zips and manifest from any HTTP endpoint.
 *
 * Supports local static serving, CDN hosting, and remote quill registries
 * with the same interface. Appends `?v={version}` to zip URLs for cache-busting.
 *
 * Works in both browser and Node.js environments.
 */
export class HttpSource implements QuillSource {
	private baseUrl: string;
	private preloadedManifest?: QuillManifest;
	private cachedManifest?: QuillManifest;
	private fetchFn: typeof globalThis.fetch;

	constructor(options: HttpSourceOptions) {
		// Ensure baseUrl ends with a slash for consistent URL construction
		this.baseUrl = options.baseUrl.endsWith('/') ? options.baseUrl : options.baseUrl + '/';
		this.preloadedManifest = options.manifest;
		this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
	}

	async getManifest(): Promise<QuillManifest> {
		if (this.preloadedManifest) {
			return this.preloadedManifest;
		}

		if (this.cachedManifest) {
			return this.cachedManifest;
		}

		const url = `${this.baseUrl}manifest.json`;
		let response: Response;
		try {
			response = await this.fetchFn(url);
		} catch (err) {
			throw new RegistryError('source_unavailable', `Failed to fetch manifest from ${url}`, {
				cause: err,
			});
		}

		if (!response.ok) {
			throw new RegistryError(
				'source_unavailable',
				`Failed to fetch manifest: ${response.status} ${response.statusText}`,
			);
		}

		let manifest: QuillManifest;
		try {
			manifest = (await response.json()) as QuillManifest;
		} catch (err) {
			throw new RegistryError('source_unavailable', 'Failed to parse manifest JSON', {
				cause: err,
			});
		}

		this.cachedManifest = manifest;
		return manifest;
	}

	async loadQuill(name: string, version?: string): Promise<QuillBundle> {
		const manifest = await this.getManifest();
		const entry = manifest.quills.find(
			(q) => q.name === name && (version === undefined || q.version === version),
		);

		if (!entry) {
			if (version && manifest.quills.some((q) => q.name === name)) {
				throw new RegistryError(
					'version_not_found',
					`Quill "${name}" exists but version "${version}" was not found`,
					{ quillName: name, version },
				);
			}
			throw new RegistryError('quill_not_found', `Quill "${name}" not found in source`, {
				quillName: name,
				version,
			});
		}

		const resolvedVersion = entry.version;
		const zipFileName = `${name}@${resolvedVersion}.zip`;
		const zipUrl = `${this.baseUrl}${zipFileName}?v=${resolvedVersion}`;

		let response: Response;
		try {
			response = await this.fetchFn(zipUrl);
		} catch (err) {
			throw new RegistryError('load_error', `Failed to fetch quill zip from ${zipUrl}`, {
				quillName: name,
				version: resolvedVersion,
				cause: err,
			});
		}

		if (!response.ok) {
			throw new RegistryError(
				'load_error',
				`Failed to fetch quill zip: ${response.status} ${response.statusText}`,
				{ quillName: name, version: resolvedVersion },
			);
		}

		let files: Record<string, Uint8Array>;
		try {
			const zipData = await response.arrayBuffer();
			const zip = await JSZip.loadAsync(zipData);
			files = {};
			const zipEntries: Promise<void>[] = [];

			zip.forEach((relativePath, zipEntry) => {
				if (!zipEntry.dir) {
					zipEntries.push(
						zipEntry.async('uint8array').then((content) => {
							files[relativePath] = content;
						}),
					);
				}
			});

			await Promise.all(zipEntries);
		} catch (err) {
			throw new RegistryError('load_error', `Failed to unzip quill "${name}"`, {
				quillName: name,
				version: resolvedVersion,
				cause: err,
			});
		}

		return {
			name: entry.name,
			version: resolvedVersion,
			data: toEngineFileTree(files),
			metadata: entry,
		};
	}
}
