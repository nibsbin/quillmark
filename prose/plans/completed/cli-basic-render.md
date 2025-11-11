# Plan: Basic CLI Rendering Implementation

> **Status**: In Progress
> **Design Reference**: `prose/designs/CLI.md`
> **Target**: Minimal viable CLI with render command for usaf_memo.md

---

## Objective

Implement a basic `quillmark-cli` binary that can render markdown files to PDF using the Quillmark engine. Initial scope focuses on the `render` subcommand with essential options.

---

## Current State

- No CLI binding exists in the workspace
- Python and WASM bindings serve as implementation references
- Core `quillmark` library provides all necessary rendering functionality
- Example markdown files exist in `quillmark-fixtures/resources/tonguetoquill-collection/quills/`

---

## Desired State

A functional CLI tool that can:
1. Parse command-line arguments for the render subcommand
2. Read markdown files from filesystem
3. Load quill templates from specified paths
4. Render markdown to PDF format
5. Write output to file or stdout
6. Display clear error messages

**Success Criteria:**
```bash
cd bindings/quillmark-cli
cargo build --release
./target/release/quillmark render \
  ../../quillmark-fixtures/resources/tonguetoquill-collection/quills/usaf_memo/usaf_memo.md \
  --quill ../../quillmark-fixtures/resources/tonguetoquill-collection/quills/usaf_memo \
  -o usaf_memo_output.pdf
# Result: usaf_memo_output.pdf created successfully
```

---

## Implementation Steps

### Step 1: Project Setup

**Create workspace structure:**
- Create `bindings/quillmark-cli/` directory
- Create `Cargo.toml` with package configuration
- Add to workspace members in root `Cargo.toml`
- Create `src/` directory with modular structure

**Package configuration:**
- Binary crate with `main.rs` entry point
- Dependencies: clap, quillmark, quillmark-core, anyhow
- Release profile optimizations for binary size
- Metadata: description, license, authors

**Files to create:**
- `bindings/quillmark-cli/Cargo.toml`
- `bindings/quillmark-cli/README.md`
- `bindings/quillmark-cli/src/main.rs`
- `bindings/quillmark-cli/src/commands/mod.rs`
- `bindings/quillmark-cli/src/commands/render.rs`
- `bindings/quillmark-cli/src/output.rs`
- `bindings/quillmark-cli/src/errors.rs`

### Step 2: CLI Argument Parsing

**Setup clap with derive macros:**
- Define root CLI structure with subcommands
- Implement `RenderArgs` struct with options
- Handle argument validation

**Arguments for render command:**
- Required: markdown file path
- Optional: --quill, --output, --format, --stdout, --glue-only
- Global flags: --verbose, --quiet

**Validation:**
- Check markdown file exists
- Validate format is one of: pdf, svg, txt
- Ensure output path is writable
- Verify quill path exists if provided

### Step 3: Core Render Command

**Implement render workflow:**
1. Read markdown file from filesystem
2. Parse markdown with `ParsedDocument::from_markdown()`
3. Determine quill source (frontmatter QUILL field or --quill flag)
4. Load quill with `Quill::from_path()`
5. Create engine and workflow
6. Execute render with specified format
7. Handle glue-only mode if requested

**Error handling:**
- Wrap all operations in Result types
- Convert library errors to user-friendly messages
- Include file paths and line numbers in diagnostics
- Differentiate between user errors and internal errors

### Step 4: Output Management

**File output:**
- Determine output filename (explicit --output or derive from input)
- Create parent directories if needed
- Write bytes to file
- Display success message with output path

**Stdout output:**
- Check for --stdout flag
- Write bytes directly to stdout
- Suppress informational messages when using stdout
- Handle binary data correctly in stdout mode

**Glue-only mode:**
- Process through glue template only
- Output intermediate Typst template
- Useful for debugging and template development

### Step 5: Integration Testing

**Test scenarios:**
- Render usaf_memo.md with explicit quill path
- Render with QUILL field in frontmatter
- Test different output formats (pdf, svg, txt)
- Test --stdout mode
- Test --glue-only mode
- Test error conditions (missing file, invalid markdown, etc.)

**Verification:**
- Compare output files with expected results
- Ensure errors display helpful messages
- Verify exit codes (0 for success, non-zero for errors)

### Step 6: Documentation

**README.md:**
- Installation instructions
- Basic usage examples
- Common use cases
- Troubleshooting guide

**Cargo.toml metadata:**
- Description and keywords for crates.io
- License and repository links
- Version alignment with workspace

---

## Implementation Notes

**Leverage existing patterns:**
- Mirror Python binding error handling approach
- Use similar workflow creation pattern from examples
- Reference demo function in `quillmark/tests/common.rs`

**Keep it simple:**
- Phase 1 focuses only on render command
- Other subcommands (info, list, validate) deferred to Phase 2
- Minimal dependencies, no fancy terminal UI
- Standard error output, no colored output yet

**Cross-reference points:**
- `prose/designs/CLI.md` - Full design specification
- `bindings/quillmark-python/` - Error handling patterns
- `quillmark/tests/common.rs` - Workflow usage example
- `quillmark/examples/usaf_memo.rs` - Simple demo reference

**Testing approach:**
- Use existing fixture files from `quillmark-fixtures`
- No need to create new test markdown files
- Focus on command-line argument variations
- Manual testing initially, automated tests later

---

## Non-Scope for Phase 1

The following features are documented in the design but deferred:

- `info` subcommand - future enhancement
- `list` subcommand - future enhancement
- `validate` subcommand - future enhancement
- Watch mode - Phase 2
- Configuration file - Phase 2
- Colored/pretty output - Phase 2
- Progress bars - Phase 2

---

## Success Validation

**Functional Tests:**
```bash
# Basic render
quillmark render usaf_memo.md --quill path/to/quill

# Different formats
quillmark render memo.md --format svg
quillmark render memo.md --format txt

# Output control
quillmark render memo.md -o custom.pdf
quillmark render memo.md --stdout > output.pdf

# Glue only
quillmark render memo.md --glue-only -o glue.typ
```

**Error Handling Tests:**
```bash
# Missing file
quillmark render nonexistent.md
# Expected: Clear error about missing file

# Invalid markdown
quillmark render invalid.md
# Expected: Parse error with line numbers

# Missing quill
quillmark render memo.md --quill /bad/path
# Expected: Error about missing quill directory
```

**Build Verification:**
```bash
cd bindings/quillmark-cli
cargo build --release
cargo test
file target/release/quillmark  # Verify it's a binary
```

---

## Risks and Mitigations

**Risk**: Binary size too large
- **Mitigation**: Use release optimizations, strip symbols, consider LTO

**Risk**: Path handling differs across platforms
- **Mitigation**: Use std::path::PathBuf consistently, test on multiple platforms

**Risk**: Stdout binary output issues on Windows
- **Mitigation**: Research proper binary stdout on Windows, may need platform-specific code

**Risk**: Error messages not user-friendly enough
- **Mitigation**: Test with real users, iterate on error message clarity

---

## Completion Criteria

This plan is complete when:

1. ✅ `bindings/quillmark-cli` project exists in workspace
2. ✅ `cargo build --release` succeeds
3. ✅ Can render `usaf_memo.md` to PDF with explicit quill path
4. ✅ Output file is created and valid
5. ✅ Error messages are clear and helpful
6. ✅ Basic README.md with usage examples exists
7. ✅ Plan moved to `prose/plans/completed/`
