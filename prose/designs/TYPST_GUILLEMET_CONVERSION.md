# Typst Guillemet Conversion

**Status**: Removed
**Component**: Typst Backend Converter

## Overview

The guillemet conversion feature has been removed. Double chevron delimiters (`<<` and `>>`) now pass through unchanged without any transformation.

## Current Behavior

- Input markdown containing `<<text>>` passes through as `<<text>>` without conversion
- No special preprocessing is applied to chevrons
- Chevrons in both body and metadata fields are preserved as-is

## Rationale for Removal

The automatic conversion of `<<>>` to guillemets was removed because:

1. Users may want to preserve the original `<<>>` syntax for their own purposes
2. The conversion was an implicit transformation that could be surprising
3. Users who need guillemets can use them directly in their content (`«` and `»`)
4. Placeholders are not allowed in metadata fields, so the stripping logic was unnecessary

## Migration

If you previously relied on `<<text>>` being converted to `«text»`:

- Use the guillemet characters directly: `«text»`
- Or implement custom preprocessing in your workflow
