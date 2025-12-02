# Convert double chevrons (<< >>) to guillemets (« ») and strip inner formatting

## Background
The Markdown → Typst converter currently preserves inline formatting (bold, italic, links, underline) based on parser events and source ranges. For guillemet conversion we will simplify behavior: when `<< ... >>` is used, the converter will produce guillemets containing the plain textual content of the inner region, stripping inline markup such as emphasis, strong, links, images, and strikethrough. This makes conversion simpler and avoids needing to preserve parser ranges for inner rendering.

This document outlines the plan, which source files to change/configure and tests to add.

---

## Goal
Convert `<<...>>` → `«...»` while removing any inline formatting inside the chevrons so the guillemets always contain plain text. The conversion must not change behavior for code spans/blocks, raw HTML, or link destination literals.

---

## High-level approach
When a starting `<<` is detected in text (and not inside code/raw HTML), collect the parser events that belong to the inner region until a matching `>>` is found. Collapse the collected events into a plain-text string by removing formatting markers and extracting textual content. Wrap that plain text with guillemets and emit it. If no closing `>>` is found, emit the original content unchanged. Conversions must be skipped inside code spans/blocks and raw HTML, and link destination literals must not be modified.

Key details:
- Use the parser's offset iterator (already used in `convert.rs`) to map event ranges to the original source string when we need to get raw source slices.
- To find matches across input boundaries (e.g., `<<` and `>>` split across `Event::Text` boundaries) use the source string and event ranges for a deterministic and predictable scan.
- Parse and collect only textual content from events between `<<` and the matching `>>`. Specifically, ignore `Start`/`End` `Tag`s that correspond to inline formatting (Emphasis, Strong, Link, Strikethrough, Image), but collect `Event::Text`s and `Event::Code` content as plain text.
- Do NOT apply the conversion if the starting `<<` is inside a code span (`Event::Code`) or code block (`Tag::CodeBlock`) or html (`Event::Html`), or if the `<<` occurs in a link destination (`Tag::Link`'s `dest_url` field). In these cases the `<<` and `>>` should remain literal.
- Bound the buffer size and number of events processed while searching for `>>` to defend against malicious input (e.g., 64KB or several hundred events).
- Ensure the `>` characters of the `>>` are on the same line as the `<<` (optional but recommended by the acceptance criteria "Chevrons must be on the same line to match").

---

## Acceptance criteria
- `<<text>>` becomes `«text»`.
- Inline formatting inside chevrons is stripped: `<<**bold**>>` → `«bold»`, and `<<__under__>>` → `«under»`.
- Code spans and code blocks remain unchanged and are not processed for guillemet conversion (i.e., if `<<` is in code, skip conversion; if code exists inside `<<...>>`, the user accepted that inner formatting will be stripped unless the `<<` is inside code span/block—see considerations below).
- Link destination literals are not modified (i.e., `Tag::Link` dest_url is left as-is, and `<<...>>` inside a destination is not converted).
- Unmatched `<<` or unmatched `>>` remain literal and do not corrupt output.
- The collection/buffering performed to gather inner content is bounded and safe under malformed or malicious input.

---

## Examples
- `Hello, <<world>>.` → `Hello, «world».`
- `<<**bold** and _italic_>>` → `«bold and italic»`
- `` `<<code>>` `` → unchanged (still an inline code span)
- `[link](<<not-a-url>>)` → link destination unchanged
- `<<__under__>>` → `«under»`

---

## Implementation details for Quillmark
Below I outline specific edits and files to modify within Quillmark. Implementation will be done primarily in the Typst backend converter, plus docs and tests updates.

### Files to change
- `crates/backends/typst/src/convert.rs`
  - Primary location of the Markdown → Typst conversion logic in Quillmark.
  - Add `<<`/`>>` detection and buffering logic into the `push_typst()` function.
  - Keep the existing `escape_markup` and escaping behavior for typst special characters.
  - Add flags to track contexts that must disable conversion: `in_code_block`, `in_inline_code`, `in_html`, and `in_link_dest` (the last one needs to be determined via the `Tag::Link` start event; the key here is: when converting link destinations we detect the tag and will avoid handling chevrons specifically there).
  - Implement scanning for `<<` and `>>` using the provided `range` offsets into the original `source` parameter already passed to `push_typst()` for detecting text boundaries across events.
  - When a `<<` is found and its context allows conversion, collect the raw source region up to a matching `>>`, then create a temporaty `pulldown_cmark::Parser` for that inner content and iterate its events while collecting plain text. For each `Event::Text` and `Event::Code` event inside the inner parser: collect its textual content. For inline `Event::Html`, treat as raw text or skip depending on how we want to handle it: but a safe default (and consistent with requirement: do not alter raw HTML) is: if inner content contains any `Event::Html` or `Event::CodeBlock` or other non-textual constructs that would change behavior, DO NOT convert and treat `<<` as literal.
  - Alternatively, we can instead simply extract the substring between the `<<` and the matching `>>` in the source, and strip a very small set of markup-bytes (like `*`, `_`, `~`, square brackets) using a simple state machine; however this can be error prone with nested syntax. Using a mini parser on the inner content gives a robust approach.
  - Ensure we limit the scanning buffer length and the number of events used to parse the inner region to mitigate resource exhaustion.
  - Add logging or debug-only feature flags to test the logic; but keep production code compact.

- `crates/backends/typst/src/convert.rs` (unit tests section):
  - Add a new suite of tests to verify guillemet conversion:
    - Simple case: `<<text>>` → `«text»`.
    - Stripping: `<<**bold** _italic_>>` → `«bold italic»`.
    - Inline code inside chevrons: `<<`code`>>` → behavior must be decided; by default, strip formatting and include code content: `«code»`.
    - Code span and code block skip: `` `<<code>>` `` must remain an inline code span with content `<<code>>`.
    - Link destination skip: `[link](<<not-a-url>>)` remains unchanged.
    - Malformed cases: `<<unclosed...` remains literal; `no << open >> but stray >>` remains literal.
    - Maximum size: very large inner content should be aborted with fallback to literal or truncated (per the acceptance criteria: buffer is bounded and safe). Add tests that feed a large content and ensure no memory blowup and bounded behavior.

- `docs/` or `docs/guides/quill-markdown.md` and `designs/CONVERT.md` (or the appropriate spec docs):
  - Document the new guillemets conversion behavior and the acceptance criteria.
  - Add an item to the converter limitations and behavior section describing that inline formatting inside `<<`/`>>` is stripped and that this is intentionally simplified.

- `quillmark-fixtures/` or `crates/fixtures/` (the test fixtures):
  - Add fixture examples for guillemets to the fixtures repo to verify CLI/Integration behavior.

- `bindings/*` (Optional):
  - If the `bindings` or `wasm` backends re-implement conversion logic independently or use the same logic, apply the same changes there as well.

### Data flow & the parsing plan (Technical)
1. Start reading events from `push_typst` iterator. Maintain flags:
   - `in_code_block: bool` (set when `Event::Start(Tag::CodeBlock(_))` and cleared at End)
   - `in_html_block: bool` (set on HTML tags)
   - `in_link_dest: bool` (set when processing a `Tag::Link` destination; the current code directly reads `dest_url` at the `Start(Tag)` and writes the link; when the destination contains `<<`/`>>`, intentionally skip conversion)
   - `in_inline_code_event: bool` — inline code is represented by `Event::Code` events; since they are atomic, they themselves will be skipped from chevron matching.

2. On `Event::Text(text)` event: if `in_code_block || in_html_block || in_link_dest` or text does not contain `<<` then treat as normal.

3. If `text` contains `<<` and it is allowed by the flags, start the extraction routine:
   a. Find the index in the underlying `source` by `range.start` + index-of-`<<` in `text`. Then scan forward in *source text* to find `>>` on the same line; keep the scanning capped with `MAX_GUILLEMET_BUFFER_SIZE` (e.g., 32KB or 64KB). The scanning must also be aware of the character boundary and not break utf-8 sequences.
   b. If no `>>` found within buffer limit or in the remainder of the line, bail and treat `<<` as literal; append the `<<` to output as-is (escaped as per current rules) and continue scanning.
   c. If we find a matching `>>`, extract the substring of `source` between the `<<` and `>>` (exclusive).
   d. Parse that substring with `pulldown_cmark::Parser::new()` but configure options to avoid interpreting code blocks; iterate the parser events and collect `Text` and `Code` events only. For `Event::Code`, *strip* the code formatting (remove backticks) and include the inner code content (plain text). For `Event::Html` or other non-text events, treat as non-convertible and skip the guillemet processing (emit literal), or optionally, include raw inner textas literal depending on the team preference. The acceptance criteria favors NOT changing raw HTML, so if `Html` occurs inside the chevrons, we should skip conversion.
   e. The result of 3.d is the `plain_text` to wrap in guillemets.
   f. Emit `«`, the `escape_markup(&plain_text)`, and `»` to `output`.
   g. Advance the main parser iterator to the event that contains the `>>` (use offsets / ranges to compute how many parser events correspond to the inside region) — careful: because we used `into_offset_iter` we can compute the number of events to skip with repeated `iter.next()` until we cross the `>>` index; we must do this carefully to ensure we don't skip other events; treat `iter.peek()` and adjust accordingly.

4. Continue with main parser loop.

### Buffering and safety
- Introduce a constant `MAX_GUILLEMET_BUFFER: usize = 64 * 1024` (64 KiB or configurable). If the inner region between `<<` and `>>` exceeds that, consider it malformed or malicious and decline to convert (emit original content) and resume normal processing.
- Add a maximum number-of-events limit to parse inner content (e.g., 512 events). If exceeded, decline to convert.

### Additional low-level implementation notes

- Suggested constants and names:
  - `const MAX_GUILLEMET_BUFFER: usize = 64 * 1024;`
  - `const MAX_GUILLEMET_EVENTS: usize = 512;`
- When we successfully parse a guillemet region, call `escape_markup()` for the final `plain_text` before writing `«` and `»` into the output.
- When scanning source offsets for `>>`, check that both `<<` and `>>` are on the same line (i.e., there is no `\n` between start and end offsets) if the policy you prefer requires same-line matching.
- The converter must not modify tag `Tag::Link` destination strings; in `Start(Tag::Link { dest_url, .. })` the code currently writes `dest_url` using `escape_markup`, so skip conversions there.
- If the inner parser for the content between `<<` and `>>` yields `Event::Html` or `Tag::CodeBlock`, prefer to treat the entire `<<...>>` as literal (no guillemet conversion), because the requirement states we must not change behavior for raw HTML and code blocks.

### Concrete test names to add

- `test_guillemet_basic`
- `test_guillemet_strips_inline_formatting`
- `test_guillemet_contains_code_inner_content`
- `test_guillemet_skip_if_inside_code_span`
- `test_guillemet_skip_in_link_destination`
- `test_guillemet_unmatched_left_is_literal`
- `test_guillemet_unmatched_right_is_literal`
- `test_guillemet_same_line_requirement`
- `test_guillemet_buffer_limit`

Add these tests in `crates/backends/typst/src/convert.rs` near the existing `mark_to_typst` tests.

### Optional: feature flag

- If you prefer a soft rollout, gate the feature behind a cargo feature flag, for example `feat=guillemet-typst`. This allows tests and early adopters to try the feature before merging it into default behavior.


### Performance considerations
- Since most `<<` occurrences will be short, the overhead will be minimal.
- Use `String::with_capacity` with estimated sizes when collecting content.
- Avoid allocating temporary buffers repeatedly; use a small stack-allocated buffer or reuse a shared buffer where possible.

### Testing plan
- Unit tests in `crates/backends/typst/src/convert.rs`:
  - For all examples listed earlier plus edge cases (nested and malformed), confirm the expected output.
  - Add `concurrency` and `fuzz` tests if present and relevant.
  - Check that `<<` inside backticks or code block remains unchanged.
  - Check that `<<` within the link destination `Tag::Link` remains unchanged.

- Integration tests:
  - Add fixtures in `quillmark-fixtures/` that include guillemet usage; use pre-existing test harness to verify it converts expectedly.

- Benchmark tests:
  - Check the performance and verify no unbounded memory usage under malformed input.

---

## Example pseudo-code for `crates/backends/typst/src/convert.rs`
(This is a high-level sketch; the actual Rust code will need careful implementation details and tests.)

- Add let `const MAX_GUILLEMET_BUFFER: usize = 64 * 1024;`
- Inside the main `while let Some((event, range)) = iter.next()` loop:
  - Track `in_code_block`, `in_html`, `in_link_dest` booleans per events.
  - When `Event::Text(text)` and `text.contains("<<")` and not `in_code_block` and not `in_html` and not `in_link_dest`:
    - For each `pos` in occurrences of `<<` in text:
      - Let `start_index = range.start + pos`.
      - Attempt to find `end_index` within the source by scanning until `>>` is found on the same line and within the `MAX_GUILLEMET_BUFFER`.
      - If no `end_index` found, treat `<<` as literal; continue scanning.
      - Else: take `inner_slice = &source[start_index + 2..end_index]`.
      - Use `pulldown_cmark::Parser::new()` on `inner_slice` (and maybe `Options::empty()` or same options) to iterate events and collect textual content only.
      - If this inner parse contains `Event::Html` or other `Tag`s that we treat as non-convertible (e.g., `Tag::Image`?), treat it as non-convertible and emit literal; else, `plain_text` is the concatenation of `Event::Text` and `Event::Code` strings.
      - Escape typst markup in `plain_text` with `escape_markup` and emit `«escaped»`.
      - Advance the main parser `iter` until `range.end >= end_index` (skip events until we pass the `>>`).
      - Continue scanning after the `>>` for additional `<<` occurrences in the rest of the text.

---

## Edge cases and risk decisions
- If `<<` or `>>` is cut in the middle of parser events such that `<<` starts in `Event::Text` A and `>>` ends in `Event::Text` B, using `range` offsets allows us to compute positions to scan directly in the original source string.
- If the `inner_slice` contains newlines, we can allow that as needed, but the acceptance criteria suggested restricting `<<`/`>>` to the same line. This can be enforced by checking if the inner content contains `\n`; if there is `\n` then decline to convert and emit literal.
- For nested `<<` inside an inner region, either use the nearest matching `>>` (non-nested matching) or reject nested chevrons; prefer nearest `>>` behavior.
- For cases where inner payload contains `<<` or `>>` as plain text but is still desired to be converted, the code should follow a deterministic match behavior and document that in the markdown doc.

---

## CI / Tests / Docs
- Add unit tests to `crates/backends/typst/src/convert.rs` next to the `push_typst()` tests.
- Add fixture examples to `quillmark-fixtures/`.
- Update `docs/guides/` and `designs/CONVERT.md` with the new behavior.
- Add a short section to `docs/CHANGELOG.md` or `RELEASE.md` describing the new guillemet behavior.

---

## Rollout plan
- Implement the converter changes behind a feature gate (`typst-guillemet` or similar) if you want to enable gradual testing without affecting users.
- Add the unit and integration test coverage.
- Run `cargo test` and CI to validate.

---

## Next steps for the implementation PR
- Implement the change in `crates/backends/typst/src/convert.rs` using the parsing approach described.
- Add unit tests in `crates/backends/typst/src/convert.rs::tests`.
- Add fixture files and update `docs/`.
- Run and validate `cargo test` and add `md` examples to docs and fixtures.

---

## Acceptance Decision
When all tests pass, and code paths do not regress preprocessing of other typst features (like link conversion and code blocks), we can merge.

---

If you want, I can also implement the code change directly in `crates/backends/typst/src/convert.rs` and add tests now; say the word and I'll work on it next.