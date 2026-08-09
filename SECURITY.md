# Security Policy

## Reporting a vulnerability

Report privately through GitHub's [security advisory
form](https://github.com/borb-sh/quillmark/security/advisories/new). Do not open
a public issue for a suspected vulnerability.

Include what you have: the version or commit, the input that triggers it, and
what you observed. A reproducing quill or document is worth more than a
description of one.

Expect an acknowledgement within a week. Quillmark is pre-1.0 and maintained by
a small team, so a fix ships in the next release rather than on a fixed clock;
if a report warrants faster movement, we will say so when we acknowledge it.

## Supported versions

Only the latest release. Pre-1.0, fixes land forward on `main` and ship in the
next minor; nothing is backported to an earlier `0.x`.

## What Quillmark treats as trusted

Two inputs, two different postures. Reports are graded against these, so a
finding that requires a hostile *quill* is a different severity from one that
requires only a hostile *document*.

| Input | Posture |
|---|---|
| **A document** (Markdown + card-yaml) | **Untrusted.** Bounded at parse (see [Operations](docs/integration/operations.md)), escaped on the way into a backend, and never able to introduce executable template code. A document that panics, hangs, or reaches outside the process is a vulnerability. |
| **A quill** (`Quill.yaml`, plate, packages, assets) | **Trusted, and equivalent to code.** A Typst plate is a program; loading a quill is running whatever the quill author wrote. Sandboxing quills is not a property Quillmark claims. |

So: loading a hostile quill is out of scope, in the same way that `cargo run` on
a hostile crate is. Treat quills like dependencies — review them, pin them, and
get them from somewhere you trust. There is no registry or integrity binding
behind `$quill` selectors today ([#1204](https://github.com/borb-sh/quillmark/issues/1204)),
so that trust is entirely yours to establish.

Within the trusted half, two properties still hold and a break in either is a
bug worth reporting:

- **No network.** `QuillWorld` loads packages only from `{quill}/packages/` in
  the in-memory tree. Quillmark never fetches a Typst package.
- **No ambient filesystem.** A plate reads what the quill tree carries, through
  the same in-memory file system.

## Known limits, already stated

Not vulnerabilities, and not news:

- **Render cost is unbounded.** No deadline and no cancellation; a large
  document or an expensive plate runs as long as it runs. See
  [Operations](docs/integration/operations.md) and
  [#1213](https://github.com/borb-sh/quillmark/issues/1213).
- **Unsigned signature fields.** `signature-field` emits an AcroForm widget.
  Quillmark performs no cryptography and signs nothing.

## Dependency advisories

`cargo audit` runs on every dependency change and weekly against an unchanged
tree (`.github/workflows/audit.yml`). Accepted advisories, each with the
reachability argument that justifies it, are in
[`.cargo/audit.toml`](.cargo/audit.toml) — an entry there is a decision with a
stated falsifier, not a mute.
