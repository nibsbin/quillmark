// The browser half of the `#quillmark-env` seam (package.json `imports`): the
// wasm byte source, resolved per environment at module-resolution time rather
// than by sniffing globals at runtime.
//
// Here it is a pass-through. wasm-bindgen's `--target web` glue already accepts
// a URL and fetches it itself (`WebAssembly.instantiateStreaming`), which is the
// streaming path and the one worth taking in a browser.
//
// The seam exists for the Node half, which cannot: `node:fs` must never reach a
// browser graph. Static resolution keeps it out — no `typeof process` branch for
// a bundler to trip over, no dynamic `import('node:fs')` for it to warn about.

/**
 * @param {unknown} source
 * @returns {unknown} `source` unchanged
 */
export function toModuleSource(source) {
	return source;
}
