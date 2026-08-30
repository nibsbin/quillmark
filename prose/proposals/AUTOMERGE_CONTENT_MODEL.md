# Automerge as the Content Model

> **Implementation**: `crates/content/src` (the model this would replace)
> **Related**: [DOCUMENT_STORAGE.md](../canon/DOCUMENT_STORAGE.md), [PREVIEW.md](../canon/PREVIEW.md)
> **Status**: spike. Measured against `automerge` 0.11.0; recommendation is *no* to the substrate, *maybe* to the shape.

## TL;DR

`Content` and Automerge's rich-text model are near-isomorphic already: both are
one text sequence with U+FFFC object slots, ranged marks, and a container path
per block. Adopting Automerge's *shape* is therefore cheap in concept and
buys three invariants' worth of simplification. Adopting Automerge as the
*substrate* costs byte-stability, which is load-bearing, and measurably
regresses the one edit lane we built for. As a **maintenance offload** it
inverts: the crate's defects are all in the layer Automerge does not implement,
and the trade adds 34 dependencies, a binary decoder, and a pre-1.0 upstream
under a published SemVer promise. Keep the model; if collaboration becomes a
goal, adapt to Automerge at a session seam rather than at rest.

## Two proposals in one question

"Replace the content model with Automerge's" resolves two ways, with opposite
cost profiles. Separate them before arguing either.

| | **A. The shape** | **B. The substrate** |
|---|---|---|
| What changes | `Content`'s layout and its canonical JSON | `Content` becomes an `automerge::AutoCommit` |
| Storage | still our canonical JSON, new schema version | Automerge's compressed op log |
| Buys | three invariants deleted, open vocabularies for free | merge, sync, offline, presence |
| Costs | one schema version, most of `crates/content` rewritten | § What the substrate costs |
| Needs Automerge | no | yes |

A does not depend on B, and B does not deliver A: Automerge's block map is a
convention over an untyped map, so B keeps whatever shape we hand it.

## What the two models already share

The convergence is not a coincidence. Both descend from Peritext, and
`crates/content/src/model.rs` reaches the same encoding independently.

| Concern | `Content` | Automerge |
|---|---|---|
| Coordinate space | USV (`Usv = usize`, Rust `char`) | `TextEncoding::UnicodeCodePoint` |
| Object slot | `ISLAND_SLOT` = `'\u{FFFC}'` | `PLACEHOLDER` = `"\u{fffc}"` (`iter/spans.rs`) |
| Structured embed | `Island { id, island_type, props, loss }` | `Span::Block(hydrate::Map)` |
| Ranged formatting | `Mark { start, end, kind }` | `Mark { start, end, name, value }` |
| Container path | `Line::containers`, outermost first | block map's `parents`, outermost first |
| Block role | `Line::kind` | block map's `type` |
| Stale-text writer | `delta::diff_import` | `Transactable::update_text` / `update_spans` |
| Stable position | `MarkKind::Anchor { id }` | `Cursor` (`get_cursor` / `get_cursor_position`) |

Two structural differences remain, and both favour Automerge:

- **Block boundaries are in the sequence.** A block marker occupies a slot;
  `Content` keys a parallel `lines` array on `\n` runs. The in-sequence form
  makes `Line::continues` unnecessary — a `\n` with no marker *is* a
  within-block break — and deletes the `lines.len() == segments` invariant
  (`Invariant::LineCountMismatch`). One block marker also subsumes `Island`, so
  `LineKindMismatch::IslandNotOneSlot` goes with it; the other two arms
  (`RuleNotEmpty`, `CodeHasSlot`) stay expressible and stay checked.
- **The block role is a string in an open map.** `type` and `attrs` carry no
  reserved-name problem, so the whole promotion apparatus goes: `RESERVED_*`,
  `fold_legacy_attrs`, `ReservedUnknownLineKind`, and the rule pinning a new
  `MarkKind` to the ordinal before `Unknown`.

One difference does **not** resolve. Container identity is still path plus
contiguity: two adjacent paragraphs whose `parents` are both `["blockquote"]`
are one quote or two, and Automerge's parents-as-type-names carries no
discriminator either. `Container::instance` survives the port under any name.

## What the substrate costs

Each row below is measured, not reasoned. Probe sources are in
§ Reproducing.

### Byte-stability does not survive

