# Content codec: v1.0.0 hardening

> **Implementation**: `crates/content/`
> **Related**: [`canon/DOCUMENT_STORAGE.md`](../canon/DOCUMENT_STORAGE.md)

## TL;DR

The canonical content form is close to freezable. Its open-set discipline is
right and its invariant surface is thorough. Five things do not yet hold the
promise the freeze makes, and each gets harder to fix after 1.0.0: the `loss`
axis and table cells are closed where everything around them is open, promotion
of a vocabulary member to a built-in is undefined and silently lossy, the
`Value` decode lane has no depth guard, and no public enum is
`#[non_exhaustive]`.

Ordered by cost of deferring past 1.0.0. Every claim below is reproduced against
the code at `deb7802`.

## What the freeze already gets right

Stated so the recommendations read as deltas, not as a rewrite.

- **One serializer for the seam and for storage.** No pair of encoders to keep
  aligned, and `to_canonical_value` / `to_canonical_json` are the same tree.
- **Recursive key sort, feature-independent.** Canonical bytes do not depend on
  `serde_json/preserve_order` in the consumer's graph. Opaque `attrs` and
  `props` are hash input and sort with everything else.
- **Open sets on four axes** — mark `type`, line `kind`, container, island
  `type` — each round-tripping opaque and projecting to a nearest safe
  neighbour. This is the decision that makes vocabulary growth cheap.
- **The authored/storage lane split.** `from_authored_value` rejects `attrs`
  beside a built-in name; `from_canonical_json` accepts it. The two lanes want
  opposite answers and get them.
- **Decode validates.** `ParseError::Invalid` means storage cannot round-trip a
  malformed value, and `normalize` is the repair-side twin of every invariant.
- **Deterministic ids.** Island ids are a pure function of import position;
  anchor ids are caller-supplied with the never-ambient rule stated as to why
  the difference is not a contradiction.

## 1. `loss` is closed where every neighbour is open

`serial::loss_from_str` maps an unrecognized class to `Loss::Unrepresentable`,
and `loss_to_str` then writes `"unrepresentable"`. The value is not carried — it
is **rewritten**.

```
in   {"id":"i1","loss":"partial","props":{},"type":"widget"}
out  {"id":"i1","loss":"unrepresentable","props":{},"type":"widget"}
```

Two consequences. A future writer's loss class is destroyed by any older reader
that touches the document. And the content hash of that document moves under a
reader that changed nothing, which is exactly what byte-stability promises will
not happen.

The safe-end default is the right *behavior*; the discard is the bug.
`to_markdown` already dispatches on island type and never reads `Loss`, so
carrying the raw tag costs nothing behaviorally.

**Recommendation.** Keep the raw string beside the parsed class — a
`Loss::Unknown(String)` arm, or a `raw: String` on `Island` — project it as
`Unrepresentable`, and re-emit the tag verbatim. This is the same shape the
other four axes already use.

## 2. Promotion is undefined, and it is where old documents break

`DOCUMENT_STORAGE.md` § Open vocabularies says adding a construct to a
vocabulary is not a schema-version event. That is true for *adding an unknown*.
The reverse trip — a later release **promoting** a tag to a built-in — is not
covered anywhere, and four things move at once:

| What moves | Why |
|---|---|
| Encoding | `{"kind":"callout","attrs":{…}}` becomes `{"kind":"callout",…}` with named siblings. |
| Old blobs | `line_kind_from_value` matches `"callout"` *before* the `Unknown` fallthrough, so the stored `attrs` are dropped unread. |
| Normalization | A promoted mark that `is_formatting()` starts unioning adjacent runs that previously stayed separate. |
| Sort order | `attrs_key` for `Unknown` is `tag\0{json}`; a built-in's is whatever its arm returns, so the `(start,end,ord)` tie-break can reorder. |

The first two are silent data loss on precisely the documents the open set
exists to protect. The last two move canonical bytes for an unchanged document.

`MarkKind::ord` says "stable across releases — part of the freeze" but does not
say *where* a new variant goes. It has to take the slot immediately before
`Unknown` (today: 7, pushing `Unknown` to 8), or an old and a new reader sort
the same document differently.

**Recommendation.** Add a "Promoting a vocabulary member" section to
`DOCUMENT_STORAGE.md` pinning four rules, and a test per rule:

1. A promoted built-in's decoder also accepts the legacy `attrs` form and
   migrates it into the named fields. Without this, promotion silently eats
   every stored blob's payload.
