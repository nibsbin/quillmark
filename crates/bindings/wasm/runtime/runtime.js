/* @ts-self-types="./runtime.d.ts" */
//
// @quillmark/wasm/runtime — the canonical consumer API.
//
// Consumers import `Quill`, `Document`, and `Engine` from here and never touch
// the build-specific subpaths. The package ships multiple WASM binaries with
// SEPARATE linear memories — a Typst-less `core` build (small, eager) that is
// the canonical home of `Quill`/`Document`, and one private backend binary per
// backend (`backends/typst/` today; more later) that carries an engine. A
// handle from one memory cannot be used by another. This module hides that seam
// and is exposed at the package root (`@quillmark/wasm`):
//
//   - `Quill` and `Document` ARE the core build's classes, re-exported. They
//     hold the canonical data and the full sync surface (schema / validate /
//     seed / mutate / toJson / toTree). No backend is loaded to use them, so
//     the editor/validation path never pays for a multi-MB backend binary.
//
//   - `Engine` is the render dispatcher. It routes on `quill.backendId`, lazily
//     imports that backend's build (so a consumer that never renders never
//     loads it), clones the canonical `Quill`/`Document` into the backend's
//     memory ON DEMAND as data (`toTree`→`fromTree`, `toJson`→`fromJson`),
//     renders, and the backend handles never escape.
//
//     CLONE LIFETIMES (not all transient): the per-call `Document` clone IS
//     transient — built and freed inside each call, because documents are small
//     and mutate freely. The `Quill` clone is CACHED instead: re-cloning a quill
//     per call means re-serializing its whole file tree Rust→JS, copying it into
//     backend memory, and re-parsing + re-validating the bundle every time — and
//     quills are validated, effectively-immutable bundles. So each `Engine`
//     memoizes the backend-memory quill per (engine, backendId, canonical quill
//     instance) in a `WeakMap` keyed on the canonical `Quill`: when the consumer
//     drops the core quill the cache entry becomes collectable and wasm-bindgen
//     weak-refs (`--weak-refs`) free the backend handle. The CONTRACT this buys:
//     a `Quill` instance's contents never change after construction — mutate by
//     replacing the instance (the clone is dropped with it via WeakMap +
//     weak-refs).
//
// The cross-memory crossing is therefore invisible: a consumer hands canonical
// `Quill`/`Document` to `engine.render(...)` and gets a `RenderResult` back.

// ── CANONICAL INVARIANT: re-export the core build, never wrap ───────────────
// The root re-exports the core build's `Quill`/`Document` classes verbatim —
// NOT subclasses or wrappers. There is exactly ONE public entry point (this
// module), so this identity is a structural fact: `Quill`/`Document` ARE the
// core classes, and the only boundary that needs crossing is core→backend (a
// separate WASM memory), which `Engine` does internally as data
// (`toTree`/`toJson`).
//
// Do NOT replace this with a wrapper class — that breaks the identity and turns
// a structural fact into a converted type (a breaking design change, not a
// refactor). The `runtime.test.js` "re-exports the internal core build classes
// verbatim" case (`Quill === CoreQuill`) is the executable guard for this
// invariant.
//
// The identity is what makes `instanceof` the whole membership test: a handle
// either belongs to this copy's classes or it belongs to another copy, and the
// second is always a consumer bug. `Engine` is NOT duck-typed on its inputs; it
// checks them. See § "Handles from another copy" below.
//
// Imported (not bare re-exported) so `Quill` is a local binding this module can
// augment — `quill.writer(doc)` is patched onto its prototype below. The
// re-export keeps the identity: the exported `Quill` IS the core class.
import { Quill, Document, init } from '../core/wasm.js';
export { Quill, Document, init };
// The document-free content codec — re-exported verbatim from the core build so
// the runtime subpath exposes `exportMarkdown(body)` (the on-demand markdown
// projection), `importMarkdown`, and the position-mapping pair (`rebase`,
// `mapPos`).
export { importMarkdown, exportMarkdown, rebase, mapPos } from '../core/wasm.js';
// The document-model path parser/serializer: `parseDocPath(str) => DocPathSeg[]`
// and its inverse `formatDocPath`, so a consumer routes on `Diagnostic.path`
// segments instead of reverse-engineering the grammar.
export { parseDocPath, formatDocPath } from '../core/wasm.js';

// ── The main-card address ───────────────────────────────────────────────────
/**
 * The main card's address — the default target of the card-scoped verbs
 * (`storeFields` / `storeExt` / `commitFields` / …). A named, `CardAddr`-typed
 * alias for the empty address `{}`, so a main-card write names its target:
 * `doc.storeFields(MAIN_CARD_ADDR, fields)`. It IS `{}` (frozen), a pure alias —
 * `{}` and `undefined` stay equally valid. Card axis only: a card selector,
 * never a field address.
 * @type {import('../core/wasm.js').CardAddr}
 */
export const MAIN_CARD_ADDR = Object.freeze({});

/**
 * Narrow an unknown caught value to a `QuillmarkError` — the error every
 * fallible method in this package throws: a real `Error` with a non-empty
 * `diagnostics` array attached (same entry shape as `RenderResult.warnings`).
 *
 * Structural by necessity AND by design: the WASM layer constructs a plain
 * `Error` and attaches the property (there is no error class to `instanceof`),
 * and a structural check narrows errors from any build or WASM instance in the
 * page. The deliberate exception to § "Handles from another copy": an error is
 * data, not a handle, so nothing is gained by refusing one that crossed.
 *
 * @param {unknown} e
 * @returns {e is Error & { diagnostics: import('../core/wasm.js').Diagnostic[] }}
 */
export function isQuillmarkError(e) {
	return e instanceof Error && Array.isArray(/** @type {any} */ (e).diagnostics);
}

/**
 * Build a `QuillmarkError` JS-side: a real `Error` carrying `diagnostics`, the
 * shape `isQuillmarkError` narrows and the shape Rust's `WasmError::to_js_value`
 * produces. Errors raised by this hand-written layer belong to the same contract
 * as the ones raised across the WASM boundary.
 * @param {string} code
 * @param {string} message
 * @param {string} hint
 * @returns {Error & { diagnostics: import('../core/wasm.js').Diagnostic[] }}
 */
function quillmarkError(code, message, hint) {
	const err = /** @type {any} */ (new Error(message));
	err.diagnostics = [{ severity: 'error', code, message, hint }];
	return err;
}

