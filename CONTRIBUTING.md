# Contributing to Quillmark

## Documentation Structure

All Quillmark crates use a hybrid documentation strategy to balance code iteration speed with comprehensive documentation:

### Inline Documentation (Minimal)

- **Public items**: Each public function/struct has a 1-2 line summary using `///`
- **Deep dive links**: Inline docs point to detailed module or external documentation
- **Examples**: Simple usage examples in inline docs where helpful

### External Documentation

- Include a `lib.md` for crate-level overview
- Include a `{module}.md` for each public module with:
  - Overview of the module's purpose and functionality
  - Key functions and types
  - Quick examples
- Include design documents in `docs/designs/` for complex topics and specifications
- Link external docs using `#[doc = include_str!("../docs/{file}.md")]`

### Testing Documentation

- Run `cargo doc --no-deps` to build documentation for all crates
- Run `cargo doc -p quillmark -p quillmark-typst -p quillmark-core --no-deps` to build documentation for specific packages
- Run `cargo test` to execute doctests from both inline and included Markdown
- Keep examples green to prevent documentation drift
- **Always check for warnings** - documentation warnings about broken links should be fixed immediately

### Discoverability

- Inline one-liners ensure IDE hover/completion shows useful information
- Module docs provide quick examples and links to deeper documentation
- External docs provide comprehensive details for complex topics

## Adding Documentation

When adding new public APIs:

1. Add a 1-2 line summary to the item with `///`
1. Create or update a `{module}.md` file in the crate's `docs/` directory
1. Link the documentation file with `#[doc = include_str!("../docs/{module}.md")]`
1. Test with `cargo doc --no-deps` and `cargo test --doc`

### Intra-Doc Links in Included Markdown

When using `#[doc = include_str!()]` to include markdown files, be aware of the scope context for intra-doc links:

**Problem**: Markdown files included on module declarations (in `lib.rs`) have different scope than when included inside the module itself.

**Example**:
```rust
// In lib.rs
#[doc = include_str!("../docs/compile.md")]
pub mod compile;

// In compile.rs
#![doc = include_str!("../docs/compile.md")]
```

The same `compile.md` is used in both contexts, but:
- When attached to `pub mod compile` in `lib.rs`, functions are NOT in direct scope
- When attached inside `compile.rs`, functions ARE in direct scope

**Solution**: Use module-qualified paths in the markdown files:
- ✅ Correct: `` [`compile::compile_to_pdf()`] ``
- ✅ Correct: `` [`convert::mark_to_typst()`] ``
- ❌ Wrong: `` [`compile_to_pdf`] `` (breaks when used in `lib.rs`)

This ensures links work in both contexts. Always verify with `cargo doc --no-deps` to catch broken links.

### Documentation Structure Standard

All Quillmark crates follow this structure:

```
crate-name/
├── docs/
│   ├── lib.md                    # Crate-level overview (included in lib.rs)
│   ├── {module}.md                # Module-level documentation (one per public module)
│   └── designs/                   # Design documents and specifications
│       └── {DESIGN}.md
├── src/
│   ├── lib.rs                     # #![doc = include_str!("../docs/lib.md")]
│   └── {module}.rs                # #![doc = include_str!("../docs/{module}.md")]
└── ...
```

This structure needs to be consistent across `quillmark-core`, `quillmark-typst`, and `quillmark` crates.

### Design Documents

Design documents and comprehensive specifications are stored in `docs/designs/` directories:

- Use `designs/` for detailed architectural documentation, specifications, and design rationale
- Module-level documentation in `docs/` should be concise and focused on usage
- Link from module docs to design docs when readers need deeper understanding
