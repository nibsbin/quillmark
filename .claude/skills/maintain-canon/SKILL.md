---
name: maintain-canon
description: Maintain prose/canon/ — the canonical, high-level design documentation. Use when adding, consolidating, or auditing canon docs, and when a code change makes a canon doc stale (a renamed API, a moved seam, a changed invariant).
---

## Purpose

`prose/canon/` captures the codebase's systems, specifications, and intent at a
high level — enough that a human or AI can get the gist without reading the
whole codebase. Canon describes *what is* and points into the code; it does not
re-document implementation detail that the code already carries.

For comment and doc *content* — density, tense, voice — use **`dense-prose`**.

## The spine

Every canon doc except `INDEX.md` opens:

1. `# Title` on line 1.
2. A blockquote anchor on line 3, carrying `> **Implementation**:` — the
   navigational hook from concept to code. It points at a folder or module,
   never a file and never a line number; files rot. `**Related**` and
   `**Package**` lines are optional.
3. `## TL;DR` as the first section — two or three sentences.

Other sections (When to use, How, Gotchas, Links) are optional. No `Status`
line, and no issue number: membership in canon means settled and implemented.
A prose line caps at 700 characters and should stay under 300.

`prose/README.md` is normative for all of this — the four tiers, the spine, the
canon↔`docs/` division, and the link invariants. Read it before a structural
edit. `scripts/check-canon.mjs` enforces what is mechanical, in CI.

## Principles

One topic per page; one canonical per topic. Prefer deletion with consolidation
over duplicates. Keep pages skimmable and high-level; include minimal code. A
fact that belongs to a neighbouring page is a link, not a copy.

## Workflow

- **Inventory** — list docs with a one-line summary; note overlaps.
- **Consolidate** — pull unique, current bits into the canonical; rewrite for skim.
- **Prune** — replace overlaps with a one-line stub linking the canonical;
  delete obsolete docs and references.
- **Organize** — keep flat or under a few theme folders; nest sparingly.
- **Index** — keep `INDEX.md` curated, with one-liners; remove drift.

## Done when

No obvious duplicates. Everything discoverable from `INDEX.md`. Docs are short,
skimmable, folder-anchored, and easy to maintain. `check-canon.mjs` passes, and
any drift note it raises is either acted on or knowingly left.