// ── Handles from another copy: always a bug ─────────────────────────────────
// A duplicate install (two copies of this package in one `node_modules` tree)
// is two `core` builds: two linear memories and two distinct `Quill`/`Document`
// classes. No topology legitimately loads a multi-megabyte WASM package twice
// AND needs handles to cross between the copies, so a crossing is a consumer
// bug. Every seam taking a core handle says so, uniformly, at the crossing.
//
// The alternative — crossing read-only handles as data, since `toJson`/`toTree`
// make that mechanically possible — was tried and reversed. It leaves a package
// where some verbs work and some throw, and it hides a cliff: a crossed read is
// a whole-document `toJson` + `fromJson`, so a form reading fifty fields pays
// fifty round trips and the symptom is "the editor got slow". A duplicate
// install that limps is a duplicate install nobody removes.
//
// What the checks below actually deliver is the ERROR, not the rejection.
// wasm-bindgen's generated glue already rejects a foreign class on every method
// declaring a reference parameter (`Document.equals`, `Quill.validate`,
// `Quill.resolve`, the `&Quill`-taking `Document._commitField` and friends): its
// `_assertClass` is emitted unconditionally and runs before any of our code. But
// it throws a bare `Error` reading `expected instance of Document` at a value
// that IS a `Document` — `isQuillmarkError` returns false and the failure leaves
// this package's error contract, naming neither the cause nor the cure. The
// checks front-run it with a `QuillmarkError` that names both.
//
// They also cover the seams that have NO `_assertClass` to front-run: `Engine`
// and `LiveSession.apply` cross into backend memory as data (`toTree`/`toJson`),
// so a foreign handle there would silently work, at the price of a per-copy
// quill clone cache (#1134) and the round-trip above.
//
// Not an API widening in either direction: the declared parameter types stay
// `Quill`/`Document`, and the accepted set is exactly this copy's instances.

/**
 * The rejection for a value that is not one of this copy's handles. Two cures,
 * so two diagnostics: a value carrying `probe` is that class from ANOTHER copy
 * (dedupe the install), anything else is the wrong argument (fix the call).
 * @param {unknown} value
 * @param {string} method
 * @param {'Quill' | 'Document'} className
 * @param {string} probe a method the class has, standing in for its shape
 * @param {string} hint what a caller who passed the wrong thing should pass
 * @returns {Error & { diagnostics: import('../core/wasm.js').Diagnostic[] }}
 */
function notLocal(value, method, className, probe, hint) {
	if (value && typeof (/** @type {any} */ (value)[probe]) === 'function') {
		return quillmarkError(
			'runtime::foreign_handle',
			`${method}: the ${className} belongs to a different copy of @quillmark/wasm. Handles never cross between copies — each copy is its own WASM linear memory and its own ${className} class.`,
			'Two copies of @quillmark/wasm are installed. Run `npm ls @quillmark/wasm` and dedupe to one.'
		);
	}
	return quillmarkError(
		className === 'Quill' ? 'runtime::not_a_quill' : 'runtime::not_a_document',
		`${method}: expected a ${className}, got ${value === null ? 'null' : typeof value}.`,
		hint
	);
}

/**
 * Throw unless `doc` is THIS copy's `Document`.
 * @param {unknown} doc
 * @param {string} method
 * @returns {void}
 */
function requireLocalDoc(doc, method) {
	if (doc instanceof Document) return;
	throw notLocal(
		doc,
		method,
		'Document',
		'toJson',
		'Pass a Document built by Document.fromMarkdown / fromJson or quill.seedDocument.'
	);
}

/**
 * Throw unless `quill` is THIS copy's `Quill`.
 * @param {unknown} quill
 * @param {string} method
 * @returns {void}
 */
function requireLocalQuill(quill, method) {
	if (quill instanceof Quill) return;
	throw notLocal(quill, method, 'Quill', 'toTree', 'Pass a Quill built by Quill.fromTree.');
}

// Marker for the patches below. `Symbol.for`, not a module-local `Symbol()`:
// re-evaluating THIS module (Vite HMR, a Vitest worker sharing a module graph)
// against a core build that is cached, and so already patched, must see the
// existing marker, or each pass wraps the previous wrapper.
const HANDLE_CHECKED = Symbol.for('@quillmark/wasm:handle-checked');

/**
 * Replace `proto[name]` with `wrap(original)`, once.
 * @param {object} proto
 * @param {string} name
 * @param {(original: Function) => Function} wrap
 */
function patchHandleChecked(proto, name, wrap) {
	const original = /** @type {any} */ (proto)[name];
	if (typeof original !== 'function' || original[HANDLE_CHECKED]) return;
	const patched = wrap(original);
	/** @type {any} */ (patched)[HANDLE_CHECKED] = true;
	// Keep the method's name so a stack trace still reads `Quill.validate`.
	Object.defineProperty(patched, 'name', { value: name, configurable: true });
	/** @type {any} */ (proto)[name] = patched;
}

// The three core methods declaring a `&Document` parameter. Each already refuses
// a foreign handle inside `_assertClass`; the patch is what makes the refusal
// legible. Only the argument can be foreign, the receiver being whichever copy
// the caller reached for.
patchHandleChecked(Document.prototype, 'equals', (original) =>
	function equals(/** @type {any} */ other) {
		requireLocalDoc(other, 'Document.equals');
		return original.call(this, other);
	}
);
for (const name of /** @type {const} */ (['validate', 'resolve'])) {
	patchHandleChecked(Quill.prototype, name, (original) =>
		function (/** @type {any} */ doc) {
			requireLocalDoc(doc, `Quill.${name}`);
			return original.call(this, doc);
		}
	);
}

// The typed writer/reader primitives (`Document._commitField` and friends) take
// the QUILL by reference, so they hit the same `_assertClass` from the other
// direction. Checked at the four writer/reader classes below, not patched onto
// `Document`: a foreign document carries its OWN prototype, so patching this
// copy's would never run.

