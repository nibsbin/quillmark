---
name: dense-prose
description: Write comments and docs at high semantic density: terse, present-tense, unsold, mostly self-documenting. Use when writing or reviewing code comments, prose/canon/, or any doc for density and house voice, or when comments narrate change ("used to", "no longer", "renamed", "as of 0.x") instead of stating what is.
---

## The measure

Density is facts per word. Cut words that carry no fact: the sell, the echo, the history, the deliberation (rules 1–4). Pack more fact into what survives (rule 5). A comment or doc earns its bytes when it states a fact the reader cannot get faster from the code, in the fewest words that stay correct.

This skill is the house voice (dense, present-tense, declarative, unsold) and owns comment and doc *content*. Canon *structure* (Title → Implementation anchor → TL;DR, one concept per page) belongs to **`maintain-canon`**.

## Prime directive: correctness over brevity

Density has a numerator. A cut that drops a fact, or shifts a claim unverified against the code, is dilution, not compression. Unsure a statement still holds? Leave it. Edits are surgical: touch a line only when it breaks a rule. Prose already dense and correct is done; over-editing is the main failure mode.

## 1. No marketing or persuasion

A sell spends words on the reader's opinion instead of their knowledge. Cut *powerful, seamless, elegant, robust, flexible, blazing(-fast), cutting-edge, state-of-the-art, first-class (citizen), rich (set of), comprehensive, battle-tested, out of the box, leverage* (meaning "use"), and *simply / just / easily* when they only imply ease. State the capability plainly: "Partial documents are first-class citizens" → "A document need not be complete."

Keep *just / simply / only* when load-bearing ("just sugar for the `raw` element", "three or more tildes"). The word is not the violation; the sell is.

## 2. Self-documenting first

The code is the primary documentation; a comment restating it is noise.

- Delete echoes (`// increment i`; `/// The name` on a field named `name`); a clearer name beats the comment.
- Collapse padded rustdoc scaffolding: "## Key Functions / ## Quick Example / ## Detailed Documentation / For comprehensive details including: …" becomes one tight paragraph and at most one runnable example.
- Never enumerate a module's public items in its header; rustdoc lists them and the hand-list rots. Describe the module's job.
- One good example beats three; "see X for comprehensive coverage" is filler.
- One fact, one home. Cross-reference rather than restate; a fact copied twice drifts.

## 3. Present tense: what is, not how it got here

Evolutionary narration ("we used to X, now Y") makes the reader reconstruct history to learn the present, then ages badly. Triage every mention of the past:

1. **Pure narration: delete or restate.** "the heuristics that used to live here couldn't keep pace", "removed in 0.87.0", "we switched to X" carry no present-state value beyond the current description.
2. **Current behavior in a historical costume: keep the fact, drop the framing.** A still-accepted compat alias *is* current behavior: "the legacy `~~~card-yaml` opener is still accepted but no longer canonical" → "`~~~card-yaml` is also accepted as a non-canonical alias."
3. **Legacy load-bearing for the present: keep.** When the old pattern is required to read the current one, the history is the documentation: the versioned envelope in `crates/core/src/document/dto.rs` decodes stored old formats, so the legacy schema *is* current reader behavior.

Reframing moves: "used to X, now Y" → assert Y; "no longer / previously / formerly Z" → "is not Z", or drop; "as of 0.x / removed in 0.x" → state the current rule, no version; a regression-test comment states the invariant guarded ("X must not happen; would cause Y"), not the bug's history.

Caution: `used to` often means "used **in order to**"; read before cutting. History a reader needs to *use* the thing (an accepted alias, a tolerated input) gets reframed, not deleted; that is most of what `docs/` carries.

## 4. State the design, not the deliberation

Cut spike/deferred/rejected narration; keep the resulting fact, plus the rationale when it explains a present choice, minus the "we tried / earlier draft" framing.

- "Investigated as a spike but deferred; not needed" → "Not supported; the preview does not require it."
- "X was the deferred half and stays deferred by design" → "X is not carried, by design: <reason>."
- Rejected-alternative rationale keeps the *why*, sheds the *when*: "A sub-handle would be justified only if paint shipped with `click()`."
- Issue and PR numbers (`(#970)`, `tracked in #736`) are status markers: they say work is in motion, which canon does not carry, and they date the sentence around them. State the shape instead ("Python omits the opaque store by intent") and drop the number. Keep it only where the issue *is* the subject, in a CI or release doc describing a process.

