# Document Rework

**Umbrella goal:** make markdown the single contract between quillmark and
its consumers. `Document.fromMarkdown → Document.toMarkdown` must be
faithful enough that consumers never need to reparse source or splice
bytes to preserve author intent.

Today `Document.to_markdown()` is a canonical emitter that drops YAML
comments and custom tags, which forces downstream consumers (registry
editors, wizards) to ship their own comment-preserving YAML AST and
byte-range splicers. The rework closes that gap inside quillmark so
consumers can delete that code.

## Taskings

1. [03-unify-cards.md](03-unify-cards.md) — one `Card` type for both
   the document entry (QUILL sentinel) and composable cards (CARD
   sentinel). `Document { main: Card, cards: Vec<Card> }`.
2. [01-frontmatter-comments.md](01-frontmatter-comments.md) — preserve
   YAML comments in `Card.frontmatter` as first-class ordered items.
3. [02-fill-typed-marker.md](02-fill-typed-marker.md) — promote `!fill`
   to a typed marker on `Field`. Reject other custom tags.

**Implementation order:** 03 → 01 → 02. Numeric prefixes reflect the
order decisions were made, not the order to implement. 03 restructures
the data model; 01 rebuilds the frontmatter representation on the
unified model; 02 layers `!fill` semantics onto the item model from 01.

## Explicit non-goals

- **No byte offsets in the public API.** Considered and rejected. Markdown
  is the serialization contract; exposing source locations would introduce
  a second contract and undo the offload.
- **No source-preserving mode.** Canonical emission stays canonical. The
  items above make canonical *faithful* for the parts consumers care
  about (comments, `!fill`); string quoting, flow vs block style, and
  similar formatting are normalized by design.
- **No general custom-tag round-trip.** Only `!fill` is integrated. Other
  tags are rejected at parse with a warning.
- **No comments inside nested YAML values.** Only top-level comments
  round-trip. Nested comments are dropped silently; a parse warning is
  emitted on the first occurrence per document.
- **No markdown body AST, and no content-level transformations of the
  body.** The body stays an opaque string between the frontmatter and
  the first card fence. We do not walk its markdown structure, and we
  do not ship utilities (comment strippers, link rewriters, etc.) that
  would imply partial parsing. Downstream editors own their body
  pipeline; when we commit to a markdown AST it will be a separate
  design, not something smuggled in through helpers.
- **No bare-YAML parse entry point.** Every Quillmark markdown document
  carries `QUILL`; there is no supported authoring format that lacks
  it. Consumers with a full document call `Document.fromMarkdown`;
  consumers with something that isn't a Quillmark document should use a
  general YAML library, not us. Speculative fragment-parse use cases
  are YAGNI until a concrete consumer need is named.
- **No document-level shortcuts for card mutators.** After 03,
  frontmatter and body mutations live on `Card`. `Document.setField`
  and friends are removed; callers write `doc.main_mut().set_field(…)`
  (or hold a `&mut Card` reference). One surface, no parallel API to
  keep in sync.
