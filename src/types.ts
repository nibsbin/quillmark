/**
 * Opaque to the registry. Defined and validated by @quillmark/wasm.
 * Currently: the JSON structure returned by loaders.fromZip() or equivalent
 * filesystem read (template files, assets, fonts, Typst packages).
 */
export type QuillData = unknown;

/** Metadata about a quill, extracted from Quill.yaml or manifest. */
export interface QuillMetadata {
	name: string;
	version: string;
	description?: string;
}

/** Manifest listing all available quills from a source. */
export interface QuillManifest {
	quills: QuillMetadata[];
}

/** A resolved quill bundle ready for engine registration. */
export interface QuillBundle {
	name: string;
	version: string;
	/** Opaque payload passed to engine.registerQuill().
	 *  Shape is defined by @quillmark/wasm — the registry passes it through untouched.
	 *  Currently: the nested file-tree structure expected by `registerQuill()`. */
	data: QuillData;
	metadata: QuillMetadata;
}

/** Pluggable backend that knows how to list and fetch Quills from a specific location. */
export interface QuillSource {
	getManifest(): Promise<QuillManifest>;
	loadQuill(name: string, version: string): Promise<QuillBundle>;
}

/**
 * Info returned by the engine after registering or resolving a quill.
 * Matches the shape returned by `@quillmark/wasm`'s `Quillmark` class.
 */
export interface QuillInfo {
	name: string;
	backend: string;
	metadata: Record<string, unknown>;
	example?: string;
	schema: Record<string, unknown>;
	defaults: Record<string, unknown>;
	examples: Record<string, unknown[]>;
	supportedFormats: string[];
}

/**
 * Minimal interface for the @quillmark/wasm engine instance.
 * The registry only calls these methods — it never imports or instantiates the engine.
 *
 * Structurally compatible with `@quillmark/wasm`'s `Quillmark` class so
 * you can pass a `Quillmark` instance directly without adapters.
 */
export interface QuillmarkEngine {
	registerQuill(quill_json: unknown): QuillInfo;
	resolveQuill(quill_ref: string): QuillInfo | null;
	listQuills(): string[];
}

