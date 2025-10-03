# Contributing to quillmark-typst

## Documentation Structure

This crate uses a hybrid documentation strategy to balance code iteration speed with comprehensive documentation:

### Inline Documentation (Minimal)

- **Public items**: Each public function/struct has a 1-2 line summary using `///`
- **Deep dive links**: Inline docs point to detailed module or external documentation
- **Examples**: Simple usage examples in inline docs where helpful

### External Documentation (Comprehensive)

Located in the `docs/` directory:

- **`lib.md`**: Module-level overview included via `#[doc = include_str!()]` 
- **`convert.md`**: Conversion module documentation
- **`compile.md`**: Compilation module documentation
- **`API.md`**: Complete API reference with detailed examples
- **`CONVERT.md`**: Full Markdown to Typst conversion specification

### Testing Documentation

- Run `cargo doc --no-deps` to build documentation
- Run `cargo test` to execute doctests from both inline and included Markdown
- Keep examples green to prevent documentation drift

### Discoverability

- Inline one-liners ensure IDE hover/completion shows useful information
- Module docs provide quick examples and links to deeper documentation
- External docs provide comprehensive details for complex topics

## Adding Documentation

When adding new public APIs:

1. Add a 1-2 line summary to the item with `///`
1. Create or update a `{module}.md` file next to the script/module
1. Link `{module}.md` link with `#[doc = include_str!("{module}.md")]`
1. Test with `cargo doc --no-deps` and `cargo test --doc`
