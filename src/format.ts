/**
 * File-extension pattern for binary assets.
 * Files matching this pattern get `number[]` contents in the engine tree;
 * everything else gets decoded as UTF-8 strings.
 */
const BINARY_EXT = /\.(ttf|otf|woff2?|jpg|jpeg|png|gif|pdf|zip)$/i;

/**
 * Converts a flat `Record<string, Uint8Array>` (as produced internally by sources)
 * into the nested file-tree object that `@quillmark/wasm`'s `registerQuill()` expects.
 *
 * The resulting shape is:
 * ```
 * {
 *   files: {
 *     'Quill.yaml': { contents: '...' },
 *     assets: {
 *       'image.jpg': { contents: [0xff, 0xd8, ...] }
 *     }
 *   }
 * }
 * ```
 *
 * Text files are decoded to strings; binary files (matched by extension) become `number[]`.
 */
export function toEngineFileTree(flatFiles: Record<string, Uint8Array>): {
	files: Record<string, unknown>;
} {
	const decoder = new TextDecoder('utf-8', { fatal: false });
	const tree: Record<string, unknown> = {};

	for (const [filePath, bytes] of Object.entries(flatFiles)) {
		const parts = filePath.split(/[/\\]/);
		let current = tree as Record<string, Record<string, unknown>>;
		for (let i = 0; i < parts.length - 1; i++) {
			const existing = current[parts[i]];
			if (existing === undefined || existing === null) {
				current[parts[i]] = {} as Record<string, unknown>;
			}
			current = current[parts[i]] as Record<string, Record<string, unknown>>;
		}
		const fileName = parts[parts.length - 1];
		(current as Record<string, unknown>)[fileName] = {
			contents: BINARY_EXT.test(fileName) ? Array.from(bytes) : decoder.decode(bytes),
		};
	}

	return { files: tree };
}
