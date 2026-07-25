# prose/

Long-form project documentation, in four tiers by maturity:

- **`canon/`** — canonical documentation: high-level, stable captures of the
  codebase's systems, specifications, and intent. The settled truth. Start at
  [`canon/INDEX.md`](canon/INDEX.md). Canon describes *what is* and points into
  the code; it does not re-document implementation detail.
- **`references/`** — authoritative, standalone specifications. Each
  reference is self-contained: it makes no outbound links to other prose
  docs, so it can be cited freely from canon, user docs, and code comments
  without forming a cycle.
- **`proposals/`** — fleshed-out proposed changes, not yet implemented. Each is
  a concrete plan. Removed once landed or abandoned.
- **`plans/`** — working plans for multi-phase reworks in flight: the
  integration HQ for a change too large for a single proposal. One subdirectory
  per rework. Removed once the rework lands.

Canonical docs never reference proposals or plans. References never link
out to other prose docs.

## Canon and `docs/`

`prose/` is contributor-facing; [`docs/`](../docs/) is the published MkDocs site
for quill authors and integrators. The two cover the same subjects from opposite
ends, and the division is by *audience*, not by topic:

- **Canon** documents the design contract — the model, the invariants, and why
  a seam sits where it does. Its reader is changing the engine.
- **`docs/`** documents the task — authoring a quill, filling a form, persisting
  a document. Its reader is using the engine.

Neither restates the other. A `docs/` page links up to its canon page for the
full model (`Full model: [ERROR.md](…)`); a canon page links down to `docs/` for
an authoring surface it deliberately does not carry. A canon page may *point at*
a `docs/` page, but never depends on one to state a contract — a contract that
exists only in `docs/` belongs in canon.

## Canon doc spine

Every canon doc except `canon/INDEX.md` (the index) opens:

1. `# Title` on line 1.
2. A blockquote anchor on line 3. Its `**Implementation**` line is the
   navigational hook from concept to code: it points at a folder or module,
   never a file and never a line number — files rot. Other lines
   (`**Related**`, `**Package**`) are optional.
3. `## TL;DR` as the first section — two or three sentences.

Title, anchor, and TL;DR are mandatory; other sections (When to use, How,
Gotchas, Links) are optional — add them when they help. No `Status` line:
membership in canon means settled and implemented. Mark status only for
genuine exceptions (e.g. a draft specification).

## Line budget

A prose line caps at **700 characters** and should stay under **300**. Past
that, a line is a paragraph crammed onto one line — dense by the byte,
unskimmable by the claim, and unreviewable in a diff, where a one-word fix
rewrites the whole line. One claim per sentence; a bullet that needs three
clauses is three bullets, a nested list, or a table. Fenced code and table rows
are exempt: a table row is a record, not a sentence.

## Enforcement

`scripts/check-canon.mjs` runs in CI. It fails on a broken spine, a link into
`proposals/`/`plans/`, a reference that links out, an `**Implementation**`
anchor that names a file or no longer exists, a canon page missing from
`INDEX.md`, a dead `INDEX.md` link, and a line over the hard budget.

It also *warns*, without failing, when a folder named in an
`**Implementation**` anchor changed and the doc anchored there did not — the
anchor is a machine-readable code→doc back-pointer, and this is what reads it.
Most such changes need no doc edit; the warning is a prompt, not a gate.

