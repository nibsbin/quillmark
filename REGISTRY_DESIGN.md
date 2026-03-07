# @quillmark/registry

**TL;DR:** New `@quillmark/registry` package replaces `@quillmark/web-utils` and the build-time packaging scripts. Provides a unified API for discovering, loading, packaging, and registering Quills with the WASM engine across browser and Node.js environments. The registry pushes resolved Quills to the engine via the existing `registerQuill()` API — the engine is unchanged.

## Dependencies

| Package           | Version   | Role                                                                                                                                                                                           |
| ----------------- | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@quillmark/wasm` | `>=0.39.0` | Peer dependency. Consumer provides an engine instance satisfying the `QuillmarkEngine` interface. The registry calls `registerQuill()`, `resolveQuill()`, and `listQuills()` on it.             |
| `jszip`           | `^3.10.1` | `HttpSource`: unzips fetched quill archives. `FileSystemSource.packageForHttp()`: zips quill directories for static hosting.                                                                   |

`@quillmark/wasm` is a **peer dependency** — the consumer provides the engine instance. The registry never imports or instantiates `Quillmark` itself; it only calls methods on the instance it receives.

The registry defines a `QuillmarkEngine` interface for the subset of engine methods it actually uses (`registerQuill`, `resolveQuill`, `listQuills`). The consumer passes any object satisfying this interface — typically a `Quillmark` instance from `@quillmark/wasm`.

No dependency on `@quillmark/web-utils`. The registry replaces it entirely — `loaders.fromZip()` logic moves into `HttpSource` internals.

## Problem

Quill loading is split across three concerns with no shared abstraction:

1. **Build script** (`scripts/package-quills.js`) — reads `Quill.yaml`, zips directories, writes `manifest.json`
2. **Client service** — fetches zips via HTTP, calls `web-utils.loaders.fromZip()`, registers with engine
3. **Server service** — same as client but fetches from its own HTTP origin (circular)

Each consumer reimplements discovery, loading, and registration. `@quillmark/web-utils` only handles zip unpacking — everything else is bespoke. The server path is especially wasteful — it HTTP-fetches zips from itself to get data that's already on disk.

## Core Abstractions

```
QuillSource  →  QuillRegistry(source, engine)  →  engine.registerQuill()
(where quills live)   (discovery + versioning + caching)       (specific Quillmark instance)
```

**QuillSource** — pluggable backend that knows how to list and fetch Quills from a specific location.

| Source             | Environment       | Reads from                                                 |
| ------------------ | ----------------- | ---------------------------------------------------------- |
| `FileSystemSource` | Node.js           | Versioned local directory (e.g., `quills/{name}/{version}/`) |
| `HttpSource`       | Browser + Node.js | Base URL serving zips + manifest                           |

**QuillRegistry** — orchestrates sources, resolves versions, caches loaded Quills. The registry is constructed with both a `QuillSource` and a `QuillmarkEngine` instance (via `QuillRegistryOptions`). On `resolve()`, it fetches quill data from the source and pushes it to that engine instance via `registerQuill()`. The registry is the single source of truth for discovery and loading; the engine is the single source of truth for rendering.

## Integration Model

The registry is scoped to a specific engine instance. On `resolve()`, it fetches quill data from the source and registers it with that engine. Loading is lazy — quills are fetched and pushed to the engine on first `resolve()` call, not at construction time.

```
registry.resolve('usaf_memo')
  → engine.resolveQuill('usaf_memo')   // already loaded? return cached QuillInfo
  → if null: source.loadQuill()        // fetch from source
             engine.registerQuill(data) // push to engine
  → return QuillBundle
```

The registry uses `engine.resolveQuill()` as a fast check before hitting the source. This avoids redundant fetches when a quill is already registered — e.g., after a `preload()`, or when multiple documents reference the same quill.

**Why push (not pull):** The engine is compiled WASM with a working internal Quill store. Removing that would require engine changes for no benefit. The registry adds discovery, lazy loading, and caching on top — the engine doesn't need to know the registry exists.

**Lazy loading:** Quills are resolved on first `resolve()` call, not at startup. `preload()` is available for latency-sensitive paths.

## API Surface

### QuillSource

```ts
interface QuillSource {
	getManifest(): Promise<QuillManifest>;
	loadQuill(name: string, version?: string): Promise<QuillBundle>;
}
```

### QuillData

```ts
/**
 * Opaque to registry callers. Shape is defined and validated by @quillmark/wasm.
 * The registry passes this payload through to `registerQuill()` without inspecting it.
 * Current built-in sources produce the nested engine file-tree format (`{ files: ... }`)
 * where file nodes contain `{ contents: string | number[] }`.
 */
