# Document Storage Serialization

> **Implementation**: `crates/core/src/document/`

## TL;DR

`Document` is the typed in-memory model of a Quillmark Markdown file. Its
layout tracks the evolving Quillmark model and is **not** a stable interface.
To persist documents (e.g. in a database) without storing Markdown (whose
syntax also evolves), `Document` serializes to a **versioned JSON envelope**,
`StoredDocument`, whose wire format is frozen per schema version.

## When to use it

| Form | Round-trips? | Stable for storage? |
|---|---|---|
| Markdown (`Document::to_markdown`) | Yes | No: syntax evolves |
| `StoredDocument` JSON | Yes: lossless | Yes: frozen per schema version |

Use `StoredDocument` JSON whenever a `Document` must survive a process
restart or a crate upgrade: database rows, caches, message payloads.

`Document::to_plate_json` also exists as a lossy, one-way export to
Plate-shaped backends; it is core-only (not exposed by the WASM or Python
bindings) and never a storage option.

## Design Principles

1. **Versioned envelope**: every blob carries a `schema` tag; readers
   dispatch on it and reject unknown versions.
2. **Frozen DTO per version**: each schema version has its own standalone
   type tree (`DocumentV0_92_0`, `CardV0_92_0`, …). These are never changed
   once shipped.
3. **Decoupled from the live model**: internal refactors of `Document` and
   its components only touch conversion code, never the wire format.
4. **Transparent API**: `Document` serializes through the envelope via
   `#[serde(into / try_from)]`; callers use `serde_json` directly.

## The Format

The current schema (`quillmark/document@0.93.0`) carries each card's full
ordered payload: typed `$` system metadata, user fields, and YAML
comments interleaved in source order: as a single discriminated-union
item list. This is what makes inline-comment preservation symmetric across
the `$`/non-`$` boundary. The payload shape is unchanged since `0.92.0`;
`body` is the canonical `Content` embedded structurally (a nested
object, not a markdown string); see Byte-stability.

```json
{
  "schema": "quillmark/document@0.93.0",
  "main": {
    "payload": {
      "items": [
        { "type": "quill", "value": "usaf_memo@0.1" },
        { "type": "kind",  "value": "main" },
        { "type": "ext",   "value": { "presentation": { "title": "Greeting Card" } } },
        { "type": "field", "key": "title", "value": "Hi" }
      ]
    },
    "body": { "islands": [], "lines": [ { "containers": [], "kind": "para" } ], "marks": [], "text": "Hi" }
  },
  "cards": [ ... ]
}
```

`StoredDocument` is an internally-tagged enum (`#[serde(tag = "schema")]`);
each variant carries a frozen DTO tree. Quill references are stored as
strings (parsed back via `QuillReference::from_str`). The discriminator on
payload items is `type` (not `kind`) to keep it unambiguous next to the
`$kind` metadata semantic. The full variant set is `quill | kind | id |
ext | seed | field | comment`; the `ext` and `seed` variants carry the
`$ext` / `$seed` maps verbatim and are stripped from `to_plate_json()`
before backends see it.
Parse-time warnings live on `Document` (`warnings: Vec<Diagnostic>`) but
are excluded from `PartialEq` and not serialized, so they never reach this
format.

### Legacy schema (V0_92_0)

Documents written before `0.93.0` carry
`"schema": "quillmark/document@0.92.0"` and store the card `body` as a
markdown string rather than the embedded canonical content. Readers accept
them and migrate forward to V0_93_0 on load; writers do not produce this
shape. The one hop that can reject is the body cold-import (see
Byte-stability). Tags older than `@0.92.0` (`@0.81.0`, `@0.82.0`) have no
reader; a blob carrying one is rejected as an unknown schema version.

## Byte-stability

Serialization is **byte-deterministic** within a given schema version:
equal `Document`s (by `PartialEq`) produce byte-equal JSON, and the same
document re-serialized in any later patch or minor release tagged with
the same `schema` produces the same bytes. This is load-bearing for
consumers that content-hash stored documents (template-divergence
detection, cache keys).

**Two disciplines in one envelope.** The outer envelope: struct field
order, the `cards` array, payload field values: stays compact,
insertion-ordered `serde_json`: `serde_json::Value` inside payload field
values keeps YAML insertion order via the workspace's
`serde_json/preserve_order` feature, and no whole-envelope key sort is
applied. Every `body` subtree, by contrast, is the recursively key-sorted
**canonical content form** (`CanonicalContent` in `dto.rs`): byte-identical
to `rt.to_canonical_json()` and independent of `preserve_order`, even in a
consumer crate graph that lacks the feature. Sortedness is semantic
*inside* the content (mark/island/attribute order carries no meaning, so the
serializer commits to one bit pattern); insertion order is semantic
*outside* it (payload item order is source order, and matters).

