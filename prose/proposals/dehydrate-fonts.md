# Font Dehydration — `@quillmark/registry` Tasking

## Goal

Move font bytes out of published Quill bundles into a shared,
content-addressed store. Fonts are 60–95% of bundle size today
(`classic_resume@0.1.0`: 2.1 MB total, 2.1 MB fonts; `usaf_memo@0.1.0`
and `0.2.0` ship byte-identical 551 KB font sets). Wire savings compound
across quills because the store deduplicates by content hash: Inter-Regular
is fetched and cached once regardless of how many quills embed it.

## Scope

**Fonts only.** Non-font assets stay inline (templates reference them by
path, making content-addressed substitution awkward). Typst packages stay
inline — after font stripping, remaining source (`.typ` files +
`typst.toml`) is negligible. The store URL shape is file-type-agnostic if
this changes later.

**The registry owns the full lifecycle.** `@quillmark/registry` dehydrates
at publish, serves the store, and rehydrates at load before handing a
complete bundle to Quillmark. Quillmark's rendering path sees no
centralization.

## Model: dehydrate at publish, rehydrate at load

A published Quill is a **dehydration** of its source tree: font files are
stripped, their bytes moved to the content-addressed store, and a sidecar
manifest records what was removed and where.

Loading a published Quill **rehydrates** the tree: the registry client
reads the manifest, fetches missing bytes from the store (parallel,
cache-first), reconstructs a complete in-memory file tree, and passes it
to Quillmark for compilation. The rehydrated bundle is indistinguishable
from the pre-strip source — the Typst backend sees a normal file tree and
requires no changes.

## Core decisions

- **Identity = MD5 of raw font bytes.** Dedup, not integrity.
- **Store is flat and content-addressed.** URL: `<base>/store/<md5-hex>`.
  Raw bytes, lowercase hex, no extension. Publisher filesystem mirrors
  the URL.
- **Persisted, write-open, idempotent uploads.** No GC in v1. No zipping,
  no format conversion (Typst does not support WOFF2). Transport
  compression is the CDN's job.
- **Strip everywhere at publish.** `*.ttf`, `*.otf`, `*.woff`, `*.woff2`
  are removed from the ZIP wherever they appear, including under
  `packages/**`.
- **`Quill.yaml` is never modified.** Author source stays clean.
- **Manifest is a sidecar inside the ZIP** — `fonts.json` at the ZIP root.
- **No font metadata sniffing.** Hash bytes, record paths. No font-parsing
  dependency required.
- **No store URL pinned in the manifest.** Consumers prepend their
  configured base URL so bundles remain portable across mirrors.

## Manifest: `fonts.json`

A dehydration record — nothing more. Maps each stripped path to its
content hash:

```json
{
  "version": 1,
  "files": {
    "assets/fonts/Inter-Regular.ttf": "3f2a8c1d9e4b5a7f0c8d6e3a1b4f9c2d",
    "packages/ttq-classic-resume/fonts/Inter-Regular.ttf": "3f2a8c1d9e4b5a7f0c8d6e3a1b4f9c2d",
    "assets/fonts/Inter-Bold.ttf": "a7e3b2d5f0c8e1a4b7d2f5c9e0a3b6d1"
  }
}
```

- **`files`**: path → md5-hex. One entry per stripped file. Identical bytes
  at multiple source paths produce multiple entries with the same hash;
  rehydration faithfully reproduces the tree.

## Schema

The `FontManifest` type is defined in Rust and is the canonical contract.
The registry validates `fonts.json` against the generated JSON Schema both
when writing it at publish time and when reading it at load time — drift
in either direction fails CI.

- **Type definition**: `crates/core/src/fonts.rs` (`quillmark-core`)
- **JSON Schema**: `crates/core/schemas/fonts-manifest.schema.json`
- **Shared test fixtures**: `crates/core/tests/fixtures/fonts-manifest/`

Node validates with `ajv` (or equivalent). The fixtures are parsed by
Rust CI and validated by Node CI against the schema; both sides regress
against the same inputs.

## Publish flow

Triggered when a Quill source tree is packaged for the registry.

1. Walk the source tree. For every file whose extension is `ttf`, `otf`,
   `woff`, or `woff2`, compute the MD5 of its raw bytes.
2. Collect the unique set of hashes. For each, upload bytes to
   `<store-base>/store/<md5-hex>` (PUT, idempotent — skip if already
   present).
3. Build the `files` map: `{ [path]: md5-hex }` for every matched file.
4. Validate the manifest against `fonts-manifest.schema.json`.
5. Build the ZIP — include `fonts.json` at the root, exclude every matched
   font file.

Print a dedup summary after publish (counts are a local walk — no store
query required):

```
fonts:
  Inter-Regular.ttf   3f2a8c…  used by 14 quills
  Inter-Bold.ttf      a7e3b2…  used by 12 quills
  EBGaramond.ttf      789abc…  used by 1 quill

bundle: stripped 47 MB across 22 quills
```

## Load flow

Triggered when the registry client fetches a Quill bundle to hand to
Quillmark.

1. Fetch and unpack the Quill ZIP in memory.
2. Check for `fonts.json` at the ZIP root.
   - **Absent** (non-dehydrated bundle): skip to step 6.
3. Parse `fonts.json` and validate against `fonts-manifest.schema.json`.
4. Collect the unique set of MD5 hashes from `files` values.
5. Fetch each hash from `<store-base>/store/<md5-hex>` in **parallel**.
   Cache fetched bytes in a session-level `Map<md5, Uint8Array>` so the
   same bytes aren't re-fetched when loading additional quills.
   **Fail the load if any hash cannot be resolved.**
6. Reconstruct the complete in-memory file tree: for every `[path, md5]`
   entry in `files`, insert the resolved bytes at `path`.
7. Hand the hydrated tree to Quillmark via the registration API. The
   bundle is a complete Quill — no font manifest, no missing files.

## Cross-quill caching

The session-level `Map<md5, Uint8Array>` is the main performance payoff.
Without it, each quill load re-fetches every font; with it, Inter-Regular
is fetched exactly once per session regardless of how many quills use it.
Keep the cache keyed by raw md5 hex and scoped to a single registry client
instance. Eviction policy: none in v1 (processes are short-lived in
practice).

## Responsibility split

**`@quillmark/registry` owns everything.**

- Publish: walk, hash, upload, strip, emit `fonts.json`, validate, build ZIP.
- Store: serve font bytes as static files at `/store/<md5-hex>`.
- Load: fetch ZIP, validate manifest, fetch font bytes in parallel,
  rehydrate file tree, hand hydrated bundle to Quillmark.
- Caching: session-level cross-quill font cache.

**Quillmark receives complete, hydrated bundles.** The rendering path is
unchanged. Dehydration is invisible to Quillmark consumers (WASM or
native).

## Key references

- Canonical schema: `crates/core/schemas/fonts-manifest.schema.json`
- Shared test fixtures: `crates/core/tests/fixtures/fonts-manifest/`
- ZIP packager:
  `references/quillmark-registry/src/sources/file-system-source.ts:206-233`

## Deferred

- **Same-family conflicts** within one Quill: v1 sorts discovery paths
  deterministically so whichever wins is reproducible. Real conflict
  detection is later.
- **Fonts inside downloaded `@preview/...` packages** are not registered
  today (file-scan only walks the Quill tree). Unchanged.
- License metadata, garbage collection, HTML/LaTeX backends.
- Generic dehydration (non-font large assets). The store URL shape and
  manifest structure extend naturally if a second use case is validated,
  but fonts remain the only dehydrated type in v1.