## 5. Compress what survives

Deleting whole sentences is half the work; the other half is more fact per word.

- **Losable test**: cut any sentence whose removal costs no fact. Length tracks surprise: the unobvious invariant gets the words, the obvious call gets none.
- **No throat-clearing**: "Note that", "It is worth noting", "Basically", "In general", and a section's first line restating its own heading.
- **No empty hedges**: *typically, essentially, somewhat, arguably, various*, unless the uncertainty is real and calibrated.
- **Shrink phrases**: *in order to* → to; *is able to / has the ability to* → can; *due to the fact that* → because; *a number of* → the count; *performs validation of* → validates.
- **Fold, don't append**: a second sentence qualifying the first becomes a clause of it.
- **Name the thing**: the specific noun beats a category plus an example; the measured number beats a vague *several*.
- **Compression is not density**: one claim per sentence. A sentence carrying seven clauses and three parentheticals is maximally compressed and unreadable, because the reader has to decompress it to reach the one fact they came for. A bullet needing three clauses is three bullets, a nested list, or a table; a set of per-case rules (per type, per format, per backend) is a table, and a table row is a record, not a sentence.

## Voice

Present tense. Lead with the invariant or contract, then the mechanism. Reuse the codebase's terms-of-art (*card-yaml block, plate, quill, backend, seam*). No em-dashes: fold to a colon when what follows names or explains what precedes it, a comma before a conjunction, a semicolon before an independent clause, and parentheses for a matched pair. One colon per sentence. Match the density of the exemplars: the comments in `crates/core/src/document/` and the "Decisions and rationale" section of `prose/canon/PREVIEW.md`.

A paragraph is one line: never hard-wrap prose at a column. A line break in markdown means a new paragraph, list item, or table row; nothing else. Comments wrap to the code's line budget. `prose/README.md` sets the numeric bound on a prose line: 700 characters, gated by `scripts/check-canon.mjs` over `prose/canon/` and `docs/`, with 300 as the target you write to. Split a long line when you are editing near it; below the gate, nothing mechanical reminds you.

## Scope

| Surface | Rule |
|---|---|
| Code and test comments, `prose/canon/`, `docs/` (non-migration) | Apply in full. |
| `docs/migrations/**`, `CHANGELOG.md`, and era-stamped records generally | **Never touch.** Accurate to their moment, immutable. |
| An em-dash that is the subject, not punctuation (an encoding table, a Unicode fixture, rendered sample output) | Keep: it is data. `crates/quillmark-pdf/src/writer.rs` maps the character to its WinAnsi byte; `prescan.rs` parses it. |
| `prose/references/`, `prose/proposals/`, `prose/plans/` | Strip marketing only; discussing other or future states is their job. |
| Load-bearing legacy (`crates/core/src/document/dto.rs` versioned wire schemas) | Keep: the old-format description *is* current reader behavior. Tighten wording, keep the fact. |
| Identifiers (fn / test / var names) | Never rename; out of scope, churn. |

## Workflow

1. **Sweep**: grep for the marketing list above; history markers (`used to`, `no longer`, `previously`, `formerly`, `as of`, `removed in`, `renamed`, `we switched`, `legacy`, `deprecated`); deliberation markers (`spike`, `deferred`, `considered`, `for now`, `eventually`, `we tried`); status markers (`#\d+`, issue and PR links); and filler (`Note that`, `in order to`, `is able to`).
2. **Triage**: each hit is a violation or a load-bearing fact in costume.
3. **Rewrite in place**: present tense, minimal, fact preserved. A comment contradicting the code gets fixed, not deleted. Identifiers stay.
4. **Verify**: build and tests pass; no doctest broken; no test asserted the old wording; `node scripts/check-canon.mjs` passes.

## Done when

Comments and docs state what is, in the house voice: dense, present-tense, unsold. Nothing restates code, no header carries a rotting list, no prose narrates history or deliberation, and no surviving sentence sheds a word without shedding a fact. Backward-compat facts survive as current-state statements.
