# Operations

What a service or an editor embedding Quillmark needs to know before it takes
real traffic: what is bounded, what is not, what can be shared across threads,
and what a long-lived process accumulates.

## Input limits

Enforced at parse, on every surface, before a backend sees anything. Each raises
a `parse::*` diagnostic rather than a panic, so all of them are recoverable and
routable by `code` (see [Error Handling](error-handling.md)).

| Limit | Value | Constant | Raised as |
|---|---|---|---|
| Markdown input | 10 MiB | `MAX_INPUT_SIZE` | `parse::input_too_large` |
| One card-yaml block | 1 MiB | `MAX_YAML_SIZE` | `parse::input_too_large` |
| Cards per document | 1 000 | `MAX_CARD_COUNT` | `parse::input_too_large` |
| Fields per card-yaml block | 1 000 | `MAX_FIELD_COUNT` | `parse::input_too_large` |
| YAML nesting | 100 container levels | `MAX_YAML_DEPTH` | `parse::yaml_error_with_location` |

**All four size and count limits raise the same code.** They differ only in the
`max` arg, so a consumer that wants to say *which* ceiling was hit compares
`max` against the constants — the code alone does not distinguish "document too
big" from "too many cards".

The depth budget is one shape enforced at every ingestion boundary, but each
lane reports it in its own vocabulary: parse hits `serde_saphyr::Budget` and
surfaces a YAML error (`budget breached: Depth`), programmatic writes raise
`edit::value_too_deep`, and the bound door raises `conform::value_too_deep`.
Each binding's own converter enforces the same 100.

The constants are public (`quillmark_core::MAX_INPUT_SIZE` and siblings). Read
them rather than hard-coding a copy; a consumer that rejects early should reject
on the same number the engine will.

## Render is not bounded

**There is no render budget.** No deadline, no cancellation, no fuel, and no
progress callback. `Quillmark::render` returns when the backend is done.

This is the one place where the parse-side limits do not carry through. They
bound the *input*, and nothing translates them into a bound on the *work*: a
document comfortably inside every limit above can occupy a core for a long time.

What that costs, in shape rather than in numbers:

- **Body size is roughly linear.** Ordinary prose scales the way you would
  expect, and a document near the 10 MiB ceiling is tens of seconds of compile.
- **Plates are not.** A plate is a Typst program, and layout has pathological
  shapes — a long run of forced breaks in one paragraph is superlinear. Cost is
  a property of the quill at least as much as of the document.
- **Typst bounds iteration, not work.** An infinite loop in a plate is refused
  (`loop seems to be infinite`) and so is unbounded recursion. A loop that
  terminates after enough expensive iterations is not.

Until a budget exists ([#1213](https://github.com/borb-sh/quillmark/issues/1213)),
the bound has to come from the host:

- **Server.** Render on a worker thread or a subprocess you can abandon, and set
  your own timeout. `Quillmark` is `Sync` (below), so a pool over one engine is
  the natural shape.
- **Browser.** `engine.render` and `session.render` are **synchronous** — they
  return a `RenderResult`, not a `Promise`. A render on the main thread freezes
  the tab for its whole duration. Run the WASM module in a Web Worker and
  `worker.terminate()` to abort; there is no in-band cancel, and terminating is
  the only way to stop a compile that has started. Budget for re-instantiating
  the module afterwards, since terminate discards it.

`RenderResult.renderTimeMs` reports what a render cost, which is worth recording
even though it arrives too late to act on. It is the cheapest way to learn which
quills in your catalogue are the expensive ones.

## Concurrency

`Quillmark`, `Quill`, and `Document` are `Send + Sync`
(`crates/quillmark/tests/facade_surface.rs` pins this). One engine serves many
threads; a loaded `Quill` is portable declarative data and is shared, not
cloned per request.

`LiveSession` is a per-document handle. Do not share one across threads — open
one per document, per editing session.

## What a long-lived process accumulates

**The Typst memo cache is process-global and self-evicting.** After each compile
the Typst backend calls `comemo::evict` with a max age of 10, which is what
keeps an editing loop (one compile per keystroke) from growing without bound.

Two consequences worth planning for:

- The cache is **shared across every session in the process**. Under concurrent
  renders, one session's compiles age out another's entries, so reuse degrades
  as concurrency rises. This costs time, never correctness: comemo entries are
  pure functions of their input.
- Steady-state memory is a function of concurrency and document size, not of
  uptime. A process that has served a million renders holds no more than one
  that has served ten of the same shape.

**Quills are cached by canonical ref and never invalidated**, because a
canonical ref is immutable content (see
[Versioning](../quills/versioning.md)). Editing a quill in place at the same
version will not be picked up by a process that already loaded it — bump the
version, or restart.

## Isolation

- **No network.** Quillmark never downloads a Typst package. `QuillWorld` loads
  packages only from `{quill}/packages/` in the quill's in-memory tree, so a
  plate's `#import` resolves against what the quill ships or fails.
- **No ambient filesystem.** A plate reads through the same in-memory tree.
- **Fonts** come from `{quill}/assets/fonts/*` and `{quill}/packages/**`, which
  take priority over the three faces embedded in the binary (Figtree regular,
  bold, italic). A quill that names a font it does not ship gets the fallback,
  not an error — check rendered output when adding a font, since this failure is
  silent by design.

A quill is trusted input and a document is not;
[SECURITY.md](https://github.com/borb-sh/quillmark/blob/main/SECURITY.md) draws
that line and says what follows from it.

## Failure modes

Nothing in the workspace catches unwind. A panic is therefore terminal on every
surface: it aborts the CLI and the Python extension, and leaves the WASM module
poisoned rather than merely erroring. Panics are treated as bugs and the parsing
surfaces are fuzzed against them (`crates/fuzz`) — report one.

Ordinary failures are not panics. Every documented error path returns a
`RenderError` or a parse diagnostic carrying a stable `code`; route on the code,
not on the message. [Error Handling](error-handling.md) has the full contract.
