import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import JSZip from 'jszip';
import type { QuillBundle, QuillManifest, QuillMetadata, QuillSource } from '../types.js';
import { RegistryError } from '../errors.js';

/** Reads files from a directory recursively, returning a map of relative paths to contents. */
async function readDirRecursive(
	dirPath: string,
	basePath: string = dirPath,
): Promise<Record<string, Uint8Array>> {
	const files: Record<string, Uint8Array> = {};
	const entries = await fs.readdir(dirPath, { withFileTypes: true });

	for (const entry of entries) {
		const fullPath = path.join(dirPath, entry.name);
		const relativePath = path.relative(basePath, fullPath);

		if (entry.isDirectory()) {
			const subFiles = await readDirRecursive(fullPath, basePath);
			Object.assign(files, subFiles);
		} else if (entry.isFile()) {
			files[relativePath] = new Uint8Array(await fs.readFile(fullPath));
		}
	}

	return files;
}

/**
 * Verifies that a Quill.yaml file exists in the given quill directory.
 * Name and version are derived from the directory structure; Quill.yaml
 * content is parsed by the @quillmark/wasm engine at registration time.
 */
async function assertQuillYamlExists(quillDir: string): Promise<void> {
	const yamlPath = path.join(quillDir, 'Quill.yaml');
	try {
		await fs.access(yamlPath);
	} catch {
		throw new RegistryError('load_error', `Missing Quill.yaml in ${quillDir}`);
	}
}

/** Lists subdirectories of a given directory. Filters out dot-prefixed entries. */
async function listSubdirectories(dirPath: string): Promise<string[]> {
	const entries = await fs.readdir(dirPath, { withFileTypes: true });
	return entries
		.filter((e) => e.isDirectory() && !e.name.startsWith('.'))
		.map((e) => e.name);
}

/** Returns true if the string looks like a semver version (digits and dots only). */
function isSemver(value: string): boolean {
	return /^\d+\.\d+\.\d+/.test(value);
}

/**
 * Compares two semver version strings. Returns a negative number if a < b,
 * zero if equal, positive if a > b. Handles versions with any number of
 * numeric segments (e.g., "1.0.0", "0.1", "2.1.0").
 */
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

/**
 * Node.js-only QuillSource that reads Quill directories from the local filesystem.
 *
 * Expects a versioned directory layout:
 *
 * ```
 * quillsDir/
 *   usaf_memo/
 *     0.1.0/
 *       Quill.yaml
 *       template.typ
 *     1.0.0/
 *       Quill.yaml
 *       template.typ
 *   classic_resume/
 *     2.1.0/
 *       Quill.yaml
 *       template.typ
 * ```
 *
 * Each version directory must contain a `Quill.yaml` file. Name and version are
 * derived from the directory structure; Quill.yaml content is validated by the
 * @quillmark/wasm engine at registration time.
 *
 * Also exposes `packageForHttp(outputDir)` to zip quills and write a manifest
 * for static hosting.
 */
export class FileSystemSource implements QuillSource {
	private quillsDir: string;

	constructor(quillsDir: string) {
		this.quillsDir = quillsDir;
	}

	async getManifest(): Promise<QuillManifest> {
		let quillNames: string[];
		try {
			quillNames = await listSubdirectories(this.quillsDir);
		} catch (err) {
			throw new RegistryError(
				'source_unavailable',
				`Failed to read quills directory: ${this.quillsDir}`,
				{ cause: err },
			);
		}

		const quills: QuillMetadata[] = [];
		for (const quillName of quillNames) {
			const quillNameDir = path.join(this.quillsDir, quillName);
			let versionDirs: string[];
			try {
				versionDirs = await listSubdirectories(quillNameDir);
			} catch {
				// Skip entries that aren't readable directories
				continue;
			}

			for (const versionDir of versionDirs) {
				if (!isSemver(versionDir)) continue;
				const versionPath = path.join(quillNameDir, versionDir);
				try {
					await assertQuillYamlExists(versionPath);
					quills.push({ name: quillName, version: versionDir });
				} catch (err) {
					if (err instanceof RegistryError) throw err;
					// Skip directories without valid Quill.yaml
				}
			}
		}

		return { quills };
	}

	async loadQuill(name: string, version?: string): Promise<QuillBundle> {
		// If no version specified, resolve to latest
		const resolvedVersion = version ?? (await this.resolveLatestVersion(name));

		const quillDir = path.join(this.quillsDir, name, resolvedVersion);

		// Verify directory exists
		try {
			await fs.access(quillDir);
		} catch {
			// Check if the quill name exists at all to give a better error
			const nameDir = path.join(this.quillsDir, name);
			try {
				await fs.access(nameDir);
				// Name exists but version doesn't
				throw new RegistryError(
					'version_not_found',
					`Quill "${name}" exists but version "${resolvedVersion}" was not found`,
					{ quillName: name, version: resolvedVersion },
				);
			} catch (err) {
				if (err instanceof RegistryError) throw err;
				throw new RegistryError('quill_not_found', `Quill "${name}" not found in source`, {
					quillName: name,
					version: resolvedVersion,
				});
			}
		}

		await assertQuillYamlExists(quillDir);

		const metadata: QuillMetadata = { name, version: resolvedVersion };

		let files: Record<string, Uint8Array>;
		try {
			files = await readDirRecursive(quillDir);
		} catch (err) {
			throw new RegistryError('load_error', `Failed to read quill directory: ${quillDir}`, {
				quillName: name,
				version: resolvedVersion,
				cause: err,
			});
		}

		return {
			name,
			version: resolvedVersion,
			data: files,
			metadata,
		};
	}

	/**
	 * Packages all quills for HTTP static hosting.
	 * Zips each quill version directory and writes the zips plus a `manifest.json` to `outputDir`.
	 */
	async packageForHttp(outputDir: string): Promise<void> {
		await fs.mkdir(outputDir, { recursive: true });

		const manifest = await this.getManifest();
		for (const entry of manifest.quills) {
			const quillDir = path.join(this.quillsDir, entry.name, entry.version);
			const files = await readDirRecursive(quillDir);

			const zip = new JSZip();
			for (const [relativePath, content] of Object.entries(files)) {
				zip.file(relativePath, content);
			}

			const zipBuffer = await zip.generateAsync({ type: 'uint8array' });
			const zipFileName = `${entry.name}@${entry.version}.zip`;
			await fs.writeFile(path.join(outputDir, zipFileName), zipBuffer);
		}

		await fs.writeFile(path.join(outputDir, 'manifest.json'), JSON.stringify(manifest, null, 2));
	}

	/**
	 * Resolves the latest version for a quill by listing version directories
	 * and picking the highest semver.
	 */
	private async resolveLatestVersion(name: string): Promise<string> {
		const nameDir = path.join(this.quillsDir, name);

		let versionDirs: string[];
		try {
			versionDirs = await listSubdirectories(nameDir);
		} catch {
			throw new RegistryError('quill_not_found', `Quill "${name}" not found in source`, {
				quillName: name,
			});
		}

		if (versionDirs.length === 0) {
			throw new RegistryError('quill_not_found', `Quill "${name}" has no versions`, {
				quillName: name,
			});
		}

		// Filter to valid semver directories only
		const semverDirs = versionDirs.filter(isSemver);

		if (semverDirs.length === 0) {
			throw new RegistryError('quill_not_found', `Quill "${name}" has no valid version directories`, {
				quillName: name,
			});
		}

		// Sort by semver descending, return highest
		semverDirs.sort((a, b) => compareSemver(b, a));
		return semverDirs[0];
	}
}
