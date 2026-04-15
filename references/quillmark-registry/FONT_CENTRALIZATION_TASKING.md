# Font Centralization — Tasking

## Goal

Move font bytes out of published Quill bundles into a shared,
content-addressed store. Fonts are 60–95% of every Quill's size today
(`classic_resume@0.1.0`: 2.1 MB total, 2.1 MB fonts; `usaf_memo@0.1.0`
and `0.2.0` ship byte-identical 551 KB font sets).

## Scope

**Fonts only.** Non-font assets stay inline (templates reference them by
path, making content-addressed substitution awkward). Typst packages stay
as-is (Typst's `@preview/...` registry already solves cross-Quill package
sharing).

## Model: dehydrate at publish, rehydrate at load

A published Quill is a **dehydration** of its source tree: font files are
stripped, their bytes moved to the shared store, and a sidecar manifest
records what was removed and where. Loading a published Quill
**rehydrates** the tree: the manifest drives fetches from the store, and
bytes are written back to their original paths.

After rehydration, the in-memory `Quill` is indistinguishable from the
pre-strip source. The Typst backend (`QuillWorld`) never sees
centralization — it scans the file tree exactly as it does today. All
of the new machinery is a load-time transformation.

## Core decisions

- **Identity = MD5 of raw font bytes.** Dedup, not integrity.
- **Store is flat and content-addressed.** URL: `<base>/store/<md5-hex>`.
  Raw bytes, lowercase hex, no extension. Publisher filesystem mirrors
  the URL.
- **Persisted, write-open, idempotent uploads.** No GC in v1. No zipping,
  no format conversion (Typst doesn't support WOFF2). Transport
  compression is the CDN's job.
- **Strip everywhere at publish.** `*.ttf`, `*.otf`, `*.woff`, `*.woff2`
  are removed from the ZIP wherever they appear, including under
  `packages/**`. Local dev rendering is unaffected.
- **`Quill.yaml` is never modified.** Author source stays clean.
- **Manifest is a sidecar inside the ZIP** — `fonts.json` at the ZIP
  root.

## Manifest: `fonts.json`

Two maps, modeling two distinct relationships:

```json
{
  "version": 1,
  "files": {
    "assets/fonts/Inter-Regular.ttf": "3f2a8c1d9e4b5a7f0c8d6e3a1b4f9c2d",
    "packages/ttq-classic-resume/fonts/Inter-Regular.ttf": "3f2a8c1d9e4b5a7f0c8d6e3a1b4f9c2d",
    "assets/fonts/Inter-Bold.ttf": "a7e3b2d5f0c8e1a4b7d2f5c9e0a3b6d1"
  },
  "fonts": {
    "3f2a8c1d9e4b5a7f0c8d6e3a1b4f9c2d": {
      "family": "Inter",
      "style": "normal",
      "weight": 400
    },
    "a7e3b2d5f0c8e1a4b7d2f5c9e0a3b6d1": {
      "family": "Inter",
      "style": "normal",
      "weight": 700
    }
  }
}
```

- **`files`**: path → md5. The dehydration record, per-Quill. One entry
  per stripped file. Identical bytes at multiple source paths produce
  multiple entries with the same hash; rehydration faithfully reproduces
  the tree.
- **`fonts`**: md5 → metadata. Deduped by hash. One entry per unique
  font.
- `style` is strictly `"normal" | "italic" | "oblique"`.
- `weight` is numeric 100–900.
- Both axes match Typst's `FontInfo` model directly.
- Annotations are advisory (human diffs, publish output). **Runtime
  trusts the hash.** On mismatch with bytes, hash wins.
- No store URL pinned; consumers prepend their configured base URL so
  bundles remain portable across mirrors.

## Publish flow (Node)

1. Walk source tree. For each file matching the strip extensions, hash
   the bytes.
2. Sniff `family` / `style` / `weight` from `name` / `OS/2` tables
   (fontkit or opentype.js — publish-only dep).
3. Upload each unique hash to the store (idempotent).
4. Build `files` and `fonts` maps; write `fonts.json` into the ZIP.
5. Strip the matched files from the ZIP.

## Rehydration flow (Rust)

1. Unpack ZIP into `FileTreeNode` (fonts missing).
2. Read `fonts.json`.
3. Collect the unique set of hashes from `files` values.
4. `FontProvider::fetch(md5)` once per unique hash.
5. Write bytes into `FileTreeNode` at every path whose value is that
   hash.
6. Hand the rehydrated `Quill` to the backend.

Render time is then **identical to today**: the existing file-scan at
`crates/backends/typst/src/world.rs:156-207` finds everything where it
expects.

## Responsibility split

**Rust — render-time + rehydration only.**

- `FontProvider` trait in `quillmark-core`:
  `fn fetch(&self, md5: &str) -> Option<Bytes>`. Sync, to match Typst's
  sync font loading and avoid async-in-WASM.
- Loader reads `fonts.json` and rehydrates `FileTreeNode` via the
  provider before backend construction.
- Typst backend unchanged. Embedded fallback fonts at
  `crates/backends/typst/src/world.rs:43-64` stay as last-resort.
- **Rust never hashes, strips, or writes manifests.**

**Node (`quillmark-registry`) — publish + transport.**

- Walks source trees, hashes fonts, sniffs metadata, uploads bytes,
  writes `fonts.json`, strips files, produces ZIPs.
- Manages the store as static files (`/store/<md5-hex>`).
- Runtime path: fetches ZIPs, fetches font bytes by hash, hands bytes to
  Rust via the `FontProvider` callback.

## Injection across the language boundary

**WASM / Node.** Node reads `fonts.json` from the unpacked ZIP, fetches
each unique hash from the store, builds `Map<string, Uint8Array>`, passes
it alongside the Quill JSON: `Quill.fromJson(quillJson, fontMap)`. Rust
wraps the map as a `MapProvider`. Fonts are eager from Rust's POV.

**Native.** Consumer supplies a concrete impl (HTTP against the registry,
local directory, in-memory map). Trait does not dictate eager vs. lazy —
but rehydration will call `fetch` for every unique hash at load time, so
lazy providers effectively pre-fetch then too.

**No shared process-wide store in v1.** Providers are per-load. Consumers
wanting cross-Quill caching implement it inside their `FontProvider`.

## Publish output

```
fonts:
  Inter Regular   abc123...  used by 14 quills
  Inter Bold      def456...  used by 12 quills
  EB Garamond     789abc...  used by 1 quill

bundle: stripped 47 MB across 22 quills
```

Counts are a local walk of the source tree being published — no store
query.

## Deferred

- **Same-family conflicts** within one Quill: v1 sorts discovery paths
  deterministically so whichever wins is reproducible. Real conflict
  detection is later.
- **Fonts inside downloaded `@preview/...` packages** are not registered
  today (file-scan only walks the Quill tree). Unchanged.
- License metadata, garbage collection, HTML/LaTeX backends.

## Key existing code

- Font loading today: `crates/backends/typst/src/world.rs:156-207`.
- `QuillWorld` construction + embedded fallbacks:
  `crates/backends/typst/src/world.rs:18-103`.
- Quill config schema: `crates/core/src/quill/config.rs:16-40`.
- In-memory file tree + `.quillignore`:
  `crates/core/src/quill/tree.rs:7-18`,
  `crates/core/src/quill/load.rs:11-41`.
- Registry ZIP packager:
  `references/quillmark-registry/src/sources/file-system-source.ts:206-233`.