The guarantee follows from: struct field order is fixed in the frozen
DTO tree; `Vec` fields preserve order by definition; the two disciplines
above each hold at their respective level. No whitespace normalization is
applied: the output is `serde_json`'s compact form otherwise. Bumping the
`schema` version is the only event that may change the byte layout of a
document written by the current writer.

**Migrated rows: a conditional caveat.** The guarantee above is unconditional
for a document the current writer serializes directly. A row still carrying
a legacy schema tag migrates forward on read, and the `0.92.0 → 0.93.0` hop
cold-imports the stored markdown `body` string through the same
Markdown → richtext path `Document::parse` uses. Byte-stability of
*that* row across a crate upgrade is therefore conditional on
`pulldown-cmark` parsing the body the same way: a forced parser or security
bump can move the migrated bytes even though the schema tag does not
change. Two ways to manage this:

- **Read-repair.** Rewrite a row under its current schema tag once it has
  been read and migrated, so the content form: not the legacy markdown
  string: becomes its byte-stable resting state.
- **Accept the movement.** For rows left un-repaired, treat a forced
  parser/security bump as either a schema-version event (if a hard
  guarantee is required) or an accepted, logged hash movement on
  not-yet-migrated rows.

## Open vocabularies

The envelope's version-and-reject discipline covers the document's **shape**:
the schema tag, the DTO tree, the keys a `body` object carries. It does *not*
cover the content's **vocabularies**. Every discriminator inside a `body` is an
open set: a mark `type`, an island `type`, a line `kind`, a container name, and
an island's `loss` class this build does not recognize each round-trip
byte-identically and project as their nearest safe neighbor.

**Two mechanisms, split by payload.** A block axis carries one, so its built-ins
decode eagerly into typed fields (`LineKind::Heading { level }`) and an
unrecognized member needs a sibling `Unknown` arm to hold its `attrs`. The two
island axes carry none, so the wire string *is* the stored value and the closed
set is a **view** over it (`KnownIslandType::parse`, `Loss::fidelity`). The
split decides which axes need the reserved-name rule below: a carrier axis has
two spellings of a built-in's name and must reject one, a view axis has one.

| Axis | Carrier | Unknown value projects as |
|---|---|---|
| Mark `type` | `Unknown { tag, attrs }` | no delimiters (the text renders bare) |
| Line `kind` | `Unknown { tag, attrs }` | a paragraph |
| Container | `Unknown { tag, attrs }` | transparent: its lines render at the enclosing level |
| Island `type` | the `String` itself | a placeholder comment; the props survive in storage |
| Island `loss` | the `String` itself | `Fidelity::Unrepresentable`, via `Loss::fidelity`: never a claim of fidelity on a class this build cannot read |

The consequence, and the point: **adding a construct to any of these
vocabularies is not a schema-version event.** An older reader degrades a future
callout to a plain paragraph rather than refusing the document, and a reader
that does understand it sees it whole, because the tag and attrs round-tripped
untouched. Openness is the same on all five axes: the block axes are open on the
mark axis' terms, not one step behind it, and both island axes carry their raw
string rather than rewriting it: a reader that merely opens a document must not
move its content hash (§ Byte-stability).

Unknown *keys* survive in designated carriers only, and the boundary is worth
stating because it is not the discriminator boundary:

- **Opaque carriers keep what they hold.** Unknown `attrs` on all three block
  axes, island `props`, a table island's top-level props, and a table **cell's**
  own keys all round-trip untouched.
- **Envelopes drop what they do not name, by design.** An island, line, mark, or
  container object is decoded into a struct and re-minted from its fields, so an
  unrecognized sibling key beside `id`/`kind`/`start` does not survive. Growing
  those shapes is a schema event; that is what the envelope's version-and-reject
  discipline is for.

A table cell sits on the first side because it never became a struct, and it
earns the place: cells are where the `table` type is likeliest to grow
(`colspan`, `rowspan`, a per-cell style handle). Canonicalization therefore
rewrites a cell's `text` and `marks` in place rather than minting a fresh
`{text, marks}` object.

Three rules bound the openness:

