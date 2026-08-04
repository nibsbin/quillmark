// The browser half of the `#quillmark-env` seam (package.json `imports`): the
// wasm byte source, resolved per environment at module resolution rather than
// by sniffing globals at runtime.
//
// A pass-through here. wasm-bindgen's `--target web` glue accepts a URL and
// fetches it itself (`WebAssembly.instantiateStreaming`), the streaming path.
//
// The seam exists for the Node half, which cannot, and `node:fs` must never
// reach a browser graph. Static resolution keeps it out: no `typeof process`
// branch for a bundler to trip over, no dynamic `import('node:fs')` to warn
// about.

/**
 * @param {unknown} source
 * @returns {unknown} `source` unchanged
 */
export function toModuleSource(source) {
	return source;
}
