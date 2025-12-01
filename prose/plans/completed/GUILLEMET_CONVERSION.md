# Guillemet Conversion Implementation Plan

**Status**: Ready for Implementation  
**Design Reference**: See prose/designs/TYPST_GUILLEMET_CONVERSION.md  
**Target**: `crates/backends/typst/src/convert.rs`  

## Overview

This plan implements guillemet conversion (`<<text>>` → `«text»`) in the Typst backend's markdown converter, with automatic stripping of inner inline formatting.

## Prerequisites

- Existing design document must be reviewed and approved
- Understanding of pulldown-cmark event-based parsing
- Familiarity with convert.rs architecture

## Implementation Phases

### Phase 1: Core Conversion Logic

**Location**: `crates/backends/typst/src/convert.rs`

**Changes**:
- Add constants for buffer limits and safety thresholds
- Add context tracking flags for code blocks, HTML, and link destinations
- Implement chevron detection in text events using source ranges
- Build buffering system to collect content between chevron pairs
- Extract plain text from buffered content by stripping formatting events
- Emit guillemets with escaped plain text content

**Safety Requirements**:
- Enforce maximum buffer size for malicious input protection
- Enforce maximum event count for inner content parsing
- Validate UTF-8 boundaries during source scanning
- Ensure same-line matching requirement

**Context Awareness**:
- Skip conversion inside code spans and code blocks
- Skip conversion inside raw HTML
- Skip conversion in link destination URLs
- Track nesting to handle overlapping structures

### Phase 2: Test Coverage

**Location**: `crates/backends/typst/src/convert.rs::tests`

**Test Cases**:
- Basic conversion: simple text in chevrons
- Formatting strip: bold, italic, underline, strikethrough
- Code handling: inline code inside chevrons, chevrons inside code
- Link handling: chevrons in link text vs link destinations
- Edge cases: unmatched chevrons, nested chevrons, multiline
- Safety: buffer limit enforcement, event count limits
- Malformed input: incomplete pairs, edge-of-buffer boundaries

### Phase 3: Documentation

**Files to Update**:
- `crates/backends/typst/docs/designs/CONVERT.md` - Add guillemet behavior section
- Integration test fixtures with guillemet examples
- Changelog entry for the new feature

**Documentation Requirements**:
- Explain conversion behavior and rationale
- Document stripping policy for inner formatting
- List contexts where conversion is disabled
- Provide examples of expected input/output

## Implementation Approach

### Event Processing Strategy

The converter already uses `into_offset_iter()` to access source ranges. Leverage this to:
- Detect `<<` sequences in text events using range offsets
- Scan forward in source string to find matching `>>`
- Extract substring and parse separately to collect plain text
- Skip appropriate number of main parser events

### Plain Text Extraction

When inner content is collected:
- Create temporary pulldown-cmark parser for the substring
- Iterate events collecting only Text and Code content
- Discard Start/End tags for formatting (Emphasis, Strong, Link, etc.)
- If HTML or non-textual constructs appear, abort conversion

### State Management

Add flags to existing state tracking:
- `in_code_block` - already tracked via Tag depth, extend coverage
- `in_link_dest` - track during Tag::Link processing
- `in_html` - track HTML event contexts

### Escaping Integration

Reuse existing `escape_markup()` function for final plain text before wrapping in guillemets.

## Testing Strategy

### Unit Tests

Add to existing test module structure following patterns like `test_underline_*`:
- `test_guillemet_basic` - Simple case
- `test_guillemet_strips_formatting` - Multiple inline formats
- `test_guillemet_in_code_span` - No conversion
- `test_guillemet_in_link_dest` - No conversion  
- `test_guillemet_unmatched` - Literal output
- `test_guillemet_buffer_limit` - Safety bounds

### Integration Tests

Add fixtures demonstrating:
- Guillemets in real document contexts
- Interaction with lists, headings, quotes
- Round-trip consistency

## Rollout Considerations

### Optional: Feature Flag

Consider gating behind cargo feature if soft rollout desired:
- Feature name: `guillemet-conversion` or similar
- Default: enabled
- Allows testing before wide deployment

### Performance Impact

Expected minimal overhead:
- Most documents don't contain `<<` sequences
- Early detection via `contains()` check avoids unnecessary work
- Buffer limits prevent resource exhaustion

### Backward Compatibility

This is an additive change:
- Existing markdown without `<<` unaffected
- Behavior is deterministic and documented
- No breaking changes to API

## Success Criteria

- All unit tests pass
- Integration tests demonstrate expected behavior
- No performance regression on typical documents
- Safety bounds enforced under malicious input
- Documentation complete and clear

## Open Questions

None - design decisions made in design document.

## References

- Design document: `prose/designs/TYPST_GUILLEMET_CONVERSION.md`
- Converter spec: `crates/backends/typst/docs/designs/CONVERT.md`
- pulldown-cmark docs: https://docs.rs/pulldown-cmark