2. A new `MarkKind` takes the ordinal immediately before `Unknown`.
3. Promoting a mark into the *formatting* class changes stored meaning (union),
   so it is a canonical-byte event and takes the read-repair / accepted-movement
   treatment § Byte-stability already documents for migrated rows.
4. `RESERVED_*` grows on promotion, so a host still authoring the old
   `Unknown{tag}` starts getting `ReservedUnknown*`. That is intended; say so.

## 3. Table cells are the one sub-structure with no carrier

Every other opaque payload survives an old reader. A table **cell** does not.
`normalize`'s `canon_cell` rebuilds each cell as `cell_to_value(text, marks)`,
a fresh two-key map, so any other key is destroyed:

```
in   props: {"caption":"Fig 1", "header":[{"colspan":2,"marks":[],"text":"h"}], …}
out  props: {"caption":"Fig 1", "header":[{"marks":[],"text":"h"}], …}
```

A new *table* prop (`caption`) survives; a new *cell* prop (`colspan`,
`rowspan`, per-cell alignment) does not. Cells are the likeliest place the table
type grows, and they are the one place growth is not carriable.

The same rebuild silently drops a cell mark that fails to parse
(`parse_cell` uses `filter_map(… .ok())`).

**Recommendation.** Preserve unrecognized cell keys through `canon_cell`
(rebuild from the existing object rather than a fresh map), and make the
authored lane reject an unparseable cell mark instead of dropping it.

## 4. The two doors to the decoder are not guarded alike

`props` and `attrs` nesting is unbounded. `sorted_value`, `sort_keys_owned`,
`is_value_key_sorted`, and `serde_json::Value`'s own `Drop` each recurse once
per level.

The string lane is accidentally safe: `serde_json::from_str` enforces a 128-deep
default limit, so `Content::from_canonical_json` returns
`ParseError::Json("recursion limit exceeded")`.

The `Value` lane has no such guard. Measured on a native 8 MB stack (debug):

| Depth | `serial::from_canonical_value` |
|---|---|
| 1 000 | Ok |
| 5 000 | stack overflow, process abort |

That lane is the **host-authored** one. `install()` reaches it through
`js_to_authored_content` → `serde_wasm_bindgen::from_value`, which has no depth
limit either, and a JS caller builds a deep value in one loop. On wasm32 the
stack is 1 MB and a stack overflow is an unrecoverable trap, not an error.

This is the container-nesting fix (`Invariant::NestingTooDeep`,
`MAX_NESTING_DEPTH`) on the axis that did not get capped.

**Recommendation.** Add a JSON-depth invariant checked in `Content::validate`,
covering island `props` and every `attrs` bag. A cap of 128 rejects nothing a
stored blob can carry, since `from_str` already refuses deeper. Making
`sort_keys_owned` iterative is a complement, not a substitute — `Value`'s `Drop`
still recurses.

## 5. No public enum is `#[non_exhaustive]`

The crate is published, `quillmark-core` depends on it, and at 1.0.0 the
variant lists freeze for downstream matchers. None of these carry the attribute:

`MarkKind` · `LineKind` · `Container` · `Loss` · `Invariant` · `ParseError` ·
`ApplyError` · `ImportError` · `LineKindMismatch` · `MarkOp` · `LineOp` ·
`Op` · `Assoc` · `KnownIslandType`

The design's whole thesis is that the first four grow. `Invariant`,
`ParseError`, and `ApplyError` have grown with nearly every hardening pass. Past
1.0.0 each addition is a major bump for every consumer that matches — and the
attribute cannot be added later without one.

`#[non_exhaustive]` does not affect matches inside the defining crate, so the
in-crate exhaustiveness that `normalize` and `validate` rely on is unchanged.

