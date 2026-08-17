---
name: dense-prose
description: Comment and doc policy — write none by default, keep only what a reader cannot get faster from the code. Use when writing or reviewing comments, docstrings, READMEs, or design docs, and when prose restates its own code, sells, or narrates how the code got here ("used to", "no longer", "as of 0.x", "tracked in #123").
---

## Default: none

Code is the documentation. A comment is a second copy of a fact — read on every visit, rotted by every edit — so the honest default is zero, and one comes back only where a reader cannot get the fact faster from the code itself.

Wrong is worse than missing. A missing fact costs a lookup; a wrong one is believed. Bloat and rot are the same failure, a claim nobody re-checked: verify against the code or write nothing. The bar cuts both ways — a cut that drops a fact is dilution, not compression.

## What earns words

| Surface | Budget |
|---|---|
| Public API: what a caller depends on without reading the body | The contract — argument and return meaning the types do not carry, errors, and the invariants a caller can violate. One example, where the call shape is not obvious. |
| A non-obvious why | One line: the ordering constraint, the upstream bug, the spec citation, the workaround that looks arbitrary and is not. |
| A hazard the compiler cannot see | Keep. Preconditions on unsafe code, suppressed warnings, deliberate deviation from the obvious. |
| A test | Its name. One line only where the assertion is unreadable without it; a regression test states the invariant guarded, never the bug's history. |
| Anything internal | Nothing, unless it states an invariant the reader would otherwise break. Then one line. |

## Delete on sight

- **Echoes.** Prose restating the line beneath it, or a field's own name. A better identifier beats the comment.
- **Hand-kept lists.** A header enumerating a module's exports, cases, or features: the tooling lists them already and the copy drifts. Name the module's job in one line, or say nothing.
- **Process instead of state.** "used to", "no longer", "previously", "as of 0.x", "renamed", "we switched", "we considered", "deferred", "for now", "tracked in #123" — each narrates how the code got here or that work is in motion, and each dates the sentence around it. Assert the present.
- **Sell.** *powerful, seamless, battle-tested, effortless*, and *simply / easily* where they only flatter. Keep *just / only* where load-bearing.
- **Throat-clearing.** "Note that", banner rules, a first line restating its own heading.

Reframe rather than delete where the past is load-bearing for the present: an accepted alias, a tolerated input, a stored old format is current behavior in a historical costume. Read before cutting, since `used to` often means "used **in order to**".

## Compress what survives

Cut any sentence whose removal costs no fact. Length tracks surprise: the unobvious invariant gets the words, the obvious call gets none. Lead with the contract, then the mechanism. Fold a qualifier into the clause it qualifies rather than appending a sentence, name the specific noun and the measured number, and reuse the term the code already uses instead of minting a synonym.

Compression is not density: one claim per sentence. A sentence carrying seven clauses is maximally compressed and unreadable, because the reader decompresses it to reach the one fact they came for. A bullet needing three clauses is three bullets; per-case rules are a table, and a table row is a record, not a sentence.

## Limits

- Era-stamped records — changelogs, applied migrations, incident notes — are **repair only**. They are accurate to their moment: fix what was wrong when written, leave what was right in its era's vocabulary.
- Never rename identifiers. Out of scope, pure churn.
- Edits are surgical. Touch a line only when it breaks a rule.
- A comment-only change can break the build: doc examples compile, and a test may assert the wording being changed.
