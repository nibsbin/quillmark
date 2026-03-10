/**
 * Tar archive packing/unpacking for bundling quill files.
 *
 * Uses node-tar for standards-compliant POSIX tar archives.
 * Packing is deterministic: paths are sorted lexicographically
 * and all metadata (uid, gid, mtime) is fixed.
 */

import { create as tarCreate, Parser as TarParser } from 'tar';
import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';

/** Collects all chunks from an async iterable into a single Uint8Array. */
async function collectStream(stream: AsyncIterable<Buffer | Uint8Array>): Promise<Uint8Array> {
	const chunks: Buffer[] = [];
	for await (const chunk of stream) {
		chunks.push(Buffer.from(chunk));
	}
	return new Uint8Array(Buffer.concat(chunks));
}

/**
 * Creates a tar archive directly from a directory on disk using node-tar.
 * Entries are sorted lexicographically and metadata is fixed for deterministic output.
 */
export async function packDirectory(dirPath: string, fileList: string[]): Promise<Uint8Array> {
	const sorted = [...fileList].sort();
	if (sorted.length === 0) {
		// node-tar requires at least one entry; return a minimal empty archive (two zero blocks)
		return new Uint8Array(1024);
	}
	const stream = tarCreate(
		{
			cwd: dirPath,
			portable: true,
			mtime: new Date(0),
		},
		sorted,
	);
	return collectStream(stream);
}

/**
 * Packs a flat file map into a tar archive using node-tar.
 * Files are written to a temporary directory, then archived.
 * Paths are sorted lexicographically for deterministic output.
 */
export async function packFiles(files: Record<string, Uint8Array>): Promise<Uint8Array> {
	const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'quill-pack-'));
	try {
		const sortedPaths = Object.keys(files).sort();
		for (const filePath of sortedPaths) {
			const fullPath = path.join(tmpDir, ...filePath.split('/'));
			await fs.mkdir(path.dirname(fullPath), { recursive: true });
			await fs.writeFile(fullPath, files[filePath]);
		}
		return packDirectory(tmpDir, sortedPaths);
	} finally {
		await fs.rm(tmpDir, { recursive: true, force: true });
	}
}

/**
 * Unpacks a tar archive into a flat file map using node-tar.
 */
export async function unpackFiles(data: Uint8Array): Promise<Record<string, Uint8Array>> {
	const files: Record<string, Uint8Array> = {};

	return new Promise((resolve, reject) => {
		const parser = new TarParser({
			onReadEntry: (entry) => {
				if (entry.type === 'File') {
					const chunks: Buffer[] = [];
					entry.on('data', (d: Buffer) => chunks.push(d));
					entry.on('end', () => {
						files[entry.path] = new Uint8Array(Buffer.concat(chunks));
					});
				} else {
					entry.resume();
				}
			},
		});

		parser.on('end', () => resolve(files));
		parser.on('error', reject);
		parser.write(Buffer.from(data));
		parser.end();
	});
}
