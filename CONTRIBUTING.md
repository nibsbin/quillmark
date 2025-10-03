# Contributing to Quillmark

Thank you for your interest in contributing to Quillmark!

## Documentation Strategy

Quillmark uses a **hybrid inline/external documentation approach** to keep code iteration fast while delivering comprehensive documentation.

### Documentation Structure

#### Inline Documentation (Minimal)
- **1-2 line summaries** on each public item using `///`
- **References to deeper docs**: `/// See [module docs](self) for examples.`
- Ensures IDE hover and completion provide helpful context

#### Module/Crate Documentation (Rich Markdown)
- **Location**: `quillmark-core/docs/` directory
- **Included via**: `#[doc = include_str!("../docs/filename.md")]`
- **Content**: Comprehensive examples, error documentation, usage patterns
- **Doctest control**: Use `no_run`, `ignore`, `should_panic` attributes as needed

#### Key Files
- **`docs/overview.md`**: Crate-level documentation with quick start
- **`docs/parsing.md`**: Parsing module deep dive
- **`docs/templating.md`**: Templating module documentation
- **`docs/backend.md`**: Backend trait implementation guide
- **`docs/errors.md`**: Error handling patterns and examples
- **`API.md`**: Complete API reference (comprehensive external docs)
- **`PARSE.md`**: Extended YAML Metadata Standard specification

### Adding New Documentation

When adding or modifying public APIs:

1. **Add a 1-2 line summary** in the source file:
   ```rust
   /// Parse markdown with frontmatter. See [module docs](crate::parse) for examples.
   pub fn decompose(markdown: &str) -> Result<ParsedDocument, Error>
   ```

2. **Add detailed examples** to the relevant `docs/*.md` file:
   - Use proper code fence attributes (`rust`, `rust,no_run`, `rust,ignore`)
   - Include working examples that compile (or are marked `no_run`)
   - Add hidden setup code with `#` prefix for context

3. **Test your documentation**:
   ```bash
   # Run doc tests
   cargo test --package quillmark-core --doc
   
   # Build docs with strict mode
   RUSTDOCFLAGS="-D warnings -D missing-docs" cargo doc --no-deps
   ```

### Testing & CI

- **`cargo test`** runs doctests from included Markdown files and README
- **Doc examples** in `docs/*.md` are tested as part of the build
- **Keep examples green** to prevent documentation drift

### Discoverability

- Inline one-liners ensure IDE hover/completion isn't empty
- Rich module docs accessible via `cargo doc --open`
- External `API.md` provides searchable reference
- Cross-references between docs help navigation

## Code Style

- Follow existing code formatting (use `rustfmt`)
- Add tests for new functionality
- Update documentation when changing public APIs
- Run `cargo clippy` to catch common issues

## Pull Request Process

1. Ensure all tests pass (`cargo test`)
2. Update documentation as needed
3. Add a clear description of changes
4. Reference any related issues

## Questions?

Feel free to open an issue for questions or discussions!
