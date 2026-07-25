---
name: dense-prose
description: Write comments and docs at high semantic density — terse, present-tense, unsold, mostly self-documenting. Use when writing or reviewing code comments, prose/canon/, or docs/ for density and a consistent voice, or when comments narrate change ("used to", "no longer", "renamed", "as of 0.x") instead of stating what is.
---

## Purpose

Comments and docs earn their bytes. Each should state a fact a reader cannot get
faster from the code itself, in the fewest words that stay correct. This skill
is the house voice: dense, present-tense, declarative, unsold.

It covers comment and doc *content*. For canon *structure* — the `prose/canon/`
doc spine and tiers — use **`maintain-canon`**.

## Prime directive: correctness over brevity

A comment makes a claim about code. Never shorten a claim you have not verified
against the code. Cutting words must not silently change meaning. When unsure a
statement is still true, leave it and keep the fact. Edits are surgical: touch a
line only when it breaks a rule; do not churn prose that is already dense and
correct. Over-editing is the main failure mode.

## What this skill owns

### 1. No marketing or persuasion

Remove words that sell rather than inform: *powerful, seamless, elegant, robust,
flexible, blazing(-fast), cutting-edge, state-of-the-art, first-class (citizen),
rich (set of), comprehensive, battle-tested, out of the box, leverage* (meaning
"use"), and *simply / just / easily* when they only imply ease. State the
capability plainly.

- "Partial documents are first-class citizens" → "A document need not be
  complete."
- "opts into canvas simply by overriding the seam" → "opts into canvas by
  overriding the seam."

Keep *just / simply / only* when they carry real meaning ("just sugar for the
`raw` element", "three or more tildes"). The word is not the violation; the sell
is.

### 2. Self-documenting first — cut over-explanation

The code is the primary documentation. A comment that restates it is noise.

- Delete comments that echo the code (`// increment i`; `/// The name` on a
  field named `name`). Prefer a clearer name over a comment.
- Collapse padded rustdoc scaffolding. A header of "## Key Functions / ## Quick
  Example / ## Detailed Documentation / For comprehensive details including: …"
  becomes a tight paragraph plus, at most, one runnable example.
- Do not enumerate a module's public items in its header — rustdoc lists them,
  and the hand-list rots. Describe the module's job instead.
- One good example beats three; drop "see X for comprehensive coverage" filler.

### 3. Present tense — describe what is, not how it got here

Evolutionary narration ("we used to X, now Y") adds words, ages badly, and makes
the reader reconstruct history to understand the present. State the current
invariant. But not every mention of the past is cruft — triage into three:

1. **Pure narration → delete or restate.** "the heuristics that used to live
   here couldn't keep pace" / "removed in 0.87.0" / "we switched to X". No
   present-state value beyond what the current description already carries.
2. **Current behavior in a historical costume → keep the fact, drop the
   framing.** A backward-compat alias that is *still accepted* is current
   behavior: "the legacy `~~~card-yaml` opener is still accepted but no longer
   canonical" → "`~~~card-yaml` is also accepted as a non-canonical alias."
3. **Legacy load-bearing for the present → keep.** When the old pattern is
   *required* to understand the current one, the history is the documentation —
   e.g. a versioned-storage envelope whose job is to read old formats: the
   legacy schema *is* current reader behavior (`core/src/document/dto.rs`).

Reframing moves:

- "used to X, now Y" → assert Y in present tense.
- "no longer / previously / formerly Z" → "is not Z" / "does not Z", or drop.
- "as of 0.x / removed in 0.x" → state the current rule, no version.
- Regression-test comment → state the invariant guarded, not the bug's history:
  "X must not happen (would cause Y)" not "X used to happen."

Caution: `used to` often means "used **in order to**" — not history; read before
cutting. In consumer `docs/`, keep history a reader needs to *use* the feature
(an accepted alias, a tolerated input) — reframe it, don't delete it.

### 4. One claim per sentence — density is bytes-per-fact, not facts-per-line

Cutting words is only half of density. A sentence carrying seven clauses, three
parentheticals, and an em-dash chain is maximally compressed and unreadable —
the reader has to decompress it to find the one fact they came for, which is the
cost a comment exists to remove. Compression is not density.

- One claim per sentence. A bullet that needs three clauses is three bullets, a
  nested list, or a table.
- A set of per-case rules (per type, per format, per backend) is a table. Table
  rows are records, not sentences, and are exempt from the budget.
- Prefer a paragraph break over a semicolon chain when the second half states a
  separate fact.

`prose/README.md` sets the numeric budget — 700 characters per prose line hard,
300 soft — and `scripts/check-canon.mjs` enforces it over `prose/canon/` and
`docs/`. The soft limit is a ratchet: it reports only on files a change already
touches, so split a long line when you are editing near it, not in a sweep.

### 5. State the design, not the deliberation

Describe what is, not what was considered. Cut spike/deferred/rejected
narration; keep the resulting fact and, when it explains a present choice, the
rationale — minus the "we tried / earlier draft" framing.

- "Investigated as a spike but deferred — not needed" → "Not supported; the
  preview does not require it."
- "X was the deferred half and stays deferred by design" → "X is not carried,
  by design: <reason>."
- Rejected-alternative rationale: "A sub-handle would be justified only if paint
  shipped with click()" — keeps the *why*, sheds the *when*.
- Issue and PR numbers (`(#970)`, `tracked in #736`) are status markers: they
  say work is in motion, which canon does not carry, and they date the sentence
  around them. State the shape instead — "Python omits the opaque store by
  intent" — and drop the number. Keep it only where the issue *is* the subject
  (a CI or release doc describing a process).

## Voice

Present tense. Lead with the invariant or contract, then the mechanism. Reuse
the codebase's terms-of-art (*card-yaml block, plate, quill, backend, seam,
Technique A*). For the target density, read the comments in
`crates/core/src/document/` and the Decisions section of `PREVIEW.md` — a
rationale that names what it rejected and why, in the fewest words that stay
correct.

## Scope

| Surface | Rule |
|---|---|
| Code & test comments, `prose/canon/`, `docs/` (non-migration) | Apply in full. |
| `docs/migrations/**` | **Never touch.** Era-accurate and immutable. |
| `prose/references/`, `prose/proposals/` | Strip marketing only. Specs and proposals legitimately discuss other/future states — leave that framing. |
| Load-bearing legacy (e.g. `core/src/document/dto.rs` versioned wire schemas) | Keep. The old-format description *is* current reader behavior; tighten wording, keep the fact. |
| Identifiers (fn / test / var names) | Never rename — out of scope, churn. |

## Workflow

1. **Sweep** — grep comments/docs for: the marketing word-list above; history
   markers (`used to`, `no longer`, `previously`, `formerly`, `as of`,
   `removed in`, `renamed`, `we switched`, `legacy`, `deprecated`); and
   deliberation markers (`spike`, `deferred`, `considered`, `for now`,
   `eventually`, `we tried`); and status markers (`#\d+`, issue and PR links).
2. **Triage** — each hit: violation, or load-bearing fact in costume?
3. **Rewrite** in place — present tense, minimal, fact preserved. Fix a comment
   that contradicts the code rather than deleting it. Leave identifiers alone.
4. **Verify** — build and tests pass; no doctest broken; no test asserted the
   old wording; `node scripts/check-canon.mjs` passes.

## Done when

Comments and docs state what is, in the house voice: dense, present-tense,
unsold. No comment restates code; no header enumerates rotting lists; no prose
narrates history or deliberation. Backward-compat facts survive as current-state
statements.