// ── Open-set discriminant guards ────────────────────────────────────────────
// `ContentIsland.type`, `ContentMark.type`, `ContentLine.kind`, and
// `ContentContainer.container` are OPEN sets: each union carries a residual
// `{ …: string; … }` arm, so a bare `x.type === 'table'` check never narrows the
// payload — TS keeps the residual arm live (a `string` can be `'table'`),
// leaving `props` / the mark payload / `level` opaque at every consumer. These
// are the checked narrowing path: on the true branch the payload's pinned shape
// is asserted. Only the payload-carrying arms get a guard — an island always
// carries `props`, a `link` mark carries `url`, an `anchor` mark carries `id`, a
// `heading` line carries `level` and a `code` line `lang`, a `list_item`
// container its shape; the payload-free arms (`strong`/`emph`/`underline`/
// `strike`/`code` marks, `para`/`island`/`rule` lines, `quote`) narrow to
// nothing. An unrecognized discriminant fails every guard and keeps its opaque
// `attrs`/`props`.

/**
 * @param {import('../core/wasm.js').ContentIsland} island
 * @returns {island is import('../core/wasm.js').ContentIsland & { type: 'table'; props: import('../core/wasm.js').TableProps }}
 */
export function isTableIsland(island) {
	return island.type === 'table';
}

/**
 * @param {import('../core/wasm.js').ContentIsland} island
 * @returns {island is import('../core/wasm.js').ContentIsland & { type: 'image'; props: import('../core/wasm.js').ImageProps }}
 */
export function isImageIsland(island) {
	return island.type === 'image';
}

/**
 * @param {import('../core/wasm.js').ContentMark} mark
 * @returns {mark is import('../core/wasm.js').ContentMark & { type: 'link'; url: string }}
 */
export function isLinkMark(mark) {
	return mark.type === 'link';
}

/**
 * @param {import('../core/wasm.js').ContentMark} mark
 * @returns {mark is import('../core/wasm.js').ContentMark & { type: 'anchor'; id: string }}
 */
export function isAnchorMark(mark) {
	return mark.type === 'anchor';
}

/**
 * @param {import('../core/wasm.js').ContentLine} line
 * @returns {line is import('../core/wasm.js').ContentLine & { kind: 'heading'; level: number }}
 */
export function isHeadingLine(line) {
	return line.kind === 'heading';
}

/**
 * @param {import('../core/wasm.js').ContentLine} line
 * @returns {line is import('../core/wasm.js').ContentLine & { kind: 'code'; lang?: string }}
 */
export function isCodeLine(line) {
	return line.kind === 'code';
}

/**
 * @param {import('../core/wasm.js').ContentContainer} container
 * @returns {container is import('../core/wasm.js').ContentContainer & { container: 'list_item'; ordered: boolean; start: number; ordinal: number }}
 */
export function isListItemContainer(container) {
	return container.container === 'list_item';
}

// ── Open-set membership guards ──────────────────────────────────────────────
// The guards above each answer "is this arm X" — one pinned arm at a time. These
// four answer the other question: "is this a value this build knows?" A consumer
// that must branch known-vs-unknown — any read-modify-write consumer, since
// lowering an edit restates every line's kind and containers — otherwise
// enumerates the built-in names in its own source, recreating the closed-set
// coupling the open set exists to remove. That list is correct until the release
// that adds a built-in, at which point the new construct is misclassified as
// unknown and round-trips through the consumer's unknown carrier, losing any
// sibling-key payload.
//
// A predicate rather than an exported name list, because the known tables below
// are upstream's business. They are pinned against the Rust source
// (`Content::RESERVED_*` and `KnownIslandType`) by the
// `known_open_set_names_are_pinned` drift-guard test in
// `crates/content/src/model.rs`: adding a built-in means editing there, here, and
// the TS unions in `crates/bindings/wasm/src/engine.rs` in one commit.
//
// These classify unknown *tags*, not unknown *payloads on known tags*. A future
// `kind: "footnote"` with a sibling `ref` loses `ref` at a consumer that predates
// it either way.

const KNOWN_LINE_KINDS = new Set(['para', 'heading', 'code', 'island', 'rule']);
const KNOWN_CONTAINERS = new Set(['list_item', 'quote']);
const KNOWN_MARK_TYPES = new Set(['strong', 'emph', 'underline', 'strike', 'code', 'link', 'anchor']);
const KNOWN_ISLAND_TYPES = new Set(['table', 'image']);

/**
 * @param {import('../core/wasm.js').ContentLine} line
 * @returns {line is import('../core/wasm.js').ContentLine & { kind: string; attrs: unknown }}
 */
export function isUnknownLine(line) {
	return typeof line?.kind === 'string' && !KNOWN_LINE_KINDS.has(line.kind);
}

/**
 * @param {import('../core/wasm.js').ContentContainer} container
 * @returns {container is import('../core/wasm.js').ContentContainer & { container: string; attrs: unknown }}
 */
export function isUnknownContainer(container) {
	return typeof container?.container === 'string' && !KNOWN_CONTAINERS.has(container.container);
}

/**
 * @param {import('../core/wasm.js').ContentMark} mark
 * @returns {mark is import('../core/wasm.js').ContentMark & { type: string; attrs: unknown }}
 */
export function isUnknownMark(mark) {
	return typeof mark?.type === 'string' && !KNOWN_MARK_TYPES.has(mark.type);
}

/**
 * @param {import('../core/wasm.js').ContentIsland} island
 * @returns {island is import('../core/wasm.js').ContentIsland & { type: string; props: unknown }}
 */
export function isUnknownIsland(island) {
	return typeof island?.type === 'string' && !KNOWN_ISLAND_TYPES.has(island.type);
}

// Backend builds are NEVER statically imported here — that would pull a
// multi-MB binary into the eager graph and defeat lazy loading. Each entry is a
// DESCRIPTOR: `load` is a thunk returning a dynamic `import()` (a backend's
// chunk is fetched only when something actually renders against that backend),
// and `formats`/`canvas` are the REQUIRED static capability manifest so the
// cheap probes (`supportedFormats`/`supportsCanvas`) ALWAYS answer without
// loading the binary or cloning the quill. The manifest values are verified
// against each backend's Rust source (`crates/backends/<id>/src/lib.rs`
// `SUPPORTED_FORMATS`) and pinned by the `runtime.test.js` drift-guard test,
// which renders once and asserts the loaded backend reports the same list.
// `canvas` mirrors `quillmark_core::formats_support_canvas`: true iff the
// format list includes a visual-page format (`svg` or `png`).
const DEFAULT_BACKENDS = {
	typst: {
		load: () => import('../backends/typst/wasm.js'),
		formats: ['pdf', 'svg', 'png'], // crates/backends/typst/src/lib.rs SUPPORTED_FORMATS
		canvas: true // has svg/png → formats_support_canvas == true
	},
	pdfform: {
		load: () => import('../backends/pdfform/wasm.js'),
		// crates/backends/pdfform/src/lib.rs SUPPORTED_FORMATS == [Pdf, Svg, Png]
		formats: ['pdf', 'svg', 'png'],
		canvas: true // has svg/png → formats_support_canvas == true
	}
};

