# Security Policy

## Reporting a vulnerability

Report privately through GitHub's [security advisory form](https://github.com/borb-sh/quillmark/security/advisories/new). Do not open a public issue for a suspected vulnerability.

Include the version or commit, the input that triggers it, and what you observed. A reproducing quill or document is worth more than a description of one.

Expect an acknowledgement within a week. Quillmark is pre-1.0, so a fix ships in the next release rather than on a fixed clock; a report that warrants faster movement gets told so in the acknowledgement.

## Supported versions

Only the latest release. Fixes land forward on `main` and ship in the next minor; nothing is backported to an earlier `0.x`.

## What Quillmark treats as trusted

Two inputs, two postures. Reports are graded against these, so a finding that needs a hostile *quill* carries a different severity from one that needs only a hostile *document*.

| Input | Posture |
|---|---|
| **A document** (Markdown + card-yaml) | **Untrusted.** Bounded at parse (see [Operations](docs/integration/operations.md)), escaped on the way into a backend, and unable to introduce executable template code. A document that panics, hangs, or reaches outside the process is a vulnerability. |
| **A quill** (`Quill.yaml`, plate, packages, assets) | **Trusted, and equivalent to code.** A Typst plate is a program, so loading a quill runs whatever the quill author wrote. Sandboxing quills is not a property Quillmark claims. |

Loading a hostile quill is therefore out of scope, as running a hostile crate under `cargo run` is. Treat quills like dependencies: review them, pin them, take them from somewhere you trust. Nothing binds a `$quill` selector to bytes across a trust boundary ([#1204](https://github.com/borb-sh/quillmark/issues/1204)), so establishing that trust is yours.

Two properties hold inside the trusted half, and a break in either is a bug worth reporting:

- **No network.** `QuillWorld` loads packages only from `{quill}/packages/` in the in-memory tree. Quillmark never fetches a Typst package.
- **No ambient filesystem.** A plate reads what the quill tree carries, through that same in-memory file system.

## Known limits

Neither is a vulnerability:

- **Render cost is unbounded.** No deadline and no cancellation, so a large document or an expensive plate runs as long as it runs. See [Operations](docs/integration/operations.md) and [#1213](https://github.com/borb-sh/quillmark/issues/1213).
- **Unsigned signature fields.** `signature-field` emits an AcroForm widget. Quillmark performs no cryptography and signs nothing.

## Dependency advisories

`cargo audit` runs on every dependency change and weekly against an unchanged tree (`.github/workflows/audit.yml`). Accepted advisories live in [`.cargo/audit.toml`](.cargo/audit.toml), each with the reachability argument that justifies it and the condition that would revoke it.