type QuillData = unknown;
```

### QuillBundle

```ts
interface QuillBundle {
	name: string;
	version: string;
	/** Payload passed to engine.registerQuill().
	 *  In practice: nested engine file-tree format (`{ files: ... }`). */
	data: QuillData;
	metadata: QuillMetadata;
}
```

### QuillInfo

```ts
/** Info returned by the engine after registering or resolving a quill. */
interface QuillInfo {
	name: string;
	backend: string; // Rendering backend (for example, "typst")
	metadata: Record<string, unknown>; // Engine-provided metadata; includes version
	example?: string;
	schema: Record<string, unknown>;
	defaults: Record<string, unknown>;
	examples: Record<string, unknown[]>;
	supportedFormats: string[];
}
```

Version is exposed by the engine in `metadata.version`.

### QuillRegistry

Constructed with a `QuillRegistryOptions` object containing the source and engine:

```ts
interface QuillRegistryOptions {
	source: QuillSource;
	engine: QuillmarkEngine;
}

class QuillRegistry {
	constructor(options: QuillRegistryOptions);

	// Discovery
	getManifest(): Promise<QuillManifest>;
	getAvailableQuills(): Promise<QuillMetadata[]>;

	// Loading — resolves from source, caches, and registers with engine
	resolve(ref: string): Promise<QuillBundle>;

	// Convenience
	preload(refs: string[]): Promise<void>;

	// State — delegates to engine.resolveQuill()
	isLoaded(name: string): boolean;
}
```

### QuillmarkEngine (interface the registry depends on)

The registry defines a minimal `QuillmarkEngine` interface for the engine methods it actually calls. This decouples the registry from the full `Quillmark` class — any object satisfying this interface works.

```ts
interface QuillmarkEngine {
	registerQuill(quill_json: unknown): QuillInfo;
	resolveQuill(quill_ref: string): QuillInfo | null; // "usaf_memo", "usaf_memo@2.1.0"
	listQuills(): string[]; // ["usaf_memo@1.0.0", "classic_resume@2.1.0"]
}
```

The full `Quillmark` class from `@quillmark/wasm` satisfies this interface and exposes additional methods (`unregisterQuill`, `getQuillInfo`, `getStrippedSchema`, `parseMarkdown`, `render`, etc.) that the registry does not depend on. App code calls those methods directly on the engine instance.

The registry calls `engine.registerQuill()` as part of `resolve()`, and uses `engine.resolveQuill()` to check whether a quill is already loaded before fetching from a source. App code calls `engine.getQuillInfo()` and `engine.render()` as before.

### Built-in Sources

**`FileSystemSource`** — Node.js only. Reads Quill directories from disk using a versioned layout:

```
quillsDir/
  usaf_memo/
    0.1.0/
      Quill.yaml     ← name: usaf_memo, version: 0.1.0
      template.typ
    1.0.0/
      Quill.yaml     ← name: usaf_memo, version: 1.0.0
      template.typ
  classic_resume/
    2.1.0/
      Quill.yaml
      template.typ
