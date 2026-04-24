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

1. [01-frontmatter-comments.md](01-frontmatter-comments.md) — preserve YAML
   comments in the frontmatter data model as first-class ordered items.
2. [02-fill-typed-marker.md](02-fill-typed-marker.md) — promote `!fill` to
   a typed marker that round-trips on emit. Reject other custom tags.
3. [03-frontmatter-only-parse.md](03-frontmatter-only-parse.md) — expose a
   parse entry point that takes a bare YAML fragment and returns the typed
   frontmatter, without requiring a full `---`-fenced markdown document.
4. [04-strip-html-comments.md](04-strip-html-comments.md) — expose a pure
   utility that strips `<!-- … -->` from markdown text.

Order is the dependency order. 01 and 02 both extend the frontmatter type
and should land in sequence (02 on top of 01). 03 surfaces the type 01+02
produce through a new entry point. 04 is independent and a ride-along.

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
- **No markdown body AST.** The body stays an opaque string. Downstream
  editors own their own body parsers (e.g. ProseMirror) and don't need
  quillmark to duplicate that work.
