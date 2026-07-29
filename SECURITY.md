# Security Policy

## Reporting a vulnerability

Report privately through GitHub's [private vulnerability
reporting](https://github.com/borb-sh/quillmark/security/advisories/new) — not a
public issue, and not a pull request.

Include what you have: the affected surface (crate, `@quillmark/wasm`, the
`quillmark` PyPI package, or the CLI), a version, and the smallest input that
reproduces it. A rendered document or quill bundle that triggers the behaviour is
worth more than a description of it.

Expect an acknowledgement within a week. If a report turns out to be a real
vulnerability, the fix ships in a release and the advisory is published with it.

## Supported versions

Pre-1.0.0, only the latest release is supported. A fix ships forward, not
backported.

## Trust model

Quillmark takes two kinds of untrusted input, and they are not equally bounded.

**Documents are hostile input.** Markdown and stored `Document` JSON are bounded
at every ingestion boundary — input size, YAML size, card count, field count, and
container nesting on both the YAML and opaque-JSON lanes. Report anything that
gets past those: an overflow, an unbounded allocation, a panic (which is
unrecoverable on wasm32, where it traps the module).

**Quills are trusted input.** A quill carries a Typst plate, which is arbitrary
Typst code, and nothing bounds its compile time or memory. The loader refuses
symlinks and caps individual files, but a quill that is merely expensive will run
until it finishes. Treat loading a quill as running its author's code: do not
render quills from sources you would not otherwise execute.

That asymmetry is deliberate, not an oversight — but if a *document* can steer a
quill into unbounded work, that crosses the boundary and is a vulnerability.
Report it.
