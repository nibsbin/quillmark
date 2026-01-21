# CLI Tool Design for Quillmark

> **Status**: Implemented
> **Package Name**: `quillmark-cli`
> **Target**: Cross-platform CLI binary

> **For implementation details, see**: `crates/bindings/cli/src/`

## Overview

This document outlines the design for `quillmark-cli`, a command-line interface that exposes the Quillmark rendering engine as a standalone executable for terminal workflows.

**Design Goals:**
- Provide simple, intuitive command-line interface for Markdown rendering
- Support batch processing and scripting workflows
- Enable quick prototyping and testing of quill templates
- Cross-platform distribution via single binary executable
- Mirror core functionality of the Rust API where applicable

**Non-Goals:**
- Interactive TUI/REPL interface (v1.0)
- Web server mode or daemon functionality (v1.0)
- Custom backend plugin system (v1.0)
- Built-in template editing or creation tools (v1.0)

---

## Command-Line Interface Design

### Command Structure

The CLI currently provides a direct rendering command:

```
quillmark render [OPTIONS] <MARKDOWN_FILE>
```

**Current Implementation:**
- `render` - Render markdown file to output format

**Future Subcommands** (not yet implemented):
- `info` - Display quill template information
- `list` - List available quills
- `validate` - Validate markdown against quill schema
- `schema` - Retrieve the Quill's field schema

---

### Command: `render`

Primary command for rendering markdown files.

**Signature:**
```
quillmark render <MARKDOWN_FILE> [OPTIONS]
```

**Required Arguments:**
- `<MARKDOWN_FILE>` - Path to markdown file with YAML frontmatter

**Options:**
- `-q, --quill <PATH>` - Path to quill directory (overrides QUILL frontmatter field)
- `-o, --output <FILE>` - Output file path (default: derived from input filename)
- `-f, --format <FORMAT>` - Output format: pdf, svg, txt (default: pdf)
- `--stdout` - Write output to stdout instead of file
- `--output-data <FILE>` - Write compiled JSON data (post-coercion/defaults/transform_fields) to file
- `--verbose` - Show detailed processing information
- `--quiet` - Suppress all non-error output

**Behavior:**
1. Parse markdown file and extract YAML frontmatter
2. Determine quill from QUILL field or --quill option
3. Load quill from filesystem
4. Create workflow and render to specified format
5. Write output to file or stdout
6. Optionally emit compiled JSON data when `--output-data` is provided

**Error Handling:**
- Invalid markdown: display parse errors with line numbers
- Missing quill: suggest available quills or provide path guidance
- Template errors: show compilation diagnostics from backend
- File I/O errors: clear error messages with paths

**Examples:**
```bash
# Render using QUILL field from frontmatter
quillmark render usaf_memo.md

# Override quill location
quillmark render memo.md --quill ./quills/usaf_memo

# Render to SVG format
quillmark render memo.md --format svg

# Output to specific file
quillmark render memo.md -o output/final.pdf

# Pipe output for further processing
quillmark render memo.md --stdout > output.pdf

# Emit compiled data for inspection
quillmark render memo.md --output-data data.json
```

---

### Command: `schema`

Retrieve the Quill's field schema as JSON.

**Signature:**
```
quillmark schema <QUILL_PATH> [OPTIONS]
```

**Required Arguments:**
- `<QUILL_PATH>` - Path to quill directory

**Options:**
- `-o, --output <FILE>` - Output file path (default: stdout)

**Behavior:**
1. Load quill from filesystem
2. Extract schema from quill configuration
3. Output schema as JSON to stdout or file

**Examples:**
```bash
# Print schema to stdout
quillmark schema ./quills/usaf_memo

# Save schema to file
quillmark schema ./quills/usaf_memo -o schema.json
```

---

### Future Commands

The following commands are planned but not yet implemented:

#### `info` - Display Quill Information
Display metadata, supported formats, and field schemas for a quill template.

#### `list` - List Available Quills
Discover and list quills from a directory or directory tree.

#### `validate` - Validate Markdown
Validate markdown frontmatter against a quill's JSON schema.

See "Future Enhancements" section for more details on planned features.

---

## Implementation Architecture

**Project Structure:**
```
crates/bindings/cli/
├── Cargo.toml           # Package manifest
├── README.md            # Usage documentation
└── src/
    ├── main.rs          # Entry point, CLI parsing
    ├── commands/
    │   ├── mod.rs       # Command module exports
    │   └── render.rs    # Render command implementation
    ├── output.rs        # Output path derivation and file writing
    └── errors.rs        # Error handling and display
```

**Dependencies:**
- `clap` - Argument parsing with derive macros
- `quillmark` - Core rendering engine
- `quillmark-core` - Types and traits
- `anyhow` - Error handling
- `serde_json` - JSON serialization for info output

**Binary Target:**
- Single executable: `quillmark`
- Optimized for release builds (size and speed)
- Static linking where possible for portability

---

## Build Configuration

**Cargo Profile:**
- Release builds use size optimization
- Strip debug symbols for smaller binary
- Consider `cargo-dist` for distribution

**Cross-Compilation:**
- Support Linux (x86_64, aarch64)
- Support macOS (x86_64, aarch64)
- Support Windows (x86_64)

**Installation Methods:**
```bash
# Via cargo
cargo install quillmark-cli

# From source
cd bindings/quillmark-cli
cargo build --release
cp target/release/quillmark /usr/local/bin/
```

---

## Development Workflow

**Local Development:**
```bash
cd crates/bindings/cli
cargo build
cargo run -- render example.md
```

**Testing:**
```bash
cargo test
cargo test --release
```

**Integration Testing:**
- Test against example markdown files from fixtures
- Verify output against expected results
- Test error conditions (missing files, invalid markdown, etc.)

---

## Distribution & Packaging

**GitHub Releases:**
- Automated builds for multiple platforms
- Checksum files for verification
- Install script for Unix-like systems

**Package Managers:**
- Cargo: `cargo install quillmark-cli`
- Homebrew: Future consideration
- apt/yum: Future consideration

**Binary Naming:**
- Executable: `quillmark`
- No platform-specific suffixes in final binary name

---

## Future Enhancements

**Phase 2 Features:**
- Watch mode: automatically re-render on file changes
- Batch processing: render multiple files in one command
- Template variable overrides via CLI flags
- Configuration file support for defaults

**Phase 3 Features:**
- Interactive mode with prompts
- Template creation wizard
- Built-in template marketplace/registry integration
- Server mode for HTTP API access

---

## Design Decisions

**Why separate CLI binding?**
- Different optimization profile (binary size vs library)
- Different dependency requirements (clap, colored output, etc.)
- Allows independent versioning and release cycle
- Clearer separation of concerns

**Why subcommands instead of flags?**
- Better extensibility for future features
- Clearer help documentation
- Follows common CLI patterns (git, cargo, docker)

**Why not embed default quills?**
- Keeps binary size minimal
- Allows users to use any quill directory
- Avoids version skew between embedded templates and filesystem

**Why stdout support?**
- Enables Unix pipeline workflows
- Allows integration with other tools
- Useful for testing and debugging
