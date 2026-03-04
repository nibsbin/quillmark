import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import JSZip from 'jszip';
import { parse as parseYaml } from 'yaml';
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

/** Extracts metadata from a Quill.yaml file within a quill directory. */
async function readQuillMetadata(quillDir: string): Promise<QuillMetadata> {
	const yamlPath = path.join(quillDir, 'Quill.yaml');

	let content: string;
	try {
		content = await fs.readFile(yamlPath, 'utf-8');
	} catch (err) {
		throw new RegistryError('load_error', `Failed to read Quill.yaml in ${quillDir}`, {
			cause: err,
		});
	}

	const parsed = parseYaml(content);

	if (!parsed || typeof parsed.name !== 'string' || typeof parsed.version !== 'string') {
		throw new RegistryError(
			'load_error',
			`Invalid Quill.yaml in ${quillDir}: missing name or version`,
		);
	}

	return {
		name: parsed.name,
		version: parsed.version,
		...(typeof parsed.description === 'string' ? { description: parsed.description } : {}),
	};
}

/**
 * Node.js-only QuillSource that reads Quill directories from the local filesystem.
 *
 * Each subdirectory of the provided `quillsDir` is treated as a quill.
 * Each must contain a `Quill.yaml` with at least `name` and `version` fields.
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
		let entries: string[];
		try {
			const dirEntries = await fs.readdir(this.quillsDir, { withFileTypes: true });
			entries = dirEntries.filter((e) => e.isDirectory()).map((e) => e.name);
		} catch (err) {
			throw new RegistryError(
				'source_unavailable',
				`Failed to read quills directory: ${this.quillsDir}`,
				{ cause: err },
			);
		}

		const quills: QuillMetadata[] = [];
		for (const dirName of entries) {
			const quillDir = path.join(this.quillsDir, dirName);
			const metadata = await readQuillMetadata(quillDir);
			quills.push(metadata);
		}

		return { quills };
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

		const quillDir = await this.findQuillDir(entry.name, entry.version);
		let files: Record<string, Uint8Array>;
		try {
			files = await readDirRecursive(quillDir);
		} catch (err) {
			throw new RegistryError('load_error', `Failed to read quill directory: ${quillDir}`, {
				quillName: name,
				version: entry.version,
				cause: err,
			});
		}

		return {
			name: entry.name,
			version: entry.version,
			data: files,
			metadata: entry,
		};
	}

	/**
	 * Packages all quills for HTTP static hosting.
	 * Zips each quill directory and writes the zips plus a `manifest.json` to `outputDir`.
	 */
	async packageForHttp(outputDir: string): Promise<void> {
		await fs.mkdir(outputDir, { recursive: true });

		const manifest = await this.getManifest();
		for (const entry of manifest.quills) {
			const quillDir = await this.findQuillDir(entry.name, entry.version);
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

	/** Finds the directory for a quill by scanning subdirectories and matching metadata. */
	private async findQuillDir(name: string, version: string): Promise<string> {
		const dirEntries = await fs.readdir(this.quillsDir, { withFileTypes: true });
		const dirs = dirEntries.filter((e) => e.isDirectory()).map((e) => e.name);

		for (const dirName of dirs) {
			const quillDir = path.join(this.quillsDir, dirName);
			try {
				const metadata = await readQuillMetadata(quillDir);
				if (metadata.name === name && metadata.version === version) {
					return quillDir;
				}
			} catch {
				// Skip directories that don't have valid Quill.yaml
			}
		}

		throw new RegistryError('quill_not_found', `Quill directory for "${name}@${version}" not found`, {
			quillName: name,
			version,
		});
	}
}
