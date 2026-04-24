# 03 — Unify Entry and Composable Cards

**Status:** Draft
**Depends on:** nothing
**Blocks:** 01, 02 (they operate on Card's frontmatter)
**Recommended implementation order:** land first.

## Background

MARKDOWN.md §2 defines every Quillmark document as a sequence of
(sentinel, frontmatter, body) triples:

```
Document = Frontmatter Body (CardFence CardBody)*
```

The first triple is the entry — its sentinel is `QUILL:` and it carries
the document-level fields plus the global body. Subsequent triples are
composable cards — their sentinel is `CARD:` and each carries a typed
record.

The data model today splits these: `Document { frontmatter, body, … }`
plus a separate `Vec<Card>`. The split is historical; grammatically,
all three regions are the same shape. Unifying them collapses the
parser, emitter, and mutator surface, and matches how `Quill.yaml`
already describes the model (a `main:` section alongside `cards:`
entries).

## Change

One `Card` type for all fences. `Document` holds one `main` plus a
`Vec<Card>` of composable cards.

```rust
pub struct Card {
    sentinel: Sentinel,
    frontmatter: Frontmatter,  // shape per tasking 01
    body: String,
}

pub enum Sentinel {
    Main(QuillReference),
    Card(String),
}

pub struct Document {
    main: Card,
    cards: Vec<Card>,
    // Invariants, enforced via private fields + smart constructors:
    //   main.sentinel matches Sentinel::Main(_)
    //   every card in cards matches Sentinel::Card(_)
}
```

### Parser

- First fence: sentinel `QUILL:` → `Sentinel::Main(ref)`; build a `Card`
  with the global body between the fence and the first card fence (or
  EOF). Store as `Document.main`.
- Subsequent fences: sentinel `CARD:` → `Sentinel::Card(tag)`; build a
  `Card`; push onto `Document.cards`.
- One code path for "parse a fence"; sentinel kind is determined by
  position.

### Emitter

Walk `once(&main).chain(&cards)`. Emit each card's fence + body
uniformly; `sentinel` drives the first content line (`QUILL: …` or
`CARD: …`). No separate "emit document body" path.

### Mutators

Frontmatter and body mutators live on `Card`:

```rust
impl Card {
    pub fn set_field(&mut self, key: &str, value: impl Into<QuillValue>);
    pub fn remove_field(&mut self, key: &str);
    pub fn replace_body(&mut self, body: impl Into<String>);
    // …plus set_fill from tasking 02.
}
```

`Document` keeps only document-level concerns:

```rust
impl Document {
    pub fn main(&self) -> &Card;
    pub fn main_mut(&mut self) -> &mut Card;
    pub fn cards(&self) -> &[Card];
    pub fn cards_mut(&mut self) -> &mut [Card];

    pub fn push_card(&mut self, tag: impl Into<String>, …);
    pub fn insert_card(&mut self, idx: usize, …);
    pub fn remove_card(&mut self, idx: usize) -> Option<Card>;
    pub fn move_card(&mut self, from: usize, to: usize);

    pub fn quill_reference(&self) -> &QuillReference; // reads main.sentinel
    pub fn set_quill_ref(&mut self, r: QuillReference);
}
```

No top-level shortcuts for frontmatter / body mutators. Callers write
`doc.main_mut().set_field(…)` explicitly. KISS: one place for each
operation; no parallel APIs to keep in sync.

### WASM surface

- `Document.main` (getter) → `Card` handle.
- `Document.cards` (getter) → `Card[]`.
- `Document.quillRef` unchanged — convenience reader over
  `doc.main.sentinel`.
- `Document.frontmatter`, `Document.body`, `Document.frontmatterItems`,
  `Document.setField`, `Document.setFill`, `Document.replaceBody`, etc.
  are **removed**. Consumers migrate to `doc.main.frontmatter`,
  `doc.main.body`, `doc.main.setField`, etc.
- `Card` gains the full mutator surface (`setField`, `setFill`,
  `removeField`, `replaceBody`) plus the read accessors (`frontmatter`,
  `frontmatterItems`, `body`).
- `Card.tag` exposes the string tag for composable cards. For main
  cards, it returns the quill reference's string form (or callers can
  read `doc.quillRef` directly).

## Non-goals

- Document-level shortcuts for card operations. The rework is a
  breaking change; don't also ship a parallel API we'd need to keep
  in sync forever.
- Changes to `QuillReference` typing or parsing. Structure only.
- A third sentinel kind. Two is sufficient for the grammar.

## Done when

- `Document::main()` returns a `Card` whose sentinel matches
  `Sentinel::Main(_)`.
- `once(&doc.main()).chain(doc.cards())` covers every fence in the
  source.
- One emit code path serves both main and composable cards.
- Frontmatter / body mutators live on `Card`, not on `Document`.
- Existing tests migrate to the new access pattern; no
  `Document::frontmatter` / `Document::body` callers remain.
- `WASM_MIGRATION.md` gains a section describing the access-pattern
  change.