- **Payload depth is capped at `MAX_JSON_DEPTH` (128).** An opaque bag is host
  JSON of arbitrary shape, but not arbitrary depth: key canonicalization, the
  content-hash key, and `serde_json::Value`'s own `Drop` each recurse one frame
  per level, so an unbounded bag overflows the stack: on wasm32, a trap that
  takes the module down rather than an error the host can catch. The cap is the
  one `serde_json::from_str` already enforces, so it refuses nothing a stored
  blob can carry. It is stated on its own because the `Value` lane: the
  host-authored one, which `install` reaches: is not parsed from a string, so
  nothing else bounds it. A bag is refused where the decoder reads it off the
  wire, before it is cloned into the model; `Content::validate` restates it as
  `Invariant::JsonTooDeep` for content that never went through a decoder. This
  is `Invariant::NestingTooDeep`'s container cap on the payload axis.

- **Payload rides `attrs`.** A built-in carries its payload in named sibling
  keys (`level`, `lang`, `url`); an unknown carries it in one opaque `attrs`
  object. A *new* construct must therefore put its payload under `attrs` to
  survive a reader that predates it: a sibling key an old reader does not
  read is dropped on re-encode.
- **No reserved name reuse.** An unknown may not take a built-in's name
  (`heading`, `quote`, `link`, …): it would serialize as the built-in and parse
  back as one, silently dropping its attrs. `Content::validate` rejects this
  (`Invariant::ReservedUnknownTag` / `ReservedUnknownLineKind` /
  `ReservedUnknownContainer`) for an in-process Rust construction. The rule is
  the three carrier axes' alone, and the `Unknown` arm is what makes it
  necessary: a view axis has one value per wire string, so a built-in's name *is*
  that built-in, with no second spelling to collide with it. The wire
  reaches that check on neither block axis nor the mark axis: a decoder resolves
  the built-in name *before* the `Unknown` fallthrough, so `{"kind": "para",
  "attrs": {…}}` decodes to `Para` and the attrs are dropped unread. The two
  wire lanes therefore split:

    - **The authored lane rejects it.** `attrs` beside a built-in discriminator
      is a shape error (`serial::line_kind_from_authored_value` and its two
      twins for the op wire, `serial::from_authored_value` for a whole content).
      An op or an `install` is host-authored now, so that shape means a stale
      copy of the built-in list, never a document from the past. Reads that hand
      back stored content (`exportMarkdown`, `rebase`) are storage-lane, not
      this one. The same split governs an unreadable **table-cell mark**.
      Storage skips it: `serial::parse_cell` is lenient, and normalization makes
      the skip permanent. The authored lane refuses it, because a host's
      malformed mark vanishing with no signal is the silent corruption this rule
      exists to catch.
    - **The storage lane accepts it, and must.** A blob written while `callout`
      was unknown carries `{"kind": "callout", "attrs": {…}}`; the release that
      promotes `callout` to a built-in has to keep opening it. Rejecting at
      `from_canonical_json` would refuse documents at rest exactly when the
      vocabulary grows: the failure this section exists to prevent. Opening it
      is half: the promoted arm also *reads* the bag rather than dropping it, see
      Promoting a vocabulary member.

The opaque attrs are hash input like everything else in the canonical form, so
they are recursively key-sorted along with the rest (see Byte-stability). What
*does* remain a schema event is a change to the content object's own structure:
a new top-level key beside `text`/`lines`/`marks`/`islands`, or a changed
meaning for an existing discriminator.

### What openness buys a consumer

"Project an unknown as its nearest safe neighbor" serves one consumer posture of
two:

- **Render-only.** Degrade and move on. An unknown line is a paragraph, an
  unknown container is absent; nothing is written back, so nothing is lost.
- **Read-modify-write.** An editor lowers a whole-field diff, restating every
  line's `kind` and `containers` whenever any of them changed. A construct its
  tree cannot hold is gone on the next keystroke: the document opens intact and
  saves mangled. Such a consumer carries unknowns *inertly* instead: a carrier
  node per axis that renders as the nearest safe neighbor and re-emits the tag
  and `attrs` verbatim.

Classifying known-vs-unknown is therefore the read-modify-write consumer's
problem, and the boundary answers it: `isUnknownLine` / `isUnknownContainer` /
`isUnknownMark` / `isUnknownIsland` on the WASM surface, `KnownIslandType::parse`
and the `RESERVED_*` lists in Rust. A consumer that re-derives the built-in list
has re-coupled to a closed set, and misreads the first release that adds a
built-in.

