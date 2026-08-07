# Quiver: semantics for a bundle of quills

> **Status**: brainstorm. Nothing on this page is implemented in `crates/`.
> **Related**: [QUILL.md](../canon/QUILL.md), [VERSIONING.md](../canon/VERSIONING.md), borb-sh/quillmark#1204 (distribution axis), `@quillmark/quiver@0.16.0`

## TL;DR

A quill is the unit of authorship and reference; a collection of quills is the unit of distribution, deduplication, and addressing. The *name* is settled by precedent: canon calls a collection a **quiver** ([QUILL.md](../canon/QUILL.md)) and `@quillmark/quiver` ships one.

The semantics are not. What a quiver names, what a ref resolves against, what the artifact guarantees, and how much of that belongs in this repo rather than in one TypeScript package are all open. This page brainstorms each fork and recommends a side.

## 1. The word is already spent, twice

`bundle` is taken three ways in this repo, and none of them is a collection of quills:

| Existing use | Where | Meaning |
|---|---|---|
| "a format bundle" | `docs/getting-started/concepts.md`, `docs/quills/creating-quills.md` | **one** quill |
| "a loaded template bundle" | [QUILL.md](../canon/QUILL.md) TL;DR | **one** quill |
| `ChangeBundle` | `quillmark-content` | one edit's delta plus op lists |
| "bundler" | WASM build docs | Vite/webpack |

So naming the multi-quill concept `bundle` collides on the axis that matters most: a reader who hears "bundle" in this codebase already hears "one quill". **Quiver** is free, is the canon term, is published, and keeps the pen/arrow metaphor the project runs on.

Alternatives worth one line each before the question closes: *case* (physical, but reads as `match`), *catalog* (accurate, but names the index rather than the thing), *library* (overloaded by every package manager), *set* (a math word doing no work), *scriptorium* (a place, not an artifact). Recommendation: **quiver**, and stop re-opening it.

Reserve one distinction the vocabulary needs anyway: *quiver* is the concept, *catalog* is its name→versions index. They are not synonyms, and §7 depends on the difference.

## 2. Vocabulary

The deliverable of this page is that these words mean exactly one thing each.

| Term | Meaning |
|---|---|
| **quill** | The unit of authorship and reference: one `Quill.yaml` tree; what a document's `$quill` names. |
| **quiver** | A named collection of quills, versioned per member. The unit of distribution and the namespace a ref resolves in. |
| **member** | One `name@x.y.z` in a quiver. |
| **catalog** | A quiver's name → versions map. Resolvable surface, no bytes. |
| **canonical ref** | `name@x.y.z`. Immutable content, per [VERSIONING.md](../canon/VERSIONING.md) §Ref Immutability. |
| **selector ref** | `name`, `name@x`, `name@x.y`, `name@latest`. Resolves to a canonical ref against a catalog. |
| **resolution** | selector ref + catalog → canonical ref. Needs no bytes. |
| **materialization** | canonical ref → file tree → `Quill`. |
| **enforcement** | The render-time pairing check the engine already performs: name equal, version inside the selector. |
| **source quiver** | The authored shape: `Quiver.yaml` plus `quills/<name>/<x.y.z>/`. |
| **built quiver** | The deployable shape: manifest, per-member archive, shared asset store. |
| **store** | Content-addressed shared assets in a built quiver. Transport, not semantics (§5). |

## 3. One concept, three shapes

Conflating the shapes is where the confusion starts, so name them apart. `@quillmark/quiver` already separates all three, and its loader names say which it reads (`fromDir` / `fromPackage` are source; `fromBuiltDir` / `fromBuiltUrl` / `fromManifest` are built).

| Shape | Lives as | Produced by | Read by |
|---|---|---|---|
| source | directory tree, one npm package or git repo | an author | a Node loader, CI, the CLI |
| built | manifest + archives + store, static files | a build step | a browser over HTTP, a server off disk |
| live | catalog + loader + per-ref cache | a constructor | `getQuill(ref)` |

The invariant tying them: **a built quiver materializes the same tree its source quiver would.** Everything the build does (hoisting fonts, zipping, hashing) is invisible above the loader seam. A consumer cannot tell which shape it was handed except by asking for its name.

## 4. Why the collection is the unit, not the quill