/**
 * Validate a backend registry descriptor, throwing a clear error naming the
 * backend id on any malformed entry. Descriptors are the ONLY accepted form:
 * `{ load, formats, canvas }` with a callable `load`, a `formats` array, and a
 * boolean `canvas`. Failing at construction (not deep inside a render) keeps the
 * capability probes free — they can answer from the manifest unconditionally.
 * @param {string} id
 * @param {unknown} entry
 * @returns {{ load: () => Promise<unknown>, formats: string[], canvas: boolean }}
 */
function validateBackend(id, entry) {
	if (!entry || typeof entry !== 'object') {
		throw new Error(
			`Engine: backend '${id}' must be a descriptor { load, formats, canvas }.`
		);
	}
	const { load, formats, canvas } = /** @type {any} */ (entry);
	if (typeof load !== 'function') {
		throw new Error(`Engine: backend '${id}' descriptor needs a callable 'load'.`);
	}
	if (!Array.isArray(formats)) {
		throw new Error(`Engine: backend '${id}' descriptor needs a 'formats' array.`);
	}
	if (typeof canvas !== 'boolean') {
		throw new Error(`Engine: backend '${id}' descriptor needs a boolean 'canvas'.`);
	}
	return { load, formats, canvas };
}

/**
 * Render dispatcher over the canonical `Quill`/`Document`. One `Engine`
 * instance can drive every backend; it resolves the right backend build from
 * each quill's declared `backendId` and loads it lazily on first use.
 */
export class Engine {
	/** backendId → Promise<backend module>, memoized so each build loads once. */
	#modules = new Map();
	/** backendId → that backend's engine instance (the WASM backend registry). */
	#engines = new Map();
	/** backendId → descriptor `{ load, formats, canvas }`. */
	#loaders;
	/**
	 * backendId → WeakMap<canonical Quill, backend-memory Quill clone>. Caches
	 * the expensive quill materialization per (engine, backend, canonical quill
	 * instance). WeakMap so dropping the canonical quill makes its clone
	 * collectable; the backend handle is then freed by wasm-bindgen weak-refs.
	 * @type {Map<string, WeakMap<object, any>>}
	 */
	#quillClones = new Map();

	/**
	 * @param {{ backends?: Record<string, { load: () => Promise<unknown>, formats: string[], canvas: boolean }> }} [options]
	 *   Extra or overriding backend descriptors, merged over the built-ins. Each
	 *   entry is a descriptor (`{ load, formats, canvas }`) with `formats` and
	 *   `canvas` REQUIRED — that static manifest is what makes
	 *   `supportedFormats`/`supportsCanvas` always free (no binary load, no quill
	 *   clone). Malformed entries throw here, at construction. The default
	 *   registry maps `"typst"` to the bundled Typst build.
	 */
	constructor(options) {
		const merged = { ...DEFAULT_BACKENDS, ...(options?.backends ?? {}) };
		/** @type {Record<string, { load: () => Promise<unknown>, formats: string[], canvas: boolean }>} */
		const loaders = {};
		for (const [id, entry] of Object.entries(merged)) {
			loaders[id] = validateBackend(id, entry);
		}
		this.#loaders = loaders;
	}

