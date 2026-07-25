#!/usr/bin/env node
// Canon spine lint — enforces the doc spine, the link invariants, and the
// prose line budget specified in prose/README.md. Zero dependencies.
//
// Usage: node scripts/check-canon.mjs [--drift[=<base>]]
//
// `--drift` adds a non-blocking anchor-drift report: it diffs against <base>
// (default `HEAD^1`) and names the canon docs whose `**Implementation**`
// folder changed while the doc itself did not. Advisory only — most code
// changes need no doc edit, and a hard gate here would only teach people to
// write vague anchors.
import { readdirSync, readFileSync, existsSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join, posix, dirname } from 'node:path';

const problems = [];
const notices = [];
const fail = (file, msg) => problems.push(`${file}: ${msg}`);
const warn = (file, msg) => notices.push(`${file}: ${msg}`);

// A file path — a slashed token with a dotted basename — inside an anchor.
// Keys on path shape, not an extension list, so a new file type can't slip past.
const FILE_IN_ANCHOR = /[\w-]+\/[\w/-]*\.[a-z0-9]+\b/;
// A markdown link target into the proposal/plan tiers, segment-anchored so a
// path like `parked-proposals/` doesn't trip it.
const PLAN_LINK = /\]\([^)]*\/(?:proposals|plans)\//;
// A relative markdown link target to a .md file (an outbound prose link).
const PROSE_LINK = /\]\((?!https?:)[^)]*\.md(?=[)#])/;
// An issue or PR reference. A status marker: it says work is in motion, which
// canon does not carry, and it dates the sentence around it. Narrow enough to
// miss heading anchors (`](#regions-overlay)`) and Typst code (`#let`, `#data`).
const ISSUE_REF = /#\d+\b|github\.com\/[^)\s]+\/(?:issues|pull)\/\d+/;
// A backticked slashed token — an anchor's folder reference.
const ANCHOR_PATH = /`([\w.-]+\/[\w./-]*)`/g;
// A relative markdown link target of any kind.
const REL_LINK = /\]\((?!https?:|#)([^)\s]+)/g;

// Line budget. A prose line past SOFT reads as a paragraph crammed onto one
// line: dense by the byte, unskimmable by the claim. HARD is the gate; SOFT is
// the ratchet, reported so the tail gets cleaned as docs are touched anyway.
const HARD = 700;
const SOFT = 300;

const mdFiles = (dir) =>
  existsSync(dir) ? readdirSync(dir).filter((n) => n.endsWith('.md')).sort() : [];

// Every .md under `dir`, minus `docs/migrations/` — released guides are
// era-accurate and immutable, so no rule applies to them.
const walkDocs = (dir) =>
  !existsSync(dir)
    ? []
    : readdirSync(dir)
        .sort()
        .flatMap((name) => {
          const p = join(dir, name);
          if (statSync(p).isDirectory()) return name === 'migrations' ? [] : walkDocs(p);
          return name.endsWith('.md') ? [p] : [];
        });

// Prose lines only: no fenced code, no table rows (a table row is a record, not
// a sentence, and rewrapping one is worse than leaving it long).
function proseLines(text) {
  const out = [];
  let fenced = false;
  text.split('\n').forEach((line, i) => {
    if (/^\s*(```|~~~)/.test(line)) {
      fenced = !fenced;
      return;
    }
    if (fenced || /^\s*\|/.test(line)) return;
    out.push([i + 1, line]);
  });
  return out;
}

const anchors = []; // { doc, path } — one row per folder a canon doc claims

for (const name of mdFiles('prose/canon')) {
  const file = join('prose/canon', name);
  const text = readFileSync(file, 'utf8');
  const lines = text.split('\n');

  const planLink = text.match(PLAN_LINK);
  if (planLink) fail(file, `links into proposals/ or plans/ (\`${planLink[0]}\`) — canon never references them`);

  for (const [n, line] of proseLines(text)) {
    const ref = line.match(ISSUE_REF);
    if (ref) fail(file, `line ${n} cites \`${ref[0]}\` — canon states the shape, not the ticket tracking it`);
  }

  if (name === 'INDEX.md') continue; // the index has no spine

  if (!lines[0]?.startsWith('# ')) fail(file, 'line 1 is not a `# Title`');

  // Anchor blockquote: contiguous `>` lines from line 3. Only Implementation
  // lines carry a path, so the file check scans the whole quote.
  if (!lines[2]?.startsWith('> ')) {
    fail(file, 'line 3 is not the anchor blockquote');
  } else {
    const quote = [];
    for (let i = 2; i < lines.length && lines[i].startsWith('>'); i++) quote.push(lines[i]);
    const impl = quote.filter((l) => l.startsWith('> **Implementation**:'));
    if (!impl.length) fail(file, 'anchor blockquote has no `> **Implementation**:` line');
    const m = quote.join('\n').match(FILE_IN_ANCHOR);
    if (m) fail(file, `Implementation anchor names a file (\`${m[0]}\`) — anchors point at folders or modules`);

    // An anchor that no longer resolves is the rot the folder rule exists to
    // prevent; it only prevents it if something checks.
    for (const [, p] of impl.join('\n').matchAll(ANCHOR_PATH)) {
      if (!existsSync(p)) fail(file, `Implementation anchor \`${p}\` does not exist`);
      else anchors.push({ doc: file, path: p.endsWith('/') ? p : `${p}/` });
    }
  }

  const firstH2 = lines.find((l) => l.startsWith('## '));
  if (firstH2 !== '## TL;DR') fail(file, `first section is \`${firstH2 ?? '(none)'}\` — canon docs open with \`## TL;DR\``);
}

// INDEX is the only entry point canon promises, so a page missing from it is
// unreachable and a link out of it must resolve.
if (existsSync('prose/canon/INDEX.md')) {
  const index = readFileSync('prose/canon/INDEX.md', 'utf8');
  for (const name of mdFiles('prose/canon')) {
    if (name === 'INDEX.md') continue;
    if (!index.includes(`(${name})`)) fail('prose/canon/INDEX.md', `does not link \`${name}\` — every canon page is reachable from the index`);
  }
  for (const [, target] of index.matchAll(REL_LINK)) {
    const resolved = posix.normalize(join('prose/canon', target.split('#')[0]));
    if (!existsSync(resolved)) fail('prose/canon/INDEX.md', `link target \`${target}\` does not resolve`);
  }
}

for (const name of mdFiles('prose/references')) {
  const file = join('prose/references', name);
  const m = readFileSync(file, 'utf8').match(PROSE_LINK);
  if (m) fail(file, `links to another prose doc (\`${m[0]}\`) — references are self-contained`);
}

// The diff, when one is available: scopes the soft limit and drives the drift
// report. `--drift` without a base diffs the last commit.
const driftArg = process.argv.slice(2).find((a) => a === '--drift' || a.startsWith('--drift='));
const base = driftArg ? driftArg.split('=')[1] || 'HEAD^1' : null;
let changed = null;
if (base) {
  try {
    changed = new Set(
      execFileSync('git', ['diff', '--name-only', base, 'HEAD'], { encoding: 'utf8' }).split('\n').filter(Boolean),
    );
  } catch {
    console.log(`check-canon: no diff against ${base} — drift and soft-limit reporting skipped`);
  }
}

// Line budget over canon and the consumer docs, minus migrations. The hard
// limit is a gate everywhere; the soft limit is a ratchet, reported only for
// files this change touches so the standing tail never becomes background noise.
let softTotal = 0;
for (const file of [...mdFiles('prose/canon').map((n) => join('prose/canon', n)), ...walkDocs('docs')]) {
  for (const [n, line] of proseLines(readFileSync(file, 'utf8'))) {
    if (line.length > HARD) fail(file, `line ${n} is ${line.length} chars (max ${HARD}) — one claim per sentence; split the clauses into bullets or a table`);
    else if (line.length > SOFT) {
      softTotal++;
      if (changed?.has(file)) warn(file, `line ${n} is ${line.length} chars (soft limit ${SOFT}) — split it while you are here`);
    }
  }
}
if (softTotal) console.log(`check-canon: ${softTotal} lines over the ${SOFT}-char soft limit tree-wide`);

// Anchor drift: the `**Implementation**` line is a machine-readable code→doc
// back-pointer. Map each changed file to the *most specific* anchor claiming
// it, so a change under `crates/core/src/quill/` doesn't also wake every doc
// anchored at `crates/core/src/`.
if (changed && anchors.length) {
  const woken = new Map(); // doc -> Set of folders
  for (const path of changed) {
    let best = 0;
    for (const { path: a } of anchors) if (path.startsWith(a) && a.length > best) best = a.length;
    if (!best) continue;
    for (const { doc, path: a } of anchors) {
      if (a.length !== best || !path.startsWith(a) || changed.has(doc)) continue;
      if (!woken.has(doc)) woken.set(doc, new Set());
      woken.get(doc).add(a);
    }
  }
  for (const [doc, folders] of [...woken].sort())
    warn(doc, `${[...folders].join(', ')} changed since ${base}; this doc did not — check it still describes what is`);
}

for (const n of notices) {
  console.log(`check-canon: note: ${n}`);
  if (process.env.GITHUB_ACTIONS) {
    const [file, ...rest] = n.split(': ');
    console.log(`::warning file=${file}::${rest.join(': ')}`);
  }
}
if (problems.length) {
  for (const p of problems) console.error(`check-canon: ${p}`);
  process.exit(1);
}
console.log('check-canon: canon spine OK');
