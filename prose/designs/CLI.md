# CLI Tool Design for Quillmark

> **Status**: Design Phase
> **Package Name**: `quillmark-cli`
> **Target**: Cross-platform CLI binary

> **For implementation details, see**: `bindings/quillmark-cli/src/`

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

The CLI follows a subcommand pattern for extensibility:

```
quillmark <SUBCOMMAND> [OPTIONS]
```

**Available Subcommands:**
- `render` - Render markdown file to output format
- `info` - Display quill template information
- `list` - List available quills
- `validate` - Validate markdown against quill schema

---

### Subcommand: `render`

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
- `--glue-only` - Only process glue template, don't render final output
- `--verbose` - Show detailed processing information
- `--quiet` - Suppress all non-error output

**Behavior:**
1. Parse markdown file and extract YAML frontmatter
2. Determine quill from QUILL field or --quill option
3. Load quill from filesystem
4. Create workflow and render to specified format
5. Write output to file or stdout

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

# Only process glue template
quillmark render memo.md --glue-only -o glue_output.typ

# Pipe output for further processing
quillmark render memo.md --stdout > output.pdf
```

---

### Subcommand: `info`

Display detailed information about a quill template.

**Signature:**
```
quillmark info <QUILL_PATH>
```

**Required Arguments:**
- `<QUILL_PATH>` - Path to quill directory

**Output Fields:**
- Name, backend, metadata
- Supported output formats
- Field schema definitions
- Example markdown (if available)

**Example:**
```bash
quillmark info ./quills/usaf_memo
```

---

### Subcommand: `list`

List quills from a directory.

**Signature:**
```
quillmark list [DIRECTORY]
```

**Optional Arguments:**
- `[DIRECTORY]` - Directory to search for quills (default: current directory)

**Options:**
- `-r, --recursive` - Search subdirectories recursively

**Example:**
```bash
quillmark list ./quills
quillmark list --recursive
```

---

### Subcommand: `validate`

Validate markdown file against quill schema.

**Signature:**
```
quillmark validate <MARKDOWN_FILE> [OPTIONS]
```

**Required Arguments:**
- `<MARKDOWN_FILE>` - Path to markdown file

**Options:**
- `-q, --quill <PATH>` - Path to quill directory (overrides QUILL field)

**Output:**
- Validation success/failure
- Schema violations with field names and expected types
- Missing required fields

**Example:**
```bash
quillmark validate memo.md
quillmark validate memo.md --quill ./quills/usaf_memo
```

---

## Implementation Architecture

**Project Structure:**
```
bindings/quillmark-cli/
├── Cargo.toml           # Package manifest
├── README.md            # Usage documentation
└── src/
    ├── main.rs          # Entry point, CLI parsing
    ├── commands/
    │   ├── mod.rs       # Command module exports
    │   ├── render.rs    # Render command implementation
    │   ├── info.rs      # Info command implementation
    │   ├── list.rs      # List command implementation
    │   └── validate.rs  # Validate command implementation
    ├── output.rs        # Output formatting and file I/O
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
cd bindings/quillmark-cli
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
- Verify output byte-for-byte against expected results
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
