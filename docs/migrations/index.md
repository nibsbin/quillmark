# Migration Guides

A release that breaks the document syntax, the plate-JSON wire format, or a
public API ships a guide for that version step. Most breaks are hard cutovers —
the old form stops parsing or compiling — so the guide is the path forward, not
an optional read.

Each guide covers one step. To cross several versions, work through them in
order; each states its own breaks in full.

!!! note "0.93 was never separately published"

    The 0.93 milestone folded into 0.94.0 — no 0.93.x release was tagged.
    Upgrading from 0.92.1 means following **0.92 → 0.93** and **0.93 → 0.94**
    in sequence.

## Guides

| Step | What changes |
|---|---|
| [0.101 → 0.102](0.101-to-0.102.md) | No API breaks. Island-id minting splits by case: a new island continues the positional sequence, while restoring a deleted one re-lands the id it was deleted under, since ids are hash input and a fresh mint is a rename. The island channel also states two contracts it previously left to the implementation: an op's `at` counts the slots this bundle's earlier island ops spliced, not the shared post-delta frame, and a computed splice carrying a slot splits into a slot-free delta plus one island insert per slot. |
| [0.100 → 0.101](0.100-to-0.101.md) | Two Rust constructors leave the published surface: `Document::from_main_and_cards` becomes crate-internal (its invariants were `debug_assert`s feeding a release-build panic), and `QuillConfig::from_yaml` is removed for `from_yaml_with_warnings`, which keeps the structured diagnostics the boxed string destroyed. The change-bundle verbs take one `ChangeBundle` struct in place of three positional op arguments, because the bundle gains an island channel: `IslandOp` edits an island's payload or inserts one with its slot, so a table or image edit keeps the field's identity anchors instead of falling back to a whole-field `install`. On the bindings the channel is additive (`applyChange`'s new optional `islandOps`); documents and stored blobs are unaffected. |
| [0.99 → 0.100](0.99-to-0.100.md) | Card `$id` is removed: the reserved key, `find_card` / `set_card_id` / `remove_card_id`, the collision errors, and `cardIndexById` / `card_index_by_id` plus the projected `id` on both bindings. Per-card consumer keys move to `$ext` under a namespace you own, with no uniqueness guarantee. `Content`, `Line`, `Mark`, and `Island` take `#[non_exhaustive]`, the four the 0.99 sweep missed, so their literals give way to `new` plus `with_*` setters. A string-valued `plaintext` field reads through the literal codec instead of markdown, and `reader.getContent` returns a content field's corpus whichever lane built the document. Load conforms content fields to one resting form per codec (`Quill::parse` / `Quill::conform`, the bound door), and `Diagnostic.args` carries the facts a message interpolates. |
| [0.98 → 0.99](0.98-to-0.99.md) | The Rust API opens ahead of 1.0.0: `#[non_exhaustive]` across the published crates, so an exhaustive `match` needs a `_` arm and a struct literal gives way to `new` plus `with_*` setters. `attrs` beside a built-in discriminator is rejected where a host authors it, instead of resolving to the built-in and dropping the payload in silence. Island `loss` opens; a handle from a second copy of `@quillmark/wasm` is refused. |
| [0.97 → 0.98](0.97-to-0.98.md) | The block vocabulary opens — an unrecognized line kind or container round-trips opaque instead of failing the load. `OutputFormat::Txt` retires on every surface. |
| [0.96 → 0.97](0.96-to-0.97.md) | The WASM transport read renames to `Document.getStored`. A card `$id` becomes unique per document and never empty. |
| [0.95 → 0.96](0.95-to-0.96.md) | One address grammar (`DocPath`) on every boundary; mutator failures gain namespaced `edit::*` codes; `view()` renames to `reader()`. |
| [0.94 → 0.95](0.94-to-0.95.md) | One insertion verb (`insertCard(card, at?)`) and one parse entry (`Document::parse`); the typed writer becomes the one schema-bound door; `datetime` splits into strict `date` and `datetime`. |
| [0.93 → 0.94](0.93-to-0.94.md) | `type: richtext(inline)` retires for `inline: true`; typed field writes land; `ui.order` is removed — declaration order is the ordering contract. |
| [0.92 → 0.93](0.92-to-0.93.md) | The blueprint placeholder splits into value and marker axes: the `!must_fill` tag replaces the `<must-fill>` string sentinel, and a bare null falls back to default/zero. |
| [0.91 → 0.92](0.91-to-0.92.md) | `!fill` renames to `!must_fill` with no alias — a stale `!fill` silently loses placeholder status. `$seed` carries per-card-kind seed overlays. |
| [0.90 → 0.91](0.90-to-0.91.md) | A card-yaml closing `~~~` must sit at column zero. Data-field names and the 100-level nesting limit are enforced on every input path. |
| [0.89 → 0.90](0.89-to-0.90.md) | `Quill` becomes engine-free data — the engine takes the quill per call. The WASM package collapses to a single `@quillmark/wasm` root import. |
| [0.88 → 0.89](0.88-to-0.89.md) | A `$quill` name or version mismatch fails the render instead of warning. |
| [0.87 → 0.88](0.87-to-0.88.md) | The schema-aware form view gives way to `quill.validate(doc)`; one `Card` shape flows both in and out. |
| [0.86 → 0.87](0.86-to-0.87.md) | Array fields require an `items` element schema; `type: date` folds into a unified `type: datetime`. |
| [0.85 → 0.86](0.85-to-0.86.md) | A partial document renders without error; the canonical card-yaml fence becomes a bare `~~~`. |
| [0.83 → 0.84](0.83-to-0.84.md) | The Must Fill / Endorsed schema model replaces `required:`. |
| [0.82 → 0.83](0.82-to-0.83.md) | The `$`-prefixed plate JSON wire format retires the legacy uppercase reserved keys. |
| [0.81 → 0.82](0.81-to-0.82.md) | card-yaml metadata replaces the `---` / `QUILL:` frontmatter and fenced cards. |
| [`@quillmark/wasm` 0.77 → 0.80](wasm-0.77-to-0.80.md) | WASM consumers crossing the card-syntax release. |

## Related

For how Quills themselves are versioned and how authors target a version, see
[Quill Versioning](../quills/versioning.md).
