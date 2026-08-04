// The pre-initialization sentinel.
//
// wasm-bindgen's `--target web` glue holds its instance exports in one
// module-level binding (`let wasmModule, wasm;`) that every generated path
// reads through: constructors, statics, methods, free functions. Until
// instantiation assigns it, that binding is `undefined`, and a consumer who
// skipped `await init()` gets `Cannot read properties of undefined (reading
// 'quill_fromTree')` from inside generated code.
//
// `scripts/build-wasm.sh` patches the binding to start as this sentinel
// instead, so the same access throws a `QuillmarkError` that names the cause
// and the fix. The patch is three line edits per variant, asserted for shape
// before it applies; the logic lives here, in reviewable source.
//
// Why not guard in the runtime layer: the canonical invariant
// (`runtime.js` § "CANONICAL INVARIANT") requires the root to re-export the
// core build's `Quill`/`Document` VERBATIM, so no wrapper, subclass, or proxy
// may stand between a consumer and those classes. Prototype patching would
// preserve identity but cannot cover `new Document(...)` — a public
// constructor, and guarding one means wrapping the class. Instrumenting the
// binding they all read through is the one mechanism that covers every path
// and touches no class.

/**
 * A stand-in for a wasm build's exports that throws on use. Every property read
 * throws except the marker the patched init guards test, and symbols, which
 * return `undefined` so an incidental `await`, `console.log`, or devtools
 * inspection reports nothing rather than throwing from the wrong place.
 *
 * @param {string} message what a consumer sees when they reach a build early
 * @param {string} hint the fix, as a `Diagnostic.hint`
 * @returns {any} the sentinel, shaped to fail loudly on first real use
 */
export function uninitSentinel(message, hint) {
	return new Proxy(
		{},
		{
			get(_target, prop) {
				if (prop === UNINIT) return true;
				// `then` first: an unawaited sentinel must not masquerade as a
				// thenable, and must not throw for merely being awaited.
				if (prop === 'then' || prop === 'constructor' || typeof prop === 'symbol') {
					return undefined;
				}
				const err = /** @type {any} */ (new Error(message));
				err.diagnostics = [
					{ severity: 'error', code: 'runtime::not_initialized', message, hint }
				];
				throw err;
			}
		}
	);
}

/**
 * The marker property the patched `if (wasm !== undefined) return wasm;` guards
 * read to tell a sentinel from real instance exports. Real exports never carry
 * it, so the test is total.
 */
export const UNINIT = '__quillmarkUninit';
