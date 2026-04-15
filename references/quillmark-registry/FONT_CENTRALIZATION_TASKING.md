# Font Centralization — Tasking

## Goal

Eliminate font duplication in published Quill bundles by moving font bytes out
of individual Quills and into a shared, content-addressed store.

Fonts currently dominate bundle size (examples: `classic_resume@0.1.0` is
2.1 MB total, 2.1 MB of which are fonts; `usaf_memo@0.1.0` and `0.2.0` ship
byte-identical 551 KB font sets). Most Quills should shrink by >90% after
this change.

Fonts are first-class across all backends. No backend-specific machinery
lives in the font pipeline.

## Design decisions

### Identity and storage

- **SHA-256 of the raw font bytes is the font's identity.** The hash is the
  source of truth at runtime.
- The store is **content-addressed**: `store/<sha256>` serves raw font bytes.
- The store is **persisted** (append-only in practice) and **write-open**
  (any publisher can upload any hash; authors carry their own license
  exposure).
- **Bytes are served as-is.** No zipping, no format conversion (Typst does
  not support WOFF2). Transport compression is the CDN's responsibility.
- Idempotent upload: writing the same hash twice is a no-op.
- **No garbage collection** in v1. Orphaned bytes persist.

### Publish-time behavior

- Files matching `*.ttf`, `*.otf`, `*.woff`, `*.woff2` anywhere in a Quill
  source tree are **automatically stripped** from the published ZIP.
- For each stripped font, the publisher:
  1. Hashes the bytes.
  2. Sniffs family/style/weight from the font's `name` and `OS/2` tables.
  3. Uploads bytes to the store (idempotent by hash).
  4. Writes an entry into that Quill's `Quill.yaml` `fonts:` section.
- The generated `Quill.yaml` manifest is **committed** — consistent with how
  Markdown templates are already handled.
- The strip list is **separate from `.quillignore`**. `.quillignore` remains
  a load-time mechanism in Rust. The publish-time strip list lives in the
  Node registry tooling.

### Load-time / runtime behavior

- `.quillignore` and the in-memory `FileTreeNode` model are unchanged.
- Local dev rendering ignores the central store. Authors can drop `.ttf`
  files into `assets/` and they load as today. Strip-on-publish is the only
  place fonts are touched automatically.
- At runtime with a published Quill, the engine reads `fonts:` from
  `Quill.yaml`, fetches each hash via a `FontProvider`, and registers the
  resolved fonts in the backend.
- The Typst backend's existing font-scan path at
  `crates/backends/typst/src/world.rs:156-207` continues to handle any font
  bytes still present in the Quill — notably fonts bundled inside
  third-party `@preview/...` packages, which cannot be stripped at publish.

### Manifest schema

Added to `Quill.yaml`:

```yaml
fonts:
  - sha256: <hex>
    family: "Inter"
    style: "Regular"
    weight: 400
```

Annotations (`family`, `style`, `weight`) exist for **human-readable diffs
and publish output only**. Runtime ignores them and reads metadata from the
font bytes directly. On any mismatch, **hash wins**.

### Publish-time output

Dedup signal is the payoff message — show it clearly. Counts are a local
walk of the source tree being published; no store query required.

Suggested shape:

```
fonts:
  Inter Regular      abc123...  used by 14 quills
  Inter Bold         def456...  used by 12 quills
  EB Garamond        789abc...  used by 1 quill

bundle: stripped 47 MB across 22 quills
```

### Same-family conflicts (deferred)

If two files in one Quill both resolve to the same family/style (e.g. one in
`assets/`, one in `packages/<pkg>/fonts/`), v1 does not error. Font
insertion order into `FontBook` must be **deterministic** (sort discovered
paths) so whichever wins is reproducible across runs. Real conflict
detection and author-facing errors are a later task.

## Responsibility split

**Rust (`quillmark` workspace) — render-time only**

- Add `FontManifest` to `quillmark-core` parsing the `fonts:` section of
  `Quill.yaml`.
- Add a `FontProvider` trait in `quillmark-core`:
  `fetch(sha256) -> Bytes` (sync/async variants as appropriate). Concrete
  impls: native HTTP, filesystem cache, WASM JS-callback shim (mirror the
  existing pattern used for ZIP fetching).
- Extend the Quill loader so that given a `Quill` and a `FontProvider` it
  resolves declared fonts before backend construction.
- Wire the Typst backend to register resolved fonts in `FontBook` alongside
  its existing file-scan output. Embedded fallback fonts at
  `crates/backends/typst/src/world.rs:43-64` stay as last-resort.
- **Rust never hashes fonts for publish, never generates manifests, never
  strips files.**

