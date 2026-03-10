/**
 * Minimal ustar tar archive packer/unpacker for bundling quill files.
 *
 * Uses the POSIX ustar format (512-byte headers, zero-padded content blocks).
 * All metadata (mode, uid, gid, mtime) is fixed for deterministic output.
 * Paths are sorted lexicographically before packing.
 */

const BLOCK = 512;

/** Writes an ASCII string into a Uint8Array at the given offset. */
function writeString(buf: Uint8Array, offset: number, str: string, len: number): void {
	for (let i = 0; i < Math.min(str.length, len); i++) {
		buf[offset + i] = str.charCodeAt(i);
	}
}

/** Reads a null-terminated ASCII string from a Uint8Array. */
function readString(buf: Uint8Array, offset: number, len: number): string {
	let end = offset;
	while (end < offset + len && buf[end] !== 0) end++;
	return String.fromCharCode(...buf.subarray(offset, end));
}

/**
 * Creates a single tar entry (header + zero-padded content) for a regular file.
 */
function createTarEntry(name: string, content: Uint8Array): Uint8Array {
	const header = new Uint8Array(BLOCK);
	const paddedSize = Math.ceil(content.length / BLOCK) * BLOCK;

	// name (0–99)
	writeString(header, 0, name, 100);
	// mode (100–107): 0644
	writeString(header, 100, '0000644\0', 8);
	// uid (108–115)
	writeString(header, 108, '0000000\0', 8);
	// gid (116–123)
	writeString(header, 116, '0000000\0', 8);
	// size (124–135): octal, null-terminated
	writeString(header, 124, content.length.toString(8).padStart(11, '0') + '\0', 12);
	// mtime (136–147): fixed at 0 for determinism
	writeString(header, 136, '00000000000\0', 12);
	// checksum placeholder (148–155): 8 spaces
	for (let i = 148; i < 156; i++) header[i] = 0x20;
	// typeflag (156): '0' = regular file
	header[156] = 0x30;
	// magic (257–262): "ustar\0"
	writeString(header, 257, 'ustar\0', 6);
	// version (263–264): "00"
	writeString(header, 263, '00', 2);

	// Compute and write header checksum
	let checksum = 0;
	for (let i = 0; i < BLOCK; i++) checksum += header[i];
	writeString(header, 148, checksum.toString(8).padStart(6, '0') + '\0 ', 8);

	const entry = new Uint8Array(BLOCK + paddedSize);
	entry.set(header, 0);
	entry.set(content, BLOCK);
	return entry;
}

/**
 * Packs a flat file map into a ustar tar archive.
 * Paths are sorted lexicographically for deterministic output.
 */
export function packFiles(files: Record<string, Uint8Array>): Uint8Array {
	const sortedPaths = Object.keys(files).sort();
	const entries: Uint8Array[] = [];
	let totalSize = 0;

	for (const p of sortedPaths) {
		const entry = createTarEntry(p, files[p]);
		entries.push(entry);
		totalSize += entry.length;
	}

	// End-of-archive marker: two 512-byte zero blocks
	totalSize += BLOCK * 2;

	const archive = new Uint8Array(totalSize);
	let offset = 0;
	for (const entry of entries) {
		archive.set(entry, offset);
		offset += entry.length;
	}
	// Trailing zero blocks are already zeros (Uint8Array default)

	return archive;
}

/**
 * Unpacks a ustar tar archive into a flat file map.
 */
export function unpackFiles(data: Uint8Array): Record<string, Uint8Array> {
	const files: Record<string, Uint8Array> = {};
	let offset = 0;

	while (offset + BLOCK <= data.length) {
		const header = data.subarray(offset, offset + BLOCK);
		// End-of-archive: zero block
		let allZero = true;
		for (let i = 0; i < BLOCK; i++) {
			if (header[i] !== 0) {
				allZero = false;
				break;
			}
		}
		if (allZero) break;

		const name = readString(header, 0, 100);
		const size = parseInt(readString(header, 124, 12), 8);
		const typeflag = header[156];
		offset += BLOCK;

		// '0' (0x30) or '\0' (0x00) = regular file
		if (typeflag === 0x30 || typeflag === 0) {
			files[name] = data.slice(offset, offset + size);
		}

		offset += Math.ceil(size / BLOCK) * BLOCK;
	}

	return files;
}
