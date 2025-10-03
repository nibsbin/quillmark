# Quillmark AI Agent Instructions

## Project Overview

Quillmark is a **template-first Markdown rendering system** built as a Rust workspace with three published crates (`quillmark-core`, `quillmark-typst`, `quillmark`) and one internal test crate (`quillmark-fixtures`). The system converts Markdown with YAML frontmatter into PDF/SVG using a **sealed engine API** backed by trait-based backends.

**Key Architecture**: Parse (Markdown + YAML) → Template (MiniJinja with backend filters) → Compile (backend-specific glue) → Artifacts (PDF/SVG/etc)

## Workspace Structure & Dependencies

```
quillmark/
├── quillmark-core/      # Core types, parsing, templating (foundation)
├── quillmark-typst/     # Typst backend (depends on core)
├── quillmark/           # Sealed engine API (depends on core + typst)
└── quillmark-fixtures/  # Test resources (NOT published to crates.io)
```

**Critical dependency order**: Always publish in order: `core` → `typst` → `quillmark`. The workspace uses **synchronized versioning** - all crates share the same version number.

## Documentation Convention (Hybrid Strategy)

This project uses a **distinctive documentation pattern** that differs from typical Rust projects:

- **Inline docs**: Minimal 1-2 line summaries on public items
- **External markdown**: Comprehensive docs in `docs/{module}.md` files included via `#[doc = include_str!("../docs/{file}.md")]`
- **Design docs**: Architecture/specs in `docs/designs/` and workspace `designs/` directories

**Intra-doc links in included markdown MUST use module-qualified paths** because the same file is included in both `lib.rs` (module scope) and `module.rs` (item scope):
- ✅ Correct: `` [`compile::compile_to_pdf()`] ``
- ❌ Wrong: `` [`compile_to_pdf`] `` (breaks in lib.rs)

Always run `cargo doc --no-deps` after doc changes to catch broken links.

## Build & Test Workflow

```bash
# Build entire workspace
cargo build --workspace --all-features

# Run all tests (including doc tests)
cargo test --workspace --all-features

# Build docs and check for warnings
cargo doc --no-deps --workspace --all-features

# Check specific crates
cargo check -p quillmark-core
cargo test -p quillmark-typst

# Run examples (uses fixtures)
cargo run --example appreciated_letter
```

**Test organization**:
- Unit tests: In-file `#[cfg(test)] mod tests`
- Integration tests: `tests/*.rs` (uses `common.rs` helper)
- Doc tests: In external `.md` files via `include_str!()`
- Examples: Uses `quillmark-fixtures` for test resources

## Extended YAML Metadata Standard (Non-Obvious)

The parser supports **inline metadata sections** with tag directives, not just frontmatter:

```markdown
---
title: Document Title
---
Main content.

---
!products
name: Widget
price: 19.99
---
Widget description.
```

This creates a `products` array with objects containing both metadata fields AND `body` content. Key rules:
- Tag directive `!attribute_name` must be on first line after opening `---`
- Metadata blocks must be **contiguous** (no blank lines within YAML)
- Opening `---` followed by blank line → horizontal rule (NOT metadata)
- Reserved field: `body` (use `BODY_FIELD` constant)
- Pattern: `[a-z_][a-z0-9_]*` for tag names

See `quillmark-core/docs/designs/PARSE.md` for full spec.

## Template System & Filter API

Templates use **MiniJinja** but backends access only the **stable filter API** in `quillmark_core::templating::filter_api`:

```rust
use quillmark_core::templating::filter_api::{State, Value, Kwargs, Error, ErrorKind};
```

**Common filters** (Typst backend):
- `String(default="...")`: Escapes quotes, handles `"none"` sentinel → unquoted Typst literal
- `Lines`: Array → JSON array of strings
- `Date`: Strict parsing → Typst `datetime()` constructor
- `Dict`: YAML object → `json(bytes("..."))` for Typst
- `Content`: Markdown → Typst markup via `eval(..., mode: "markup")`
- `Asset`: Dynamic asset filename → `"assets/DYNAMIC_ASSET__{filename}"`

**Dynamic assets**: Added via `Workflow.with_asset(name, bytes)`, prefixed with `DYNAMIC_ASSET__` to avoid collisions.

## Backend Implementation Pattern

Backends implement the `Backend` trait from `quillmark-core`:

