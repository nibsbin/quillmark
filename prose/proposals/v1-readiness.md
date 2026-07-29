# v1.0.0 Production Readiness Review

> Reviewed at `3e5c3b1`, workspace version `0.98.0`.

Scope: what stops us tagging `1.0.0` and telling production consumers the
surface is stable. Ordered by whether it blocks the tag, not by effort.

## TL;DR

The engine is in good shape — no `unsafe`, 1083 tests green, every ingestion
boundary depth- and size-bounded, symlinks refused at the quill loader, near-clean
clippy, disciplined migration guides, Trusted Publishing with npm provenance.

What is not ready is the *promise*. `1.0.0` means "adding a variant, a field, or
a dependency bump is a major release from here". Four things make that promise
unkeepable today, and the repo currently says the opposite out loud: the README
reads **UNDER DEVELOPMENT — APIs may change**, and the docs site carries an
"Unstable APIs" warning. History agrees with them — 18 migration guides across
`0.81` → `0.99`, roughly one breaking change per minor.

## Blockers

### B1 — No `#[non_exhaustive]` in the engine crates — #1100

97 top-level `pub struct` / `pub enum` declarations across `core`, `quillmark`,
`quillmark-pdf`, and `backends/*`. Zero carry `#[non_exhaustive]`.
`quillmark-content` carries it 12 times, so the pattern is understood — it just
never reached the crates whose types every consumer names.

At `1.0.0` each of these freezes. Adding an enum variant or a struct field
becomes `2.0.0`. The types most likely to grow:

| Type | Why it grows |
|---|---|
| `OutputFormat` | a fourth output format |
| `RenderOptions` | grew `producer` and `regions` in the last six minors |
| `RenderResult` | grew `regions` alongside it |
| `Diagnostic` | grew `path` and `source_chain` |
| `ParseError`, `EditError`, `ValidationError`, `StorageError`, `WireError`, `CardKindError`, `CoercionError` | a new failure mode is a new variant |
| `FieldType`, `PayloadItem`, `FieldSource`, `HitGranularity` | open vocabularies by design |

`OutputFormat` is frozen twice over: `pub const ALL: [OutputFormat; 3]` encodes
the variant count *in its type*, so a fourth format breaks every caller that
names the constant's type even if the enum were open. `ALL` wants to be
`&'static [OutputFormat]`.

Fix: audit all 97, mark everything not deliberately closed, and swap `ALL` to a
slice. A closed type (`Severity`, `Location`, `Version`) is a decision worth
recording in its doc comment rather than a default.

### B2 — `serde-saphyr 0.0.23` in `quillmark-core`'s public API — #1099

- `QuillValue::from_yaml_str() -> Result<Self, serde_saphyr::Error>` — `value.rs:221`
- `QuillConfig::schema_yaml() -> Result<String, serde_saphyr::ser::Error>` — `quill/schema_yaml.rs:5`

Under Cargo's rules every `0.0.z` is incompatible with every other `0.0.z`.
Taking a `serde-saphyr` bugfix is therefore a breaking change to
`quillmark-core` — after `1.0.0`, a major one. A downstream crate cannot even
name these error types without pinning `serde-saphyr = "=0.0.23"` itself.

Fix: wrap both in owned crate-local error types before the tag. This is the one
finding here that is genuinely unfixable after it.

### B3 — No declared stability policy — #1105

Nothing states what `1.0.0` covers. Three surfaces ship (crates.io, PyPI, npm)
plus two wire formats (`StoredDocument`, plate JSON) and the document syntax
itself; consumers need to know which of those the major version speaks for.

Fix: a stability section in the README and `docs/index.md`, replacing the two
"unstable" warnings, naming per-surface what is covered and what stays
independently versioned. `StoredDocument` already has its own version tags and
should be called out as separate from the crate's semver.

### B4 — No MSRV — #1105

No `rust-version` in any manifest; `prose/canon/CI_CD.md` lists MSRV among the
deliberate CI exclusions. Consumers on a pinned toolchain get a compile failure
instead of a resolver message, and nothing stops a patch release raising the
floor.

Fix: set `rust-version` in `[workspace.package]` and add a CI job on that exact
toolchain. Absent that, document "latest stable" as the policy — but decide it,
since after `1.0.0` a raise is arguably breaking either way.

## Should fix before the tag

### S1 — Internal seams that 1.0 would freeze for the world — #1103

`quillmark-pdf` exposes `pub mod reader` and `pub mod writer`: `find_object_bytes`,
`find_dict_value`, `splice_dict_value`, `extract_outer_dict`, `dict_object`,
`alloc_id`, `pdf_escape`, `winansi_encode`, `UpdatedObject`. Raw PDF byte
surgery, public only so `quillmark-pdfform`'s `flatten.rs` can reach it across
the crate boundary.

`quillmark-typst` exposes `pub mod emit`: `emit_content`, `Emission`,
`SegmentMap`, `EscapeCtx`, `EmitError` — public only so `quillmark-fuzz`
(`publish = false`) can reach it.

Neither is a surface we want to support for a decade. Fix: `#[doc(hidden)]` with
a "workspace-internal, not covered by semver" note, or gate behind an
`internal` feature.

### S2 — The backend extension point is doc-hidden — #1103

`Backend` is public and implementable by any downstream crate, but implementing
it requires `#[doc(hidden)] SessionHandle` and `#[doc(hidden)] LiveSession::new`.
So third-party backends are either supported (and the seam should be documented
and frozen) or they are not (and `Backend` should be sealed). Right now `1.0.0`
would freeze a trait nobody outside the workspace can usefully implement, while
promising nothing about the two items they would need.

### S3 — One YAML entry point skips the depth budget — #1101

