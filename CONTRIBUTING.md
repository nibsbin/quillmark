# Contributing to Quillmark

Thank you for your interest in contributing to Quillmark!

## Documentation Strategy

Quillmark uses a **hybrid inline/external documentation approach** to balance code iteration speed with comprehensive documentation:

### Inline Documentation (Minimal)

- **Public items**: Each public type, function, or method has a brief 1-2 line summary using `///`
- **Cross-references**: Inline docs point to deeper documentation with `/// See [module docs](self) for examples`
- **IDE support**: Brief summaries ensure IDE hover and completion aren't empty

### External Documentation (Detailed)

- **Location**: Rich documentation lives in Markdown files under `quillmark/docs/`
- **Inclusion**: External docs are attached using `#[doc = include_str!("../docs/filename.md")]`
- **Content**: External docs include:
  - Comprehensive examples
  - Usage patterns and best practices
  - Detailed explanations of behavior
  - Edge cases and gotchas

### Doctest Control

- Use doctest attributes to control test execution:
  - `no_run` - Compile but don't execute (for examples requiring filesystem)
  - `ignore` - Skip compilation and execution
  - `should_panic` - Expect the code to panic
  - `compile_fail` - Expect compilation to fail

### Examples

- **Location**: End-to-end demos live in `quillmark/examples/`
- **Purpose**: Demonstrate real-world usage patterns
- **Testing**: Run `cargo test --examples` to ensure examples stay green

## Documentation Files

Current external documentation files:

- `quillmark/docs/lib.md` - Crate-level overview and quick start
- `quillmark/docs/quillmark.md` - Quillmark engine documentation
- `quillmark/docs/workflow.md` - Workflow API documentation  
- `quillmark/docs/quillref.md` - QuillRef enum documentation

## Testing Documentation

```bash
# Run all doctests (both inline and from external Markdown)
cargo test --doc

# Build documentation locally
cargo doc --open

# Check for broken links and warnings
cargo doc --no-deps 2>&1 | grep warning
```

## Documentation Guidelines

1. **Inline summaries**: Keep to 1-2 lines, focus on "what" not "how"
2. **External details**: Put comprehensive examples and explanations in Markdown files
3. **Keep examples working**: Update examples when APIs change
4. **Test your changes**: Run `cargo test --doc` before submitting
5. **IDE-friendly**: Ensure inline docs provide enough context for code completion

## Questions?

If you're unsure where documentation should live, err on the side of external Markdown files - they're easier to maintain and update without touching code.
