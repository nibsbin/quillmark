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

!!! warning "Crossing 0.92 → 0.93 rewrites stored bytes"

    `@0.92.0` stores a card body as a markdown string; `@0.93.0` embeds it
    structurally. The hop is a re-parse, not a reshape: a legacy body imports
    under current parse rules on every read, and re-serializes on the next
    write. Three consequences for a consumer holding stored blobs.

    **Stored bytes grow on the next save.** The growth lands on the body
    subtree, and the factor tracks mark and list density — roughly 1.5x for
    prose carrying no marks, ~2.5x for memo-shaped content, several times that
    for lists, and highest for nested ones. No single multiplier covers a mixed
    corpus; measure your own. Re-size any bound sitting on those bytes — a
    database `CHECK`, a column limit, a quota — before shipping the upgrade.

    **Migration is lazy.** A blob migrates when it is read, so the stored
    population is mixed for as long as some rows go untouched, and a size
    failure surfaces on a user's first edit rather than at deploy.

    **The first write is one-way.** A 0.92.1 reader rejects the `@0.93.0`
    schema tag, so a row saved once cannot be read by the old version. Land the
    cap change before the upgrade, not after.

## Guides

| Step | What changes |
|---|---|
| [0.98 → 0.99](0.98-to-0.99.md) | `attrs` beside a built-in discriminator is rejected where a host authors it, instead of resolving to the built-in and dropping the payload in silence. `isUnknown*` guards answer known-vs-unknown on each open set; `ContentLineKind` is re-exported. Opaque payload depth is bounded at 128 on the `Value` lane, where an unbounded one trapped the WASM module. |
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
