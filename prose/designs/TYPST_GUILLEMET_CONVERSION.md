# Typst Guillemet Conversion

**Status**: Implemented
**Component**: Typst Backend Converter  
**Related**: [CONVERT.md](../../crates/backends/typst/docs/designs/CONVERT.md)

## Overview

The Typst backend markdown converter transforms double chevrons (`<<text>>`) into guillemets (`«text»`) while stripping inner inline formatting. This provides a simple, predictable way to add French-style quotation marks to Typst output.

## Rationale

### Why Guillemets?

Typst doesn't have native markdown syntax for guillemets. Users authoring content requiring French, German, or other European typographic conventions need a straightforward mechanism. The `<<` and `>>` symbols are visually suggestive of guillemet appearance and rarely used in technical markdown.

### Why Strip Formatting?

Stripping inner formatting simplifies implementation and creates predictable behavior:
- Avoids complex parsing of nested markdown within chevrons
- Eliminates ambiguity about how formatting interacts with guillemets
- Reduces edge cases and potential for malformed output
- Makes the feature easy to document and understand

## Behavior Specification

### Basic Conversion

Input markdown containing `<<text>>` converts to Typst output containing `«text»`.

**Example**:
```markdown
She said <<Hello, world>>.
```

**Output**:
```typst
She said «Hello, world».
```

### Formatting Stripping

All inline formatting inside chevrons is removed, leaving only plain text.

**Supported Strip Targets**:
- Bold/strong (`**text**` or `__text__`)
- Italic/emphasis (`*text*` or `_text_`)
- Links (`[text](url)`) - link text extracted, URL discarded
- Strikethrough (`~~text~~`)
- Images (`![alt](url)`) - alt text extracted, URL discarded
- Inline code (`` `text` ``) - backticks removed, text preserved

**Example**:
```markdown
Quote: <<**bold** and _italic_ text>>
```

**Output**:
```typst
Quote: «bold and italic text»
```

### Disabled Contexts

Conversion does NOT occur in these contexts:

**Code Spans and Blocks**:
- Inline code: `` `<<text>>` `` remains unchanged
- Code blocks with `<<` content remain unchanged

**Raw HTML**:
- HTML containing `<<` is not processed

**Link Destinations**:
- URLs containing `<<` are not modified: `[link](<<not-url>>)` keeps destination literal

**Rationale**: These contexts require literal text preservation for correct rendering.

### Same-Line Requirement

Opening `<<` and closing `>>` must appear on the same line to match. Newlines between chevrons cause literal output.

**Example**:
```markdown
<<text on
different line>>
```

**Output**: Literal `<<text on different line>>` (unchanged)

**Rationale**: Multi-line matching complicates parsing and could capture unintended content spans.

### Unmatched Chevrons

Unmatched `<<` or `>>` output as literals.

**Examples**:
- `<<unmatched` → literal `<<unmatched`
- `unmatched>>` → literal `unmatched>>`
- `<<first>> <<second` → `«first»` then literal `<<second`

## Edge Cases

### Nested Chevrons

Inner `<<` or `>>` within an outer pair use nearest-match logic:

```markdown
<<outer <<inner>> text>>
```

First `<<` matches first `>>`, producing: `«outer` followed by literal `<<inner>>` then `text>>`

**Rationale**: Simple left-to-right matching is deterministic and doesn't require backtracking.

### Mixed Content

If inner content contains HTML events or non-textual constructs during parsing, conversion is aborted and chevrons output as literals.

**Rationale**: Preserves safety when complex markdown structure appears between chevrons.

### Buffer Limits

Maximum buffer size (64 KiB) and event count (512 events) enforced when scanning for matching chevrons.

**Behavior**: If limits exceeded, treat as unmatched and output literals.

**Rationale**: Protects against malicious or malformed input causing resource exhaustion.

## Design Constraints

### No Configuration

The feature has no configuration options. Behavior is fixed and consistent.

**Rationale**: Avoids complexity. Users wanting different behavior can post-process Typst output or use Typst's native quotation mechanisms.

### Event-Based Implementation

Implementation uses pulldown-cmark's event stream with source offsets.

**Rationale**: Consistent with existing converter architecture. Enables accurate source location tracking.

### Typst Character Escaping

Plain text extracted from chevrons goes through standard `escape_markup()` before output.

**Rationale**: Maintains safety by preventing Typst markup injection through chevron content.

## Future Considerations

### Potential Enhancements

These are NOT part of the current design but noted for future discussion:

- Configuration to disable the feature
- Alternative delimiter syntax
- Preservation of select formatting types
- Multi-line matching with explicit opt-in
- Nested guillemet support with proper pairing

### Not Planned

- Automatic language-aware quote selection
- Smart quote conversion for regular quotes
- Guillemet styling or appearance customization (belongs in Typst template)

## Success Metrics

The design is successful if:
- Common use cases work intuitively
- Edge cases behave predictably
- Implementation is maintainable
- Performance impact is negligible
- Security boundaries are respected

## Alternatives Considered

### Alternative 1: Preserve Formatting

**Rejected**: Significantly increases complexity. Requires careful handling of Typst's formatting syntax within string literals. Creates unclear precedence for conflicting format types.

### Alternative 2: Different Delimiter Syntax

**Rejected**: `<<`/`>>` are the most intuitive visual match for guillemets. Other options (e.g., `{{`/`}}`, `[[`/`]]`) conflict with existing markdown or Typst syntax.

### Alternative 3: Typst-Native Solution

**Rejected**: Requires users to write Typst markup directly or use custom Typst functions. Breaks markdown-centric authoring flow.

## Implementation Notes

See [GUILLEMET_CONVERSION.md](../plans/GUILLEMET_CONVERSION.md) for detailed implementation plan.

## Related Documentation

- Typst Conversion Spec: `crates/backends/typst/docs/designs/CONVERT.md`
- Markdown Parsing: [PARSE.md](PARSE.md)
- Error Handling: [ERROR.md](ERROR.md)
