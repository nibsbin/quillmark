---
name: dense-prose
description: Comment and doc policy — write none by default, keep only what a reader cannot get faster from the code. Use when writing or reviewing comments, docstrings, or any doc, and when prose restates its own code, sells, or narrates how the code got here ("used to", "no longer", "as of 0.x", "tracked in #123").
---

## Default: none

Code is the documentation. A comment is a second copy of a fact, read on every visit and rotted by every edit. The default is none; one comes back only where a reader cannot get the fact faster from the code itself.

Wrong is worse than missing. A missing fact costs a lookup; a wrong one is believed. Bloat and rot are the same failure, a claim nobody re-checked: verify against the code or write nothing. The bar cuts both ways: a cut that drops a fact is dilution, not compression. A comment contradicting the code gets fixed, not deleted.

## What earns words

| Surface | Budget |
|---|---|
| Public API: what a caller depends on without reading the body | The contract — argument and return meaning the types do not carry, the errors, the invariants a caller can violate. One example, where the call shape is not obvious. |
| A reason not visible from the code | One line: the ordering constraint, the upstream bug, the spec citation, the choice that looks arbitrary and is not. |
| A hazard the tooling will not show | Keep. An unsafe precondition, a suppressed warning or lint, a deliberate deviation from the obvious. |
| A test | Its name. A regression test states the invariant guarded, never the bug's history. |
| Everything else | Nothing, unless a reader would otherwise break an invariant. Then one line. |

## Delete on sight

- **Echoes.** Prose restating the line beneath it, or a field's own name.
- **Hand-kept lists.** A header enumerating a module's exports, cases, or features: the tooling lists them already and the copy drifts. Name the module's job in one line, or say nothing.
- **Process instead of state.** "used to", "no longer", "as of 0.x", "we switched", "we considered", "deferred", "for now", "tracked in #123" — the story of how the code got here, or that work is in motion. State what is, in the present tense: not what the code will do, not how it came to do it. The reason for a choice survives; the account of reaching it does not.
- **Sell.** *powerful, seamless, battle-tested, effortless*, and *simply / easily* where they only flatter. Keep *just / simply / only* where load-bearing.
- **Throat-clearing.** "Note that", a banner comment (`// ---- helpers ----`), a first line restating its own heading.

Reframe rather than delete where the past is load-bearing for the present: an accepted alias, a tolerated input, a stored old format is current behavior in a historical costume. Read before cutting, since `used to` often means "used **in order to**".

## Compress what survives

Cut any sentence whose removal costs no fact. Length tracks surprise: the unobvious invariant gets the words, the obvious call gets none. Name the specific noun and the measured number. Reuse the term the code already uses rather than minting a synonym.

Compression is not density: one claim per sentence. A sentence carrying seven clauses is maximally compressed and unreadable, because the reader decompresses it to reach the one fact they came for. A bullet needing three clauses is three bullets; per-case rules are a table.

## Limits

- A doc about another moment keeps it. A changelog, an applied migration, or an incident note is **repair only**: fix what was wrong when written, leave what was right in its era's vocabulary. A proposal argues a future state, and flattening it into what is destroys the document.
- Touch a line only when it breaks a rule.
