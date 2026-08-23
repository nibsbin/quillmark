# Spike: what a real tree would look like

**Status: spike, not a proposal.** Nothing here is implemented in production
code. The measurements come from `crates/content/tests/spike_tree.rs`, which
builds the tree `Content` declines to store and runs the corpus through both
conversions.

`model.rs` states the current choice and its reason:

> The line tree is *derived* from this flat list plus each line's `containers`
> path, never stored, so a split/join is a single-char edit with no paragraph
> identity to reconcile.

The question is whether that trade still holds.

## What the tree is

```rust
enum Node {
    List { ordered: bool, start: u64, items: Vec<Vec<Node>> },
    Quote(Vec<Node>),
    Unknown { tag: String, attrs: Value, children: Vec<Node> },
    Block { kind: LineKind, lines: Vec<usize> },   // a `continues` run
}
```

Text, marks and islands stay in the flat `Content`; a `Block` indexes into
`lines`. That is deliberate and it is the cheap variant — see "The expensive
variant" below for what moving text into the nodes costs.

Four things are gone **by construction**, not by rule:

| flat | tree |
|---|---|
| `ordinal` | an item's index in `items` |
| a sibling discriminator | two sibling nodes are two nodes |
| depth-prefix comparison in four consumers | children are children |
| a path that jumps depth 3 → depth 1, which nothing rejects | unrepresentable |

## Measurement 1: the tree loses nothing

`flat → tree → flat` is the identity over a 30-case corpus covering every
construct the codecs know, and the markdown projection is unchanged too. The
tree is at least as expressive as the flat form. No surprise, but it had to be
checked before the interesting direction means anything.

## Measurement 2: what the flat form loses

`tree → flat → tree` over 252 enumerated trees — every ordered pair and triple
drawn from `{ul×1, ul×2, ol×1, ol×2, quote, para}`:

```
  trees enumerated:            252
  lost by the shipped reader:  100 (39.7%)
  lost by the best reader:      60 (23.8%)
  recoverable by reading the ordinal decrease (#1359):  40
  irreducible without a stored boundary:                60
```

Two readers, because how hard the flat→tree derivation tries turns out to
matter more than anything else here:

- **Shipped** is the rule `emit.rs::list_run_end` and `quill::support::census`
  use on `main`: group list items by `(ordered, start)`, ignore `ordinal`.
- **Best** additionally reads an `ordinal` *decrease* as the next list opening
  — the rule #1359 proposes.

The 40-document gap between them is exactly #1359's scope. The 60 that survive
neither are the residue: `ul1 + ul1`, `ul1 + ul2`, `ol1 + ol1`, `ol1 + ol2`,
`quote + quote`, and their extensions. A list ending on one item followed by
another list, and adjacent quotes — no reading rule recovers those, because the
data does not contain the distinction.

**Writing the reader as the obvious thing reproduced the shipped bug.** The
first `to_tree` in this spike grouped list items by shape alone, which is the
natural way to write it, and `1. a / 2. b / <!-- --> / 1. c / 2. d` came back
as one list of four. That is defect 2 of #1359, re-derived accidentally by a
fifth consumer inside an afternoon. It is the strongest evidence in the spike
that the flat encoding's difficulty is not incidental: the *weakened* grouping
rule a list needs is easy to get wrong and there is nothing to check it against.

## The finding that matters

**A tree is not what fixes this. A stored boundary is.**

The 23.8% irreducible residue is a property of the flat form having nowhere to
put sibling identity — not of it being flat. Adding one discriminator field to
`Container` (PR #1360) drives that residue to zero while leaving the storage
model flat, the coordinate space intact, and the wire backward-compatible.

So the adjacency problem is not an argument for a tree. It is an argument for
a field, and that argument is already settled.

## What a tree would still buy

Independent of adjacency:

1. **Invalid states become unrepresentable.** A container path that skips a
   depth, an `ordinal` inconsistent with position, a `continues` line opening a
   block — the flat form can spell all of these and `validate` catches none of
   them (it is strictly per-line; every container invariant is a property of a
   line *pair*).
2. **One traversal instead of five re-derivations.** `export::emit_block`,
   `emit::try_emit_container`, `census`, the TS codec, and now this spike each
   re-derive grouping. A tree has no grouping step at all.
3. **Nesting is checked by the type.** `MAX_NESTING_DEPTH` becomes a recursion
   guard rather than a per-line length check in two places.

## What it would cost

Sized from the code, not guessed:

- **139 sites** across 4 crates index `lines` flat (`.lines[i]`, `lines.len()`,
  `line: usize` in the op wire).
- **`LineOp` is flat by definition.** `Split { at }`, `Join { line }`,
  `SetContainers { line, .. }` — all address a line by index. A tree op
  vocabulary means node paths, and `SetContainers` becomes wrap/unwrap/reparent.
  This is the binding wire, so it changes in three languages.
- **The canonical JSON body becomes nested.** New schema version, and
  `to_canonical_json` byte-stability is defined against the flat shape
  (`DOCUMENT_STORAGE.md` § Byte-stability). Every stored row migrates.
- **The Typst source map is per-segment USV ranges** (`emit.rs`'s `line_usv`,
  built from `export::line_segments`). It survives a tree that indexes lines;
  it does not survive one that owns text.

### The expensive variant

If nodes own their text rather than indexing it, the single USV coordinate
space goes — and that space is the model's spine, not a detail:

- `Mark` is `[start, end)` over the whole field. Node-local marks cannot
  express a mark spanning two blocks, which the model currently allows.
- `Delta` is retain/insert/delete over the same sequence, and it is the wire
  the `rebase` codec and `applyChange` bundle carry across every binding.
  `delta::diff` and the move detector both assume one linear text.
- Islands occupy one USV slot in that text.

That variant is not an incremental change to `quillmark-content`; it is a
different content model. It should only be on the table for reasons that have
nothing to do with #1359.

## Where the original trade stands

`model.rs`'s rationale still holds, and the spike shows it concretely
(`a_split_costs_a_node_identity_decision`). A flat split is one `\n` insertion:
every container path is untouched, and no identity is minted or destroyed. The
tree split has to replace one child with two and then answer a question the
flat form never asks — *which of the two is the original node* — because
anchors, comments, and any future per-node handle hang off that answer.

The flat model bought that property by giving up sibling identity. The
discriminator buys sibling identity back for one optional field. That is a
better deal than trading the property away.

## Recommendation

Don't. Not for this.

The tree is the right shape if the model ever needs per-node identity for its
own sake — stable block handles, node-level comments, collaborative structural
editing. Those are real reasons and none of them is on the table today. Until
one is, the flat form plus a stored boundary covers the same ground for two
orders of magnitude less change.

Worth keeping from this spike:

- `validate` is per-line and every container invariant is relational. A cheap
  relational pass would catch the depth-skip and ordinal-inconsistency states a
  tree would make unrepresentable, without the tree.
- The grouping rule genuinely is re-derived five times now. Sharing one
  traversal helper is worth doing on its own merits, whatever the storage shape.