[DOCUMENT_STORAGE.md](../canon/DOCUMENT_STORAGE.md) § "Byte-stability" promises
that equal `Document`s produce byte-equal JSON, and names the consumers
spending it: template-divergence detection and cache keys. Automerge's `save()`
is a function of *history*, not of content.

| Probe | Same text | Same bytes |
|---|---|---|
| Same construction, default (random) actor | yes | **no** |
| Same construction, pinned actor | yes | yes |
| Same content, two edit histories, pinned actor | yes | **no** (213 vs 222 B) |
| Two peers, same merge, opposite orders | yes | **no** |
| `save(load(save(d)))` vs `save(d)` | yes | yes |

Rows 1 and 3 are the fatal ones. Row 1 is the never-ambient rule
(§ "Island-id determinism") violated at the document level: `Automerge::new`
takes `ActorId::random()`, so identity leaks from the process into the wire.
Row 3 is the promise itself: two producers reaching the same document by
different routes disagree on its bytes. Row 4 says even *converged replicas*
disagree, so there is no peer-independent hash to fall back on.

Row 5 is the consolation, and it is narrow: a stored blob is stable under
reload, so hashing one row for its own cache key still works. Comparing two
independently authored documents does not, and that is what divergence
detection is.

There is no canonicalizing save. `SaveOptions` carries `deflate` and
`retain_orphans`; neither normalizes history.

### The mark algebra clips silently

`delta.rs` states the reason the model is not an attribute map: it "cannot
represent overlapping same-kind marks or two distinct identity anchors over one
range". Automerge's `marks.rs` states the same limit from the other side —
"each position in the sequence can be affected by only one Mark of the same
name" — and enforces it by truncation.

Two anchors, `thread-1` over `[0, 9)` and `thread-2` over `[4, 15)`, read back
as `[0, 4)` and `[4, 15)`. The first anchor lost five characters, with no error.

The workaround is name-mangling — `anchor-thread-1` as the mark *name* — which
does preserve both. It makes the mark namespace unbounded, turns "remove every
anchor" into a prefix scan, and makes `MarkKind`'s ordinal-stability rule
meaningless because the sort key is now a user-supplied id.

### The bundle roughly doubles

A minimal `cdylib` exercising `splice_text` / `split_block` / `mark` / `save` /
`load` / `spans`, built for `wasm32-unknown-unknown` at `opt-level = "z"` with
LTO and strip:

| Build | Raw | Gzip |
|---|---|---|
| Automerge probe | 1.15 MB | 416 KB |
| Empty `cdylib` baseline | 121 B | 134 B |
| Current core WASM build ([BINDINGS.md](../canon/BINDINGS.md)) | — | ~660 KB |

Some of that 416 KB is std the core build already links, so the marginal cost
is lower — but the order of magnitude is half the core bundle again, on the
build that exists to be small. The Typst backend's ~8 MB would not notice; the
core one would.

Automerge also pulls `getrandom`, which does not compile for
`wasm32-unknown-unknown` without `--cfg getrandom_backend="wasm_js"` and the
matching feature. That is a flag across the whole WASM pipeline, not a
dependency line.

### Small fields pay a fixed toll

A 31-byte `plaintext` value (`Cost * Benefit Analysis *DRAFT*`) is a 188-byte
Automerge document — 6×. Per-field documents are the shape that matches
`Content`'s "one per content field", and it is the shape that pays worst. One
document per `Document` amortizes the header but makes a field no longer a
self-contained, independently-storable value, which `store_field` and
`get_content_at` both assume.

History growth is the fear that turns out **not** to be justified: 200 chars
typed one keystroke at a time saves to the same 370 bytes as one 200-char
splice, because sequential inserts run-length encode. Real churn does cost: 300
rounds of five-character replacements over a 1080-character field grew it from
220 B cold to 1651 B, 7.5×, with no compaction short of authoring a fresh
document and losing every anchor.

### A pre-1.0 dependency in a published crate

`quillmark-content` is `publish = true` and carries a SemVer promise
([COMPATIBILITY.md](../canon/COMPATIBILITY.md)). Automerge is 0.11.0, where
every minor may break. Putting its types in `Content`'s public API makes each
Automerge minor a Quillmark major.

## What the substrate buys, and the lane where it loses