```rust
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn glue_type(&self) -> &'static str;
    fn register_filters(&self, glue: &mut Glue);
    fn compile(&self, glue_content: &str, quill: &Quill, opts: &RenderOptions) 
        -> Result<Vec<Artifact>, RenderError>;
}
```

**Typst-specific gotchas**:
- Markdown→Typst: Escape `* _ # $ @ [ ] < > \`` in text content
- Lists: Convert markdown `-` to Typst `+` for unordered lists
- `QuillWorld`: Implements Typst `World` for virtual file system
- Packages: Load from `packages/` with `typst.toml` metadata (namespace, version, entrypoint)
- Fonts: Search order: `assets/fonts/` → `assets/` → system fonts (via `typst-kit::fonts::FontSearcher`)
- Virtual paths: MUST use forward slashes, construct manually (not `join()`)

## Error Handling (Structured Diagnostics)

**Never stringify errors prematurely**. Use the `Diagnostic` type everywhere:

```rust
pub struct Diagnostic {
    pub severity: Severity,        // Error, Warning, Note
    pub code: Option<String>,      // e.g., "minijinja::UndefinedError"
    pub message: String,
    pub primary: Option<Location>, // File, line, column
    pub related: Vec<Location>,
    pub hint: Option<String>,
}
```

Map external errors (MiniJinja, Typst) to `Diagnostic` to preserve line/column context. Backends return `RenderError::CompilationFailed(count, Vec<Diagnostic>)` for multi-error reporting.

## Version Management & Release

Uses **cargo-release** with workspace synchronization:

```bash
# Preview release (dry-run)
cargo release minor

# Execute release
cargo release minor --execute
```

Configuration in `release.toml`. All workspace crates (except fixtures) share the same version and are released together. Follow [Keep a Changelog](https://keepachangelog.com/) format in `CHANGELOG.md`.

**IMPORTANT**: Version bumps must be synchronized across all `Cargo.toml` files. The `release.toml` handles this automatically via `dependent-version = "upgrade"`.

## Package & Asset Management

Quill templates follow this structure:

```
quill-name/
├── Quill.toml           # metadata (backend, name, glue file, external packages)
├── glue.typ             # Template file (extension matches backend)
├── packages/            # Embedded packages with typst.toml
│   └── my-pkg/
│       ├── typst.toml   # namespace, name, version, entrypoint
│       └── src/lib.typ
└── assets/              # Fonts, images, data
    └── fonts/           # .ttf, .otf, .woff, .woff2
```

**Quill.toml external packages** (optional):

```toml
[typst]
packages = ["@preview/bubble:0.2.2"]
```

External packages **dominate** (override) embedded packages. Package loading algorithm:
1. Download external packages from Typst registry
2. Load embedded packages from `packages/`
3. Register in virtual file system with preserved directory structure

## Common Pitfalls

1. **Broken doc links**: Always use module-qualified paths in included markdown
2. **Virtual paths**: Use forward slashes, don't use `Path::join()` for Typst paths
3. **Character escaping**: Typst has many reserved chars (`*_#$@[]<>\``) - escape in text
4. **Metadata blocks**: Must be contiguous, blank line after `---` → horizontal rule
5. **Test fixtures**: Never hardcode paths - use `quillmark_fixtures::resource_path()`
6. **Version sync**: All publishable crates must have identical versions
7. **Filter API**: Import only from `filter_api` module, never directly from MiniJinja

## Testing Strategy

- **Unit tests**: Focus on individual functions, use in-file `#[cfg(test)]` modules
- **Integration tests**: Test workflows end-to-end, use `tests/common.rs` helper
- **Doc tests**: Keep examples in external `.md` files, must compile and pass
- **Fixtures**: Centralized test resources in `quillmark-fixtures/resources/`

Run `cargo test --workspace` frequently. Doc test failures indicate either broken examples or broken intra-doc links.

## Key Files to Reference

- `designs/DESIGN.md`: Complete architecture and design decisions
- `designs/CI_CD.md`: Build, test, and release workflows
- `CONTRIBUTING.md`: Documentation standards and patterns
- `quillmark-core/docs/designs/PARSE.md`: Extended YAML Metadata Standard
- `release.toml`: Cargo-release configuration
- `Cargo.toml` (workspace): Dependency versions and workspace config
