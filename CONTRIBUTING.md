# Contributing to Quillmark

## Documentation Structure

All Quillmark crates use inline Rust documentation to keep code and documentation together:

### Inline Documentation

- **Crate-level docs**: Use `//!` at the top of `lib.rs` with comprehensive overview, examples, and architecture
- **Module-level docs**: Use `//!` at the top of each module file with overview, key types, and examples
- **Public items**: Each public function/struct/trait has detailed doc comments using `///`
- **Examples**: Include code examples in doc comments - they serve as both documentation and tests
- **Cross-references**: Use intra-doc links like `[`TypeName`]`, `[`function_name()`]`, `[`module::item`]`

### Design Documentation

- **External design docs only**: Keep high-level design documents in `docs/designs/` for architectural decisions and specifications
- **Examples**: API.md, PARSE.md, CONVERT.md, ERROR.md, DESIGN.md
- **Purpose**: Detailed specifications, rationale, and design decisions that don't fit well in inline comments
- **Link from code**: Reference design docs from inline documentation using GitHub URLs

### Testing Documentation

- Run `cargo doc --no-deps` to build documentation for all crates
- Run `cargo doc -p quillmark -p quillmark-typst -p quillmark-core --no-deps` to build documentation for specific packages
- Run `cargo test --doc` to execute doctests from inline documentation
- Keep examples green to prevent documentation drift
- **Always check for warnings** - documentation warnings about broken links should be fixed immediately

### Discoverability

- Inline documentation ensures IDE hover/completion shows useful information
- Full documentation visible without switching files
- Design docs provide deep dives into complex topics

## Adding Documentation

When adding new public APIs:

1. Add comprehensive doc comments using `///` for items and `//!` for modules
2. Include examples in doc comments (these become doctests)
3. Use intra-doc links to cross-reference types: `[`TypeName`]`, `[`module::function()`]`
4. Link to design documents for complex specifications
5. Test with `cargo doc --no-deps` and `cargo test --doc`

### Documentation Structure Standard

All Quillmark crates follow this structure:

```
crate-name/
├── docs/
│   └── designs/                   # Design documents and specifications only
│       └── {DESIGN}.md
└── src/
    ├── lib.rs                     # //! Crate-level documentation
    └── {module}.rs                # //! Module-level documentation
```

This structure is consistent across `quillmark-core`, `quillmark-typst`, and `quillmark` crates.

### Design Documents

Design documents and comprehensive specifications are stored in `docs/designs/` directories:

- Use `designs/` for detailed architectural documentation, specifications, and design rationale
- Examples: API.md (comprehensive API reference), PARSE.md (parsing specification), CONVERT.md (conversion details)
- Link from inline docs to design docs when readers need deeper understanding
- Use GitHub URLs for stable links: `https://github.com/nibsbin/quillmark/blob/main/quillmark-core/docs/designs/API.md`

### Example Documentation Pattern

```rust
//! # Module Name
//!
//! Brief module overview.
//!
//! ## Overview
//!
//! Detailed description of module purpose and functionality.
//!
//! ## Examples
//!
//! ```
//! use crate_name::ModuleName;
//!
//! let example = ModuleName::new();
//! ```
//!
//! ## See Also
//!
//! - [Design Doc](https://github.com/nibsbin/quillmark/blob/main/docs/designs/DESIGN.md)

/// Brief description of function/type.
///
/// More detailed explanation of behavior, parameters, and return values.
///
/// # Examples
///
/// ```
/// use crate_name::function_name;
///
/// let result = function_name(42);
/// assert_eq!(result, 42);
/// ```
pub fn function_name(param: i32) -> i32 {
    param
}
```
