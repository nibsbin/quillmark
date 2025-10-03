# Markdown to Typst Conversion

This module transforms CommonMark markdown into Typst markup language.

## Key Functions

- [`mark_to_typst`] - Primary conversion function for Markdown to Typst
- [`escape_markup`] - Escapes text for safe use in Typst markup context
- [`escape_string`] - Escapes text for embedding in Typst string literals

## Quick Example

```rust
use quillmark_typst::convert::mark_to_typst;

let markdown = "This is **bold** and _italic_.";
let typst = mark_to_typst(markdown);
// Output: "This is *bold* and _italic_.\n\n"
```

## Detailed Documentation

For comprehensive conversion details including:
- Character escaping strategies
- CommonMark feature coverage  
- Event-based conversion flow
- Implementation notes

See **[CONVERT.md](CONVERT.md)** for the complete specification.