B has two honest cases. One is capability — merge, sync, offline editing, and
presence, none of which we can build cheaply ourselves; nothing in the
repository asks for them today, and `PREVIEW.md` § "One session type" turns on
there being exactly one owned consumer. The other is maintenance offload, which
§ The maintenance argument takes on its own terms.

The near-term case would be `delta.rs`: a hand-rolled Myers diff plus a move
detector, carrying a documented residual (text both moved and rewritten in one
round drops its anchor). `update_text` is the maintained equivalent. It is
worse.

Anchor over `"Bravo paragraph here."` in a three-paragraph field, then rewrite
the whole field with the paragraphs reordered:

| Rewrite | `update_text` result |
|---|---|
| Verbatim block move | anchor covers `"rlie paragraph here."` |
| Moved and rewritten | anchor covers `"arlie paragraph here."` |
| Edit inside the anchored range | correct |
| Edit before the anchor | correct |

`Cursor` fails identically, so this is the diff strategy, not the anchor
representation. Both of our failure modes are safer: the move detector re-homes
the anchor across a verbatim move, and a move-plus-rewrite *drops* it. A dropped
comment thread is visible; one silently re-attached to a different paragraph is
not.

Automerge documents `update_text` as the fallback for when user input cannot be
captured. That is exactly our lane — the stale-text writer exists because an LLM
rewrites a whole field with no preservation contract — so we would be adopting
Automerge at its documented weak point and losing our compensation for it. A
real editor emitting per-keystroke ops would invert this comparison completely.

## The maintenance argument

Merge is not the only reason to want B. The stronger motive is **offloading
maintenance**: hand the security, robustness and stability of hard code to
people who work on it full time.

The premise checks out. Automerge's README states that two maintainers work on
it full time; the crate carries **zero `unsafe` blocks**; it has shipped
steadily since 2022. Offloading hard code to that is a good instinct, and the
instinct is not what fails here. The fit is.

### The bugs are not where the dependency is

Fix commits landing in `crates/content`, by file. (History is a shallow clone,
so this is a 14-day window, not the project's life — but it is the window in
which the crate took 27 of the repository's 188 commits, so it is not a quiet
one.)

Eleven fix commits touched the crate. A commit reaches several files, so the
column counts commits per file rather than partitioning them.

| File | Fixes touching it | Would Automerge own it? |
|---|---|---|
| `model.rs` | 6 | No — projection and block model |
| `serial.rs` | 6 | No — canonical form, deleted only if we accept the binary wire |
| `ops.rs` | 5 | Partly — the op channels, not their semantics |
| `import.rs` / `export.rs` | 4 | No — Automerge has no Markdown |
| `delta.rs` | **0** | **Yes** — position mapping, diff, rebase |

The one file Automerge genuinely replaces is the one file that has needed no
fixing. Every defect landed in the Markdown projection, the canonical form, or
the block-and-container model — the layer that stays ours under B. Two of them
name that layer outright: *clear a `continues` that crosses a container
boundary*, and *give a container the instance its identity was missing*.

Two more are stack-overflow guards — *walk export's block tree on a frame
stack*, *walk census iteratively*. Those are the robustness class the argument
is about, and they are in Markdown export and a content census, neither of
which Automerge implements.

### Most of the crate is not offloadable at all

| Component | Lines | Under B |
|---|---|---|
| `import.rs` + `export.rs` (Markdown codecs) | 3,015 | Kept whole — no counterpart |
| `serial.rs` (canonical JSON) | 1,791 | Kept, or byte-stability goes |
| `model.rs` + `ops.rs` | 4,457 | Partly replaced; the projection and validation stay |
| `delta.rs` | 734 | Replaced, and measurably worse (§ the rebase table) |
| Rest | 646 | Kept |

Of 10,643 lines, the Markdown codecs alone are 28% with no counterpart
upstream. What B actually removes is a minority of the crate, and it adds a
`Content` ⇄ `AutoCommit` adapter we then own — the hardest kind of code to
maintain, because it encodes assumptions about someone else's model.

### The trade adds surface on each named axis

| Axis | Today | Under B |
|---|---|---|
| Supply chain | 21 transitive crates | 55 (+34: `zlib-rs`, `flate2`, `chacha20`, `sha2`, `rand`, `tracing`, …) |
| Untrusted-input decoder | `serde_json` into a validated struct | a 10,347-line columnar binary decoder, ~70 `unwrap`/`expect` in its storage path |
| Stability | our own internal API | pre-1.0 for four years; 0.8.0 → 0.11.0 in five months, each minor a Cargo-semver break |

