# Typst Guillemet Conversion

**Status**: Deprecated (conversion disabled)
**Component**: Typst Backend Converter
**Related**: [CONVERT.md](../../crates/backends/typst/docs/designs/CONVERT.md)

## Overview

~~The Typst backend markdown converter transforms double chevrons (`<<text>>`) into guillemets (`«text»`) while stripping inner inline formatting.~~

**Update**: Double chevron delimiters (`<<` and `>>`) now pass through unchanged. The guillemet conversion feature has been disabled. Chevrons are detected and preserved to prevent them from being interpreted as HTML tokens.

## Current Behavior

Input markdown containing `<<text>>` passes through as `<<text>>` without conversion. The detection logic ensures chevrons are not accidentally picked up as HTML tokens by downstream processors.

## Legacy Behavior (No Longer Applied)

The following behavior was previously implemented but is no longer active:

### Basic Conversion (Disabled)

Input markdown containing `<<text>>` was converted to Typst output containing `«text»`.

### Formatting Stripping (Disabled)

All inline formatting inside chevrons was removed, leaving only plain text.

## Rationale for Change

The automatic conversion of `<<>>` to guillemets was removed because:

1. Users may want to preserve the original `<<>>` syntax for their own purposes
2. The conversion was an implicit transformation that could be surprising
3. Users who need guillemets can use them directly in their content

## Available Functions

The following functions remain available in `quillmark_core::guillemet` for users who need explicit guillemet conversion:

- `preprocess_guillemets(text)` - Converts `<<text>>` to `«text»` in simple text
- `preprocess_markdown_guillemets(markdown)` - Same conversion but skips code blocks/spans
- `strip_chevrons(text)` - Strips chevrons, extracting inner content: `<<text>>` → `text`

These functions can be used manually if guillemet conversion is desired.

## Related Documentation

- Typst Conversion Spec: `crates/backends/typst/docs/designs/CONVERT.md`
- Markdown Parsing: [PARSE.md](PARSE.md)
- Error Handling: [ERROR.md](ERROR.md)