```

Each subdirectory of `quillsDir` is a quill name; subdirectories within are version directories. Each version directory must contain a `Quill.yaml` file. Name/version are derived from directory structure.

When `loadQuill()` is called without a version, the source resolves to the latest version by semver-sorting the version directories.

Also exposes `packageForHttp()` to zip quills and write a manifest for static hosting. Constructor takes a single `quillsDir` path.

**`HttpSource`** — Browser or Node.js. Fetches zips and manifest from any HTTP endpoint (local static directory, CDN, remote server). Constructed with `HttpSourceOptions`:

```ts
interface HttpSourceOptions {
	baseUrl: string; // Base URL serving zips + manifest
	manifest?: QuillManifest; // Optional pre-loaded manifest (for SSR bootstrap)
	fetch?: typeof globalThis.fetch; // Optional custom fetch (for testing / non-browser)
}
```

Appends `?v={version}` to zip URLs for cache-busting. Caches the manifest after first fetch.

## Static Hosting / Remote Serving

The registry owns the full lifecycle: packaging quills for static hosting and fetching them back.

### Packaging (FileSystemSource → static files)

`FileSystemSource` exposes a `packageForHttp(outputDir)` method that reads all quill directories, zips each one (with assets, fonts, Typst packages), and writes the zips plus a `manifest.json` to the output directory. This replaces `scripts/package-quills.js` entirely.

### Fetching (HttpSource ← static files)

`HttpSource` fetches the packaged zips and manifest from any HTTP endpoint. This supports local static serving, CDN hosting, and remote quill registries with the same interface.

## Error Handling

### Error Types

```ts
type RegistryErrorCode =
	| 'quill_not_found' // No quill with that name exists in any source
	| 'version_not_found' // Quill exists but requested version doesn't
	| 'load_error' // Source failed to fetch/parse quill data
	| 'source_unavailable'; // Network failure, filesystem error, etc.

class RegistryError extends Error {
	code: RegistryErrorCode;
	quillName?: string;
	version?: string;

	constructor(
		code: RegistryErrorCode,
		message: string,
		options?: { quillName?: string; version?: string; cause?: unknown },
	);
}
```

### Failure Semantics

- **`resolve()`** — throws `RegistryError` on any failure. Callers handle or propagate.
- **`preload()`** — **fail-fast**. If any quill fails to load, rejects immediately. Callers who want best-effort can call `resolve()` individually and catch per-quill.
- **`getManifest()`** — throws `RegistryError` with `source_unavailable` on network/filesystem failure.

## Caching

The registry maintains an in-memory cache of resolved `QuillBundle` objects, keyed by `name@version`.

- **Browser:** Cache lives for the page session. Invalidation via version-tagged URLs.
- **Server (request-scoped):** New registry per request, or shared with caching.
- **Server (long-running):** Shared registry with caching. Invalidate manually or recreate on deploy.

## Version Resolution

The registry owns version resolution. When a quill reference includes a version (e.g., `usaf_memo@0.1`), the registry resolves the exact match. When no version is specified, it resolves to latest available.

The resolution flow:

1. Check engine via `resolveQuill()` — return immediately if already registered
2. Check registry cache — return if cached
3. Ask source for the bundle (or throw `version_not_found` / `quill_not_found`)
4. Register with engine via `registerQuill()`

Future: version ranges, pinning, deprecation warnings — all live in the registry.

## What Moves Where

| Current location                     | Moves to                            |
| ------------------------------------ | ----------------------------------- |
| `web-utils.loaders.fromZip()`        | `HttpSource` internals              |
| `Quill.yaml` existence checks in build script | `FileSystemSource` |
| Quill versioning via single directory| `FileSystemSource` versioned layout |
| Manifest generation in build script  | `FileSystemSource.getManifest()`    |
| Zip packaging in build script        | `FileSystemSource.packageForHttp()` |
| `preloadQuills()` in client service  | `registry.preload()`                |
| `loadQuillZip()` in server service   | `FileSystemSource.loadQuill()`      |
| Name/version resolution              | Directory layout + registry logic   |
| `@quillmark/web-utils` dependency    | Removed entirely                    |

## What Stays Outside the Registry

- **Rendering, parsing, diagnostics** — `@quillmark/wasm` engine (unchanged)
- **`registerQuill()`, `getQuillInfo()`** — engine's existing API (unchanged)
- **Template management** — separate concern (templates are markdown files, not Quills)
- **Ephemeral documents** — app-level feature
- **Parse cache** — app-level LRU cache, orthogonal to registry

## Migration Path

1. Publish `@quillmark/registry` with `FileSystemSource`, `HttpSource`, `QuillRegistry`
2. Replace client and server quillmark services to resolve via registry, then use engine as before
3. Replace `scripts/package-quills.js` with `FileSystemSource.packageForHttp()`
4. Remove `@quillmark/web-utils` dependency
5. Update SSR layout to inject manifest into `HttpSource`
