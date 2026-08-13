---
name: dense-prose
description: Comment and doc policy — delete by default, prose only where the reader cannot get the fact from the code. Use when writing or reviewing code comments, prose/canon/, or any doc, and when a comment restates code, narrates change ("used to", "no longer", "as of 0.x"), or carries a list that will rot.
---

## Default: no comment

Code is the documentation. A comment is an admission the code failed to carry the fact, and it bills forever: read on every visit, rotted by every edit. Start at none and add back only what a reader demonstrably cannot get from the code.

Wrong is worse than missing. An unsaid fact costs a lookup; a wrong one is believed. Bloat and rot are one failure — a claim nobody re-checked. Verify against the code or say nothing.

## What earns prose

| Surface | Budget |
|---|---|
| `pub` API of a published crate, TypeScript declarations, `.pyi` stubs, READMEs | A tight paragraph: the contract, the argument and return meaning the type does not carry, errors, panics. One example, and only where the call shape is not obvious. |
| The non-obvious why | One line: workaround, upstream bug, ordering constraint, spec citation, wire-format fact. |
| `SAFETY:` on `unsafe`, `#[allow]` and `cfg` rationale | Keep. |
| Anything private or `pub(crate)` | Nothing, unless it states an invariant the reader would otherwise violate. Then one line. |
| Load-bearing legacy (versioned wire schemas in `crates/core/src/document/dto.rs`) | Keep the fact: the old-format description *is* current reader behavior. |

## Delete on sight

- **Echoes.** `// increment i`; `/// The name` on `name`; a doc re-narrating the body. A better identifier beats the comment.
- **Rot-prone lists.** Never enumerate a module's items, features, or behaviors: rustdoc lists items and the hand-list drifts. Name the module's job in one line, or say nothing.
- **History.** "used to", "no longer", "previously", "formerly", "as of 0.x", "removed in", "renamed", "we switched" — assert the present instead. Current behavior in a historical costume keeps the fact and drops the framing: "the legacy `~~~card-yaml` opener is still accepted but no longer canonical" → "`~~~card-yaml` is also accepted as a non-canonical alias." (`used to` often means "used **in order to**"; read before cutting.)
- **Deliberation.** "we considered", "spike", "deferred", "for now", "eventually", rejected-alternative essays. Keep the resulting fact and the rationale for the present choice; drop the when and the who.
- **Status markers.** `(#970)`, "tracked in #736" say only that work is in motion, and they date the sentence around them. Keep only where the issue is the subject, in a CI or release doc.
- **Sell.** *powerful, seamless, battle-tested, simply, easily*. Keep *just / simply / only* when load-bearing ("just sugar for the `raw` element").
- **Banners and throat-clearing.** `// ---- helpers ----`, "Note that", a first line restating its own heading.

## Compress what survives

Cut any sentence whose removal costs no fact. Length tracks surprise: the unobvious invariant gets the words, the obvious call gets none. One claim per sentence — fold a qualifier into the clause it qualifies, and split a sentence carrying seven clauses, because the reader decompresses it to reach the one fact they came for. A bullet needing three clauses is three bullets or a table row; per-case rules (per type, per format, per backend) are a table.

Present tense. Lead with the contract, then the mechanism. Name the specific noun and the measured number. Reuse the terms of art (*card-yaml block, plate, quill, backend, seam*).

A paragraph is one line: never hard-wrap prose at a column. A line break means a new paragraph, list item, or table row. Comments wrap to the code's line budget.

## Tests

A test's name is its documentation. Delete test comments; keep one line only where the assertion is unreadable without it. A regression test states the invariant guarded ("X must not happen; would cause Y"), never the bug's history.

## Limits

- `docs/migrations/**`, `CHANGELOG.md`, and era-stamped records: **repair only**. They are accurate to their moment — fix what was wrong when written, leave what was right in its era's vocabulary.
- `prose/references/`, `prose/proposals/`, `prose/plans/`: strip the sell only. Discussing other or future states is their job.
- Never rename identifiers. Out of scope, pure churn.
- Edits are surgical: touch a line only when it breaks a rule. A cut that drops a fact is dilution, not compression.

Canon *structure* (Title → Implementation anchor → TL;DR, one concept per page) belongs to **`maintain-canon`**.

## Done when

Nothing restates code, no header carries a rotting list, no prose narrates history or deliberation, and every surviving sentence states a verified fact the reader could not get faster from the code. Verify: build and tests pass, no doctest broken, `node scripts/check-canon.mjs` passes.