**Node (`quillmark-registry`) — publish-time and transport**

- Walk source trees; hash fonts; sniff family/style/weight via fontkit or
  opentype.js (publish-only dep).
- Write the `fonts:` section back into each source-tree `Quill.yaml`.
- Enforce the implicit strip list when packaging ZIPs.
- Manage the content-addressed store: write `store/<hash>`; serve as
  static files.
- Runtime path: fetch ZIPs, fetch font bytes by hash, hand bytes into Rust
  via the `FontProvider` callback.

Schema is the only cross-language contract. It is small and stable:
hash + family + style + weight.

## Font injection across the language boundary

Rust exposes a sync `FontProvider` trait in `quillmark-core`; hosts supply
a concrete impl and hand it to Rust at Quill construction. The trait is
sync to match Typst's own sync font loading and avoid async-in-WASM
complexity.

```rust
trait FontProvider {
    fn fetch(&self, sha256: &str) -> Option<Bytes>;
}
```

**Injection API (Rust):**

```rust
Quill::from_json_with_fonts(json, provider)
Quill::from_path_with_fonts(path, provider)
```

The loader reads the `fonts:` manifest, pulls bytes via the provider, and
attaches them to the `Quill`. From the Typst backend's perspective,
resolved fonts look like any other font bytes — it does not need to know a
provider was involved.

**Node / WASM flow:**

1. Fetch the Quill ZIP; unpack; read `Quill.yaml` `fonts:` section.
2. Fetch each hash from `store/<hash>` (honoring HTTP cache).
3. Build a `Map<string, Uint8Array>` of hash → bytes.
4. Pass it through the WASM boundary alongside the Quill JSON:
   `Quill.fromJson(quillJson, fontMap)`.
5. Rust wraps the map in a `MapProvider` impl of `FontProvider`.

All font bytes are eager from Rust's perspective — pre-resolved before
`QuillWorld::new` runs. This matches today's semantics (where ZIP-inlined
font bytes are also all-in-memory by the time the backend sees them).

**Native flow:**

Host supplies whichever concrete impl fits:

- `HttpFontProvider { base_url, cache }` — blocking HTTP against the
  registry with local disk cache.
- `DirFontProvider { path }` — reads `store/<hash>` from a local
  directory.
- `MapProvider` — in-memory, useful for tests.

Native consumers may pre-fetch eagerly or lazy-fetch at render time; the
trait does not dictate.

**No shared process-wide store in v1.** Providers are per-load. Consumers
that want cross-Quill font caching (long-running servers) can implement a
`FontProvider` that delegates to a shared cache internally — that is a
consumer concern, not a core API concern.

## Loose ends (implementer's discretion)

- **Node font parser**: fontkit, opentype.js, and fonteditor-core are all
  acceptable if they read family/style/weight reliably.
- **Store file layout**: flat `store/<hash>` or sharded `store/<ab>/<cdef...>`
  — implementer's call.
- **URL / filename extension** in the store: not required for correctness
  (font magic bytes identify format); include or omit based on CDN
  needs.
- **`FontProvider` caching strategy**: per-process vs. per-request depends
  on consumer context. The trait should accommodate both; pick sensible
  defaults.
- **Fixture migration**: writing a one-time `fonts import` command to
  populate manifests for the existing four fixture Quills is reasonable but
  optional — manual migration is fine if faster.

## Non-goals

- Changing the rendering API beyond wiring in the provider.
- Replacing `.quillignore` or altering load-time file-tree behavior.
- Named font packs, semver-versioned fonts, a curated registry, or license
  enforcement.
- Transport-layer optimization (compression, HTTP/2, etc.) — CDN territory.
- HTML / LaTeX / other backends. The `FontProvider` + manifest is
  deliberately backend-agnostic, but only the Typst consumption path is
  landed here.

## Relevant existing code

- Font loading today:
  `crates/backends/typst/src/world.rs:156-207` (extension-based scan across
  `assets/fonts/*`, `assets/*`, `packages/**`).
- `QuillWorld` construction:
  `crates/backends/typst/src/world.rs:18-103`.
- Embedded fallback fonts:
  `crates/backends/typst/src/world.rs:43-64`.
- Quill config schema:
  `crates/core/src/quill/config.rs:16-40`.
- In-memory file tree + `.quillignore`:
  `crates/core/src/quill/tree.rs:7-18`,
  `crates/core/src/quill/load.rs:11-41`.
- Registry ZIP packager:
  `references/quillmark-registry/src/sources/file-system-source.ts:206-233`.
- Registry bundle format:
  `references/quillmark-registry/src/bundle.ts:23-31`.