The distribution unit could have been one quill (borb-sh/quillmark#1129 asked for exactly that: `Quill.toBytes` / `Quill.fromBytes`). Measured against this repo's own fixtures, assets dominate every quill that has any:

| Fixture quill | Total bytes | Fonts + images | Share |
|---|---|---|---|
| `classic_resume@0.1.0` | 2,115,133 | 2,103,852 | 99.5% |
| `usaf_memo@0.2.0` | 1,236,709 | 1,116,388 | 90.3% |
| `taro@0.1.0` | 122,412 | 121,100 | 98.9% |
| `cmu_letter@0.1.0` | 574,373 | 551,204 | 96.0% |

A per-quill artifact cannot dedupe those bytes, because a `Quill` only sees itself. A quiver can, and that is the whole argument for the concept existing: **the quiver is the smallest scope in which asset sharing is expressible.**

The guaranteed payoff is across *versions* of one member: a MINOR bump usually touches `Quill.yaml` and the plate and nothing else, so ~99% of the two versions is identical. The cross-member payoff arrives when authors converge on a font family, which no two fixture quills do today.

Corollary the semantics must carry: a member's authored tree and its transported bytes are not the same object. Identity is therefore defined on the materialized tree, not the archive: **two quivers are equivalent for a ref when they materialize equal trees for it**, whatever their compression, hoisting, or layout.

## 5. Deduplication, never inheritance

These look identical at rest (one copy of the font on disk) and mean opposite things:

- **Deduplication**: each member still *declares* every asset it uses; the builder notices two declarations name equal bytes and stores them once. Every member stays extractable and renderable alone.
- **Inheritance**: the quiver declares fonts its members inherit. A member is then no longer self-contained, and `Quill` stops being portable declarative data ([QUILL.md](../canon/QUILL.md)), which every consumer of `from_tree` depends on.

Recommendation: **dedupe, never inherit**, and write it down, because inheritance is the obvious-looking next feature request once a quiver-level asset store exists. Sharing is an optimization below the loader seam; it never becomes an authoring surface.

## 6. Namespace and composition

`$quill: usaf_memo` is unscoped today: the name is global by assumption. A quiver introduces a namespace, so two decisions follow.

**Is the quiver part of a ref?** Three options:

1. **Flat, quiver-local.** The quiver is a search path; refs stay `name@selector`. Documents are portable between quivers that happen to carry the name, and silently mean a different template when they do not.
2. **Qualified.** `quiver/name@selector`. Unambiguous, and a document names its template exactly once. Breaks the `$quill` grammar in `crates/core/src/version.rs`, so it costs a MAJOR and a migration of every stored document.
3. **Flat now, qualified reserved.** Ship (1), and document `/` as reserved in a quill name so (2) is additive later. Core's `is_valid_quill_name` already rejects `/` (snake_case only), so the reservation costs one sentence and no code.

Recommendation: **(3)**. The cheap half of (2) is available for free today and only stays free while nobody ships a name containing a separator.

**What does a set of quivers mean?** The vestigial comment in `@quillmark/quiver`'s `ref.ts` ("highest in first-winning quiver") describes a composition the package never implemented: one `Quiver` holds one catalog. Two candidate semantics, both with a footgun:

- **First-winning search path**: quivers are ordered, the first carrying the name answers, and version selection happens *within* that quiver. Prepending a quiver silently shadows a member.
- **Union then highest**: names merge across quivers and the highest version wins. A stale quiver can win, and which quiver answered is not knowable from the ref.

Recommendation: **first-winning**, plus a diagnostic when a name is carried by more than one quiver in the path. Shadowing that announces itself is a tool; shadowing that does not is the npm-hoisting failure mode.

## 7. Resolution: one grammar, three functions

This is the sharpest gap, and it is no longer hypothetical. borb-sh/quillmark#1204 predicts "two resolvers can disagree on selector semantics with no document to arbitrate". Two implementations exist and already disagree three ways:

| | `quillmark-core` | `@quillmark/quiver@0.16.0` |
|---|---|---|
| `@latest` in a ref | accepted (`VersionSelector::Latest`, `version.rs`) | `invalid_ref`: the selector regex is digits only (`ref.ts`) |
| quill `version:` | `x.y` or `x.y.z`, patch defaults to 0 (`version.rs`) | canonical `x.y.z` only (`semver.ts`, and the directory name) |
| quill `name` charset | snake_case, `[a-z][a-z0-9_]*` (`config.rs`) | `[A-Za-z0-9_-]+` (`ref.ts`, `quiver-yaml.ts`) |

So `$quill: memo@latest` parses in the engine and throws in the resolver, and a quill the engine loads (`version: "1.2"`, `name: My-Memo`) cannot be placed in a quiver at all. Neither is a bug in either implementation: there is no document either one is wrong against.

The semantics that fix it split resolution into three total functions with one invariant:

- `parse(ref) -> (name, selector)`, owned by core, which already has it.
- `resolve(selector, versions) -> Option<canonical>`: the highest member of `versions` matching `selector`. The resolver's only freedom is what `versions` contains.
- `enforce(ref, quill)`: the render-time check core already performs.

**Invariant: resolve-then-enforce never fails.** Anything `resolve` returns satisfies `enforce`. That is one property test in `quillmark-fuzz` over an arbitrary version set, and it is exactly what makes a second implementation safe to write.

Two smaller semantics worth pinning while the spec is open:

- **`@latest` has no prerelease concept.** It selects the numerically highest version, including a `0.x`. There is no prerelease grammar and inventing one is a separate decision.
- **Two failure codes, not one.** "no such name in this quiver" and "name found, no version matches the selector" have different remedies (add the quiver against relax the pin). `@quillmark/quiver` returns `quill_not_found` for both.

## 8. What a quiver declares about itself

`Quiver.yaml` today is two fields, `name` and `description`, strict on unknown keys. Candidates, each with its cost:

| Candidate | For | Against | Recommendation |
|---|---|---|---|
| `version:` for the quiver | build output wants a stamp | a second version axis no ref mentions; npm/git already version the publication, and the manifest hash already identifies the catalog | **no**, and say so once so it stops being re-asked |
| `quills:` explicit member list | drift can't happen silently | a second source of truth for what the directory already states | **no**; scan, and report what was found at build |
| `requires:` engine floor | a quiver using 0.100-era `card_kinds` can say so instead of failing per-member with a parse error | needs a runtime that knows its own version | **yes**: the highest-value missing field |
| quiver-level assets | dedupe is already implicit | it is §5's inheritance in a hat | **no** |

`requires:` is a floor, not a range: an engine older than the floor refuses the catalog with one error naming the quiver, rather than N parse errors naming fields. A runtime that cannot determine its own version warns instead.

## 9. Integrity

[VERSIONING.md](../canon/VERSIONING.md) §Ref Immutability already states the hard part: a canonical ref is immutable content, and every cache keys on that with no invalidation API. What is missing is a binding across a trust boundary.

`@quillmark/quiver` hashes for cache-busting, not integrity: `built-loader.ts` validates that a store hash *looks* like a hash and never checks that the fetched bytes produce it.

Two semantics to choose between:

- **Content-addressed, verified**: the manifest carries each member's tree hash; a loader that materializes a tree whose hash differs rejects it. Makes "immutable content" enforceable against a party who does not already behave, which is the property a CDN deployment wants.
- **Content-addressed, advisory**: hashes discriminate cache entries and nothing verifies. Cheaper, and what exists.

Recommendation: **verified**, and hash the *canonical uncompressed tree*, not the archive. Hashing compressed bytes makes the digest a function of the compressor: `fflate` at level 6 and Rust `flate2` at level 6 do not emit identical deflate, so the first non-TypeScript producer gets a different hash for identical content. That surfaces as a cache miss, not an error.

## 10. Where the concept should live

| Option | Cost | What it buys |
|---|---|---|
| Stay external, TypeScript only | none | status quo: Python and the CLI have no distribution story, and the resolution spec is de facto whatever `semver.ts` does |
| A spec in `prose/references/` | one document | the word means something a third party can implement; kills #1204's "no resolution spec" and the §7 divergences |
| A `Quiver` type in Rust plus CLI verbs | a crate surface and a compat promise | `quillmark pack` / `resolve`, Python parity, one conformance suite |

Recommendation: **spec first, narrowly-scoped code second.** A reference is self-contained by construction ([prose/README.md](../README.md)), which is the tier a document a third party implements against belongs to, and it is the artifact #1204 scores. Code without the spec adds a third disagreeing implementation.

## Open questions

- Does a quiver carry non-quill members (shared packages, a font pack) as first-class entries, or is everything either a member or store bytes?
- Is the catalog enumerable as a contract (`quillNames()` / `versionsOf()` exist in `@quillmark/quiver`), or is `resolve` the whole promised surface? Enumeration is what a template picker in a UI needs.
- Does a member ever get yanked, and if so what does a document pinned to it do? Immutability says content never changes; it says nothing about availability.
- Does the CLI grow a quiver-aware `render` (`--quiver` plus a bare `$quill`), or does it stay one-quill-one-path?
