# Automerge as the Content Model

> **Implementation**: `crates/content/src` (the model this would replace)
> **Related**: [DOCUMENT_STORAGE.md](../canon/DOCUMENT_STORAGE.md), [PREVIEW.md](../canon/PREVIEW.md)
> **Status**: spike. Measured against `automerge` 0.11.0 and `yrs` 0.26/0.27; recommendation is *no* to the substrate, *yes* to the shape while the window is open.

## TL;DR

`Content` and Automerge's rich-text model are near-isomorphic already: both are
one text sequence with U+FFFC object slots, ranged marks, and a container path
per block. Adopting Automerge's *shape* is therefore cheap in concept and
buys three invariants' worth of simplification. Adopting Automerge as the
*substrate* costs byte-stability, which is load-bearing, and measurably
regresses the one edit lane we built for. As a **maintenance offload** it
inverts: the crate's defects are all in the layer Automerge does not implement.
`yrs` is smaller and more used but fits the model far worse and offloads
strictly less. Being pre-1.0 makes the *migration* cheap, not the steady-state
bill, and what the open window licenses is the shape — of which the vocabulary
half (A1) is nearly free and the block-marker half (A2) buys a merge-safety
property a single writer is not spending. Keep the model; if collaboration
becomes a goal, adapt to Automerge at a session seam rather than at rest.

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

## yrs, measured the same way

If the motive is offloading maintenance, the library choice should follow the
maintenance evidence rather than the first name to mind. `yrs` (the Rust
y-crdt) is the obvious alternative: 1.13M recent downloads against Automerge's
210K, backed by Yjs's deployment base.

It wins the axes Automerge lost, loses the ones Automerge won, and fails worse
on the one that decides the question.

| Axis | Automerge 0.11.0 | yrs 0.26/0.27 |
|---|---|---|
| Model fit to `Content` | near-isomorphic | **poor** — see below |
| Same content, two histories | breaks (213 vs 222 B) | breaks (30 vs 40 B) |
| Default identity | `ActorId::random()` | `ClientID::random()` |
| Converged merge, opposite orders | breaks | **holds** |
| Overlapping same-name marks | clipped `[0,9)` → `[0,4)` | clipped `[0,9)` → `[0,4)` |
| Anchor across a verbatim block move | wrong paragraph | wrong paragraph |
| Stale-text writer built in | `update_text` / `update_spans` | **none** |
| WASM, encode + decode + read | 416 KB gzip | **94 KB gzip** |
| 31-char field | 188 B | **44 B** |
| 1080-char field, one write | **220 B** | 1094 B |
| 300 edit rounds on it | 1651 B (7.5×) | 6859 B (6.3×) |
| `unsafe` blocks | **0** | 73 |
| Transitive crates, unioned with ours | 55 | **43** |
| Current release builds on rustc 1.94.1 | yes | **no** |

Four of those need saying plainly.

**The model fit is the disqualifier.** Automerge's block markers made the
convergence table above almost a mapping. yrs offers neither half of it.
Formatting is `Attrs = HashMap<Arc<str>, Any>` — a literal per-position
attribute map, the exact structure `delta.rs` names as unable to carry the mark
algebra, confirmed by the same clipping probe. Block structure is not in the
sequence at all: it lives in a separate `XmlFragment` **tree**, which is what
`y-prosemirror` drives. Storing the tree is precisely what `Content` refuses —
"the line tree is *derived* from this flat list … never stored, so a split/join
is a single-char edit with no paragraph identity to reconcile". yrs would force
back the design the model was written to avoid.

**It offloads strictly less.** There is no `update_text` equivalent, so
`delta.rs` — the one file Automerge would genuinely take over, and the one file
that has needed no fixing — stays entirely ours. On the maintenance argument
specifically, yrs takes over nothing that is hard.

**It is the wrong direction on the security axis.** 73 `unsafe` blocks against
Automerge's zero. The dependency tree is smaller and cleaner (no compression,
no crypto), which is a real gain, but memory-safety assurance is the axis the
motive names first.

**Its current release does not compile here.** `cargo add yrs` resolves 0.27.4,
which fails on rustc 1.94.1 (`if let` guards), and the crate declares no
`rust-version` — so Cargo cannot resolve away from it, and the failure surfaces
as a compile error inside a dependency. 0.26.0 builds. This is the kind of thing
an offload inherits.