Zero `unsafe` bounds the decoder's failure to panics rather than memory
corruption, which is a real assurance. But a panic on `wasm32` is a trap that
takes the module down with nothing for the host to catch — the exact failure
`MAX_JSON_DEPTH` was written to prevent, now reintroduced at a wider door.

The stability axis inverts hardest. `quillmark-content` is published and makes a
SemVer promise ([COMPATIBILITY.md](../canon/COMPATIBILITY.md)); B pins that
promise to an upstream shipping a breaking minor roughly every seven weeks.
Automerge's README is candid about why: the Rust crate is "oriented around
producing a performant backend for the Javascript wrapper", with an API that is
"low level and not well documented". The artifact those two maintainers stabilize
is the JavaScript package. The Rust crate is the means.

### The redirect

The instinct is right; the target is wrong. The genuinely hard, genuinely
offloadable work in this crate has **already been offloaded**: Markdown parsing
to `pulldown-cmark`, sequence diffing to `similar`. What remains hand-rolled is
`delta.rs`'s position mapping and move detector — 734 lines that have produced
no bugs and that Automerge does worse.

The change that actually shrinks the surface we maintain is A, and it needs no
dependency: deleting `continues`, `LineCountMismatch`, `RESERVED_*`,
`fold_legacy_attrs`, and the `MarkKind::ord` placement rule removes code *and*
removes the invariants that code exists to uphold. One of the eleven fixes above
is in machinery A deletes outright. That is a smaller claim than "offload the
model", and it is the one the evidence supports.

## Recommendation

**Do not adopt the substrate.** It trades a promise we spend (byte-stability)
for a capability we do not yet want (merge), regresses the one lane we invested
in, and doubles the core bundle. As a maintenance offload it inverts: the bugs
are in the layer B keeps, and B adds 34 dependencies, a binary decoder, and a
pre-1.0 upstream under a published SemVer promise. The precondition to revisit
is a real multi-writer product requirement, and the revisit should start from the
adapter below rather than from replacing `Content`.

**When collaboration lands, adapt at the session seam.** `Content` ⇄
`AutoCommit` is a pure projection: both sides speak U+FFFC-slotted USV text
with ranged marks and a container path, and the table in § What the two models
already share is most of the mapping. That keeps the CRDT a live-session
substrate, keeps canonical JSON the resting form, and keeps the hash contract
intact. It also stays optional: a separate crate, not a dependency of the
published leaf.

**The shape's simplifications are worth taking on their own merits, and are what
actually serves the maintenance motive.** Moving block markers into the sequence
deletes `continues`, `LineCountMismatch`, and one `LineKindMismatch` arm; making
the block role a string plus an open `attrs` map deletes `RESERVED_*`,
`fold_legacy_attrs`, and the `MarkKind::ord` placement rule. Each deletion
removes an invariant as well as its code, and takes no dependency to do it. That is a schema-version event
(`quillmark/document@0.94.0`), a structural migration off `0.93.0`, and a
rewrite of most of `crates/content` plus `emit.rs` and the binding content
types. It should be argued as its own proposal against its own benefit, not
carried in on Automerge's coat-tails — and the one thing it must not claim to
deliver is compatibility with Automerge, since B is what would need that and B
is not recommended.

## Reproducing

The probes are four small programs against `automerge = "0.11.0"`, none of them
touching this workspace:

1. **Determinism.** Build one text+mark document two ways (one splice vs. two),
   with a random and then a pinned `ActorId`; compare `text()` and `save()`.
   Fork, edit both sides, merge in each order, compare again.
2. **Mark clipping.** `mark(name: "anchor", value: "thread-1", 0..9)` then
   `mark(name: "anchor", value: "thread-2", 4..15)`; read `marks()` back.
3. **Size.** A `cdylib` calling `splice_text` / `split_block` / `mark` / `save`
   / `load` / `spans`, built with
   `RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --release
   --target wasm32-unknown-unknown`, against an empty `cdylib` baseline.
4. **Anchor rebase.** Anchor a middle paragraph, call `update_text` with the
   paragraphs reordered, read back what the mark and a `Cursor` now cover.