`document/limits.rs` states the invariant: "every YAML entry point — card-yaml
payloads and `Quill.yaml` — enforces the same nesting limit". `assemble.rs:153`
and `quill/config.rs:1311` both use `from_str_with_options` with the budget.
`QuillValue::from_yaml_str` (`value.rs:222`) calls bare `serde_saphyr::from_str`
— no budget. It is public API; a consumer handing it untrusted YAML gets
unbounded recursion, which is a stack overflow, which on wasm32 is an
unrecoverable trap.

`QuillValue::from_json` has the adjacent hole: `Node::from_json` recurses
per level with no guard. The wire lanes are safe because `serde_json::from_str`
caps at 128, but a programmatically-built deep `Value` handed to the public
constructor is not.

### S4 — Library warnings go to stderr and vanish — #1102

Four `eprintln!` in `backends/typst/src/world.rs` — invalid asset path (226),
unparseable `typst.toml` (283), invalid package-file path (336), missing package
entrypoint (369). All four are conditions the caller should see, and the crate
already has `Diagnostic` warnings threaded to every binding. On wasm32 stderr
goes nowhere at all, so a quill with a malformed `typst.toml` silently degrades
to an "unknown import" compile error with no trace of the real cause.

Fix: route them through the warning channel `Quill::from_tree_with_warnings`
already established.

### S5 — Fuzzing misses the decode lanes the bindings expose — #1104

Covered: markdown parse, emit round-trip, Typst escaping, schema coercion.

Not covered: `StoredDocument` decode (`Document.fromJson`), `CardWire` decode,
`Content` canonical decode, and the `applyChange` op wire. Those are exactly the
paths that take caller-supplied JSON, and a panic in any of them traps the WASM
module unrecoverably — the user loses the document. That class has shipped once
already: the `0.99` guide records an unbounded opaque payload that "trapped the
WASM module".

Fix: proptest round-trips over the three decode lanes plus an op-sequence
target, before the tag rather than after.

### S6 — Published crates carry no license text — #1106

`include = ["src/**", "Cargo.toml", "README*", "LICENSE*"]`, but no crate
directory holds a `LICENSE`. Verified against a real package:
`quillmark-core-0.98.0.crate` contains `README.md` (inherited from the workspace
root) and no license file, while declaring `license = "Apache-2.0"`.
Apache-2.0 §4(a) asks that redistributions carry a copy.

Fix: symlink or copy `LICENSE` into each published crate directory.

### S7 — Single-platform CI — #1107

`ubuntu-latest` only. The CLI, the Python wheels, and every crates.io consumer
run on macOS and Windows. Path handling looks portable by construction — tree
keys are `/`-joined in `walk_files`, and `get_node` resolves via
`Path::components()`, which accepts either separator — but nothing proves it,
and the Windows path where `list_directories` produces `packages\foo` and feeds
it back through `get_file` is untested.

Fix: add macOS and Windows to the `test` job matrix.

### S8 — No security policy, no advisory scanning — #1107

No `SECURITY.md`, so there is no disclosure path. No `cargo audit` / `cargo
deny` job and no dependabot config over 440 resolved dependencies, including the
whole Typst toolchain and a hand-rolled PDF reader/writer. No `CODEOWNERS`.

### S9 — `cargo publish --no-verify` — #1106

`release.yml:143` publishes with `--no-verify`, so no packaged crate is ever
built from its `.crate`. Combined with a hand-maintained `include` list, a
missing file surfaces first as a broken crates.io release. A `cargo package`
job in PR CI closes it.

## Accepted risks worth writing down

- **No render timeout or cancellation** (#1108). `Quillmark::render` and
  `LiveSession::apply` run Typst to completion; nothing bounds compile time and
  no cancellation token exists. Fine for a CLI, a liability for a server. At
  minimum, state that quills are trusted input.
- **`comemo`'s cache is process-global**, and so is its eviction clock —
  documented at `compile.rs:31`, with no consumer-facing knob. Concurrent
  sessions lose reuse; memory stays bounded by the `evict(10)` after each
  compile.
- **`load_dir` recursion is unbounded** in depth and in total bytes; only the
  per-file 50 MiB cap applies. A hostile quill directory can still exhaust
  memory.
- **`RenderError`'s non-empty invariant is `debug_assert!` only** — deliberate
  and documented, with an `[]` fallback in `Display`.
- **Storage schema versions are named after crate versions**
  (`quillmark/document@0.93.0`). Decide whether `1.0.0` mints `@1.0.0` and
  whether `@0.92.0` read support retires with the major.
- **Clippy is not gated** — deliberate, recorded at `ci.yml:41`. The tree is
  clean today: two real warnings, both cosmetic.

## Issue index

Every finding above is filed. #1099 first — it is the only one that cannot be
fixed after the tag.

| Issue | Covers |
|---|---|
| #1099 | B2 — `serde-saphyr` in the public API |
| #1100 | B1 — `#[non_exhaustive]` |
| #1101 | S3 — the unbudgeted YAML entry point |
| #1102 | S4 — stderr warnings |
| #1103 | S1, S2 — internal seams and the `Backend` seal |
| #1104 | S5 — fuzz coverage on the decode lanes |
| #1105 | B3, B4 — stability policy and MSRV |
| #1106 | S6, S9 — license text and publish verification |
| #1107 | S7, S8 — CI matrix, advisory scanning, `SECURITY.md` |
| #1108 | render timeout, cancellation, and the trust boundary |

## Suggested order

1. B2 — the only finding that cannot be fixed after the tag.
2. B1 and S1/S2 — one sweep over the public surface: mark it open, hide what is
   internal, decide whether `Backend` is sealed.
3. S3, S4, S5 — the correctness and observability holes, all small.
4. B3, B4, S6–S9 — policy and infrastructure; independent of the code.