	/**
	 * Look up the registered descriptor for `backendId`, throwing the canonical
	 * "no backend registered" error if none. Pure — touches no binary.
	 * @param {string} backendId
	 * @returns {{ load: () => Promise<unknown>, formats: string[], canvas: boolean }}
	 */
	#descriptorFor(backendId) {
		const descriptor = this.#loaders[backendId];
		if (!descriptor) {
			throw new Error(
				`Engine: no backend registered for '${backendId}'. ` +
					`Known backends: ${Object.keys(this.#loaders).join(', ') || '(none)'}.`
			);
		}
		return descriptor;
	}

	/**
	 * Resolve (and lazily load) the backend module + its engine for `backendId`.
	 * @param {string} backendId
	 * @returns {Promise<{ mod: any, engine: any }>}
	 */
	async #resolveBackend(backendId) {
		const descriptor = this.#descriptorFor(backendId);

		let modPromise = this.#modules.get(backendId);
		if (!modPromise) {
			// Set the promise synchronously (before any await) so concurrent first
			// renders share ONE import. Self-heal on failure so a transient load
			// error doesn't poison every later attempt.
			modPromise = Promise.resolve()
				.then(descriptor.load)
				.catch((err) => {
					this.#modules.delete(backendId);
					throw err;
				});
			this.#modules.set(backendId, modPromise);
		}
		const mod = await modPromise;

		let engine = this.#engines.get(backendId);
		if (!engine) {
			engine = new mod.Quillmark();
			this.#engines.set(backendId, engine);
		}
		return { mod, engine };
	}

	/**
	 * Get (or materialize-and-cache) the backend-memory `Quill` clone for
	 * `quill` under `backendId`. On a cache miss the clone is built from `tree`
	 * — the caller's pre-await `toTree()` snapshot; the canonical handle may be
	 * freed by now — and stored in the per-backend `WeakMap` keyed on the
	 * canonical `Quill` instance, so a later call with the same instance reuses
	 * it (the hot-path fix: no re-serialize / re-copy / re-validate per call).
	 * @param {any} mod the backend build module
	 * @param {string} backendId
	 * @param {object} quill the canonical instance (cache key only)
	 * @param {Map<string, Uint8Array> | null} tree pre-await snapshot; `null` on a cache hit
	 * @returns {any} the backend-memory quill clone
	 */
	#cachedQuillClone(mod, backendId, quill, tree) {
		let perQuill = this.#quillClones.get(backendId);
		if (!perQuill) {
			perQuill = new WeakMap();
			this.#quillClones.set(backendId, perQuill);
		}
		let backendQuill = perQuill.get(quill);
		if (!backendQuill) {
			backendQuill = mod.Quill.fromTree(tree);
			perQuill.set(quill, backendQuill);
		}
		return backendQuill;
	}

	/**
	 * Materialize the backend-memory clones for `quill` + `doc` in `backendId`'s
	 * memory and run `fn` against the backend engine. Only `render`/`open` call
	 * this, so `doc` is always present.
	 *
	 * OWNERSHIP WINDOW: both caller handles are snapshotted (`doc.toJson()`, and
	 * `quill.toTree()` on a clone-cache miss) BEFORE the first await. The backend
	 * load below is a real suspension point — a multi-MB `import()` on first
	 * render — so reading the handles after it would race a caller that
	 * `free()`s them as soon as this call returns its promise ("null pointer
	 * passed to rust"). The snapshot makes that natural calling pattern correct.
	 *
	 * Clone lifetimes differ by design: the `doc` clone is TRANSIENT — freed in
	 * the `finally` of every call. The `quill` clone is CACHED per (engine,
	 * backend, canonical quill instance) and is NOT freed here; a `Quill`
	 * instance's contents never change after construction, so it is dropped with
	 * the canonical quill (WeakMap collection → wasm-bindgen weak-ref free) when
	 * the consumer replaces the instance. A cache miss materializes it once;
	 * subsequent calls reuse it.
	 *
	 * Both handles are this copy's — the public entry points check before they
	 * read `quill.backendId`. The crossing here is core→backend (always two
	 * memories, always as data); core→core is a duplicate install and never gets
	 * this far. See § "Handles from another copy".
	 * @param {string} backendId
	 * @param {Quill} quill
	 * @param {Document} doc
	 * @param {(ctx: { mod: any, engine: any, quill: any, doc: any }) => any} fn
	 */
	async #withClones(backendId, quill, doc, fn) {
		const docJson = doc.toJson();
		const quillTree = this.#quillClones.get(backendId)?.has(quill) ? null : quill.toTree();
		const { mod, engine } = await this.#resolveBackend(backendId);
		// The quill clone is cached (see #cachedQuillClone); only the per-call doc
		// clone is transient. Bring the doc clone + `fn` under one try so the doc
		// clone is freed even if a later step throws. The cached quill clone is
		// intentionally NOT freed here. `fn` MUST be synchronous — the doc clone is
		// freed as soon as it returns, so an async `fn` would have it freed
		// mid-flight.
		const backendQuill = this.#cachedQuillClone(mod, backendId, quill, quillTree);
		let backendDoc = null;
		try {
			backendDoc = mod.Document.fromJson(docJson);
			return fn({ mod, engine, quill: backendQuill, doc: backendDoc });
		} finally {
			backendDoc?.free();
		}
	}

	/**
	 * Render `doc` against `quill` in one shot, returning a `RenderResult`.
	 * Both handles are read synchronously before the first await, so the caller
	 * may `free()` them as soon as this call returns.
	 * @param {Quill} quill
	 * @param {Document} doc
	 * @param {object} [options] render options (`{ format, ppi, pages, producer }`)
	 * @returns {Promise<import('./runtime.js').RenderResult>}
	 */
	async render(quill, doc, options) {
		requireLocalQuill(quill, 'engine.render(quill, doc)');
		requireLocalDoc(doc, 'engine.render(quill, doc)');
		return this.#withClones(quill.backendId, quill, doc, ({ engine, quill: q, doc: d }) =>
			engine.render(q, d, options ?? undefined)
		);
	}

	/**
	 * Open a live render session (canvas preview / per-page paint / `apply`).
	 * The session is self-contained (it retains what it needs for `apply`), so
	 * the transient quill and document clones are freed before this returns;
	 * the caller owns the returned session and must `.free()` it. The `quill`
	 * and `doc` handles are read synchronously before the first await, so the
	 * caller may `free()` them as soon as this call returns.
	 * @param {Quill} quill
	 * @param {Document} doc
	 * @returns {Promise<LiveSession>}
	 */
	async open(quill, doc) {
		requireLocalQuill(quill, 'engine.open(quill, doc)');
		requireLocalDoc(doc, 'engine.open(quill, doc)');
		return this.#withClones(
			quill.backendId,
			quill,
			doc,
			({ mod, engine, quill: q, doc: d }) => new LiveSession(engine.open(q, d), mod)
		);
	}

	/**
	 * The output formats `quill`'s backend can emit. A cheap, non-failing,
	 * ALWAYS-free pre-render probe: it answers from the descriptor's required
	 * `formats` manifest — NO binary load and NO quill clone — depending only on
	 * `quill.backendId`. Stays `async` for API stability (it never awaits a load).
	 * @param {Quill} quill
	 * @returns {Promise<import('./runtime.js').OutputFormat[]>}
	 */
	async supportedFormats(quill) {
		requireLocalQuill(quill, 'engine.supportedFormats(quill)');
		const descriptor = this.#descriptorFor(quill.backendId);
		// Defensive copy so callers can't mutate the shared manifest.
		return descriptor.formats.slice();
	}

	/**
	 * Whether `quill`'s BACKEND can paint sessions to a canvas — a pre-session
	 * ESTIMATE, not a fact about any particular compile. Same ALWAYS-free probe
	 * as `supportedFormats`: answered from the descriptor's required `canvas`
	 * manifest, no load and no clone. A specific compile can still refuse to
	 * paint (e.g. a 0-page document), so this can answer `true` while the
	 * resulting `LiveSession.supportsCanvas` answers `false` — gate mounting a
	 * canvas UI on this, gate the actual `paint` call on the session's getter.
	 * @param {Quill} quill
	 * @returns {Promise<boolean>}
	 */
	async supportsCanvas(quill) {
		requireLocalQuill(quill, 'engine.supportsCanvas(quill)');
		const descriptor = this.#descriptorFor(quill.backendId);
		return descriptor.canvas;
	}
}

/**
 * Thin wrapper over a backend's live render session. Reads serve the current
 * compile; `apply(doc)` recompiles in place (transactional: on throw, reads
 * keep serving the last-good compile). The quill/document clones it was
 * opened from have already been freed — the session retains what `apply`
 * needs.
 *
 * Geometry reads (`regions`, `positionAt`, `locate`) resolve against the
 * current compile; anchoring a caret or selection across edits is the editor's
 * job (its own transaction mapping) — re-read geometry after each committed
 * `apply`.
 *
 * `paint` writes a COMPLETE page raster — all content visible, no caller-side
 * compositing — for every backend that supports canvas (Typst rasterizes
 * natively; pdfform rasterizes its pre-flattened page). See `runtime.d.ts`.
 */
