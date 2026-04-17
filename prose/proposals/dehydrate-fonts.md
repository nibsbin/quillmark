# Font Dehydration — `@quillmark/registry` Tasking

## Supersedes

`prose/proposals/centralize_fonts.md`. The goal and store model are
unchanged. This document revises the responsibility split: rehydration
moves entirely into the `@quillmark/registry` Node layer so that
`quillmark` (Rust/WASM) receives only complete, hydrated bundles and
requires no API changes.

## Goal

Move font bytes out of published Quill bundles into a shared,
content-addressed store. Fonts are 60–95% of bundle size today
(`classic_resume@0.1.0`: 2.1 MB total, 2.1 MB fonts; `usaf_memo@0.1.0`
and `0.2.0` ship byte-identical 551 KB font sets). Wire savings compound
across quills because the store deduplicates by content hash: Inter-Regular
is fetched and cached once regardless of how many quills embed it.

## Scope

**Fonts only.** Non-font assets stay inline. Typst packages stay inline —
after font stripping, remaining source (`.typ` files + `typst.toml`) is
negligible. The store URL shape is file-type-agnostic if this changes later.

## Model: dehydrate at publish, rehydrate at load

A published Quill is a **dehydration** of its source tree: font files are
stripped, their bytes moved to the content-addressed store, and a sidecar
manifest records what was removed and where.

Loading a published Quill **rehydrates** the tree: the registry client reads
the manifest, fetches missing bytes from the store (parallel, cache-first),
reconstructs a complete in-memory file tree, and passes it to
`registerQuill`. Quillmark receives a hydrated bundle identical to the
pre-strip source. The Typst backend never sees centralization.

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

## Manifest: `fonts.json`

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
- No store URL pinned in the manifest; consumers prepend their configured
  base URL so bundles remain portable across mirrors.

## Schema ownership

**Rust is canonical.** `quillmark-core` owns the `FontManifest` type (serde
+ `schemars` derives). This is already shipped:

- Type: `crates/core/src/fonts.rs`
- Generated JSON Schema: `crates/core/schemas/fonts-manifest.schema.json`
- Shared fixtures: `crates/core/tests/fixtures/fonts-manifest/`

**Node validates against the committed schema** (ajv or equivalent) before
writing `fonts.json` at publish time and before reading it at load time. CI
fails on schema drift in either direction.

## Publish flow

Triggered when a Quill source tree is packaged for the registry.

1. Walk the source tree. For every file whose extension is `ttf`, `otf`,
   `woff`, or `woff2`, compute the MD5 of its raw bytes.
2. Collect the unique set of hashes. For each, upload bytes to
   `<store-base>/store/<md5-hex>` (PUT, idempotent — skip if already
   present).
3. Build the `files` map: `{ [path]: md5-hex }` for every matched file.
4. Build the ZIP — include `fonts.json` at the root, exclude every matched
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
`quillmark-wasm`.

1. Fetch and unpack the Quill ZIP in memory.
2. Check for `fonts.json` at the ZIP root.
   - **Absent** (non-dehydrated bundle or pre-centralization): proceed
     directly to step 6.
3. Validate `fonts.json` against `fonts-manifest.schema.json`.
4. Collect the unique set of MD5 hashes from `files` values.
5. Fetch each hash from `<store-base>/store/<md5-hex>` in **parallel**.
   Cache fetched bytes for the duration of the session (cross-quill dedup).
   **Fail the load if any hash cannot be resolved.**
6. Reconstruct the complete file tree: for every `[path, md5]` entry in
   `files`, write the resolved bytes at `path`.
7. Serialize the hydrated tree as Quill JSON and call
   `engine.registerQuill(quillJson)` — unchanged API, no font map argument.

## Responsibility split

**`@quillmark/registry` owns everything.**

- Publish: walk, hash, upload, strip, emit `fonts.json`, build ZIP.
- Store: serve font bytes as static files at `/store/<md5-hex>`.
- Load: fetch ZIP, validate manifest, fetch font bytes in parallel,
  rehydrate file tree, hand hydrated JSON to `quillmark-wasm`.
- Cross-quill font cache: in-process `Map<md5, Uint8Array>` for the
  duration of a registry session (prevents re-fetching the same bytes when
  loading multiple quills).

**`quillmark` (Rust/WASM) — no changes required.**

The `FontManifest` type and JSON Schema are already shipped in
`quillmark-core` for schema validation use by Node. The `registerQuill` API
is unchanged. The Rust `FontProvider` / `rehydrate_tree` machinery from the
initial implementation (`crates/core/src/fonts.rs`) can be removed in a
follow-up — it is no longer part of the load path.

## Key existing code

- ZIP packager:
  `references/quillmark-registry/src/sources/file-system-source.ts:206-233`
- Schema to validate against:
  `crates/core/schemas/fonts-manifest.schema.json`
- Shared test fixtures:
  `crates/core/tests/fixtures/fonts-manifest/`

## Deferred

- Same-family conflicts within one Quill: v1 sorts discovery paths
  deterministically. Real conflict detection is later.
- Fonts inside downloaded `@preview/…` packages are not registered today
  (file-scan only walks the Quill tree). Unchanged.
- GC, license metadata, HTML/LaTeX backends.
- Generic dehydration (non-font large assets): not in scope. If added, the
  store URL shape and manifest structure extend naturally; fonts remain the
  only dehydrated type until a second use case is validated.