The bound: **the carrier preserves unknown tags, not unknown payloads on known
tags.** A future `kind: "footnote"` carrying a sibling `ref` loses `ref` at any
consumer that predates it, predicates or no: the first rule above (*payload
rides `attrs`*) read from the other end.

## Promoting a vocabulary member

Adding an unknown is not a schema event. The reverse trip: a later release
**promoting** a tag to a built-in, which is what the open set exists for: moves
four things at once.

| What moves | How |
|---|---|
| Encoding | `{"kind":"callout","attrs":{…}}` becomes `{"kind":"callout",…}` with named siblings. |
| Stored blobs | The decoder resolves the built-in name *before* the `Unknown` fallthrough, so the stored `attrs` reach an arm that reads siblings. |
| Normalization | A promoted mark that `is_formatting()` unions adjacent runs that were two marks. |
| Sort order | `attrs_key` is `tag\0{json}` for an `Unknown` and whatever its own arm returns for a built-in, so the `(start, end, ord)` tie-break can reorder. |

The first two are data loss on exactly the documents the open set protects; the
last two move canonical bytes for a document nobody edited. Four rules bound
them.

**A promoted built-in's decoder also reads the legacy `attrs` form.** This is
the load-bearing one: without it the first promotion eats every stored blob's
payload, silently. It is structural rather than a discipline: `fold_legacy_attrs`
folds an `attrs` bag into the object whenever the discriminator names a reserved
member, before the built-in arms run, on all three block-and-mark axes. A named
sibling wins over a bag entry, only reserved names fold, and the discriminator is
read from the original object. A promotion adds its name to `RESERVED_*` in the
same edit that adds its arm, so it inherits the fold and carries its own legacy
form. Re-encoding a folded blob writes the promoted spelling, so the read is a
byte movement of the read-repair kind that § Byte-stability governs.

The island axis has no such gap: `props` is the payload carrier for known and
unknown types alike, so a promoted island type reads what its unknown wrote. Nor
does `loss`, which carries no payload.

**A new `MarkKind` takes the ordinal immediately before `Unknown`.** That is the
one placement where a build that knows the type and a build that reads it as
`Unknown` order the mark identically against every built-in; any other slot gives
one document two canonical forms, one per reader. The rule is stated on
`MarkKind::ord` itself. It is the mark axis' alone: the block axes sort by
nothing.

**Promoting a mark into the formatting class changes stored meaning**, since
adjacent runs that round-trip as two marks begin to union. It is a
canonical-byte event, and takes the read-repair-or-accept-the-movement treatment
that § Byte-stability sets out for migrated rows.

**`RESERVED_*` growth rejects previously-valid authored content**, by design. A
host still authoring `Unknown { tag: "callout" }` after the promotion gets
`ReservedUnknownLineKind`, and `from_authored_value` starts refusing `attrs`
beside `"callout"`: the reserved-name rule above, applied to a name that changed
sides. From the host's seat it reads as a release breaking its writes, so it is a
release note, not a silent tightening.

## The two id handles

Islands and anchors each carry an id, and both are the same handle: **opaque,
unique within a scope, hash input, stable for the session, rebased through edits
and never rewritten.** They differ on one axis: **who mints one, and why only
they can.** Uniqueness scope, collision response, and markdown round-trip all
follow from that.

|                     | Island `id`                                        | Anchor `id`                                        |
| ------------------- | -------------------------------------------------- | -------------------------------------------------- |
| Minted by           | the engine, at import                              | the caller                                         |
| Because             | content determines it: the nth island minted      | the referent is external; no content determines it |
| Unique across       | the `Content`'s islands                            | the `Content`'s prose marks                        |
| Required            | yes                                                | yes: the empty id is rejected                     |
| On collision        | unreachable: mint is sequential; `validate` scans | `add` rejects                                      |
| Markdown round-trip | re-minted identically                              | lost: export emits none, import mints none        |

A card carries no id handle: it is addressed by index, and a consumer needing a
durable per-card key of its own carries one in `$ext` under its own namespace
([CARDS.md](CARDS.md) § Out-of-band Metadata), which the engine round-trips and
never interprets.

Each section below states one handle's policy whole.

## Island-id determinism

An island's `id` is part of the canonical form (`{id, type, props, loss}`),
so it is hash input like every other field, and byte-stability's promise:
equal content → equal bytes, *whatever the producer*: requires that equal
islands carry equal ids. The rule: **an id is a deterministic function of
content, never drawn from an ambient source** (RNG, wall-clock, UUID,
allocation identity, session or process state). An ambient id would make
re-importing the same markdown yield different bytes for the same document,
silently breaking divergence detection and cache keys.

