// The pre-initialization sentinel.
//
// wasm-bindgen's `--target web` glue holds its instance exports in one
// module-level binding (`let wasmModule, wasm;`) that every generated path
// reads through: constructors, statics, methods, free functions. Until
// instantiation assigns it, that binding is `undefined`, so a consumer who
// skipped `await init()` gets `Cannot read properties of undefined (reading
// 'quill_fromTree')` from inside generated code.
//
// `scripts/build-wasm.sh` (`guard_wasm_js`) patches the binding to start as
// this sentinel, so the same access throws a `QuillmarkError` naming the cause
// and the fix. The patch is three line edits per variant, shape-asserted before
// it applies; the logic lives here rather than inside the awk program.
//
// It sits in the generated build, not in the runtime layer, because the
// canonical invariant (`runtime.js` § "CANONICAL INVARIANT") requires the root
// to re-export `Quill`/`Document` VERBATIM: no wrapper, subclass, or proxy may
// stand between a consumer and those classes. Prototype patching preserves
// identity but cannot cover a public constructor (`new Document(...)`) without
// wrapping the class. The binding they all read through is the one place that
// covers every path and touches no class.

/**
 * A stand-in for a wasm build's exports that throws on use. Every property read
 * throws except the marker the patched init guards test, and `then`,
 * `constructor`, and symbols, which return `undefined`: an incidental `await`
 * or `util.inspect` reports nothing rather than throwing somewhere unrelated to
 * the cause.
 *
 * @param {string} message what a consumer sees when they reach a build early
 * @param {string} hint the fix, as a `Diagnostic.hint`
 * @returns {any} the sentinel
 */
export function uninitSentinel(message, hint) {
	return new Proxy(
		{},
		{
			get(_target, prop) {
				if (prop === UNINIT) return true;
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
 * The marker the patched `if (wasm !== undefined)` guards read to tell a
 * sentinel from real instance exports, which never carry it.
 */
export const UNINIT = '__quillmarkUninit';
