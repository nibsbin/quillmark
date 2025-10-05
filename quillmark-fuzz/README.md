# Quillmark Fuzzing Tests

This crate contains comprehensive property-based fuzzing tests for Quillmark using the `proptest` framework. These tests validate the security of Quillmark's escaping functions, markdown parser, and filter inputs.

**Note:** This crate is not published to crates.io and is only used for internal testing.

## Test Coverage

### Escaping Function Security (`convert_fuzz`)

Tests for `escape_string` and `escape_markup` functions in `quillmark-typst`:
- Injection attack vectors with quotes and eval patterns
- Control character handling (null bytes, ASCII control chars)
- Property tests ensuring no unescaped quotes can break out of string context
- Dangerous patterns like `\"); eval(...)` that could enable code injection
- Validation that all Typst special characters are properly escaped
- Backslash handling to prevent double-escaping vulnerabilities

### Markdown Parser Fuzzing (`convert_fuzz`)

DoS attack prevention:
- Deeply nested structures (blockquotes, lists up to 20 levels deep)
- Large input handling (up to 10,000 characters)
- Ensures parser doesn't panic on malicious inputs

### Filter Input Fuzzing (`filter_fuzz`)

Tests for the `inject_json` helper function:
- Validates proper escaping in JSON injection contexts
- Tests dangerous character combinations (`\`, `"`, control chars)
- Ensures no unescaped quotes that could break out of `json(bytes("..."))` wrapper
- Tests Unicode handling and various input sizes

### YAML Parser Fuzzing (`parse_fuzz`)

YAML frontmatter security:
- Tests malformed YAML handling
- Validates tag directive parsing with random inputs
- Tests nested YAML structures for stability
- Unicode and special character handling

## Running Tests

```bash
# Run all fuzzing tests
cargo test --package quillmark-fuzz --all-features

# Run specific test module
cargo test --package quillmark-fuzz --test convert_fuzz
cargo test --package quillmark-fuzz --test filter_fuzz
cargo test --package quillmark-fuzz --test parse_fuzz
```

## Security Properties Validated

The fuzzing tests validate critical security properties:

1. **No injection vulnerabilities**: Quotes are always escaped in string contexts
2. **Control character safety**: ASCII control characters are properly escaped as `\u{...}`
3. **Backslash handling**: Backslashes are escaped first to prevent double-escaping
4. **DoS resistance**: Parser handles deeply nested and large inputs without panicking
5. **Unicode safety**: Handles arbitrary Unicode input without crashes

## Architecture

The fuzzing tests are organized into three modules:

- `convert_fuzz.rs` - Tests for markdown to Typst conversion and escaping functions
- `filter_fuzz.rs` - Tests for filter input validation and injection safety
- `parse_fuzz.rs` - Tests for YAML frontmatter and markdown parsing

All fuzzing tests use `proptest` for property-based testing, which generates random inputs to validate that security properties hold across a wide range of inputs.

## Contributing

When adding new features to Quillmark, consider adding corresponding fuzzing tests to this crate to ensure security properties are maintained.

## References

- See `designs/SECURITY_REC.md` in the main repository for detailed security recommendations
- [proptest documentation](https://docs.rs/proptest/) for property-based testing guidelines