The normative scheme is the importer's positional `isl-{n}`: the nth island
minted takes `isl-{n-1}` (`mint_island`), so cold import is a pure function
of its markdown; export drops ids and re-import re-mints the same sequence.
Ids then travel with their island across edits: deleting a slot drops that
island and survivors keep their ids, so an id is *stable within a session*,
not re-derived from position. The invariant that holds for every `Content`,
checked by `Content::validate`, is therefore **uniqueness**
(`Invariant::IslandIdCollision`), not `id == isl-{index}`: after an edit
`isl-1` may legitimately sit at index 0.

The id stays in the hash input, so "canonical bytes == hash input" holds
exact: no id-stripping, no separate hash form. Any future id-minting
producer (a live editor, a richer island type) is bound by the same rule:
continue the positional sequence for appended islands; never mint an ambient
id.

## Anchor-id identity

An anchor (`MarkKind::Anchor { id }`) sits at the caller-minted end of the mint
axis (§ The two id handles) because it has **no markdown projection**: it
names an external referent (a comment thread, an editor bookmark) that no
content determines. The never-ambient rule therefore cannot apply, and need not:
that rule exists to keep *import* a pure function of markdown, and import mints
no anchor, so equal markdown still imports to equal bytes.

The policy: **an anchor id is caller-supplied, unique per `Content`, opaque and
invariant while the mark lives; the mark is best-effort under edits and absent
from markdown.**

- **Caller-supplied.** The engine mints no anchor id and cannot: only the
  consumer knows the referent. Each consumer (editor, MCP writer) supplies its
  own; the runtime persists it verbatim. Engine-minting is a non-goal by
  design: a counter is history-dependent (the same final content reached two
  ways carries different ids: the ambient source the twin rule forbids) and
  forfeits referent-convergence (two `add`s for one thread would mint two
  handles). A client wanting generated ids mints them client-side and supplies
  them; the auto-fallback middle ground (engine mints when the caller omits) is
  a non-goal too.
- **Unique per `Content`.** `add` *rejects* a collision: not replace (which
  silently retargets a live thread) nor coexist (already incoherent: an anchor
  is *the* handle `RemoveAnchor { id }` retains-out, so a shared id makes
  removing one destroy both). The empty id is rejected as a degenerate handle.
  Enforced at op time in the `Add` arm and as a `Content::validate` invariant
  (`Invariant::AnchorIdCollision`, the uniqueness scan the island check shares).
  Scope is prose `marks`: an anchor inside an island cell is unreachable by
  `RemoveAnchor` (it scans prose marks only), so it sits outside the op surface
  and outside this uniqueness check: explicit partial enforcement, not silent
  half-enforcement.
- **Opaque and invariant.** The runtime never rewrites an id. Positions rebase
  through splices (`map_pos`); the id passes untouched. A mark whose text is
  deleted, or moved-and-rewritten in one round, drops *whole*: never
  partially, never re-id'd (the documented diff-rebase residual).

No markdown round-trip guarantee: export emits nothing for an anchor and import
mints none, so a cold export→import loses every anchor. Anchors are edit-lane
infrastructure: they survive only through diff-rebase (`revise` / `rebase`).
Non-rendering is a property of review-time metadata, not a gap; a future render
projection (proof annotations, PDF destinations) would render the referent or a
position, never the id, so this policy holds either way.

## Schema Versioning

The schema version is tied to the **crate version at which the `Document`
wire format was last changed**: not the running crate version. The
current format was fixed in `0.93.0`, so the version tag is
`quillmark/document@0.93.0`; every later patch release writes that same
value, because patches do not change the format.

The oldest wire format still read is `0.92.0`: a unified payload-item list
(typed `$` entries living alongside user fields and comments in a single
`Vec<PayloadItem>`), a per-field `nested_fills` list so `!must_fill` markers
nested inside a field value survive a storage round-trip (the JSON `value`
projection is fill-free), and the `seed` payload-item variant (the `$seed`
per-card-kind overlay map). `0.93.0` leaves the payload model unchanged and
instead embeds the card `body` as the **canonical content**:
structurally, as a nested object, not a markdown string (see Byte-stability).

