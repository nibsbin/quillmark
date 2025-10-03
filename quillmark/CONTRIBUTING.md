# Contributing to quillmark

## Documentation Structure

This crate uses a hybrid documentation strategy to balance code iteration speed with comprehensive documentation:

### Inline Documentation (Minimal)

- **Public items**: Each public function/struct has a 1-2 line summary using `///`
- **Deep dive links**: Inline docs point to detailed module or external documentation
- **Examples**: Simple usage examples in inline docs where helpful

### External Documentation (Comprehensive)

Located in the `docs/` directory:

- **`lib.md`**: Crate-level overview included via `#[doc = include_str!()]` in `lib.rs`
- **`workflow.md`**: Workflow API documentation included via `#[doc = include_str!()]`
- **`quillmark.md`**: Quillmark engine documentation included via `#[doc = include_str!()]`

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
1. Create or update a `{module}.md` file in the `docs/` directory
1. Link the documentation file with `#[doc = include_str!("../docs/{module}.md")]`
1. Test with `cargo doc --no-deps` and `cargo test --doc`

### Documentation Structure Standard

All Quillmark crates follow this structure:

```
crate-name/
├── docs/
│   ├── overview.md        # Crate-level overview (included in lib.rs)
│   ├── {module}.md        # Module-level documentation (one per public module)
│   └── {DESIGN}.md        # Optional: Design documents and specifications
├── src/
│   ├── lib.rs             # #![doc = include_str!("../docs/overview.md")]
│   └── {module}.rs        # #![doc = include_str!("../docs/{module}.md")]
└── CONTRIBUTING.md        # This file
```

This structure is consistent across `quillmark-core`, `quillmark-typst`, and `quillmark` crates.

**Note**: The `quillmark` crate uses `lib.md` instead of `overview.md` for the crate-level documentation, but follows the same pattern otherwise.
