# MD Parsing / Typst Conversion Sanity Check — Known Issues & Recommendations

This document documents issues discovered while reviewing `crates/backends/typst/src/convert.rs` and running the `quillmark-typst` test suite. It includes reproduction cases, severity/impact, suggested fixes, and tests to add.

## Summary

- Most of the conversion logic is implemented and well-tested. All `quillmark-typst` tests currently pass (110/110).
- We identified a handful of potential pitfalls and edge cases that either are intentionally unimplemented or should be fixed to avoid accidental conversions and maintain correctness.

---

## Findings: Issues, Rationale, and Reproductions

1) Multi-backtick code spans not supported by preprocess_guillemets (Severity: High)

- Why: `preprocess_guillemets()` tracks inline code with a boolean toggled each time it sees a backtick (`'`), which fails for code spans that use multi-backtick delimiters per CommonMark (for example, ```` `` code with ` inside `` ````).
- Repro:
  - Input: ```` ` `` <<text>> ` ````  (e.g., two backticks to wrap code containing a backtick)
  - Expected: Preprocessor should not convert `<<text>>` inside this inline code span.
  - Current: Since we toggle on every single backtick, double-backtick spans could be incorrectly interpreted and converted.
- Suggested fix: When encountering backticks, count the number of consecutive backticks and treat a code span as starting/ending with that exact count; ignore `<<` until the closing sequence is reached.
- Tests to add: Unit test verifying guillemet markers inside multi-backtick spans are not converted.

---

2) Indented code blocks (4+ spaces) not recognized by preprocessing (Severity: Medium)

- Why: The preprocessor only recognizes fenced code blocks (```) and inline backtick spans; CommonMark also includes indented code blocks with four spaces (or tabs) at line start.
- Repro:
  - Input: `    <<not converted>>` (line starts with 4 spaces)
  - Expected: No guillemet conversion inside an indented code block.
  - Current: The preprocessor will not detect this, so `<<...>>` may be converted inside what is intended to be a code block.
- Suggested fix: Consider line-based detection for leading 4+ spaces or a tab — treat those lines as code blocks and skip `<<` conversions. Alternatively, document this limitation and adjust the test suite accordingly.
- Tests to add: Unit test using indented code block with `<<...>>`, and ensuring conversion is skipped.

---

3) Code block fence types & widths (```` ``` `` and `~~~`) (Severity: Low/Medium)

- Why: The implementation checks for exactly three backticks (```) to toggle `in_code_block`. Fenced code block syntax can legally use 3+ backticks or 3+ tildes and may optionally include a language identifier (e.g., ```` ```rust ````). The preprocessor does the right thing for the case ` ```rust ` (it toggles on encountering exactly three backticks and then leaves the `rust` text as literal tokens), but it doesn't correctly handle fences of longer lengths.
- Repro:
  - Input: ` ```` <<text>> ```` ` (4 backticks) or `~~~ <<text>> ~~~` (tildes)
  - Expected: No guillemet conversion while inside fences of any length >= 3 and either backticks or tildes.
- Suggested fix: When we detect a fence, capture the fence char and the fence length (n backticks or tildes >= 3) and ignore content until the same fence char repeated n is encountered.
- Tests to add: Unit tests for fences of length > 3 and for `~~~` fences.

---

4) Byte vs char offset note (Severity: Informational)

- Observation: We compute ranges via `result.len()` (bytes) and Event ranges from pulldown-cmark are also byte offsets. This alignment makes sense as long as the bytes in the resulting preprocessed string match the string used by the parser. Avoid non-idempotent transformations that alter byte layout before parsing or change the mapping between original and preprocessed text.
- Recommendation: Keep testing with multibyte characters (UTF-8) to ensure range offset logic remains correct. Add a test case with non-ASCII characters preceding guillemet content to verify range correctness.

---

5) Images (Event::Image) not handled (Severity: Low)