export class LiveSession {
	/**
	 * @param {{ pageCount: number, backendId: string, supportsCanvas: boolean, warnings: any[], apply: Function, render: Function, regions: Function, pageSize: Function, paint: Function, free: Function }} inner backend-build LiveSession (typst or pdfform)
	 * @param {{ Document: { fromJson(json: string): any } }} mod the session's backend build, used to materialize `apply` documents in its linear memory
	 */
	constructor(inner, mod) {
		this.#inner = inner;
		this.#mod = mod;
	}
	#inner;
	#mod;

	/**
	 * Recompile the session against `doc` — the edit verb of a live preview.
	 * Transactional: on throw every read keeps serving the last-good compile.
	 * On success reads serve the new compile; repaint `dirtyPages ∩ visible`.
	 * @param {Document} doc
	 * @returns {import('./runtime.d.ts').ChangeSet}
	 */
	apply(doc) {
		requireLocalDoc(doc, 'session.apply(doc)');
		let backendDoc = null;
		try {
			backendDoc = this.#mod.Document.fromJson(doc.toJson());
			return this.#inner.apply(backendDoc);
		} finally {
			backendDoc?.free();
		}
	}

	get pageCount() {
		return this.#inner.pageCount;
	}
	get backendId() {
		return this.#inner.backendId;
	}
	/**
	 * `true` iff `paint`/`pageSize` will succeed for THIS compile — the
	 * authoritative answer, derived from the session's canvas seam, so it can
	 * never disagree with what `paint` actually does. This can be `false` even
	 * when `Engine.supportsCanvas` answered `true` for the same `quill` (that
	 * probe is a pre-session backend estimate; e.g. a canvas-capable backend
	 * compiled to a 0-page document has nothing to paint). Re-check this getter
	 * after `open()` rather than relying on the engine hint alone.
	 * @returns {boolean}
	 */
	get supportsCanvas() {
		return this.#inner.supportsCanvas;
	}
	get warnings() {
		return this.#inner.warnings;
	}

	/** @param {object} [options] */
	render(options) {
		return this.#inner.render(options ?? undefined);
	}

	/**
	 * Schema-field geometry for this compiled session — one region per
	 * schema-bound field, keyed on its quill schema field path. A session-level
	 * query (no render); read it to place field overlays / cross-navigation over
	 * a `paint`-ed canvas.
	 * @returns {import('./runtime.d.ts').FieldRegion[]}
	 */
	regions() {
		return this.#inner.regions();
	}

	/**
	 * The whole-field highlight boxes for `field` — one union rect per page,
	 * over the field's `span`-bearing content segments. Owns the union
	 * `regions()` leaves derived (span-filter + per-page union), so a "highlight
	 * the focused field" consumer stops reimplementing it. Content only: a field
	 * placed solely as a scalar reference or a bound widget returns `[]` — its
	 * box is a single `regions()` rect.
	 * @param {string} field
	 * @returns {import('./runtime.d.ts').FieldRegion[]}
	 */
	fieldBoxes(field) {
		return this.#inner.fieldBoxes(field);
	}

	/**
	 * The schema field whose content is under a point on `page` — the forward
	 * (click → field) direction, resolving *every* placement, not just the first
	 * that `regions` enumerates. `x`/`y` are PDF points with a bottom-left origin
	 * (the `FieldRegion.rect` space). See `runtime.d.ts` for the click-to-point
	 * inverse transform.
	 * @param {number} page
	 * @param {number} x
	 * @param {number} y
	 * @returns {string | undefined}
	 */
	fieldAt(page, x, y) {
		return this.#inner.fieldAt(page, x, y);
	}

	/**
	 * @param {number} page
	 * @param {number} x
	 * @param {number} y
	 * @returns {import('./runtime.d.ts').ContentHit | undefined}
	 */
	positionAt(page, x, y) {
		return this.#inner.positionAt(page, x, y);
	}

	/**
	 * @param {string} field
	 * @param {number} pos
	 * @returns {import('./runtime.d.ts').FieldRegion | undefined}
	 */
	locate(field, pos) {
		return this.#inner.locate(field, pos);
	}

	/** @param {number} page */
	pageSize(page) {
		return this.#inner.pageSize(page);
	}

	/**
	 * Paint `page` into a 2D canvas context. The painted raster is COMPLETE —
	 * all page content visible, no caller-side compositing — for both the Typst
	 * and pdfform backends. See `runtime.d.ts` for the DPR/clamp math and the
	 * region-overlay coordinate transform.
	 * @param {CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D} ctx
	 * @param {number} page
	 * @param {object} [options]
	 */
	paint(ctx, page, options) {
		return this.#inner.paint(ctx, page, options);
	}

	free() {
		this.#inner.free();
	}
}

// ── Typed-writer sugar: bind the quill once ─────────────────────────────────
// Rust exposes `quill.writer(&mut doc)` so a caller issues bare `set` / `set_all`
// without threading the schema per write. The WASM `commit*` verbs can't borrow
// like that — a `Document` carries only a `$quill` REFERENCE, not the resolved
// schema, so each `commit*` method takes the `quill` handle as its first
// argument. These pure-JS classes restore the Rust ergonomics: bind `quill` +
// `doc` once, then issue `set` / `setAll` / `card(i).set`.
//
// They hold JS references to the caller's EXISTING handles — no WASM object of
// their own, no `free()` burden, no second owner of either handle — and every
// write delegates straight to the underlying `commit*` verb: a schema field is
// typed-committed (coerced to canonical form, mismatch throws now), and a name
// the schema does not declare throws `UnknownField` rather than falling to the
// opaque store — on the typed path an undeclared name is a typo. Opaque storage
// stays available through the raw addressed `Document.storeField` verb.

/**
 * A {@link Document} bound to its {@link Quill} for typed writes — the JS twin
 * of Rust's `quill.writer(&mut doc)`. Writes target the main card; use
 * {@link card} for a composable card. Holds both handles by reference and owns
 * neither, so there is nothing to `free()`.
 */
