# Operations

Quillmark bounds what it ingests and not what it renders. That asymmetry, plus what a long-lived process holds and what crosses threads, is what a service or an editor has to plan around.

## Input limits

Enforced at parse, on every surface, before a backend sees anything. Each raises a `parse::*` diagnostic rather than a panic, so all of them are recoverable and routable by `code` (see [Error Handling](error-handling.md)).

| Limit | Value | Constant | Raised as |
|---|---|---|---|
| Markdown input | 10 MiB | `MAX_INPUT_SIZE` | `parse::input_too_large` |
| One card-yaml block | 1 MiB | `MAX_YAML_SIZE` | `parse::input_too_large` |
| Cards per document | 1 000 | `MAX_CARD_COUNT` | `parse::input_too_large` |
| Fields per card-yaml block | 1 000 | `MAX_FIELD_COUNT` | `parse::input_too_large` |
| YAML nesting | 100 container levels | `MAX_YAML_DEPTH` | `parse::yaml_error_with_location` |

**All four size and count limits raise the same code**, differing only in the `max` arg, so naming which ceiling was hit means comparing `max` against the constants.

The depth budget is one shape at every ingestion boundary, reported in each lane's own vocabulary: parse hits `serde_saphyr::Budget` and surfaces a YAML error (`budget breached: Depth`), programmatic writes raise `edit::value_too_deep`, the bound door raises `conform::value_too_deep`, and each binding's converter enforces the same 100.

The constants are public (`quillmark_core::MAX_INPUT_SIZE` and siblings). Read them rather than copying the numbers, so a consumer that rejects early rejects on what the engine will.

## Render is not bounded

**There is no render budget.** No deadline, no cancellation, no fuel, no progress callback. `Quillmark::render` returns when the backend is done.

This is where the parse limits stop carrying: they bound the input, and nothing translates them into a bound on the work, so a document inside every ceiling above can hold a core indefinitely.

- **Body size is linear.** A document near the 10 MiB ceiling is tens of seconds of compile.
- **Plates are not.** A plate is a Typst program, and layout has superlinear shapes — a long run of forced breaks in one paragraph among them. Cost is a property of the quill at least as much as of the document.
- **Typst bounds iteration, not work.** An infinite loop is refused (`loop seems to be infinite`), as is unbounded recursion. A loop that terminates after enough expensive iterations is neither.

So the bound comes from the host:

- **Server.** Render on a thread or subprocess you can abandon, under your own timeout. `Quillmark` is `Sync`, so one engine backs the whole pool.
- **Browser.** `engine.render` and `session.render` are **synchronous** — they return a `RenderResult`, not a `Promise` — so a render on the main thread freezes the tab for its duration. Run the module in a Web Worker and `worker.terminate()` to abort: there is no in-band cancel, and terminate discards the module, so plan on re-instantiating it.

`RenderResult.renderTimeMs` reports one render's cost after the fact. It arrives too late to act on and is the only measurement of which quills are the expensive ones.

## Concurrency

`Quillmark`, `Quill`, and `Document` are `Send + Sync` (`crates/quillmark/tests/facade_surface.rs` pins this). One engine serves many threads, and a loaded `Quill` is portable declarative data, shared rather than cloned per request.

`LiveSession` is a per-document handle: open one per editing session rather than sharing one across threads.

## What a long-lived process accumulates

**The Typst memo cache is process-global and self-evicting.** The Typst backend calls `comemo::evict` with a max age of 10 after each compile, which is what keeps an editing loop (one compile per keystroke) from growing without bound. Two consequences:

- The cache is **shared across every session in the process**, so under concurrent renders one session's compiles age out another's entries and reuse degrades as concurrency rises. This costs time, never correctness: comemo entries are pure functions of their input.
- Steady-state memory is a function of concurrency and document size, not of uptime. A process that has served a million renders holds no more than one that has served ten of the same shape.

**Quills are cached by canonical ref and never invalidated**, because a canonical ref is immutable content (see [Versioning](../quills/versioning.md)). A process that already loaded a quill will not see it edited in place at the same version: bump the version, or restart.

## Isolation

- **No network.** Quillmark never downloads a Typst package. `QuillWorld` loads packages only from `{quill}/packages/` in the quill's in-memory tree, so a plate's `#import` resolves against what the quill ships or fails.
- **No ambient filesystem.** A plate reads through the same in-memory tree.
- **Fonts** come from `{quill}/assets/fonts/*` and `{quill}/packages/**`, which take priority over the three faces embedded in the binary (Figtree regular, bold, italic). A quill naming a font it does not ship gets the fallback rather than an error, so adding a font means checking rendered output.

A quill is trusted input and a document is not; [SECURITY.md](https://github.com/borb-sh/quillmark/blob/main/SECURITY.md) draws that line and says what follows from it.

## Failure modes

Nothing in the workspace catches unwind, so a panic is terminal on every surface: it aborts the CLI and the Python extension, and leaves the WASM module poisoned rather than erroring. Panics are bugs, the parsing surfaces are fuzzed against them (`crates/fuzz`), and one that survives is worth reporting.

Ordinary failures are not panics. Every documented error path returns a `RenderError` or a parse diagnostic carrying a stable `code`; route on the code, not the message. [Error Handling](error-handling.md) has the full contract.