- Observation: `push_typst` does not handle `Event::Image`, which means images renderers are dropped. Depending on intended output mapping, this can be an omission.
- Suggested fixes:
  - Minimal: Preserve alt text by producing e.g., `#image("url")[alt]` or output inline alt text if images are not supported in Typst pipeline.
  - Full: Add a structured image mapping to actual Typst `#image(...)` calls.
- Tests to add: Verify `![alt text](url)` behavior.

---

6) BlockQuote (`Tag::BlockQuote`) not handled (Severity: Low)

- Observation: Blockquote tags are ignored in `push_typst`, which means structural formatting is lost. While tests may not rely on blockquote rendering, it's a UI/UX concern.
- Suggested fix: Implement blockquote handling or document that blockquote semantics are not presented in the Typst output.
- Tests to add: `> quoted text` -> (expected Behavior: some typst representation) or maintain `> quoted text` verbatim.

---

7) Inline HTML dropped (Severity: Intentional)

- Observation: `Event::Html` and `Event::InlineHtml` events are ignored in the converter. This is standard and intentional in many converters, but document the behavior.
- Tests: Optionally add test that inline HTML is dropped or converted.

---

8) Division of escaping logic - string vs markup context

- Observation: Historically we duplicated `escape_markup` behavior to avoid escaping `//` for code that is inside `#link("...")`. We refactored to use `escape_string` for link URL inside `#link("...")`, which is semantically correct. That fixed a few failing tests. Keep tests to ensure `#link("https://example.com")[text]` puts the URL in `"https://..."` string form (no double-slash escaping).

---

9) Performance considerations: multiple guillemet ranges scanning

- Observation: `push_typst` checks all `guillemet_ranges` with `.iter().any(...)` for every event. If a file has a large number of small guillemet spans (adversarial), the check is O(events * ranges). The code sets a `MAX_GUILLEMET_LENGTH` bound for content but not a `MAX_GUILLEMET_COUNT`.
- Recommendation: If needed, add a limit on the number of guillemets or use a faster data structure (e.g., interval map) or keep the existing constraint and document it.

---

## Recommended Action Plan & PR Notes

1. High priority fixes
   - Implement code-span detection for N-backtick delimiters (multi backtick inline code detection inside `preprocess_guillemets`). Add test(s):
     - ```` input: `` <<text>> `` => output remains code block with no guillemet conversion
   - Implement indentation-based code block detection for preprocessor (4-space or tab-indented lines). Add tests for indented code block conversions.

2. Medium priority fixes
   - Support code fence lengths >= 3 and both `backtick` and `tilde` fences. Add tests for both.
   - Add tests for non-ASCII bytes before a guillemet to verify event ranges remain valid under UTF-8.

3. Low priority / to be discussed
   - Implement support for `Image` events (map to a `#image(...)` Typst call) or preserve alt text.
   - Add `BlockQuote` handling and tests.
   - Document `Event::Html` behavior and ensure that ignoring it is acceptable.

4. Add tests for all discussed cases and check for regressions.

---

## Tests to Add (Suggested Minimal Test Cases)

- `test_guillemet_in_indented_code_block`: verify `<<...>>` inside 4-space code block does not convert.
- `test_guillemet_in_multibacktick_code_span`: verify `<<...>>` inside `` `` code span remains unconverted.
- `test_guillemet_in_~~fence~~`: verify `<<...>>` in tildes fence with language string is skipped.
- `test_guillemet_utf8_byte_offsets`: test string with multi-byte characters before `<<...>>` to ensure ranges are correct.
- `test_image_alt_text_preserved`: verify image alt text rendering or preservation.
- `test_blockquote_handling`: Basic reproduction of `>` blockquote.

---

## Acceptance Criteria for Fixes

- All existing tests continue to pass.
- New tests cover the regressions described and pass.
- No performance backslide (consider adding a synthetic stress test measuring many small guillemet spans).

---

If you'd like, I can implement the high-priority fixes with tests; I recommend starting with robust inline code span detection (multi-backtick) and indented code block detection, then follow up with fence/delimiter improvements.