export class DocumentWriter {
	#quill;
	#doc;
	/**
	 * @param {Quill} quill the schema source for typed commits
	 * @param {Document} doc the document to mutate, held by reference (not owned)
	 */
	constructor(quill, doc) {
		requireLocalQuill(quill, 'quill.writer(doc)');
		requireLocalDoc(doc, 'quill.writer(doc)');
		this.#quill = quill;
		this.#doc = doc;
	}
	/** The bound document — the same instance passed in, mutated in place. */
	get document() {
		return this.#doc;
	}
	/**
	 * Typed-commit one main-card field (strict coerce, mismatch throws now).
	 * Throws `UnknownField` for a name the schema does not declare.
	 * @param {string} name
	 * @param {unknown} value
	 * @returns {void}
	 */
	set(name, value) {
		return this.#doc._commitField(this.#quill, name, value);
	}
	/**
	 * Typed-commit several main-card fields atomically — nothing is applied on
	 * error (throws a per-field diagnostic bundle, including an `UnknownField`
	 * for each undeclared name).
	 * @param {Record<string, unknown>} fields
	 * @returns {void}
	 */
	setAll(fields) {
		return this.#doc._commitFields(this.#quill, MAIN_CARD_ADDR, fields);
	}
	/**
	 * Set the main body from markdown (edit semantics: surviving anchors rebase),
	 * discarding the text delta — the receipt-free body write. Call
	 * `doc.revise({}, md)` for the {@link Delta} receipt.
	 * @param {string} markdown
	 * @returns {void}
	 */
	setBody(markdown) {
		this.#doc.revise({}, markdown);
	}
	/**
	 * Revise the richtext main-card field `name` from markdown — typed *and*
	 * anchor-preserving. Surviving anchors rebase, then the diffed result is
	 * schema-conformed (`richtext(inline)` rejects a multi-block result). Throws
	 * `UnknownField` for a name the schema does not declare. Returns the text
	 * {@link Delta}.
	 * @param {string} name
	 * @param {string} markdown
	 * @returns {import('../core/wasm.js').Delta}
	 */
	reviseField(name, markdown) {
		return this.#doc._reviseField(this.#quill, name, markdown);
	}
	/**
	 * Build a composable card of `kind`, typed-commit `fields` onto it, set its
	 * body from optional markdown, and place it — the fused `makeCard` + typed
	 * commit + insertion. `at` picks the position: omitted appends, a number
	 * inserts at that index (`0..=cardCount`), so a positioned typed insert is one
	 * atomic call rather than `addCard` + `moveCard`. Transactional: the card is
	 * committed in full before it joins the document, so a rejected field (throws
	 * a per-field diagnostic bundle, `UnknownField` per undeclared name) or an
	 * invalid kind/body/position leaves the document untouched.
	 * @param {string} kind
	 * @param {Record<string, unknown>} [fields]
	 * @param {string} [body]
	 * @param {number} [at] insertion index; appends when omitted
	 * @returns {void}
	 */
	addCard(kind, fields, body, at) {
		return this.#doc._addCard(this.#quill, kind, fields, body, at);
	}
	/**
	 * Remove the composable card at `index`, returning it (or `undefined` if the
	 * index is out of range) — the writer spelling of `Document.removeCard`.
	 * @param {number} index
	 * @returns {import('../core/wasm.js').Card | undefined}
	 */
	removeCard(index) {
		return this.#doc.removeCard(index);
	}
	/**
	 * A {@link CardWriter} bound to the composable card at `index`. Index
	 * validity is checked lazily by the underlying write (it throws
	 * `IndexOutOfRange` at commit time), so an out-of-range index does not throw
	 * here.
	 *
	 * The cursor is ephemeral — bind, write, discard. It holds `index`, not the
	 * card: a `removeCard`/`addCard` between binding and writing silently
	 * retargets it. For durable addressing stamp `$id` and re-resolve the index
	 * at write time.
	 * @param {number} index
	 * @returns {CardWriter}
	 */
	card(index) {
		return new CardWriter(this.#quill, this.#doc, index);
	}
}

/**
 * A single composable card bound to its {@link Quill} for typed writes, from
 * {@link DocumentWriter.card}. Same `set` / `setAll` verbs as
 * {@link DocumentWriter}, targeting the card at its bound index.
 */
export class CardWriter {
	#quill;
	#doc;
	#index;
	/**
	 * @param {Quill} quill the schema source
	 * @param {Document} doc the document to mutate, held by reference (not owned)
	 * @param {number} index the composable card's index
	 */
	constructor(quill, doc, index) {
		requireLocalQuill(quill, 'writer.card(index)');
		requireLocalDoc(doc, 'writer.card(index)');
		this.#quill = quill;
		this.#doc = doc;
		this.#index = index;
	}
	/** The bound card index. */
	get index() {
		return this.#index;
	}
	/**
	 * The bound card's `$kind` (empty string when it carries none), read through
	 * the document — mirrors core `CardWriter::kind()`. Ephemeral like the cursor
	 * itself: throws `IndexOutOfRange` if the bound index is out of range.
	 * @returns {string}
	 */
	get kind() {
		return this.#doc.card(this.#index).kind;
	}
	/**
	 * Typed-commit one field on this card, addressed at `{ card, field }`. Throws
	 * `UnknownField` for an undeclared name and `IndexOutOfRange` if the bound
	 * index is out of range.
	 * @param {string} name
	 * @param {unknown} value
	 * @returns {void}
	 */
	set(name, value) {
		return this.#doc._commitField(this.#quill, { card: this.#index, field: name }, value);
	}
	/**
	 * Typed-commit several fields on this card atomically, addressed at
	 * `{ card }`. Throws a per-field diagnostic bundle on error and
	 * `IndexOutOfRange` if the bound index is out of range.
	 * @param {Record<string, unknown>} fields
	 * @returns {void}
	 */
	setAll(fields) {
		return this.#doc._commitFields(this.#quill, { card: this.#index }, fields);
	}
	/**
	 * Set this card's body from markdown (edit semantics), discarding the delta —
	 * the card twin of {@link DocumentWriter.setBody}.
	 * @param {string} markdown
	 * @returns {void}
	 */
	setBody(markdown) {
		this.#doc.revise({ card: this.#index }, markdown);
	}
	/**
	 * Revise the richtext field `name` on this card from markdown — typed *and*
	 * anchor-preserving; the card twin of {@link DocumentWriter.reviseField}.
	 * Throws `UnknownField` for an undeclared name and `IndexOutOfRange` if the
	 * bound index is out of range. Returns the text {@link Delta}.
	 * @param {string} name
	 * @param {string} markdown
	 * @returns {import('../core/wasm.js').Delta}
	 */
	reviseField(name, markdown) {
		return this.#doc._reviseField(this.#quill, { card: this.#index, field: name }, markdown);
	}
}

// ── `quill.writer(doc)` — the typed front door ──────────────────────────────
// The schema-bound writer: bind the quill's schema to a document and issue bare
// typed writes. Mirrors core's `quill.writer(&mut doc)` — the schema grants the
// typing, so the quill (not the document) is the factory. Patched onto the
// re-exported `Quill` prototype rather than wrapped: `Quill === CoreQuill`
// stays true (the identity invariant above); this only adds a method that
// constructs the pure-JS writer, which owns no WASM handle.
/**
 * A {@link DocumentWriter} binding this quill's schema to `doc` for typed
 * writes — the documented front door. The returned writer holds both handles by
 * reference and owns neither, so there is nothing to `free()`. Ephemeral by
 * convention: bind, write, discard.
 * @this {Quill}
 * @param {Document} doc the document to mutate, held by reference (not owned)
 * @returns {DocumentWriter}
 */
Quill.prototype.writer = function writer(doc) {
	return new DocumentWriter(this, doc);
};

// ── Typed-reader sugar: the schema-plane read surface ──────────────────────────
// The read twin of the writer above. The transport `Document.getStored` is schema-free
// — a `Document` cannot say which fields are richtext, so an unknown field name
// reads back `undefined` rather than as the typo it is. Binding the quill's
// schema (`_readerGet` takes the handle, like the `commit*` verbs) lets one `get`
// interpret by declared type: a richtext field to markdown, a plaintext field to
// its literal text, every other type verbatim, and an unknown name throws
// `UnknownField`. A field's markdown lives here, not on the body-only
// `getMarkdown`. Like the writer classes these hold the caller's handles by
// reference, own no WASM object, and have nothing to `free()`.

/**
 * A {@link Document} bound to its {@link Quill} for typed reads — the JS twin of
 * Rust's `quill.reader(&doc)` and the read counterpart of {@link DocumentWriter}.
 * Reads target the main card; use {@link card} for a composable card. Holds both
 * handles by reference and owns neither, so there is nothing to `free()`.
 */
export class DocumentReader {
	#quill;
	#doc;
	/**
	 * @param {Quill} quill the schema source for interpreted reads
	 * @param {Document} doc the document to read, held by reference (not owned)
	 */
	constructor(quill, doc) {
		requireLocalQuill(quill, 'quill.reader(doc)');
		requireLocalDoc(doc, 'quill.reader(doc)');
		this.#quill = quill;
		this.#doc = doc;
	}
	/** The bound document — the same instance passed in. */
	get document() {
		return this.#doc;
	}
	/**
	 * Read the value at `addr`, interpreted by its declared type: a richtext field
	 * to markdown, every other type verbatim. A bare string is `Addr` shorthand
	 * for `{ field }`; an absent `addr.field` reads the body markdown. `undefined`
	 * for an absent field; throws `UnknownField` for a name the schema does not
	 * declare, `FieldRichtextDecode` for a richtext field holding an undecodable
	 * value, and `IndexOutOfRange` for a bad `addr.card`.
	 * @param {import('../core/wasm.js').Addr | string} addr
	 * @returns {unknown}
	 */
	get(addr) {
		return this.#doc._readerGet(this.#quill, addr);
	}
	/**
	 * The main body's markdown — the quill-free body read (a body's type is a
	 * format fact, not a schema fact). Equivalent to `get({})`.
	 * @returns {string}
	 */
	getBody() {
		return this.#doc._readerGet(this.#quill, {});
	}
	/**
	 * A {@link CardReader} bound to the composable card at `index`. Index validity
	 * is checked lazily by the underlying read (it throws `IndexOutOfRange` at read
	 * time), so an out-of-range index does not throw here. Ephemeral like the
	 * writer cursor —
	 * it holds `index`, not the card, so a `removeCard`/`addCard` between binding
	 * and reading silently retargets it.
	 * @param {number} index
	 * @returns {CardReader}
	 */
	card(index) {
		return new CardReader(this.#quill, this.#doc, index);
	}
}

/**
 * A single composable card bound to its {@link Quill} for typed reads, from
 * {@link DocumentReader.card}. Same `get` / `getBody` verbs as
 * {@link DocumentReader}, reading the card at its bound index.
 */
export class CardReader {
	#quill;
	#doc;
	#index;
	/**
	 * @param {Quill} quill the schema source
	 * @param {Document} doc the document to read, held by reference (not owned)
	 * @param {number} index the composable card's index
	 */
	constructor(quill, doc, index) {
		requireLocalQuill(quill, 'reader.card(index)');
		requireLocalDoc(doc, 'reader.card(index)');
		this.#quill = quill;
		this.#doc = doc;
		this.#index = index;
	}
	/** The bound card index. */
	get index() {
		return this.#index;
	}
	/**
	 * The bound card's `$kind` (empty string when it carries none), read through
	 * the document. Throws `IndexOutOfRange` if the bound index is out of range.
	 * @returns {string}
	 */
	get kind() {
		return this.#doc.card(this.#index).kind;
	}
	/**
	 * Read the field `name` on this card, interpreted by its declared type,
	 * addressed at `{ card, field }`. `undefined` when absent; throws
	 * `UnknownField` for an undeclared name and `IndexOutOfRange` for a bad index.
	 * @param {string} name
	 * @returns {unknown}
	 */
	get(name) {
		return this.#doc._readerGet(this.#quill, { card: this.#index, field: name });
	}
	/**
	 * This card's body markdown — the card twin of {@link DocumentReader.getBody}.
	 * @returns {string}
	 */
	getBody() {
		return this.#doc._readerGet(this.#quill, { card: this.#index });
	}
}

// ── `quill.reader(doc)` — the schema-plane read front door ─────────────────────
// The read twin of `quill.writer(doc)`, patched onto the same re-exported `Quill`
// prototype (the `Quill === CoreQuill` identity invariant holds — this only adds
// a method constructing the pure-JS reader, which owns no WASM handle).
/**
 * A {@link DocumentReader} binding this quill's schema to `doc` for interpreted
 * reads — the read front door, mirroring core's `quill.reader(&doc)`. The returned
 * reader holds both handles by reference and owns neither, so there is nothing to
 * `free()`. Ephemeral by convention: bind, read, discard.
 * @this {Quill}
 * @param {Document} doc the document to read, held by reference (not owned)
 * @returns {DocumentReader}
 */
Quill.prototype.reader = function reader(doc) {
	return new DocumentReader(this, doc);
};