The V0_92_0 → V0_93_0 migration is the one hop that can fail: it
cold-imports the stored markdown `body` string through the same
Markdown → richtext path `Document::parse` uses, so a
pathologically over-nested legacy body is rejected
(`StorageError::Malformed`) rather than silently truncated. Only the newest
DTO converts to the live `Document`; a `0.92.0` blob migrates one hop first.

Schema tags older than `0.92.0` (`@0.81.0`, `@0.82.0`) have no reader: a
blob carrying one is rejected as an unknown version. Their shims are retired
because no stored population in those shapes remains on this lineage (see
"Adding a Schema Version" on retiring a variant).

## Adding a Schema Version

When the `Document` wire format changes again:

1. **Freeze** the current `DocumentV0_93_0` type tree: leave its struct
   /enum definitions and serde derives untouched so existing rows still parse.
2. **Remove** the conversions binding the old DTO to the *live* `Document`
   (`From<&Document>` and `TryFrom<… for Document>`): a frozen tree cannot
   convert to a model it predates, and step 3 supersedes them.
3. **Add** a new frozen tree `DocumentV0_NN_0` reflecting the new model,
   plus its `From<&Document>` and `TryFrom<… for Document>` conversions.
4. **Add** the `StoredDocument::V0_NN_0` variant, tagged
   `#[serde(rename = "quillmark/document@0.NN.0")]`.
5. **Write the migration**: `From<DocumentV0_93_0> for DocumentV0_NN_0` if
   the mapping cannot fail (a purely structural rename/restructure), or
   `TryFrom<DocumentV0_93_0> for DocumentV0_NN_0` if it can reject, as the
   V0_92_0 → V0_93_0 cold-import does for an over-nested legacy body. This is
   the only real labor: it encodes how old fields map to the new model
   (renames, restructures, defaults for new fields, and: for a `TryFrom`
   hop: which malformed inputs get rejected).
6. **Extend** the reader (each older blob migrates one hop, then chains).
   Every arm below the newest already funnels through the V0_92_0 → V0_93_0
   hop, which can reject, so every one of those arms threads `?`, whether
   or not the new V0_93_0 → V0_NN_0 hop (shown here as infallible) adds
   another:
   ```rust
   match stored {
       StoredDocument::V0_NN_0(p) => Document::try_from(p),
       StoredDocument::V0_93_0(p) => Document::try_from(DocumentV0_NN_0::from(p)),
       StoredDocument::V0_92_0(p) => Document::try_from(DocumentV0_NN_0::from(
           DocumentV0_93_0::try_from(p)?,
       )),
   }
   ```
   If the new hop is itself a `TryFrom`, thread a second `?` after
   `DocumentV0_NN_0::try_from(...)` in every arm.

A new frozen DTO can also reject at parse time through a custom
`Deserialize` rather than through a `TryFrom` migration: `CanonicalContent`
(the `body` field's type) normalizes and validates the embedded content,
failing with a serde error before any `TryFrom` in the chain above runs.
Design a new DTO's `Deserialize` to fail the same way if it embeds
structured (non-string) data of its own.

Old and new DTOs **coexist** in `dto.rs`, so a row written by any
still-supported past version always loads. Migrations chain
(`V0_92_0 → V0_93_0 → V0_NN_0 → …`); only the newest DTO converts to the live
`Document`, so each migration step stays small as versions accumulate. The
cost is one frozen type tree per schema version plus one migration function
per version bump.

A legacy variant may be **retired**: its DTO tree, migration, and tests
deleted: once a product/release-history call confirms no stored population
remains in that shape (the `0.81.0` and `0.82.0` shims were dropped this
way). A row that later surfaces in a retired shape then fails as an unknown
version, so retirement is reserved for shapes with no live rows.

## Gotchas

- The schema version is a hand-set constant (`SCHEMA_V0_93_0`), **not**
  `CARGO_PKG_VERSION`: bumping it is a deliberate act tied to a model change.
- Unknown schema versions are rejected on read, never silently ignored.
- DTO type names carry version suffixes with underscores
  (`DocumentV0_92_0`); `non_camel_case_types` is allowed module-wide for this.
- No file extension is part of the storage contract: the interchange forms
  are card-yaml markdown **text** and `StoredDocument` **JSON**.

## Links

- [ARCHITECTURE.md](ARCHITECTURE.md): `Document` in the core type overview
- [markdown-spec.md](../references/markdown-spec.md): Markdown syntax and the in-memory data model
- [VERSIONING.md](VERSIONING.md): quill version resolution (a separate concern)
- `QuillValue` (`crates/core/src/value.rs` rustdoc): value type stored inside payload fields
