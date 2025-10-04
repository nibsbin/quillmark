# Contributing to Quillmark

## Documentation Strategy

- Use standard in-line Rust doc comments (`///`)
- Only create minimal examples for public APIs
- Err on the side of brevity

### Testing Documentation

- Run `cargo doc --no-deps` to build documentation for all crates
- Run `cargo doc -p quillmark -p quillmark-typst -p quillmark-core --no-deps` to build documentation for specific packages
- Run `cargo test --doc` to execute doctests from inline documentation
- Keep examples green to prevent documentation drift
- **Always check for warnings** - documentation warnings about broken links should be fixed immediately

### Design Documents

Design documents and comprehensive specifications are stored in `docs/` directories:

- Use `designs/` for detailed architectural documentation, specifications, and design rationale