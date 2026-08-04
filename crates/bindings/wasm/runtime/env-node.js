// The Node half of the `#quillmark-env` seam (package.json `imports`). See
// env-web.js for why the seam is resolution-time rather than a runtime branch.
//
// Node's `fetch` rejects `file:` URLs, so the default source — a `file:` URL
// pointing at the binary beside this package — is read off disk into bytes.
// Everything else (caller-supplied bytes, a `Response`, a `WebAssembly.Module`,
// an `http:` URL) passes through to the generated glue untouched.

import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

/**
 * @param {unknown} source
 * @returns {Promise<unknown> | unknown} bytes for a `file:` URL, else `source`
 */
export function toModuleSource(source) {
	return source instanceof URL && source.protocol === 'file:'
		? readFile(fileURLToPath(source))
		: source;
}
