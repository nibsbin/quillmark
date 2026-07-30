# Crate API Compatibility

> **Implementation**: `crates/core/src`, `.github/workflows`
> **Related**: [CI_CD.md](CI_CD.md), [VERSIONING.md](VERSIONING.md)

## TL;DR

Every publishable crate carries one workspace version and one SemVer promise to
crates.io consumers. `cargo semver-checks` in CI is what holds it; the rules
below are what a writer decides *before* the tool sees the change. This page is
about the Rust crate API — [VERSIONING.md](VERSIONING.md) is about quill
versions, a separate axis.

## What the promise covers

The published crates are `quillmark-core`, `quillmark`, `quillmark-content`,
`quillmark-pdf`, `quillmark-typst`, `quillmark-pdfform`, and `quillmark-cli`.
The Python, WASM, fuzz, and fixture crates are `publish = false`: they path-dep
the core beside them and ship as one build, so nothing there is a compatibility
surface.

Two seams are `pub` but explicitly outside the promise, each saying so in its
own rustdoc:

| Seam | Why it is exempt |
|---|---|
| `Backend` + `SessionHandle` | Sealed and `#[doc(hidden)]`; an out-of-workspace backend writes against items this crate does not hold stable. |
| `quillmark_typst::emit` | `pub` only so `quillmark-fuzz` can drive the escapers directly. |

## `#[non_exhaustive]`

The attribute is narrower than it reads, and the two halves cost different
things.

| On | Forbids out-of-crate | Still allowed out-of-crate |
|---|---|---|
| enum | exhaustive `match` | constructing any variant |
| struct | struct literals, exhaustive destructuring | reading and assigning every `pub` field |

Inside the defining crate it does nothing, so an exhaustive match in `core` is
still a compile error when a variant lands. The guardrail stays where the work
happens.

### The enum rule

Mark it, unless a downstream `_` arm can be **silently** wrong.

The question is what a missed variant does, not whether the concept feels
closed. Ontological arguments — "a severity is two levels", "a tree is files and
directories" — are the ones that get falsified; `OutputFormat` dropped `Txt` in
0.98. Ask instead:

- **Loud on a miss** → mark it. A `_` arm that raises, refuses, or degrades to a
  documented lower bound loses nothing.
- **Silent on a miss** → keep it exhaustive, say so in the rustdoc, and accept
  semver-major as the price.

`quillmark_pdf::FieldType` is the case that keeps the exemption: `pdfform`'s
value resolver and its content-stream flattener both dispatch over the whole set
from another crate, and a variant they miss **draws nothing on the page and
reports nothing**. There the compile error is the guardrail.

### Picking the fallback

A forced `_` arm is a new opportunity to re-spell an existing behavior slightly
differently. Two rules keep that from drifting:

- Take the direction that cannot be silently wrong. `Severity` escalates to
  `Error` — over-reporting shows a note too loudly, the other direction hides a
  fatal. `HitGranularity` degrades to `Segment`, a lower bound on precision.
- Where no safe direction exists, refuse. No fallback `OutputFormat` is honest;
  every one of them promises bytes the caller did not ask for.

When the arm restates a diagnostic another site already builds, share the
constructor rather than hand-writing the body — `quillmark_core::unsupported_format`
exists because three sites had drifted to two payloads under one code.

### The struct rule

Mark it, and give it a constructor when another crate builds one.

A struct with private fields needs nothing: it is already unconstructible from
outside. For one with `pub` fields the attribute buys exactly one freedom —
adding a field — and no others, since every field stays readable and assignable.
Making a field private, changing its type, or computing it lazily still needs
private fields plus accessors, the shape `YamlError` uses.

Constructor shape: `new` takes the fields a value *always* carries; everything
optional starts absent and has a `with_*` setter beside it. A `new` that takes
every field is exactly as brittle as the literal it replaced.

`RenderOptions` is the type this cost is real for. `#[non_exhaustive]` forbids
functional update, so `RenderOptions { .., ..Default::default() }` does not
compile out-of-crate and `RenderOptions::default().with_output_format(…)` is the
path. That break was taken at the 1.0.0 freeze deliberately: it is the last
point where it is free, and this is the type that gained two options in six
minors.

## What the attribute does not cover

Most of what breaks a consumer is not a new variant or a new field. Adding a
method to an unsealed public trait, removing or renaming anything, widening a
bound, a public type losing `Send`/`Sync` through a private field, and a public
signature naming a `0.0.x` dependency (see [ERROR.md](ERROR.md) on the YAML
boundary) are all majors that no attribute sweep sees.

The `semver` job in [CI_CD.md](CI_CD.md) runs `cargo semver-checks` over the
published crates against their last release, which catches that whole class on
every PR. It is advisory while the version is `0.x` — cargo reads a `0.x` minor
bump as major, so there is no stable baseline to judge against yet — and becomes
a gate at 1.0.0.
