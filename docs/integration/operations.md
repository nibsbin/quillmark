# Operations

Quillmark bounds what it ingests and not what it renders. That asymmetry, plus what a long-lived process holds and what crosses threads, is what a service or an editor has to plan around.

## Input limits

The values are the spec's: [Markdown Specification §8](../reference/markdown-spec.md#8-limits) lists all five and is the one place they are written down. They are enforced at parse, on every surface, before a backend sees anything, and each raises a diagnostic rather than a panic, so all of them are recoverable and routable by `code` (see [Error Handling](error-handling.md)).

What the spec does not say is which code you get, and the mapping is lopsided:

| Exceeded | Raised as |
|---|---|
| Document size, YAML payload size, card count, field count | `parse::input_too_large` |
| YAML nesting depth | `parse::yaml_error_with_location` |

**The four size and count ceilings share one code**, differing only in the `max` arg, so naming which one was hit means comparing `max` against the constants.

Depth is one budget at every ingestion boundary, reported in each lane's own vocabulary: parse surfaces it as a YAML error, programmatic writes raise `edit::value_too_deep`, the bound door raises `conform::value_too_deep`, and each binding's converter enforces the same limit.

The constants are public — `quillmark_core::error::{MAX_INPUT_SIZE, MAX_YAML_SIZE, MAX_CARD_COUNT, MAX_FIELD_COUNT}` and `quillmark_core::document::limits::MAX_YAML_DEPTH`. Read them rather than copying the numbers, so a consumer that rejects early rejects on what the engine will.

## Render is not bounded

**There is no render budget.** No deadline, no cancellation, no fuel, no progress callback. `Quillmark::render` returns when the backend is done.

This is where the parse limits stop carrying: they bound the input, and nothing translates them into a bound on the work, so a document inside every ceiling above can hold a core indefinitely.

Cost is a property of the quill at least as much as of the document: a plate is a Typst program, and layout has superlinear shapes. Typst refuses an infinite loop and unbounded recursion, which bounds iteration rather than work — a loop that terminates after enough expensive iterations is refused by neither.

So the bound comes from the host:

- **Server.** Render on a thread or subprocess you can abandon, under your own timeout. `Quillmark` is `Sync`, so one engine backs the whole pool.
- **Browser.** `Engine.render` and `Engine.open` return promises, but the compile inside them is synchronous: awaiting does not yield while it runs, and `LiveSession.render` does not even return a promise. Either way the thread is blocked for the render's full duration, so a render on the main thread freezes the tab. Run the module in a Web Worker and `worker.terminate()` to abort — there is no in-band cancel, and terminate discards the module, so plan on re-instantiating it.

`RenderResult.renderTimeMs` reports one render's cost after the fact. It arrives too late to act on and is the only measurement of which quills are the expensive ones.

## Concurrency

`Quillmark`, `Quill`, and `Document` are `Send + Sync` (`crates/quillmark/tests/facade_surface.rs` pins this). One engine serves many threads, and a loaded `Quill` is portable declarative data, shared rather than cloned per request.

`LiveSession` is a per-document handle: open one per editing session rather than sharing one across threads.

## What a long-lived process accumulates

**The Typst memo cache is process-global and self-evicting.** The Typst backend calls `comemo::evict` with a max age of 10 after each compile, which is what keeps an editing loop (one compile per keystroke) from growing without bound. Two consequences:

- The cache is **shared across every session in the process**, so under concurrent renders one session's compiles age out another's entries and reuse degrades as concurrency rises. This costs time, never correctness: comemo entries are pure functions of their input.
- Steady-state memory is a function of concurrency and document size, not of uptime. A process that has served a million renders holds no more than one that has served ten of the same shape.

The engine caches no quills: `Quillmark::render` takes a `&Quill` the caller owns. Where a cache exists it is the caller's, and a canonical ref is immutable content ([Versioning](../quills/versioning.md)), so keying on the ref and never invalidating is sound — with the corollary that editing a quill in place at the same version is invisible to anything holding it. Bump the version.

## Isolation

- **No network.** Quillmark never downloads a Typst package. `QuillWorld` loads packages only from `{quill}/packages/` in the quill's in-memory tree, so a plate's `#import` resolves against what the quill ships or fails.
- **No ambient filesystem.** A plate reads through the same in-memory tree.
- **Fonts** come from `{quill}/assets/fonts/*` and `{quill}/packages/**`, which take priority over the three faces embedded in the binary (Figtree regular, bold, italic). A quill naming a font it does not ship gets the fallback rather than an error, so adding a font means checking rendered output.

A quill is trusted input and a document is not; [SECURITY.md](https://github.com/borb-sh/quillmark/blob/main/SECURITY.md) draws that line and says what follows from it.

## Failure modes

Nothing in the workspace catches unwind, so a panic is terminal on every surface: it aborts the CLI and the Python extension, and leaves the WASM module poisoned rather than erroring. Panics are bugs, the parsing surfaces are fuzzed against them (`crates/fuzz`), and one that survives is worth reporting.

Ordinary failures are not panics. Every documented error path returns a `RenderError` or a parse diagnostic carrying a stable `code`; route on the code, not the message. [Error Handling](error-handling.md) has the full contract.