The one place yrs is clearly better is size: 94 KB gzip against 416 KB, roughly
13% on the core bundle instead of 63%, and a third of Automerge's per-field
overhead. If the substrate question ever reopens on capability grounds, that
number is worth remembering. It does not reopen it here.

## Greenfield: would yrs's model have been right from scratch?

Migration cost aside, is `Content` actually a good model, or merely the
incumbent? Asked of yrs specifically, the answer is **no**, for one decisive
reason and against one genuine trade the flat model loses.

**The attribute map is a ceiling, not an accident.** `Attrs` is one value per
key per position, so overlapping same-type marks and two identity anchors over
one range are unrepresentable — and two reviewers commenting on overlapping
spans is ordinary, not exotic. Greenfield freedom does not lift that; it is the
model's shape. Key-mangling (`anchor-{id}`) restores the capability and spends
the key space to do it, leaving no way to ask for "every anchor" but a prefix
scan. This is Peritext's own critique of attribute-map rich text, and it is why
`Mark` carries a kind rather than a key.

**A CRDT could not have been the resting form either.** Every CRDT stores
replica identity per element, so equal content never implies equal bytes
(§ Byte-stability, measured for both libraries). Content-addressed documents
therefore need a canonical projection whatever the substrate — `serial.rs` gets
written in every timeline. The escape hatch, hashing the materialized view
rather than the stored bytes, *is* writing `serial.rs`. So yrs was never a
candidate to be the model; at most it was an addition beside one.

**Where a tree genuinely beats us, and beats Automerge too.** Two adjacent
blockquotes in a yrs `XmlFragment` are two nodes, distinct by construction — no
discriminator, no adjacency rule. `Container::instance` is the receipt the flat
encoding pays for the same fact, and Automerge's `parents`-as-type-names pays it
too. On this one axis the tree is strictly better than both flat encodings.

The trade it buys that with is the one `Content` opens on: storing the tree makes
a paragraph split or join a node surgery with paragraph identity to reconcile,
where the flat encoding makes it one `\n`. For an engine whose authoring form is
Markdown — where block structure is *derived* from a line's prefix and every edit
is a text edit — flat is the better side of that trade, and `instance` is a fair
price. For a ProseMirror-style structured editor with Markdown as the export, the
tree wins instead. The fork is the product's, not the library's.

**What the greenfield answer actually is.** Automerge's *shape*: flat USV text,
block markers in the sequence rather than a `\n`-keyed sidecar, marks carrying
identity rather than map keys, open string discriminators — with a canonical
byte form as the resting form and a CRDT adapter only if collaboration arrives.
That is A. The greenfield answer and the migration recommendation converge,
which is the useful part: A is not the cheapest change from here, it is the model
worth having.

One condition flips this. Had multi-writer collaboration been a day-one
requirement, the calculus inverts: hand-rolling convergence would be the error,
and a two-artifact design (CRDT for the session, canonical form for rest) would
be the starting point rather than the fallback. Quillmark's requirements are
single-writer, content-addressed, and Markdown-first, and the answer is
conditional on all three.

## What pre-1.0 changes, and what it does not

Quillmark is 0.111.0 and already ships breaking minors, so "this is the moment
for traumatic changes" is right, and it discounts three of the objections
above:

- **The SemVer-promise objection mostly dissolves.** Pinning a published crate
  to a pre-1.0 upstream matters far less when the published crate breaks freely
  itself.
- **The schema-version event is cheap.** A migration off `0.93.0` costs what the
  deployed corpus costs, and that is at its smallest now.
- **The rewrite's *breakage* half is free.** The labor stays; the compatibility
  ceremony around it does not.

None of those were the argument. The costs that decide it are steady-state, not
migration:

| Cost | Does 1.0 make it worse later? |
|---|---|
| Byte-stability lost (divergence detection, cache keys) | No — it is gone permanently either way |
| Marks clipped on overlap | No — architectural |
| Anchors silently re-homed across a move | No — architectural |
| +34 crates, a binary decoder, 63% bundle | No — permanent |
| The bugs sit in the layer the substrate keeps | No — that is what the fix history shows |

Being pre-1.0 lowers the price of *changing*, not the price of *living with the
result*. The substrate's bill is almost all the second kind.

