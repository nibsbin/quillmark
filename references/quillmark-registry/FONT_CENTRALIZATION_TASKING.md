# Font Centralization — Tasking

## Goal

Move font bytes out of published Quill bundles into a shared,
content-addressed store. Fonts are 60–95% of every Quill today
(`classic_resume@0.1.0` is 2.1 MB of which 2.1 MB is fonts;
`usaf_memo@0.1.0` and `0.2.0` ship byte-identical 551 KB font sets).

## Scope

**Fonts only.** Images/SVGs stay inline (templates reference them by path,
making substitution awkward). Typst packages stay as-is (Typst's own
registry `@preview/...` already solves cross-Quill package sharing).

## Core decisions

- **Identity = MD5 of raw font bytes.** Dedup, not integrity.
- **Store is flat and content-addressed.** URL shape: `<base>/store/<md5-hex>`.
  Raw bytes, lowercase hex, no extension. Publisher filesystem layout
  mirrors the URL.
- **Persisted, write-open, idempotent uploads.** No garbage collection in v1.
- **Raw bytes only.** No zipping, no format conversion (Typst does not
  support WOFF2). Transport compression is the CDN's job.
- **Strip at publish, everywhere.** `*.ttf`, `*.otf`, `*.woff`, `*.woff2`
  are stripped from the published ZIP wherever they appear in the source
  tree, including under `packages/**`. Local dev rendering is unaffected —
  the strip happens only at publish.
- **Transparent to authors.** `Quill.yaml` is never modified by the publish
  tool. The source tree stays clean.
- **Manifest is a sidecar in the published ZIP.** File named `fonts.json`
  at the ZIP root. Generated at publish, lives inside the bundle, never in
  source control.

### Manifest: `fonts.json`

```json
{
  "version": 1,
  "fonts": [
    {
      "md5": "3f2a8c1d9e4b5a7f0c8d6e3a1b4f9c2d",
      "family": "Inter",
      "style": "normal",
      "weight": 400
    },
    {
      "md5": "a7e3b2d5f0c8e1a4b7d2f5c9e0a3b6d1",
      "family": "Inter",
      "style": "italic",
      "weight": 400
    }
  ]
}
```

- `style` is strictly `"normal" | "italic" | "oblique"` — the italic axis.
- `weight` is numeric 100–900 — the weight axis.
- Both match Typst's `FontInfo` model directly.
- Annotations are for human-readable diffs and publish output. **Runtime
  uses the hash as the source of truth.** On mismatch, hash wins.
- No store URL pinned; consumers prepend their configured base URL, so
  bundles stay portable across registry mirrors.

## Responsibility split

**Rust (`quillmark` crates) — render-time only.**

- `FontProvider` trait in `quillmark-core`:
  `fn fetch(&self, md5: &str) -> Option<Bytes>`. Sync, to match Typst's
  sync font loading and avoid async-in-WASM.
- Quill loader reads `fonts.json` from the published bundle, resolves each
  hash via the provider, attaches resolved bytes to the `Quill` before
  backend construction.
- Typst backend registers resolved fonts in `FontBook` alongside the
  existing file-scan output at
  `crates/backends/typst/src/world.rs:156-207`. Embedded fallback fonts
  stay as last-resort.
- **Rust never hashes, strips, or generates manifests.**

**Node (`quillmark-registry`) — publish and transport.**

- Walks source trees; hashes fonts; sniffs family/style/weight from font
  `name` / `OS/2` tables (fontkit or opentype.js; publish-only dep).
- Enforces the strip list and writes `fonts.json` into the ZIP.
- Manages the store: writes `store/<md5-hex>`, serves as static files.
- Runtime path: fetches ZIPs, fetches font bytes by hash, hands bytes into
  Rust via the `FontProvider` callback.

## Injection across the language boundary

**WASM / Node:** Node fetches every declared hash up front, builds
`Map<string, Uint8Array>`, passes it alongside the Quill JSON through the
WASM boundary: `Quill.fromJson(quillJson, fontMap)`. Rust wraps the map as
a `MapProvider`. All fonts are eager from Rust's POV.

**Native:** Consumer supplies a concrete `FontProvider` impl (HTTP against
the registry, local directory, in-memory map, etc.). The trait does not
dictate eager vs. lazy.

**No shared process-wide store in v1.** Providers are per-load. Consumers
wanting cross-Quill caching implement it inside their own `FontProvider`.

## Publish output

Show dedup stats. Counts are a local walk of the source tree — no store
query.

```
fonts:
  Inter Regular   abc123...  used by 14 quills
  Inter Bold      def456...  used by 12 quills
  EB Garamond     789abc...  used by 1 quill

bundle: stripped 47 MB across 22 quills
```

## Deferred

- **Same-family conflicts** (two files both "Inter Regular"). v1 sorts
  font discovery paths deterministically so whichever wins is
  reproducible. Real conflict detection is a later task.
- **Fonts inside downloaded `@preview/...` packages** — not registered
  today (file-scan only walks the in-memory Quill tree); this task does
  not change that.
- **License metadata, garbage collection, HTML/LaTeX backends.**

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