**One deliberate exception.** `KnownIslandType`'s value is that adding a variant
is a compile error at every dispatch site, and two of those sites (the typst
backend's emitters) are in *other* crates, where `#[non_exhaustive]` would
force a `_` arm and defeat the guarantee. Keep it exhaustive, accept that a new
island type is semver-major, and say so in its rustdoc rather than leaving it
implicit.

Structs are the same question one step down: `Content`, `Line`, `Mark`,
`Island`, `Segment` have all-public fields, so a fifth `Content` component
(footnotes, decorations) is a major bump. `#[non_exhaustive]` there costs
external construction, so it needs a constructor first — worth deciding for
`Content` and `Line`, not urgent for the rest.

## 6. What the content object is, absent an envelope

Unknown top-level keys are dropped, and so is any unrecognized sibling key on a
built-in discriminator:

```
in   {"footnotes":[…], "islands":[], "lines":[…], "marks":[], "text":"hi"}
out  {"islands":[], "lines":[…], "marks":[], "text":"hi"}

in   lines[0]: {"containers":[], "id":"sec-1", "kind":"heading", "level":2}
out  lines[0]: {"containers":[], "kind":"heading", "level":2}
```

Inside `StoredDocument` this is covered: a new top-level key is a schema event
and the envelope's version-and-reject discipline catches it. But the content
object also travels **unversioned** — the WASM `Content` interface, `install`,
`getFieldContent`, `exportMarkdown`, `rebase`, and `CardWire`. There, a
future-shaped content degrades silently instead of being refused.

**Recommendation.** Decide and state it. Either the content object is only ever
valid inside a versioned envelope and the seam is same-build-only — cheapest,
and true today — or it carries its own optional version discriminator, which
costs bytes on every field. State the choice in canon; the drop is a contract
either way and today it is undocumented.

## 7. Limits: the content lane has none

`markdown-spec.md` § 8 pins document size, YAML depth, field count, and card
count. The content lane pins only container depth. Nothing bounds `marks.len()`,
`islands.len()`, text length, table area, or props size.

One amplification worth naming: `table_cols` takes the max of header, `aligns`,
and the *widest* row, and `pad_row` only ever grows. A single malformed
10 000-cell row inflates the header and every other row to 10 000 cells.

`normalize` and `validate` each materialize `text.chars().collect()` — O(text)
per keystroke on a live seam.

**Recommendation.** A limits table for the content lane, mirroring § 8, so
refusal is explicit rather than emergent: JSON depth (§ 4), marks per content,
islands per content, table columns and rows.

## 8. Testing the freeze

`golden_bytes_are_feature_independent` is the only byte-level pin, over an
11-character sample with two marks. It comments "if this string changes, the
freeze changed" — which is right, and is carrying more weight than one sample
can.

Three additions make the freeze mechanical rather than noticed:

1. **A golden corpus.** One checked-in canonical blob per vocabulary arm: every
   `LineKind` and `Container` including unknowns with attrs, every `MarkKind`,
   both island types plus an unknown one, each `Loss` class, astral text, a
   table with marked cells. Byte-compared.
2. **A forward-compat corpus.** Blobs shaped as a future writer would produce —
   unknown line kind, unknown container, unknown mark, unknown island type,
   unknown loss class, unknown top-level key, sibling key on a built-in — each
   asserting either byte-identical round-trip or a *documented* drop. Today
   three of those are silently lossy (§ 1, § 3, § 6) and no test says so.
3. **Two properties.** `from_canonical_json ∘ to_canonical_json` as a fixed
   point over arbitrary generated contents including unknown arms, and
   `normalize` idempotence over the same. The existing proptests cover the
   markdown round trip and delta rebase, not the canonical form itself.

## 9. Smaller notes

- An absent `attrs` decodes to `Value::Null` and re-encodes as `"attrs":null`.
  It converges after one pass, so it is cosmetic — but omitting it when null
  would parallel `continues`, which is already omitted when false.
- `MarkKind::Link` with no `url` decodes to `""` rather than erroring; `anchor`
  with no `id` decodes to `""` and is then caught by `AnchorIdCollision`. The
  authored lane should reject both up front, on the reserved-name rule's logic.
- `ParseError::Shape(&'static str)` carries no path or index. A 400-mark
  document that fails reports only `"mark start"`.
- `Container::ListItem`'s `start` and `ordinal` are unbounded `u64`. Export
  saturates on the sum, which is correct; `validate` does not ceiling them.
- The content model has no canon page. Its contract is spread across
  `crates/content/` rustdoc and `DOCUMENT_STORAGE.md` § Open vocabularies,
  § Island-id determinism, and § Anchor-id identity — sections that exist to
  document *storage*. A v1.0.0 wire format deserves its own page, with
  `DOCUMENT_STORAGE.md` pointing at it for the parts it currently owns.

## Suggested order

Blocking on 1.0.0, in the sense that each is unfixable or expensive after it:

1. `#[non_exhaustive]` (§ 5) — free now, needs a 2.0 later.
2. `loss` carries its tag (§ 1) — a wire-behavior change.
3. Cell keys survive (§ 3) — a wire-behavior change.
4. Promotion protocol written and tested (§ 2) — the first promotion after
   1.0.0 does the damage.
5. JSON depth invariant (§ 4) — an abort on a host-reachable path.

Deferrable past 1.0.0 without cost:

6. Golden and forward-compat corpora (§ 8).
7. Limits table (§ 7), envelope statement (§ 6), canon page (§ 9).
