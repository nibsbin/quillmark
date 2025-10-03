# Contributing to Quillmark

## Documentation Structure

All Quillmark crates use a hybrid documentation strategy to balance code iteration speed with comprehensive documentation:

### Inline Documentation (Minimal)

- **Public items**: Each public function/struct has a 1-2 line summary using `///`
- **Deep dive links**: Inline docs point to detailed module or external documentation
- **Examples**: Simple usage examples in inline docs where helpful

### External Documentation (Comprehensive)

Located in each crate's `docs/` directory:

#### quillmark-core

- **`overview.md`**: Crate-level overview included via `#![doc = include_str!()]` in `lib.rs`
- **`parse.md`**: Parsing module documentation included via `#[doc = include_str!()]`
- **`templating.md`**: Templating module documentation included via `#[doc = include_str!()]`
- **`backend.md`**: Backend trait documentation included via `#[doc = include_str!()]`
- **`errors.md`**: Error handling documentation included via `#[doc = include_str!()]`
- **`designs/API.md`**: Complete API reference with detailed examples
- **`designs/PARSE.md`**: Full Extended YAML Metadata Standard specification

#### quillmark-typst

- **`overview.md`**: Crate-level overview included via `#![doc = include_str!()]` in `lib.rs`
- **`compile.md`**: Compilation module documentation included via `#[doc = include_str!()]`
- **`convert.md`**: Conversion module documentation included via `#[doc = include_str!()]`
- **`designs/CONVERT_DESIGN.md`**: Full Markdown to Typst conversion specification and design notes

#### quillmark

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
1. Create or update a `{module}.md` file in the crate's `docs/` directory
1. Link the documentation file with `#[doc = include_str!("../docs/{module}.md")]`
1. Test with `cargo doc --no-deps` and `cargo test --doc`

### Documentation Structure Standard

All Quillmark crates follow this structure:

```
crate-name/
├── docs/
│   ├── overview.md (or lib.md)  # Crate-level overview (included in lib.rs)
│   ├── {module}.md              # Module-level documentation (one per public module)
│   └── designs/                 # Design documents and specifications
│       └── {DESIGN}.md
├── src/
│   ├── lib.rs                   # #![doc = include_str!("../docs/overview.md")]
│   └── {module}.rs              # #![doc = include_str!("../docs/{module}.md")]
└── ...
```

This structure is consistent across `quillmark-core`, `quillmark-typst`, and `quillmark` crates.

### Design Documents

Design documents and comprehensive specifications are stored in `docs/designs/` directories:

- Use `designs/` for detailed architectural documentation, specifications, and design rationale
- Module-level documentation in `docs/` should be concise and focused on usage
- Link from module docs to design docs when readers need deeper understanding