The corollary runs the other way, and it is the useful one: the change this
argument most strongly licenses is **A**. A is a schema-version event with a
migration — exactly the cost that is cheap now and expensive after 1.0 — and its
benefit (deleted invariants, deleted code, no dependency) is permanent. If the
window is the reason to act, A is what the window is for.

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

**Take the shape — but it is two changes, and single-writer splits them.** They
were bundled above because Automerge ships both. Nothing else couples them, and
their costs differ by an order of magnitude.

**A1, one spelling per vocabulary.** Today a built-in carries its payload in
named sibling keys (`{"kind":"heading","level":1}`) and an unknown carries it in
an opaque bag (`{"kind":"callout","attrs":{…}}`). Every mechanism in
§ Promoting a vocabulary member exists to bridge those two spellings:
`fold_legacy_attrs` at three call sites, `RESERVED_LINE_KINDS` /
`RESERVED_CONTAINERS` / `RESERVED_MARK_TYPES` with their three `reject_*_attrs`
twins, the three `ReservedUnknown*` invariants, and the rule pinning a new
`MarkKind` to the ordinal before `Unknown`. Give every member the *same*
spelling — `{"kind":"heading","attrs":{"level":1}}` — and all of it goes, because
promotion stops being an encoding change: the bytes a build wrote while
`callout` was unknown are the bytes the build that knows it reads. The sort
tie-break becomes the type string, which two builds compute identically whether
or not either knows the member, so the ordinal rule has nothing left to protect.
It is a schema-version event and a pure re-encode. It moves no text and no
offset.

**A2, block markers in the sequence.** A U+FFFC marker opens each block carrying
`{type, parents, attrs}`; a `\n` with no marker following it is a within-block
break. That deletes `Line::continues` (which reaches fourteen files),
`Invariant::LineCountMismatch`, `FirstLineContinues`,
`ContinuesAcrossContainers`, and `LineKindMismatch::IslandNotOneSlot`, and states
a code fence's `lang` once instead of on every line of it.

Its cost is not the deletions' size but the **coordinate space**: markers occupy
USV positions, so every offset moves. `FieldRegion.span`, `ContentHit.pos`,
`locate(field, pos)`, `Delta` ops (which must then never split a marker),
`emit.rs`'s slot table and per-segment source map, and every consumer's caret
arithmetic all shift, and the migration must rebase every mark range in every
stored row.

And its headline benefit — a split or join is one splice, with no parallel array
to reconcile — is a **merge-safety** property. With one writer, `ops.rs`
maintains that array transactionally and no concurrent edit can catch it
mid-update, so the guarantee is bought where it was not being lost. A2 is worth
what A1 is worth only under concurrent editing, which § Recommendation's first
paragraph has already declined.

So: **A1 now**, on the pre-1.0 window, as its own proposal arguing its own
benefit. **A2 deferred**, revisited if a structural editor or a second writer
ever makes the coordinate change pay for itself. Neither may claim to deliver
Automerge compatibility — B is what would need that, and B is not recommended.

## Reproducing

Small programs against `automerge = "0.11.0"` and `yrs = "0.26.0"`, none of them
touching this workspace. Probes 1–4 run against both; the yrs spellings are
`Doc::with_options(Options::with_client_id(…))`, `Text::format` with an `Attrs`
map, `encode_state_as_update_v1`, and `Text::sticky_index` for the anchor.

1. **Determinism.** Build one text+mark document two ways (one splice vs. two),
   with a random and then a pinned `ActorId`; compare `text()` and `save()`.
   Fork, edit both sides, merge in each order, compare again.
2. **Mark clipping.** `mark(name: "anchor", value: "thread-1", 0..9)` then
   `mark(name: "anchor", value: "thread-2", 4..15)`; read `marks()` back.
3. **Size.** A `cdylib` calling `splice_text` / `split_block` / `mark` / `save`
   / `load` / `spans`, built with
   `RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --release
   --target wasm32-unknown-unknown`, against an empty `cdylib` baseline.
4. **Anchor rebase.** Anchor a middle paragraph, reorder the paragraphs
   (`update_text` on Automerge; the delete-then-insert a diff would emit, on
   yrs), read back what the mark, `Cursor`, or `StickyIndex` now covers.
5. **Fix locality.** `git log --format=%s -- crates/content | grep ^fix`, then
   `git show --name-only` each, tallying commits per file.
