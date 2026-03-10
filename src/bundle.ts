/**
 * Zip archive packing/unpacking for bundling quill files.
 *
 * Uses fflate for lightweight, cross-platform zip support.
 * Packing is deterministic: paths are sorted lexicographically.
 */

import { zipSync, unzipSync } from 'fflate';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';

/**
 * Fixed date for deterministic zip output (DOS date minimum: 1980-01-01).
 * Constructed via local-time components so that fflate's getFullYear() etc.
 * always return 1980-01-01 00:00:00 regardless of the host timezone.
 */
const ZIP_EPOCH = new Date(1980, 0, 1);

/**
 * Creates a zip archive directly from a directory on disk.
 * Entries are sorted lexicographically for deterministic output.
 */
export async function packDirectory(dirPath: string, fileList: string[]): Promise<Uint8Array> {
	const sorted = [...fileList].sort();
	const entries: Record<string, Uint8Array> = {};
	for (const filePath of sorted) {
		const fullPath = path.join(dirPath, filePath);
		entries[filePath] = new Uint8Array(await fs.readFile(fullPath));
	}
	return zipSync(entries, { level: 0, mtime: ZIP_EPOCH });
}

/**
 * Packs a flat file map into a zip archive.
 * Paths are sorted lexicographically for deterministic output.
 */
export async function packFiles(files: Record<string, Uint8Array>): Promise<Uint8Array> {
	const sorted = Object.keys(files).sort();
	const entries: Record<string, Uint8Array> = {};
	for (const filePath of sorted) {
		entries[filePath] = files[filePath];
	}
	return zipSync(entries, { level: 0, mtime: ZIP_EPOCH });
}

/**
 * Unpacks a zip archive into a flat file map.
 */
export async function unpackFiles(data: Uint8Array): Promise<Record<string, Uint8Array>> {
	return unzipSync(data);
}
